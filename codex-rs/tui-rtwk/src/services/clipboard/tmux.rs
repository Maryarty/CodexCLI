//! Clipboard forwarding to one attached client in the current pane's tmux
//! session (oracle `clipboard_copy/tmux.rs`). The choice of the client is
//! [`super::tmux_target`]; capability inspection and delivery use the same
//! client and the trusted system executable. The payload cap is the routing
//! layer's.

use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;

use super::tmux_target;

/// Copy text through tmux's native clipboard integration: `load-buffer -w -`
/// lets tmux read the text from stdin, keep a matching paste buffer and
/// forward the contents to the outer terminal's clipboard when possible,
/// without relying on DCS passthrough.
pub(super) fn copy(text: &str) -> Result<(), String> {
    let executable = codex_utils_path::system_executable("tmux")
        .ok_or_else(|| "tmux is unavailable in the system PATH".to_string())?;
    let path = codex_utils_path::system_path()
        .map_err(|error| format!("failed to resolve system PATH: {error}"))?;
    let pane = std::env::var("TMUX_PANE")
        .map_err(|_| "tmux clipboard forwarding is unavailable: no current pane".to_string())?;
    let client = tmux_target(
        || command_output(&executable, &path, ["show-options", "-gv", "set-clipboard"]),
        || {
            let session = command_output(
                &executable,
                &path,
                ["display-message", "-p", "-t", &pane, "#{session_id}"],
            )?;
            command_output(
                &executable,
                &path,
                [
                    "list-clients",
                    "-t",
                    session.trim(),
                    "-F",
                    "#{client_activity} #{client_name}",
                ],
            )
        },
        |client| command_output(&executable, &path, ["show-messages", "-T", "-t", client]),
    )?;

    let mut child = std::process::Command::new(&executable)
        .env("PATH", &path)
        .args(["load-buffer", "-w", "-t", &client, "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn tmux: {e}"))?;

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("failed to open tmux stdin".to_string());
    };

    if let Err(err) = stdin.write_all(text.as_bytes()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("failed to write to tmux: {err}"));
    }

    drop(stdin);

    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for tmux: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(failure(&output))
    }
}

fn command_output<const N: usize>(
    executable: &Path,
    path: &OsStr,
    args: [&str; N],
) -> Result<String, String> {
    let output = std::process::Command::new(executable)
        .env("PATH", path)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("failed to spawn tmux: {e}"))?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| format!("tmux output was not UTF-8: {e}"))
    } else {
        Err(failure(&output))
    }
}

/// What a failed tmux run says: its stderr, or its status.
fn failure(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        let status = output.status;
        format!("tmux exited with status {status}")
    } else {
        format!("tmux failed: {stderr}")
    }
}
