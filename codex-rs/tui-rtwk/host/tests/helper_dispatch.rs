//! The binary is also the sandbox helper: codex re-runs its own executable
//! with `--codex-run-as-fs-helper`, `--codex-run-as-arg0-exec-helper` and
//! `--codex-run-as-apply-patch`, and `arg0_dispatch_or_else` has to see
//! those arguments before the app parses its own command line. These tests
//! run the built binary.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

use std::process::Command;
use std::process::Stdio;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codex-rtwk"))
}

fn unknown_argument(stderr: &[u8]) -> bool {
    String::from_utf8_lossy(stderr).contains("unknown argument")
}

#[test]
fn the_fs_helper_argument_reaches_the_dispatch() {
    let output = binary()
        .arg("--codex-run-as-fs-helper")
        .stdin(Stdio::null())
        .output()
        .expect("run");
    assert!(
        !unknown_argument(&output.stderr),
        "the app parsed the helper argument: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_exec_helper_argument_reaches_the_dispatch() {
    let output = binary()
        .arg("--codex-run-as-arg0-exec-helper")
        .stdin(Stdio::null())
        .output()
        .expect("run");
    assert!(
        !unknown_argument(&output.stderr),
        "the app parsed the helper argument: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn apply_patch_argument_applies_the_patch() {
    let dir = std::env::temp_dir().join(format!("codex-rtwk-patch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let output = binary()
        .current_dir(&dir)
        .arg("--codex-run-as-apply-patch")
        .arg("*** Begin Patch\n*** Add File: hello.txt\n+hi\n*** End Patch")
        .output()
        .expect("run");
    assert!(
        !unknown_argument(&output.stderr),
        "the app parsed the helper argument: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{:?}", output.status);
    let text = std::fs::read_to_string(dir.join("hello.txt")).expect("the patch wrote the file");
    assert_eq!(text, "hi\n");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unknown_argument_still_fails_in_the_app() {
    let output = binary().arg("--no-such-flag").output().expect("run");
    assert!(unknown_argument(&output.stderr));
    assert!(!output.status.success());
}
