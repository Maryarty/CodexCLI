//! The view half of the limit prompts (ADR-013): the switch prompt (`Approaching rate limits`) and
//! the workspace owner nudge, each a sheet over the chat that the presenter opens once for a wish
//! of the feature (`codex_tui_core::features::rate_limit_prompt`), and the words of the transcript
//! note that the e-mail's answer leaves. What the user picks goes back as a verb; "never show
//! again" is the settings' write (`hide_rate_limit_prompt`).

pub mod sheet;
pub mod strings;

use std::rc::Rc;

use rtwk::AnyWidget;

use codex_tui_core::features::Scope;
use codex_tui_core::features::rate_limit_prompt::{Note, RateLimitPrompt, Wish};

use self::sheet::{Kind, Prompt};
use super::{FeatureView, Shown};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for RateLimitPrompt {
    /// A prompt opens once for the wish, over the chat, at the bottom like the oracle's pane.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let kind = match wish {
            Wish::Switch(candidate) => Kind::Switch(candidate.clone()),
            Wish::Nudge(nudge) => Kind::Nudge(*nudge),
        };
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget { Prompt { kind: kind.clone() }.into() }),
            options: crate::sheets::sheet_options(),
        })
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
