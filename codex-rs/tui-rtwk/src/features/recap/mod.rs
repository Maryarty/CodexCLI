//! The view half of `/recap` (ADR-013): the words of its notes. The flow is
//! `codex_tui_core::features::recap`. Not built yet: the cell of the recap (`↳ Recap:` with its
//! next action) and the "Recapping..." row while `State::generating`, and the automatic recap
//! after the user has been away (the window's focus and a timer are the surface's).

pub mod strings;

use codex_tui_core::features::recap::{Note, Recap};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Recap {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
