//! The text prompt of the selection sheet (oracle `bottom_pane/custom_prompt_view`, the shared-picker
//! layout `picker.rs`): a title, an optional line that says what the text is for, the input behind the
//! `›` mark and the hint under both. `/review` asks its instructions with it, `/plugins` the source
//! of a marketplace. In Flutter it is a `TextField` over a `TextEditingController` in the same sheet.

use std::rc::Rc;

use rtwk::editing::{TextArea, TextEditingController};
use rtwk::focus::{Focus, FocusNode};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::overlay::Dialog;
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;

use crate::app::theme::Tokens;
use crate::keymap::bind_fixed;

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
    pub hint: &'static str,
    /// The mark before the input (`›`).
    pub mark: &'static str,
    pub on_answer: Handler<Answer>,
}

pub struct PromptState {
    text: TextEditingController,
    node: FocusNode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Submit(Rc<str>),
    Back,
}

impl StatefulWidget for Prompt {
    type State = PromptState;

    fn create_state(&self) -> PromptState {
        PromptState {
            text: TextEditingController::new(""),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq Prompt);

impl State for PromptState {
    type Widget = Prompt;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.text.dispose();
    }

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
        let tokens = Tokens::of(cx);
        let gap = || SizedBox::new(None, Some(1), None);
        let input = TextArea::new()
            .controller(self.text.clone())
            .placeholder(w.placeholder)
            .placeholder_style(tokens.dim())
            .min_lines(1)
            .max_lines(8)
            .focus_node(self.node.clone())
            .autofocus(true)
            .on_submitted(cx.map(Msg::Submit));
        let inset = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::only(2, 0, 0, 0), child).into()
        };
        let sides = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::symmetric(2, 0), child).into()
        };
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(gap())
            .child(sides(
                Text::new(w.title)
                    .style(tokens.bold())
                    .wrap(TextWrap::Word)
                    .into(),
            ));
        if let Some(context) = w.context {
            column = column.child(sides(
                Text::new(context)
                    .style(tokens.accent())
                    .wrap(TextWrap::Word)
                    .into(),
            ));
        }
        let column = column
            .child(gap())
            .child(
                Row::new()
                    .child(Text::line(w.mark).style(tokens.accent()))
                    .child(SizedBox::new(Some(1), None, None))
                    .child(Expanded::new(1, input))
                    .child(SizedBox::new(Some(2), None, None)),
            )
            .child(gap())
            .child(inset(Text::line(w.hint).style(tokens.dim()).into()));
        // The text area leaves Esc to the nodes above it.
        let focus = Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(column),
        ))
        .can_request_focus(false);
        bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Back)).into()
    }
}
