//! The dialog of a presented request (S13): the question, the header, the
//! options and the hint, drawn from `present::body` of the prompt the model
//! holds for the request. Enter, the digits and each option's shortcut answer
//! it; Esc cancels (the oracle's `cancel_current_request`). A request whose
//! prompt the model does not hold gets the skeleton's dismiss dialog.

use std::rc::Rc;

use rtwk::controls::SelectionList;
use rtwk::focus::Focus;
use rtwk::layout::{Column, MainAxisSize, SizedBox};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::app::RequestQueue;
use codex_tui_rtwk_core::ids::RequestKey;
use codex_tui_rtwk_core::screens::ScreenIntent;
use codex_tui_rtwk_core::screens::ScreenStates;
use codex_tui_rtwk_core::screens::approvals::{Decision, Intent, Prompt};

use super::dismiss::DismissDialog;
use super::present::{self, Body};
use super::strings;
use crate::theme;
use crate::ui::view::Hs;

#[derive(Clone, Debug, PartialEq)]
pub struct RequestDialog {
    pub key: RequestKey,
}

pub struct RequestDialogState {
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Answer(usize),
    Cancel,
}

impl StatefulWidget for RequestDialog {
    type State = RequestDialogState;

    fn create_state(&self) -> RequestDialogState {
        RequestDialogState {}
    }
}

impl_widget!(stateful eq RequestDialog);

/// The prompt the model holds for `key`, read where it is drawn or answered.
fn prompt_of(key: &RequestKey, cx: &mut Cx<'_, RequestDialogState>) -> Option<Prompt> {
    let key = key.clone();
    select::<ScreenStates, Option<Prompt>, _>(cx, move |states| {
        states.approvals.prompt(&key.thread, &key.id).cloned()
    })
}

impl State for RequestDialogState {
    type Widget = RequestDialog;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, w: &RequestDialog, cx: &mut Cx<'_, Self>) {
        let key = w.key.clone();
        let queued = select::<RequestQueue, bool, _>(cx, move |queue| queue.contains(&key));
        if !queued {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn update(&mut self, msg: Msg, w: &RequestDialog, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Some(prompt) = prompt_of(&w.key, cx) else {
            return Rebuild::No;
        };
        let decisions = prompt.decisions();
        let decision = match msg {
            Msg::Answer(index) => decisions.get(index).cloned(),
            // The oracle's cancel: the list's cancel option.
            Msg::Cancel => decisions
                .iter()
                .find(|d| matches!(d, Decision::Cancel))
                .cloned(),
        };
        let Some(decision) = decision else {
            return Rebuild::No;
        };
        let hs = read::<Hs, _>(cx);
        cx.emit(
            &hs.screen,
            ScreenIntent::Approvals(Intent::Answer {
                key: w.key.clone(),
                decision,
            }),
        );
        cx.emit(&hs.dismiss, w.key.clone());
        close_dialog(cx, Some(()));
        Rebuild::No
    }

    fn build(&self, w: &RequestDialog, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(prompt) = prompt_of(&w.key, cx) else {
            return DismissDialog { key: w.key.clone() }.into();
        };
        let prompt = &prompt;
        let tokens = theme::Tokens::of(cx);
        let s = strings::of(cx);
        let body = present::body(prompt, s);
        let rows = Rc::new(rows(&body));
        let count = rows.len();
        let list = SelectionList::new(
            count,
            |index| index,
            move |index| match rows.get(index) {
                Some(row) => Text::new(row.clone()).into(),
                None => AnyWidget::default(),
            },
        )
        .on_submitted(cx.map(Msg::Answer))
        .autofocus(true);
        let mut column = Column::new().main_axis_size(MainAxisSize::Min);
        for line in &body.header {
            column = column.child(Text::new(line.clone()));
        }
        let height = i32::try_from(count).unwrap_or(i32::MAX);
        column = column
            .child(Text::new(String::new()))
            .child(SizedBox::new(None, Some(height), Some(list.into())))
            .child(Text::new(String::new()))
            .child(Text::new(body.hint.clone()).style(tokens.dim()));
        let mut dialog = Dialog::new(column).max_width(Some(84));
        if let Some(title) = &body.title {
            dialog = dialog.title(title.clone());
        }
        let mut focus = Focus::new(dialog)
            .can_request_focus(false)
            .bind(rtwk::focus::KeyPress::ESC, cx.send(Msg::Cancel));
        for (index, option) in body.options.iter().enumerate() {
            if let Some(key) = option.key {
                focus = focus.bind(key, cx.send(Msg::Answer(index)));
            }
            if let Some(digit) = char::from_digit(u32::try_from(index + 1).unwrap_or(0), 10) {
                focus = focus.bind(digit, cx.send(Msg::Answer(index)));
            }
        }
        focus.into()
    }
}

/// The option rows: `1. label (key)` for an approval, `1. label  description`
/// with the label column padded for a form.
fn rows(body: &Body) -> Vec<String> {
    let width = body
        .options
        .iter()
        .map(|option| option.label.chars().count())
        .max()
        .unwrap_or(0);
    body.options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let number = index + 1;
            if body.form {
                let pad = " ".repeat(width - option.label.chars().count());
                let description = option.description.as_deref().unwrap_or_default();
                format!("{number}. {}{pad}  {description}", option.label)
            } else {
                match &option.key_label {
                    Some(key) => format!("{number}. {} ({key})", option.label),
                    None => format!("{number}. {}", option.label),
                }
            }
        })
        .collect()
}
