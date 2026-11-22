//! `/diff`: the git commands of the oracle's `get_git_diff.rs` as a step machine.
//!
//! The machine is pure (R10): it says which commands to run (`Step::Run`), is
//! told what they printed (`feed`), and ends with the diff (`Step::Done`). Where
//! and how a command runs (the app server's `command/exec`, a fake in a test) is
//! the shell's; the runner that drives it is `services/shell.rs`.
//!
//! Policy kept from the oracle: repository configuration must not select an
//! executable helper, so `core.fsmonitor` is overridden (the built-in daemon is
//! kept only when it is the effective value and git advertises it, the policy of
//! `codex_git_utils::detect_fsmonitor_override`), hooks are off, and every
//! configured filter driver's `clean`/`process` is blanked and made optional
//! through `GIT_CONFIG_*` for the diff commands.

/// `codex_git_utils::SAFE_BARE_REPOSITORY_CONFIG`.
const SAFE_BARE_REPOSITORY_CONFIG: &str = "safe.bareRepository=explicit";
const DISABLE_HOOKS_CONFIG: &str = if cfg!(windows) {
    "core.hooksPath=NUL"
} else {
    "core.hooksPath=/dev/null"
};
const NULL_DEVICE: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };
const EXECUTABLE_FILTER_CONFIG_PATTERN: &str = r"^filter\..*\.(clean|process)$";
const DIFF_ARGS: [&str; 6] = [
    "diff",
    "--no-textconv",
    "--no-ext-diff",
    "--submodule=short",
    "--ignore-submodules=dirty",
    "--color",
];

/// Seconds a probe may take (`WorkspaceCommand::new`).
pub const PROBE_TIMEOUT_SECS: u64 = 5;
/// Seconds a diff command may take (`DIFF_COMMAND_TIMEOUT`); there is no
/// deadline over all of them.
pub const DIFF_TIMEOUT_SECS: u64 = 30;

/// What a command is allowed: a probe is short and its output capped; a diff
/// may be long and is returned whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limits {
    Probe,
    Diff,
}

impl Limits {
    pub fn timeout_secs(self) -> u64 {
        match self {
            Self::Probe => PROBE_TIMEOUT_SECS,
            Self::Diff => DIFF_TIMEOUT_SECS,
        }
    }

    pub fn uncapped(self) -> bool {
        self == Self::Diff
    }
}

/// One command for the shell to run in the workspace: the program and its
/// arguments (no shell), the environment to set, what it may take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub limits: Limits,
}

/// What a finished command printed. A command that could not run at all is an
/// `Err` of the shell instead (`feed`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub exit_code: i32,
    pub stdout: String,
}

/// The diff of the working directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// The directory is inside a git work tree; `text` is empty when not.
    pub in_repo: bool,
    /// Tracked changes, then each untracked file as a new file (ANSI colours).
    pub text: String,
}

/// What the machine wants next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Run these commands together (their outputs come back in this order).
    Run(Vec<Call>),
    Done(Result<Report, String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fsmonitor {
    Disabled,
    BuiltIn,
}

impl Fsmonitor {
    fn config_arg(self) -> &'static str {
        match self {
            Self::Disabled => "core.fsmonitor=false",
            Self::BuiltIn => "core.fsmonitor=true",
        }
    }
}

#[derive(Clone, Debug)]
enum Phase {
    /// `rev-parse --is-inside-work-tree` is out.
    InsideRepo,
    /// The raw effective `core.fsmonitor` is asked.
    ProbeRaw,
    /// Git is asked to normalise an uncommon spelling (`value`).
    ProbeTyped,
    /// Git's build options are asked for the daemon.
    ProbeCapability,
    /// The configured filter drivers are asked.
    Filters { fsmonitor: Fsmonitor },
    /// The tracked diff and the untracked file names are out.
    Tracked {
        fsmonitor: Fsmonitor,
        overrides: Vec<(String, String)>,
    },
    /// One diff per untracked file, one after the other.
    Untracked {
        fsmonitor: Fsmonitor,
        overrides: Vec<(String, String)>,
        files: Vec<String>,
        at: usize,
        text: String,
    },
    Finished,
}

/// The `/diff` command sequence; one per run.
#[derive(Clone, Debug)]
pub struct Diff {
    phase: Phase,
}

impl Diff {
    /// The machine and its first commands.
    pub fn start() -> (Diff, Step) {
        let first = git_call(Fsmonitor::Disabled, &[], &["rev-parse", "--is-inside-work-tree"]);
        (
            Diff {
                phase: Phase::InsideRepo,
            },
            Step::Run(vec![first]),
        )
    }

