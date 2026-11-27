//! The view half of the Windows sandbox prompts (ADR-013): the words of its notes. The feature is
//! `codex_tui_core::features::windows_sandbox` (the sandbox facts, the prompt that is due and its
//! `Page` data, the setup that runs, the verbs). The two prompt sheets (enable, fallback: header
//! with the learn-more link, the rows of `State::page`, Esc coming back while `choice_required`)
//! are `sheet.rs`, shown for the feature's `Wish::Prompt`. The `Setting up sandbox...` status and
//! the disabled composer while `State::setting_up` are the session page's
//! (`pages/session/activity.rs`, `session/slot.rs`, with the words of `strings.rs`); the
//! `(non-admin sandbox)` row label and the `/setup-default-sandbox` hint of the permissions page
//! (`State::degraded`, `State::elevate_hint`) are `sheets/select/permissions.rs`'s.

pub mod sheet;
pub mod strings;

use std::rc::Rc;

use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::Scope;
use codex_tui_core::features::windows_sandbox::{Intent, Note, Sheet, WindowsSandbox, Wish};
use codex_tui_core::ids::{Cwd, ThreadId};

use self::sheet::PromptSheet;
use super::{FeatureSheet, FeatureView, Shown};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for WindowsSandbox {
    /// The prompt that fell due opens once for the wish, over the chat, at the bottom like the
    /// oracle's pane.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let (prompt, owner) = match wish {
            Wish::Prompt(prompt) => (*prompt, None),
            Wish::OwnedPrompt { prompt, token } => (*prompt, Some(token.clone())),
        };
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                PromptSheet {
                    prompt,
                    owner: owner.clone(),
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

impl FeatureSheet for WindowsSandbox {
    /// `/setup-default-sandbox` opens no sheet: it is the verb.
    type Args = ();

    fn args(_target: Sheet, _cwd: Cwd, _thread: Option<ThreadId>) {}

    fn open<S: 'static>(cx: &mut Cx<'_, S>, _args: ()) {
        super::verb(cx, Intent::Elevate);
    }
}
