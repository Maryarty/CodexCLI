#![cfg(unix)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::expect_used
)]

use std::path::Path;
use std::process::Command;

fn run_scenario(scenario: &str) -> String {
    let check =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools/rtwk-port/mock-model/check.py");
    let output = Command::new("python3")
        .arg(check)
        .args(["--bin", "rtwk", "--only", scenario, "--dump"])
        .env("RTWK_BIN", env!("CARGO_BIN_EXE_codex-rtwk"))
        .output()
        .expect("run the mock model and PTY");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    stdout.into_owned()
}

/// The mock model asks for approval through the embedded server. The PTY
/// confirms the existing sheet and the next model request must carry the
/// command's output.
#[test]
fn embedded_server_resolves_exec_approval() {
    let stdout = run_scenario("exec-approval");
    assert!(
        stdout
            .lines()
            .any(|line| line.contains("output call-exec-1:") && line.contains("created")),
        "the tool did not execute after approval:\n{stdout}"
    );
}

#[test]
fn standalone_escape_disconnects_the_live_stream() {
    let stdout = run_scenario("interrupted");
    assert!(
        stdout.contains("Escape disconnected the model stream"),
        "{stdout}"
    );
    assert!(stdout.contains("sentinel on screen: yes"), "{stdout}");
}