    /// The outputs of the commands the last `Step::Run` asked for, in order.
    pub fn feed(&mut self, outputs: Vec<Result<Output, String>>) -> Step {
        let mut outputs = outputs.into_iter();
        let (Some(first), second) = (outputs.next(), outputs.next()) else {
            return self.fail("the shell answered no command".to_owned());
        };
        match std::mem::replace(&mut self.phase, Phase::Finished) {
            Phase::InsideRepo => match first {
                Err(error) => Step::Done(Err(error)),
                Ok(output) if output.exit_code == 0 => {
                    self.phase = Phase::ProbeRaw;
                    Step::Run(vec![probe_call(&["config", "--null", "--get", "core.fsmonitor"])])
                }
                // not a repository: no probe may run outside one
                Ok(_) => Step::Done(Ok(Report {
                    in_repo: false,
                    text: String::new(),
                })),
            },
            Phase::ProbeRaw => {
                let Some(config) = probed(first) else {
                    return self.filters(Fsmonitor::Disabled);
                };
                // exactly one value, NUL-terminated
                let Some(config) = config.strip_suffix('\0').filter(|c| !c.contains('\0')) else {
                    return self.filters(Fsmonitor::Disabled);
                };
                let is = |spellings: [&str; 3]| {
                    spellings.iter().any(|s| config.eq_ignore_ascii_case(s))
                };
                if is(["true", "yes", "on"]) {
                    self.capability()
                } else if is(["false", "no", "off"]) {
                    self.filters(Fsmonitor::Disabled)
                } else {
                    // a number, a valueless key, a helper path: git decides
                    self.phase = Phase::ProbeTyped;
                    Step::Run(vec![probe_call(&[
                        "config",
                        "--null",
                        "--type=bool",
                        "--fixed-value",
                        "--get",
                        "core.fsmonitor",
                        config,
                    ])])
                }
            }
            Phase::ProbeTyped => {
                if probed(first).as_deref() == Some("true\0") {
                    self.capability()
                } else {
                    self.filters(Fsmonitor::Disabled)
                }
            }
            Phase::ProbeCapability => {
                // git 2.35.1 and older read "true" as a hook path: require the
                // feature line git added for capability checks
                let daemon = probed(first).is_some_and(|options| {
                    options
                        .lines()
                        .any(|line| line.trim() == "feature: fsmonitor--daemon")
                });
                self.filters(if daemon {
                    Fsmonitor::BuiltIn
                } else {
                    Fsmonitor::Disabled
                })
            }
            Phase::Filters { fsmonitor } => {
                let args = ["config", "--null", "--name-only", "--get-regexp", EXECUTABLE_FILTER_CONFIG_PATTERN];
                let output = match first {
                    Err(error) => return Step::Done(Err(error)),
                    Ok(output) => output,
                };
                if output.exit_code != 0 && output.exit_code != 1 {
                    return Step::Done(Err(failed(&args, output.exit_code)));
                }
                let overrides = filter_overrides(&output.stdout);
                self.phase = Phase::Tracked {
                    fsmonitor,
                    overrides: overrides.clone(),
                };
                Step::Run(vec![
                    git_call(fsmonitor, &overrides, &DIFF_ARGS),
                    git_call(
                        fsmonitor,
                        &[],
                        &["ls-files", "--others", "--exclude-standard"],
                    ),
                ])
            }
            Phase::Tracked {
                fsmonitor,
                overrides,
            } => {
                let tracked = match diff_output(first, &DIFF_ARGS) {
                    Ok(text) => text,
                    Err(error) => return Step::Done(Err(error)),
                };
                let listed = match second {
                    None => return Step::Done(Err("the shell answered no command".to_owned())),
                    Some(Err(error)) => return Step::Done(Err(error)),
                    Some(Ok(output)) if output.exit_code == 0 => output.stdout,
                    Some(Ok(output)) => {
                        return Step::Done(Err(failed(
                            &["ls-files", "--others", "--exclude-standard"],
                            output.exit_code,
                        )));
                    }
                };
                let files: Vec<String> = listed
                    .split('\n')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
                    .collect();
                self.untracked(fsmonitor, overrides, files, 0, tracked)
            }
            Phase::Untracked {
                fsmonitor,
                overrides,
                files,
                at,
                mut text,
            } => {
                let args = untracked_args(&files[at]);
                match diff_output(first, &args) {
                    Ok(diff) => text.push_str(&diff),
                    Err(error) => return Step::Done(Err(error)),
                }
                self.untracked(fsmonitor, overrides, files, at + 1, text)
            }
            Phase::Finished => Step::Done(Err("the diff had already finished".to_owned())),
        }
    }

