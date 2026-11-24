//! The view half of `/side` and `/btw` (ADR-013): the words of its notes. The feature itself is
//! `codex_tui_core::features::side` (the fork, the links between a parent and its side
//! conversation, the footer label); it has no sheet of its own: `Ctrl+/` and `Ctrl+C` are verbs
//! the session page says.

pub mod strings;

use codex_tui_core::features::side::{Note, Side};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Side {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
