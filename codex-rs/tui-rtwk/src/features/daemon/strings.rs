//! The words of `/daemon` (oracle `app/daemon_menu.rs`, `event_dispatch.rs` `RunDaemonRestart`):
//! its transcript notes. The wording is the oracle's English until the catalog has these entries.

use codex_tui_core::features::daemon::Note;

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::RestartFailed(error) => format!("Could not restart the background server: {error}"),
    }
}
