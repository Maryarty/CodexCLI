//! Playing a scenario against one binary, and against both.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::backend::{self, Backend};
use super::compare::{Finding, Report, compare, side_by_side};
use super::normalize::Normalize;
use super::pty::{self, Pty, Shot};
use super::script::{Scenario, Step};

/// No output for this long, outside a synchronized update, is a quiet screen.
const QUIET: Duration = Duration::from_millis(150);
/// A screen that never gets quiet (a spinner) is shot after this long.
const QUIET_MAX: Duration = Duration::from_secs(5);
const EXIT_WAIT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Oracle,
    Rtwk,
}

impl Side {
    pub fn label(self) -> &'static str {
        match self {
            Side::Oracle => "oracle",
            Side::Rtwk => "rtwk",
        }
    }
}

/// What one binary did with the script.
#[derive(Debug, Default)]
pub struct SideRun {
    /// The shots at the checkpoints, normalized, in script order.
    pub checkpoints: Vec<(String, Shot)>,
    /// What went wrong on the way (a `wait` that timed out, ...).
    pub notes: Vec<String>,
    /// What went wrong at a checkpoint (a wait in vain, a screen that never
    /// got quiet): (checkpoint, note). They belong to the checkpoint's
    /// finding, so a gap named there covers them.
    pub checkpoint_notes: Vec<(String, String)>,
    pub exit_code: Option<i32>,
    pub server_log: String,
    /// The program ended before the script did: (exit code, its screen).
    pub ended_early: Option<(Option<i32>, String)>,
}

