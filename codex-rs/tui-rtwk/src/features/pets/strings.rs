//! Pet selection notices (oracle `app/pets.rs`).

use codex_tui_core::features::pets::Note;

pub(super) fn note(note: &Note) -> String {
    match note {
        Note::LoadFailed(error) => format!("Failed to load pet: {error}"),
        Note::SaveFailed {
            disabling: true,
            error,
        } => format!("Failed to disable pets: {error}"),
        Note::SaveFailed {
            disabling: false,
            error,
        } => format!("Failed to save pet selection: {error}"),
        Note::SaveInProgress => "A pet selection is already being saved.".to_owned(),
    }
}
