//! The words of `/recap` (oracle `app/recap.rs` messages, `history_cell/notices.rs`
//! `ThreadRecapHistoryCell`): the notes' text. The recap itself is the oracle's cell, drawn by
//! the view half (UI LEFT): `↳ Recap: ` and the summary, then `Next: ` and the action.

use codex_tui_core::features::recap::Note;

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::Recap {
            summary,
            next_action: None,
        } => format!("Recap: {summary}"),
        Note::Recap {
            summary,
            next_action: Some(action),
        } => format!("Recap: {summary}\nNext: {action}"),
        Note::Failed => "Could not generate a recap. Please try again.".to_owned(),
        Note::InProgress => "A recap is already being generated.".to_owned(),
        Note::EmptyHistory => "There is no conversation history to recap.".to_owned(),
    }
}
