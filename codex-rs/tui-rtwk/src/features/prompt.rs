//! The text prompt of a feature (`/rename`, the file name of `/export`, `/goal edit`; oracle
//! `bottom_pane/custom_prompt_view.rs` on the shared-picker layout): a title, the line that says
//! where a suggestion stands, the input behind the `›` mark and the hint. In Flutter it is a
//! `TextField` over a `TextEditingController` in a bottom sheet: the controller, the focus node and
//! whether the user typed are this widget's state (ADR-013: field text before submit is UI state),
//! and the feature hears only the verb the opener makes of the submitted text.
//!
//! A generated suggestion (the title `/rename` asks the host for) is a prop: it fills the field
//! once, and only while the user has not typed (`apply_text_suggestion`).

use std::rc::Rc;

use rtwk::editing::{TextArea, TextEditingController};
use rtwk::focus::{Focus, FocusNode};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::style::Style;
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::fixed;

use crate::app::theme::Tokens;
use crate::keymap::bind_fixed;
use crate::sheets::dialogs::strings;

/// Where the generated suggestion of a prompt stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Suggest {
    /// None is asked for: no label.
    Off,
    /// `Generating a title suggestion…`: the answer is awaited.
    Waiting,
    /// The answer: the text for the field, or nothing came out.
    Ready(Option<String>),
}

/// A text prompt: what it asks, what it starts with, and who hears the submitted text.
#[derive(Clone, Debug, PartialEq)]
pub struct TextPrompt {
    pub title: String,
    pub placeholder: String,
    /// The text the field starts with.
    pub initial: String,
    pub suggest: Suggest,
    /// The trimmed text of a submit; a blank one is not an answer and the prompt stays open
    /// (`CustomPromptView`). The opener makes its verb of it; the prompt closes after.
    pub on_submit: Handler<Rc<str>>,
}

pub struct TextPromptState {
    text: TextEditingController,
    node: FocusNode,
    /// The user typed or pasted since the prompt opened.
    edited: bool,
    /// The suggestion was put in the field.
    applied: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
    /// The field changed by the user.
    Typed,
    /// Enter in the field.
    Submit(Rc<str>),
}

impl StatefulWidget for TextPrompt {
    type State = TextPromptState;

    fn create_state(&self) -> TextPromptState {
        TextPromptState {
            text: TextEditingController::new(""),
            node: FocusNode::new(),
            edited: false,
            applied: false,
        }
    }
}

impl_widget!(stateful eq TextPrompt);

impl TextPromptState {
    /// Puts a ready suggestion in the field, once, while the user has not typed.
    fn apply(&mut self, w: &TextPrompt) {
        if let Suggest::Ready(Some(title)) = &w.suggest
            && !self.applied
            && !self.edited
        {
            self.applied = true;
            self.text.reset(title);
        }
    }
}

impl State for TextPromptState {
    type Widget = TextPrompt;
    type Msg = Msg;

    fn init_state(&mut self, w: &TextPrompt, _cx: &mut Cx<'_, Self>) {
        self.text.reset(&w.initial);
        self.apply(w);
    }

    fn did_update_widget(&mut self, _old: &TextPrompt, w: &TextPrompt, _cx: &mut Cx<'_, Self>) {
        self.apply(w);
    }

    fn dispose(&mut self) {
        self.text.dispose();
    }

    fn update(&mut self, msg: Msg, w: &TextPrompt, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Close => {
                close_dialog::<_, ()>(cx, None);
                Rebuild::No
            }
            Msg::Typed => {
                self.edited = true;
                Rebuild::No
            }
            Msg::Submit(text) if text.trim().is_empty() => Rebuild::No,
            Msg::Submit(text) => {
                cx.emit(&w.on_submit, Rc::from(text.trim()));
                close_dialog::<_, ()>(cx, None);
                Rebuild::No
            }
        }
    }

    fn build(&self, w: &TextPrompt, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let s = strings::of(cx);
        let context = match (&w.suggest, self.applied) {
            (Suggest::Waiting, _) => Some(s.prompt_suggestion_loading),
            (_, true) => Some(s.prompt_suggestion_ready),
            _ => None,
        };
        let gap = || SizedBox::new(None, Some(1), None);
        let sides = |child: AnyWidget| -> AnyWidget {
            Padding::new(EdgeInsets::symmetric(2, 0), child).into()
        };
        let input = TextArea::new()
            .controller(self.text.clone())
            .placeholder(w.placeholder.clone())
            .placeholder_style(tokens.dim())
            .min_lines(1)
            .max_lines(8)
            .focus_node(self.node.clone())
            .autofocus(true)
            .on_changed(cx.map(|_| Msg::Typed))
            .on_submitted(cx.map(Msg::Submit));
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(gap())
            .child(sides(
                Text::new(w.title.clone())
                    .style(tokens.bold())
                    .wrap(TextWrap::Word)
                    .into(),
            ));
        if let Some(context) = context {
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
                    .child(Text::line(s.prompt_mark).style(tokens.accent()))
                    .child(SizedBox::new(Some(1), None, None))
                    .child(Expanded::new(1, input))
                    .child(SizedBox::new(Some(2), None, None)),
            )
            .child(gap())
            .child(Padding::new(
                EdgeInsets::only(2, 0, 0, 0),
                Text::line(s.prompt_hint).style(tokens.dim()),
            ));
        // The text area leaves Esc to the nodes above it.
        let focus = Focus::new(Dialog::bare(
            BoxWidget::new().style(Style::new()).child(column),
        ))
        .can_request_focus(false);
        bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Close)).into()
    }
}
