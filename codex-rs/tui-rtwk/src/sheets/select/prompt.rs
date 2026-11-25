//! The text prompt of the selection sheet (oracle `bottom_pane/custom_prompt_view`): `/review` asks
//! its instructions with it, `/plugins` the source of a marketplace. It is the shared text prompt
//! (`features::prompt::TextPrompt`: the plain composer with Vim, the paste burst and the hint row
//! with the mode of Vim), left open for its owner, which navigates on the answer, and with Esc told
//! to the owner (back to where the prompt was opened from).

use std::rc::Rc;

use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use crate::features::prompt::{Suggest, TextPrompt};

/// What the prompt answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Enter on a text that is not blank (trimmed).
    Submit(String),
    /// Esc: back to where the prompt was opened from.
    Back,
}

/// The words and the answer handler of one prompt.
#[derive(Clone, Debug, PartialEq)]
pub struct Prompt {
    pub title: &'static str,
    /// A line under the title in the accent colour (the oracle's `context_label`).
    pub context: Option<&'static str>,
    pub placeholder: &'static str,
    pub on_answer: Handler<Answer>,
}

pub struct PromptState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Submit(Rc<str>),
    Back,
}

impl StatefulWidget for Prompt {
    type State = PromptState;

    fn create_state(&self) -> PromptState {
        PromptState
    }
}

impl_widget!(stateful eq Prompt);

impl State for PromptState {
    type Widget = Prompt;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &Prompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        let answer = match msg {
            Msg::Back => Answer::Back,
            // A blank text is not an answer: the prompt stays open.
            Msg::Submit(text) if text.trim().is_empty() => return Rebuild::No,
            Msg::Submit(text) => Answer::Submit(text.trim().to_string()),
        };
        cx.emit(&w.on_answer, answer);
        Rebuild::No
    }

    fn build(&self, w: &Prompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let prompt = TextPrompt::new(
            w.title,
            w.placeholder,
            "",
            Suggest::Off,
            cx.map(Msg::Submit),
        )
        .on_cancel(cx.map(|()| Msg::Back))
        .keep_open();
        match w.context {
            Some(context) => prompt.context(context),
            None => prompt,
        }
        .into()
    }
}
