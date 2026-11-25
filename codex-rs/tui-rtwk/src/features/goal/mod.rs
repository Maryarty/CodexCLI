//! The view half of `/goal` (ADR-013): the words of its notes, the two questions over a goal and
//! the edit prompt. The feature itself is `codex_tui_core::features::goal` (the asks of the goal,
//! its notes and the wishes for a question or the prompt); the presenter shows a wish through
//! `shown` and the answer goes back as a verb (`Intent::Resume`, `Replace`, `Edit`).

pub mod cell;
pub mod sheet;
pub mod strings;

use std::rc::Rc;

use rtwk::AnyWidget;

use codex_tui_core::features::Scope;
use codex_tui_core::features::goal::{Goal, Note, Wish};

use self::sheet::{Ask, EditPrompt};
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

    /// A message with a hint is one wrapped line, not two rows; the bare `/goal` summary is a plain cell of rows (oracle `add_plain_history_lines`), not
    /// a bullet and its message.
    fn cell(note: &Note) -> Option<AnyWidget> {
        match note {
            Note::Summary(goal) => Some(cell::SummaryCell { goal: goal.clone() }.into()),
            Note::Usage(_) | Note::Status(_) | Note::NothingToClear => {
                Some(cell::HintCell { note: note.clone() }.into())
            }
            _ => None,
        }
    }
}
