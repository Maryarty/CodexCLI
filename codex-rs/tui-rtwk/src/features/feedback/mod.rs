//! The view half of `/feedback` (ADR-013): the words of its notes. The flow itself is
//! `codex_tui_core::features::feedback` (the report's checks, its delivery to the host and the
//! line that says how it went). The category and consent steps are still
//! `sheets::dialogs::feedback`, opened by `Target::Dialogs`; the note step and the move of those
//! two into this directory are the UI half's.

pub mod strings;

use codex_tui_core::features::feedback::{Feedback, Note};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Feedback {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
