//! The view half of the external editor (ADR-013): the words of its notes. The feature itself is
//! `codex_tui_core::features::external_editor` (the draft goes to the host's editor runner and
//! the text it left comes back as `State::edited`); it has no sheet of its own.

pub mod strings;

use codex_tui_core::features::external_editor::{ExternalEditor, Note};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for ExternalEditor {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
