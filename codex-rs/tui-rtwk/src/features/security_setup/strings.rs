//! The words of the security-setup reminder's notes (oracle `open_url_in_browser`).

use codex_tui_core::features::security_setup::Note;

pub fn note(note: &Note) -> String {
    match note {
        Note::PageFailed { url, error } => format!("Failed to open browser for {url}: {error}"),
    }
}
