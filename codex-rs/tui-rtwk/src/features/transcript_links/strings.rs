use codex_tui_core::features::transcript_links::Note;

pub(super) fn note(note: &Note) -> String {
    format!("Failed to open browser for {}: {}", note.url, note.error)
}
