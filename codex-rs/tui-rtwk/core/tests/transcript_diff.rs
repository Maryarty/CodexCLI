#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of `/diff` (S10): the command sequence of the oracle's
//! `get_git_diff.rs` as the step machine of `screens::transcript::diff`, and
//! the fsmonitor policy of `codex_git_utils::detect_fsmonitor_override`. The
//! expected commands, environments and results are the oracle's tests'
//! (`get_git_diff.rs` :276-:605, `git-utils/src/fsmonitor_tests.rs`); the
//! oracle's two tests that run a real git against marker helper scripts
//! (:607, :688) test git's own behaviour with a process and are not state
//! tests of the sequence.

use codex_tui_core::screens::transcript::diff::{
    Call, Diff, DiffStep, Failure, Limits, Output, Report,
};
use pretty_assertions::assert_eq;

const FILTER_PATTERN: &str = r"^filter\..*\.(clean|process)$";
const DIFF: [&str; 6] = [
    "diff",
    "--no-textconv",
    "--no-ext-diff",
    "--submodule=short",
    "--ignore-submodules=dirty",
    "--color",
];

fn null_device() -> &'static str {
    if cfg!(windows) { "NUL" } else { "/dev/null" }
}

/// A git command of the diff (`git_command`): fsmonitor `core.fsmonitor=<fs>`.
fn git(fsmonitor: &str, args: &[&str]) -> Vec<String> {
    let hooks = if cfg!(windows) {
        "core.hooksPath=NUL"
    } else {
        "core.hooksPath=/dev/null"
    };
    [
        "git",
        "-c",
        "safe.bareRepository=explicit",
        "-c",
        &format!("core.fsmonitor={fsmonitor}"),
        "-c",
        hooks,
    ]
    .into_iter()
    .chain(args.iter().copied())
    .map(str::to_owned)
    .collect()
}

/// A git command of the fsmonitor policy (`git_probe_command`).
fn probe(args: &[&str]) -> Vec<String> {
    ["git", "-c", "safe.bareRepository=explicit"]
        .into_iter()
        .chain(args.iter().copied())
        .map(str::to_owned)
        .collect()
}

