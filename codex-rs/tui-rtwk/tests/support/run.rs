//! Playing a scenario against one binary, and against both.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::backend::{self, Backend};
use super::compare::{compare, side_by_side};
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
    pub exit_code: Option<i32>,
    pub server_log: String,
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
    let args: Vec<String> = template.iter().map(|a| a.replace("{workdir}", &workdir)).collect();
    let env = pty::environment(&backend.codex_home, &backend.home);
    let mut pty = Pty::start(exe, &args, &backend.workdir, &env, scn.header.size)?;
    let mut run = SideRun::default();
    for step in &scn.steps {
        match step {
            Step::Wait(text) => {
                if !pty.wait_text(text, scn.header.timeout) {
                    run.notes.push(format!("waited {:?} for {text:?} in vain", scn.header.timeout));
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
            Step::Check { name, until } => {
                if let Some(text) = until
                    && !pty.wait_text(text, scn.header.timeout)
                {
                    run.notes.push(format!("{name}: waited {:?} for {text:?} in vain", scn.header.timeout));
                }
                if !pty.wait_quiet(QUIET, QUIET_MAX) {
                    run.notes.push(format!("{name}: the screen never got quiet; shot after {QUIET_MAX:?}"));
                }
                run.checkpoints.push((name.clone(), n.shot(&pty.shot())));
            }
            Step::WaitExit => {
                run.exit_code = pty.wait_exit(EXIT_WAIT);
                if run.exit_code.is_none() {
                    run.notes.push(format!("did not exit within {EXIT_WAIT:?}"));
                }
                pty.wait_quiet(QUIET, QUIET_MAX);
                run.checkpoints.push(("after-exit".to_owned(), n.shot(&pty.shot())));
            }
        }
    }
    run.server_log = backend.server_log();
    Ok(run)
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
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("scenarios").join(name)
}

/// Both binaries, one after the other (never together: the screens'
/// timing is the thing compared). The differences, one string each; empty
/// when the two agree at every checkpoint. On a difference the checkpoint's
/// two screens are written side by side into `report_dir`.
pub fn run_both(scn: &Scenario) -> Result<Vec<String>, String> {
    let oracle = run_side(scn, Side::Oracle, &backend::binary("oracle")?)?;
    let rtwk = run_side(scn, Side::Rtwk, &backend::binary("rtwk")?)?;
    Ok(differences(scn, &oracle, &rtwk))
}

pub fn differences(scn: &Scenario, oracle: &SideRun, rtwk: &SideRun) -> Vec<String> {
    let n = Normalize::new();
    let dir = report_dir(&scn.header.name);
    let _ = std::fs::create_dir_all(&dir);
    let mut found = Vec::new();
    for note in &oracle.notes {
        found.push(format!("oracle: {note}"));
    }
    for note in &rtwk.notes {
        found.push(format!("rtwk: {note}"));
    }
    if oracle.exit_code != rtwk.exit_code {
        found.push(format!("exit code: oracle {:?}, rtwk {:?}", oracle.exit_code, rtwk.exit_code));
    }
    for (name, o) in &oracle.checkpoints {
        let Some((_, r)) = rtwk.checkpoints.iter().find(|(other, _)| other == name) else {
            found.push(format!("{name}: rtwk has no such checkpoint"));
            continue;
        };
        let diffs = compare(o, r, &n, scn.header.scrollback);
        if diffs.is_empty() {
            continue;
        }
        let file = dir.join(format!("{name}.txt"));
        let _ = std::fs::write(&file, side_by_side(o, r, &n));
        found.push(format!("{name}: {} difference(s); both screens in {}", diffs.len(), file.display()));
        found.extend(diffs.into_iter().take(8).map(|d| format!("    {d}")));
    }
    found
}
