//! The words of the external editor (oracle `app/input.rs` `launch_external_editor`): the two
//! lines it says in the transcript when the editor could not run, and the footer hint that stands
//! while one runs (`EXTERNAL_EDITOR_HINT`). The wording is the oracle's English until the catalog
//! has these entries.

use codex_tui_core::features::external_editor::Note;

/// The footer hint while the editor holds the terminal.
pub const HINT: &str = "Save and close external editor to continue.";

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::Missing => {
            "Cannot open external editor: set $VISUAL or $EDITOR before starting Codex.".to_owned()
        }
        Note::Failed(error) => format!("Failed to open editor: {error}"),
    }
}
