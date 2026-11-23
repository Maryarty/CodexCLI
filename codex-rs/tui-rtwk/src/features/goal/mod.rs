//! The view half of `/goal` (ADR-013): the words of its notes, the two questions over a goal and
//! the edit prompt. The feature itself is `codex_tui_core::features::goal` (the asks of the goal,
//! its notes and the wishes for a question or the prompt); the presenter shows a wish through
//! `shown` and the answer goes back as a verb (`Intent::Resume`, `Replace`, `Edit`).

pub mod sheet;
pub mod strings;

use std::rc::Rc;

use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::goal::{Goal, Note, Wish};
use codex_tui_core::features::{Never, Scope};
use codex_tui_core::ids::{Cwd, ThreadId};

use self::sheet::{EditPrompt, Ask};
use super::{FeatureView, Shown};
use crate::pages::session::strings::Strings as SessionStrings;

/// The two confirmations over a goal (oracle `show_resume_paused_goal_prompt`, the replace
/// confirmation of `thread_goal_actions.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    /// `Resume paused goal?`
    Resume,
    /// `Replace goal?`
    Replace,
}

impl FeatureView for Goal {
    /// A verb, not a sheet: no command opens one.
    type Args = Never<Self>;

    fn args(target: Never<Self>, _cwd: Cwd, _thread: Option<ThreadId>) -> Never<Self> {
        target.absurd()
    }

    fn open<S: 'static>(_cx: &mut Cx<'_, S>, args: Never<Self>) {
        args.absurd()
    }

    /// A question or the prompt opens once for the wish, over the chat, at the bottom like the
    /// oracle's pane; nothing keeps a scope's sheet open for it.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let wish = wish.clone();
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                match wish.clone() {
                    Wish::Resume { thread, objective } => Ask {
                        question: Question::Resume,
                        thread,
                        objective,
                    }
                    .into(),
                    Wish::Replace { thread, objective } => Ask {
                        question: Question::Replace,
                        thread,
                        objective,
                    }
                    .into(),
                    Wish::Edit {
                        thread,
                        objective,
                        status,
                        token_budget,
                    } => EditPrompt {
                        thread,
                        objective,
                        status,
                        token_budget,
                    }
                    .into(),
                }
            }),
            options: crate::sheets::sheet_options(),
        })
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
