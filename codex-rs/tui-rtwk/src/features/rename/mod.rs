//! The view half of `/rename` (ADR-013): the name prompt and the words of its notes. The feature
//! itself is `codex_tui_core::features::rename` (the checks of a bare `/rename`, the title
//! suggestion kept by thread, the rename a submit makes); the presenter shows its wish through
//! `shown` and the submitted name goes back as `Intent::Set`.

pub mod prompt;
pub mod strings;

use std::rc::Rc;

use rtwk::AnyWidget;

use codex_tui_core::features::Scope;
use codex_tui_core::features::rename::{Note, Rename, Wish};

use self::prompt::NamePrompt;
use super::{FeatureView, Shown};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Rename {
    /// The prompt opens once for the wish, over the chat, at the bottom like the oracle's pane.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let Wish::Name {
            thread,
            generation,
            existing,
        } = wish.clone();
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                NamePrompt {
                    thread: thread.clone(),
                    generation,
                    existing: existing.clone(),
                }
                .into()
            }),
            options: crate::sheets::sheet_options(),
        })
    }

    fn note_words(note: &Note, words: &SessionStrings) -> String {
        strings::note(note, words)
    }
}