    fn fail(&mut self, error: String) -> Step {
        self.phase = Phase::Finished;
        Step::Done(Err(error))
    }

    fn capability(&mut self) -> Step {
        self.phase = Phase::ProbeCapability;
        Step::Run(vec![probe_call(&["version", "--build-options"])])
    }

    fn filters(&mut self, fsmonitor: Fsmonitor) -> Step {
        self.phase = Phase::Filters { fsmonitor };
        Step::Run(vec![git_call(
            fsmonitor,
            &[],
            &[
                "config",
                "--null",
                "--name-only",
                "--get-regexp",
                EXECUTABLE_FILTER_CONFIG_PATTERN,
            ],
        )])
    }

    /// The diff of file `at` of `files`, or the end when none is left.
    fn untracked(
        &mut self,
        fsmonitor: Fsmonitor,
        overrides: Vec<(String, String)>,
        files: Vec<String>,
        at: usize,
        text: String,
    ) -> Step {
        let Some(file) = files.get(at) else {
            return Step::Done(Ok(Report {
                in_repo: true,
                text,
            }));
        };
        let call = git_call(fsmonitor, &overrides, &untracked_args(file));
        self.phase = Phase::Untracked {
            fsmonitor,
            overrides,
            files,
            at,
            text,
        };
        Step::Run(vec![call])
    }
}

/// `git diff ... --no-index -- /dev/null <file>`: an untracked file as new.
fn untracked_args(file: &str) -> Vec<&str> {
    let mut args = DIFF_ARGS.to_vec();
    args.extend(["--no-index", "--", NULL_DEVICE, file]);
    args
}

/// A git command with the safety overrides, in the diff's limits.
fn git_call(fsmonitor: Fsmonitor, overrides: &[(String, String)], args: &[&str]) -> Call {
    let mut argv: Vec<String> = [
        "git",
        "-c",
        SAFE_BARE_REPOSITORY_CONFIG,
        "-c",
        fsmonitor.config_arg(),
        "-c",
        DISABLE_HOOKS_CONFIG,
    ]
    .iter()
    .map(|part| (*part).to_owned())
    .collect();
    argv.extend(args.iter().map(|arg| (*arg).to_owned()));
    let mut env = Vec::new();
    if !overrides.is_empty() {
        env.push(("GIT_CONFIG_COUNT".to_owned(), overrides.len().to_string()));
        for (index, (key, value)) in overrides.iter().enumerate() {
            env.push((format!("GIT_CONFIG_KEY_{index}"), key.clone()));
            env.push((format!("GIT_CONFIG_VALUE_{index}"), value.clone()));
        }
    }
    Call {
        argv,
        env,
        limits: Limits::Diff,
    }
}

/// A git command of the fsmonitor policy: only the bare-repository guard, in
/// a probe's limits.
fn probe_call(args: &[&str]) -> Call {
    let mut argv = vec![
        "git".to_owned(),
        "-c".to_owned(),
        SAFE_BARE_REPOSITORY_CONFIG.to_owned(),
    ];
    argv.extend(args.iter().map(|arg| (*arg).to_owned()));
    Call {
        argv,
        env: Vec::new(),
        limits: Limits::Probe,
    }
}

/// What a probe printed: nothing for a failure, a timeout or a non-zero exit.
fn probed(output: Result<Output, String>) -> Option<String> {
    output
        .ok()
        .filter(|output| output.exit_code == 0)
        .map(|output| output.stdout)
}

/// The output of a diff, where git's exit code 1 means "there are differences".
fn diff_output(output: Result<Output, String>, args: &[&str]) -> Result<String, String> {
    let output = output?;
    if output.exit_code == 0 || output.exit_code == 1 {
        Ok(output.stdout)
    } else {
        Err(failed(args, output.exit_code))
    }
}

fn failed(args: &[&str], exit_code: i32) -> String {
    format!("git {args:?} failed with status {exit_code}")
}

/// The overrides that blank every configured filter driver: for each driver
/// (sorted, once) its `clean` and `process` emptied and `required` false.
fn filter_overrides(stdout: &str) -> Vec<(String, String)> {
    let mut drivers: Vec<&str> = stdout
        .split('\0')
        .filter_map(|key| {
            key.strip_suffix(".clean")
                .or_else(|| key.strip_suffix(".process"))
        })
        .collect();
    drivers.sort_unstable();
    drivers.dedup();
    drivers
        .into_iter()
        .flat_map(|driver| {
            [
                (format!("{driver}.clean"), String::new()),
                (format!("{driver}.process"), String::new()),
                (format!("{driver}.required"), "false".to_owned()),
            ]
        })
        .collect()
}
