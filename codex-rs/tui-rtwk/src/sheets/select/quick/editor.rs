//! The editor of a preset (oracle `bottom_pane/custom_prompt_view.rs` with a child flow and
//! `preserve_whitespace`, on the shared-picker layout `picker.rs`): a title, the input behind the `›`
//! mark with the text the preset has (or the composer had) and the hint under both. The text
//! is submitted as typed, whitespace and all; a blank text is not an answer. In Flutter it is a
//! `TextField` over a `TextEditingController(text: initial)` in the same sheet.

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

/// What the editor answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Enter on a text that is not blank: the text as it stands.
    Submit(String),
    /// Esc: back to the page under the editor, or out of the sheet.
    Back,
}

/// The words, the text it opens with and the answer handler of one editor.
#[derive(Clone, Debug, PartialEq)]
pub struct Editor {
    pub title: String,
    pub initial: String,
    pub placeholder: &'static str,
    pub hint: &'static str,
    /// The mark before the input (`›`).
    pub mark: &'static str,
    pub on_answer: Handler<Answer>,
}

pub struct EditorState {
    text: TextEditingController,
    node: FocusNode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Submit(Rc<str>),
    Back,
}

impl StatefulWidget for Editor {
    type State = EditorState;

    fn create_state(&self) -> EditorState {
        EditorState {
            text: TextEditingController::new(&self.initial),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq Editor);

impl State for EditorState {
    type Widget = Editor;
    type Msg = Msg;

    fn dispose(&mut self) {
        self.text.dispose();
    }

    fn update(&mut self, msg: Msg, w: &Editor, cx: &mut Cx<'_, Self>) -> Rebuild {
        let answer = match msg {
            Msg::Back => Answer::Back,
            // A blank text is not an answer: the editor stays open.
            Msg::Submit(text) if text.trim().is_empty() => return Rebuild::No,
            Msg::Submit(text) => Answer::Submit(text.to_string()),
        };
        cx.emit(&w.on_answer, answer);
        Rebuild::No
    }

    fn build(&self, w: &Editor, cx: &mut Cx<'_, Self>) -> AnyWidget {
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
        let column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(gap())
            .child(sides(
                Text::new(w.title.clone())
                    .style(tokens.bold())
                    .wrap(TextWrap::Word)
                    .into(),
            ))
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