fn filter_env(driver: &str) -> Vec<(String, String)> {
    [
        ("GIT_CONFIG_COUNT", "3".to_owned()),
        ("GIT_CONFIG_KEY_0", format!("{driver}.clean")),
        ("GIT_CONFIG_VALUE_0", String::new()),
        ("GIT_CONFIG_KEY_1", format!("{driver}.process")),
        ("GIT_CONFIG_VALUE_1", String::new()),
        ("GIT_CONFIG_KEY_2", format!("{driver}.required")),
        ("GIT_CONFIG_VALUE_2", "false".to_owned()),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect()
}

/// The shell's end of the test: each command the machine asks for must be the
/// next scripted one (`FakeRunner`); returns the machine's result and the
/// commands it asked for.
fn drive(responses: Vec<(Vec<String>, i32, &str)>) -> (Result<Report, Failure>, Vec<Call>) {
    let mut responses = responses.into_iter();
    let mut asked = Vec::new();
    let (mut diff, mut step) = Diff::start();
    loop {
        match step {
            DiffStep::Done(result) => {
                assert_eq!(responses.len(), 0, "unused scripted responses");
                return (result, asked);
            }
            DiffStep::Run(calls) => {
                let mut outputs = Vec::new();
                for call in calls {
                    let (argv, exit_code, stdout) = responses
                        .next()
                        .expect("a scripted response for the command");
                    assert_eq!(call.argv, argv);
                    outputs.push(Ok(Output {
                        exit_code,
                        stdout: stdout.to_owned(),
                    }));
                    asked.push(call);
                }
                step = diff.feed(outputs);
            }
        }
    }
}

/// Probes are short and capped, diff commands long and whole
/// (`assert_command_metadata`).
fn assert_limits(asked: &[Call]) {
    for call in asked {
        if matches!(
            call.argv.get(3).map(String::as_str),
            Some("config" | "version")
        ) {
            assert_eq!(call.limits, Limits::Probe, "{:?}", call.argv);
            assert!(call.env.is_empty());
            assert_eq!(call.limits.timeout_secs(), 5);
            assert!(!call.limits.uncapped());
        } else {
            assert_eq!(call.limits, Limits::Diff, "{:?}", call.argv);
            assert_eq!(call.limits.timeout_secs(), 30);
            assert!(call.limits.uncapped());
        }
    }
}

/// What a scripted command is: its argv, the exit code and the stdout.
type Scripted = (Vec<String>, i32, &'static str);

fn inside() -> Scripted {
    (
        git("false", &["rev-parse", "--is-inside-work-tree"]),
        0,
        "true\n",
    )
}

fn raw_probe(exit_code: i32, stdout: &'static str) -> (Vec<String>, i32, &'static str) {
    (
        probe(&["config", "--null", "--get", "core.fsmonitor"]),
        exit_code,
        stdout,
    )
}

fn filters(
    fsmonitor: &str,
    exit_code: i32,
    stdout: &'static str,
) -> (Vec<String>, i32, &'static str) {
    (
        git(
            fsmonitor,
            &[
                "config",
                "--null",
                "--name-only",
                "--get-regexp",
                FILTER_PATTERN,
            ],
        ),
        exit_code,
        stdout,
    )
}

fn tracked(
    fsmonitor: &str,
    exit_code: i32,
    stdout: &'static str,
) -> (Vec<String>, i32, &'static str) {
    (git(fsmonitor, &DIFF), exit_code, stdout)
}

fn ls_files(fsmonitor: &str, stdout: &'static str) -> (Vec<String>, i32, &'static str) {
    (
        git(fsmonitor, &["ls-files", "--others", "--exclude-standard"]),
        0,
        stdout,
    )
}

fn new_file(
    fsmonitor: &str,
    name: &str,
    exit_code: i32,
    stdout: &'static str,
) -> (Vec<String>, i32, &'static str) {
    let mut args = DIFF.to_vec();
    args.extend(["--no-index", "--", null_device(), name]);
    (git(fsmonitor, &args), exit_code, stdout)
}

#[test]
fn a_directory_outside_a_repository_is_reported_without_any_probe() {
    let (result, asked) = drive(vec![(
        git("false", &["rev-parse", "--is-inside-work-tree"]),
        128,
        "",
    )]);
    assert_eq!(
        result,
        Ok(Report {
            in_repo: false,
            text: String::new()
        })
    );
    assert_eq!(asked.len(), 1);
    assert_limits(&asked);
}

#[test]
fn the_diff_blanks_the_filter_drivers_and_covers_tracked_and_untracked_files() {
    let (result, asked) = drive(vec![
        inside(),
        raw_probe(0, "/tmp/fsmonitor-helper\0"),
        (
            probe(&[
                "config",
                "--null",
                "--type=bool",
                "--fixed-value",
                "--get",
                "core.fsmonitor",
                "/tmp/fsmonitor-helper",
            ]),
            128,
            "",
        ),
        filters("false", 0, "filter.evil.clean\0filter.evil.process\0"),
        tracked("false", 1, "tracked\n"),
        ls_files("false", "new.txt\n"),
        new_file("false", "new.txt", 1, "untracked\n"),
    ]);
    assert_eq!(
        result,
        Ok(Report {
            in_repo: true,
            text: "tracked\nuntracked\n".to_owned()
        })
    );
    assert_limits(&asked);
    // the tracked diff and the per-file diff carry the blanked drivers; the
    // names of the untracked files do not
    assert_eq!(asked[4].env, filter_env("filter.evil"));
    assert_eq!(asked[5].env, Vec::new());
    assert_eq!(asked[6].env, filter_env("filter.evil"));
}

#[test]
fn the_builtin_fsmonitor_is_kept_for_every_command_of_the_diff() {
    let (result, asked) = drive(vec![
        inside(),
        raw_probe(0, "true\0"),
        (
            probe(&["version", "--build-options"]),
            0,
            "feature: fsmonitor--daemon\n",
        ),
        filters("true", 1, ""),
        tracked("true", 1, "tracked\n"),
        ls_files("true", "new.txt\n"),
        new_file("true", "new.txt", 1, "untracked\n"),
    ]);
    assert_eq!(
        result,
        Ok(Report {
            in_repo: true,
            text: "tracked\nuntracked\n".to_owned()
        })
    );
    assert_limits(&asked);
}

#[test]
fn exit_code_one_of_the_diff_is_a_diff_not_an_error() {
    let (result, asked) = drive(vec![
        inside(),
        raw_probe(1, ""),
        filters("false", 1, ""),
        tracked("false", 1, "tracked\n"),
        ls_files("false", ""),
    ]);
    assert_eq!(
        result,
        Ok(Report {
            in_repo: true,
            text: "tracked\n".to_owned()
        })
    );
    assert_limits(&asked);
}

#[test]
fn another_exit_code_of_the_diff_is_an_error_naming_the_command() {
    let (result, asked) = drive(vec![
        inside(),
        raw_probe(1, ""),
        filters("false", 1, ""),
        tracked("false", 2, ""),
        ls_files("false", ""),
    ]);
    assert_eq!(
        result,
        Err(Failure::Git {
            args: [
                "diff",
                "--no-textconv",
                "--no-ext-diff",
                "--submodule=short",
                "--ignore-submodules=dirty",
                "--color",
            ]
            .map(str::to_owned)
            .to_vec(),
            status: 2
        })
    );
    assert_limits(&asked);
}

#[test]
fn several_untracked_files_are_diffed_one_after_the_other_and_a_failure_ends_the_run() {
    let (result, _) = drive(vec![
        inside(),
        raw_probe(1, ""),
        filters("false", 1, ""),
        tracked("false", 0, ""),
        ls_files("false", "a.txt\n\n  b.txt \n"),
        new_file("false", "a.txt", 1, "A\n"),
        new_file("false", "b.txt", 1, "B\n"),
    ]);
    assert_eq!(
        result,
        Ok(Report {
            in_repo: true,
            text: "A\nB\n".to_owned()
        })
    );
    let (result, _) = drive(vec![
        inside(),
        raw_probe(1, ""),
        filters("false", 1, ""),
        tracked("false", 0, ""),
        ls_files("false", "a.txt\nb.txt\n"),
        new_file("false", "a.txt", 128, ""),
    ]);
    assert!(matches!(result, Err(Failure::Git { status: 128, .. })));
}

#[test]
fn a_filter_listing_that_fails_for_another_reason_ends_the_run() {
    let (result, _) = drive(vec![inside(), raw_probe(1, ""), filters("false", 129, "")]);
    assert_eq!(
        result,
        Err(Failure::Git {
            args: [
                "config",
                "--null",
                "--name-only",
                "--get-regexp",
                FILTER_PATTERN
            ]
            .map(str::to_owned)
            .to_vec(),
            status: 129
        })
    );
}

#[test]
fn a_command_that_cannot_run_ends_the_run_with_its_error() {
    let (mut diff, _) = Diff::start();
    assert_eq!(
        diff.feed(vec![Err("the server is gone".to_owned())]),
        DiffStep::Done(Err(Failure::Shell("the server is gone".to_owned())))
    );
}

/// The fsmonitor policy, case by case (`detects_supported_builtin_fsmonitor_values`):
/// the filter listing that follows says which override the policy chose.
#[test]
fn the_fsmonitor_policy_keeps_the_builtin_daemon_only_when_it_is_on_and_advertised() {
    let capability = |stdout: &'static str| (probe(&["version", "--build-options"]), 0, stdout);
    let typed = |value: &str, exit_code: i32, stdout: &'static str| {
        (
            probe(&[
                "config",
                "--null",
                "--type=bool",
                "--fixed-value",
                "--get",
                "core.fsmonitor",
                value,
            ]),
            exit_code,
            stdout,
        )
    };
    let daemon = "feature: fsmonitor--daemon\n";
    let cases: Vec<(&str, Vec<Scripted>, &str)> = vec![
        ("missing config", vec![raw_probe(1, "")], "false"),
        (
            "helper path",
            vec![
                raw_probe(0, "/tmp/fsmonitor-helper\0"),
                typed("/tmp/fsmonitor-helper", 128, ""),
            ],
            "false",
        ),
        ("false spelling", vec![raw_probe(0, "OFF\0")], "false"),
        (
            "unsupported Git",
            vec![raw_probe(0, "yes\0"), capability("")],
            "false",
        ),
        (
            "common true spelling",
            vec![raw_probe(0, "On\0"), capability(daemon)],
            "true",
        ),
        (
            "numeric true",
            vec![
                raw_probe(0, "2k\0"),
                typed("2k", 0, "true\0"),
                capability(daemon),
            ],
            "true",
        ),
        (
            "valueless true",
            vec![
                raw_probe(0, "\0"),
                typed("", 0, "true\0"),
                capability(daemon),
            ],
            "true",
        ),
        (
            "explicit empty false",
            vec![raw_probe(0, "\0"), typed("", 0, "false\0")],
            "false",
        ),
        ("two values", vec![raw_probe(0, "true\0false\0")], "false"),
        ("no terminator", vec![raw_probe(0, "true")], "false"),
    ];
    for (name, probes, fsmonitor) in cases {
        let mut script = vec![inside()];
        script.extend(probes);
        script.extend([
            filters(fsmonitor, 1, ""),
            tracked(fsmonitor, 0, ""),
            ls_files(fsmonitor, ""),
        ]);
        let (result, _) = drive(script);
        assert_eq!(
            result,
            Ok(Report {
                in_repo: true,
                text: String::new()
            }),
            "case: {name}"
        );
    }
}
