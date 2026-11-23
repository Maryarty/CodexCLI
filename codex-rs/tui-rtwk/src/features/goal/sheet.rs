//! The two questions over a goal and the edit prompt (ADR-013): sheets over the chat that the
//! presenter opens for a wish of the goal feature. In Flutter the question is a
//! `showModalBottomSheet` over a list of two options and the edit a `TextField` in one; what the
//! user decides goes back to the feature as a verb, and a leave or a cancel says nothing.

use std::rc::Rc;

use rtwk::overlay::close_dialog;
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::goal::Intent;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::GoalStatus;

use super::{Question, strings};
use crate::features::prompt::{Suggest, TextPrompt};
use crate::features::verb;
use crate::widgets::picker::{Event, Picker};

/// `Resume paused goal?` or `Replace goal?` over the goal of `thread`.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub question: Question,
    pub thread: ThreadId,
    /// The goal's objective (`Resume`) or the new one (`Replace`).
    pub objective: String,
}

pub struct AskState;

#[derive(Clone, Debug, PartialEq)]
pub enum AskMsg {
    Picked(Event),
}

impl StatefulWidget for Ask {
    type State = AskState;

    fn create_state(&self) -> AskState {
        AskState
    }
}

impl_widget!(stateful eq Ask);

impl State for AskState {
    type Widget = Ask;
    type Msg = AskMsg;

    fn update(&mut self, msg: AskMsg, w: &Ask, cx: &mut Cx<'_, Self>) -> Rebuild {
        let AskMsg::Picked(event) = msg;
        // A leave (`Leave paused`, `Cancel`) and Esc only close.
        if let Event::Choose(id) = &event {
            match (id.as_str(), w.question) {
                (strings::RESUME, Question::Resume) => verb(
                    cx,
                    Intent::Resume {
                        thread: w.thread.clone(),
                    },
                ),
                (strings::REPLACE, Question::Replace) => verb(
                    cx,
                    Intent::Replace {
                        thread: w.thread.clone(),
                        objective: w.objective.clone(),
                    },
                ),
                (strings::LEAVE | strings::CANCEL, _) => {}
                _ => return Rebuild::No,
            }
        } else if !matches!(event, Event::Cancel) {
            return Rebuild::No;
        }
        close_dialog::<_, ()>(cx, None);
        Rebuild::No
    }

    fn build(&self, w: &Ask, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let on: Handler<Event> = cx.map(AskMsg::Picked);
        Picker::new(Rc::new(strings::menu(w.question, &w.objective)), on).into()
    }
}

/// The edit prompt over the objective of `thread`'s goal, which keeps its status and token budget.
#[derive(Clone, Debug, PartialEq)]
pub struct EditPrompt {
    pub thread: ThreadId,
    pub objective: String,
    pub status: GoalStatus,
    pub token_budget: Option<i64>,
}

pub struct EditPromptState;

#[derive(Clone, Debug, PartialEq)]
pub enum EditMsg {
    Submit(Rc<str>),
}

impl StatefulWidget for EditPrompt {
    type State = EditPromptState;

    fn create_state(&self) -> EditPromptState {
        EditPromptState
    }
}

impl_widget!(stateful eq EditPrompt);

impl State for EditPromptState {
    type Widget = EditPrompt;
    type Msg = EditMsg;

    fn update(&mut self, msg: EditMsg, w: &EditPrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        let EditMsg::Submit(text) = msg;
        verb(
            cx,
            Intent::Edit {
                thread: w.thread.clone(),
                objective: text.to_string(),
                status: w.status,
                token_budget: w.token_budget,
            },
        );
        Rebuild::No
    }

    fn build(&self, w: &EditPrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        TextPrompt {
            title: strings::EDIT_TITLE.to_owned(),
            placeholder: strings::EDIT_PLACEHOLDER.to_owned(),
            initial: w.objective.clone(),
            suggest: Suggest::Off,
            on_submit: cx.map(EditMsg::Submit),
        }
        .into()
    }
}
