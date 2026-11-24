//! The view half of `/copy` (ADR-013): the words of its notes. The flow itself is
//! `codex_tui_core::features::copy` (the last response, the copy a row of the picker or Ctrl+O
//! asks). The picker sheet that shows `Wish::Picker` (its rows are cut from the response's
//! Markdown source: the whole response, then each code block and quote) is not built yet: the
//! presenter has nothing to open for the wish until it is.

pub mod strings;

use codex_tui_core::features::copy::{Copier, Note};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Copier {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
