//! The view half of `/recap` (ADR-013): the words of its notes. The flow is
//! `codex_tui_core::features::recap`. The cell of the recap (`↳ Recap:` with its
//! next action) is `cell.rs`; the "Generating conversation recap…" row while `State::generating`
//! is the status area's (`pages/session/activity.rs`); the automatic recap after the user has
//! been away (the window's focus and a timer) is the surface's and not built.

pub mod cell;
pub mod strings;

use rtwk::AnyWidget;

use codex_tui_core::features::recap::{Note, Recap};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Recap {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    fn cell(note: &Note) -> Option<AnyWidget> {
        cell::of(note)
    }
}
