//! The view half of `/copy` (ADR-013): the words of its notes. The flow itself is
//! `codex_tui_core::features::copy` (the last response, the copy a row of the picker or Ctrl+O
//! asks). The picker sheet that shows `Wish::Picker` (its rows are cut from the response's
//! Markdown source: the whole response, then each code block and quote) is `picker`.

pub mod picker;
pub mod strings;
pub mod targets;

use std::rc::Rc;

use rtwk::AnyWidget;

use codex_tui_core::features::Scope;
use codex_tui_core::features::copy::{Copier, Note, Wish};

use super::{FeatureView, Shown};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Copier {
    /// The picker opens once for the wish, at the bottom of the chat; nothing keeps a scope's sheet
    /// open for it.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let Wish::Picker { response, .. } = wish.clone();
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                picker::CopyPicker {
                    response: response.clone(),
                }
                .into()
            }),
            options: crate::sheets::sheet_options(),
        })
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