/// Plays `scn` against `exe`: a fresh backend and home, the binary in a pty.
pub fn run_side(scn: &Scenario, side: Side, exe: &Path) -> Result<SideRun, String> {
    let backend = Backend::start(&scn.header.mock, scn.header.seed.as_deref())?;
    let n = normalizer(&backend);
    let template = match side {
        Side::Oracle => &scn.header.oracle_args,
        Side::Rtwk => &scn.header.rtwk_args,
    };
    let workdir = backend.workdir.display().to_string();
    let args: Vec<String> = template
        .iter()
        .map(|a| a.replace("{workdir}", &workdir))
        .collect();
    let env = pty::environment(&backend.codex_home, &backend.home);
    let mut pty = Pty::start(exe, &args, &backend.workdir, &env, scn.header.size)?;
    let mut run = SideRun::default();
    for step in &scn.steps {
        if pty.ended() && !matches!(step, Step::WaitExit { .. }) {
            let code = pty.wait_exit(EXIT_WAIT);
            run.ended_early = Some((code, pty.shot().text()));
            break;
        }
        match step {
            Step::Wait(text) => {
                if !pty.wait_text(text, scn.header.timeout) {
                    run.notes.push(format!(
                        "waited {:?} for {text:?} in vain",
                        scn.header.timeout
                    ));
                }
            }
            Step::Send(bytes) => pty.send(bytes),
            Step::Paste(text) => pty.paste(text),
            Step::Resize(w, h) => pty.resize(*w, *h),
            Step::Sleep(d) => {
                let end = std::time::Instant::now() + *d;
                while std::time::Instant::now() < end {
                    pty.pump();
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            Step::Check { name, until, .. } => {
                if let Some(text) = until
                    && !pty.wait_text(text, scn.header.timeout)
                {
                    run.checkpoint_notes.push((
                        name.clone(),
                        format!("waited {:?} for {text:?} in vain", scn.header.timeout),
                    ));
                }
                if !pty.wait_quiet(QUIET, QUIET_MAX) {
                    run.checkpoint_notes.push((
                        name.clone(),
                        format!("the screen never got quiet; shot after {QUIET_MAX:?}"),
                    ));
                }
                let shot = n.shot(&pty.shot());
                save(&scn.header.name, name, side, &shot);
                run.checkpoints.push((name.clone(), shot));
            }
            Step::WaitExit { .. } => {
                run.exit_code = pty.wait_exit(EXIT_WAIT);
                if run.exit_code.is_none() {
                    run.checkpoint_notes.push((
                        "after-exit".to_owned(),
                        format!("did not exit within {EXIT_WAIT:?}"),
                    ));
                }
                pty.wait_quiet(QUIET, QUIET_MAX);
                let shot = n.shot(&pty.shot());
                save(&scn.header.name, "after-exit", side, &shot);
                run.checkpoints.push(("after-exit".to_owned(), shot));
            }
        }
    }
    run.server_log = backend.server_log();
    Ok(run)
}

/// A checkpoint's screen as a file of its own, for reading.
fn save(scenario: &str, checkpoint: &str, side: Side, shot: &Shot) {
    let dir = report_dir(scenario);
    let _ = std::fs::create_dir_all(&dir);
    let mut text = shot.rows.join("\n");
    text.push_str(&format!(
        "\n--- cursor {:?} visible {} alt {}\n",
        shot.cursor, shot.cursor_visible, shot.alt_screen
    ));
    for line in &shot.scrollback {
        text.push_str(&format!("scrollback: {line}\n"));
    }
    let _ = std::fs::write(dir.join(format!("{checkpoint}.{}.txt", side.label())), text);
}

/// The normalizer of one run: its temp root and its mock server's address.
fn normalizer(backend: &Backend) -> Normalize {
    let mut n = Normalize::new();
    for root in backend.roots() {
        n = n.literal(&root, "<ROOT>");
    }
    n.literal(&backend.url, "<MOCK>")
}

/// Where the side-by-side files go: `target/tmp/scenarios/<name>/`.
pub fn report_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("scenarios")
        .join(name)
}

/// Both binaries, one after the other (never together: the screens'
/// timing is the thing compared). On a difference the checkpoint's two
/// screens are written side by side into `report_dir`.
pub fn run_both(scn: &Scenario) -> Result<Outcome, String> {
    let oracle = run_side(scn, Side::Oracle, &backend::binary("oracle")?)?;
    if let Some((code, text)) = &oracle.ended_early {
        return Err(format!("the oracle ended early (exit {code:?}):\n{text}"));
    }
    let rtwk = run_side(scn, Side::Rtwk, &backend::binary("rtwk")?)?;
    if let Some((code, text)) = &rtwk.ended_early
        && text.contains("unknown argument")
    {
        return Ok(Outcome::Skipped(format!(
            "codex-rtwk does not take {:?} yet (exit {code:?}): {}",
            scn.header.rtwk_args,
            text.trim()
        )));
    }
    Ok(Outcome::Compared(differences(scn, &oracle, &rtwk)))
}

/// What `run_both` found.
#[derive(Debug)]
pub enum Outcome {
    /// Both ran.
    Compared(Report),
    /// codex-rtwk cannot take the scenario yet; why.
    Skipped(String),
}

pub fn differences(scn: &Scenario, oracle: &SideRun, rtwk: &SideRun) -> Report {
    let n = Normalize::new();
    let dir = report_dir(&scn.header.name);
    let _ = std::fs::create_dir_all(&dir);
    let mut report = Report::default();
    report
        .general
        .extend(oracle.notes.iter().map(|note| format!("oracle: {note}")));
    report
        .general
        .extend(rtwk.notes.iter().map(|note| format!("rtwk: {note}")));
    if oracle.exit_code != rtwk.exit_code {
        report.general.push(format!(
            "exit code: oracle {:?}, rtwk {:?}",
            oracle.exit_code, rtwk.exit_code
        ));
    }
    let gap_of = |name: &str| {
        scn.steps.iter().find_map(|step| match step {
            Step::Check { name: n, gap, .. } if n == name => gap.clone(),
            Step::WaitExit { gap } if name == "after-exit" => gap.clone(),
            _ => None,
        })
    };
    for (name, o) in &oracle.checkpoints {
        let Some((_, r)) = rtwk.checkpoints.iter().find(|(other, _)| other == name) else {
            report
                .general
                .push(format!("{name}: rtwk has no such checkpoint"));
            continue;
        };
        let mut diffs = compare(o, r, &n, scn.header.scrollback);
        for (side, run) in [("oracle", oracle), ("rtwk", rtwk)] {
            diffs.extend(
                run.checkpoint_notes
                    .iter()
                    .filter(|(checkpoint, _)| checkpoint == name)
                    .map(|(_, note)| format!("{side}: {note}")),
            );
        }
        if !diffs.is_empty() {
            let file = dir.join(format!("{name}.txt"));
            let _ = std::fs::write(&file, side_by_side(o, r, &n));
        }
        report.findings.push(Finding {
            checkpoint: name.clone(),
            gap: gap_of(name),
            diffs,
        });
    }
    report
}
