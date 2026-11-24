//! The words of the Windows sandbox notes (oracle `platform_actions.rs`).

use codex_tui_core::features::windows_sandbox::Note;

pub fn note(note: &Note) -> String {
    match note {
        Note::Disallowed => "That Windows sandbox option is disallowed by requirements.".into(),
        Note::StillRunning => "Windows sandbox setup is still running. Restart Codex to retry.".into(),
        Note::UpdateServer => "Update the local app server to set up the Windows sandbox.".into(),
        Note::Failed => "Windows sandbox setup failed.".into(),
        Note::NotStarted => "Windows sandbox setup did not start.".into(),
        Note::SetupFailed(error) => format!("Windows sandbox setup failed: {error}"),
        Note::Mixed => "Windows sandbox setup is unavailable when local and remote executors are configured together.".into(),
    }
}
