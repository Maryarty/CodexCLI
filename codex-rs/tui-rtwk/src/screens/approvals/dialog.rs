//! The dialog of a presented request (S13): the question, the header, the
//! options and the hint, drawn from `present::body` of the prompt the model
//! holds for the request. Enter, the digits and each option's shortcut answer
//! it; Esc cancels (the oracle's `cancel_current_request`). A request whose
//! prompt the model does not hold gets the skeleton's dismiss dialog.

use std::rc::Rc;

use rtwk::focus::Focus;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::style::{Attrs, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::RequestQueue;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::approvals::{Decision, Intent, Prompt};

use super::dismiss::DismissDialog;
use super::present::{self, Body};
use super::strings;
use crate::theme;
use crate::ui::options::{OptionRow, OptionStyles, Options};
use crate::ui::view::Hs;

#[derive(Clone, Debug, PartialEq)]
pub struct RequestDialog {
    pub key: RequestKey,
}

pub struct RequestDialogState {
    options: Options,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Answer(usize),
    Move(i64),
    Enter,
    Cancel,
}

impl StatefulWidget for RequestDialog {
    type State = RequestDialogState;

    fn create_state(&self) -> RequestDialogState {
        RequestDialogState {
            options: Options::new(),
        }
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

    fn dispose(&mut self) {
        self.options.dispose();
    }

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
            Msg::Move(delta) => {
                self.options.step(delta, decisions.len());
                return Rebuild::No;
            }
            Msg::Enter => decisions.get(self.options.cursor()).cloned(),
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
        let tokens = theme::Tokens::of(cx);
        let s = strings::of(cx);
        let body = present::body(&prompt, s);
        // The oracle's bottom pane: a blank row, the question in bold, the
        // header, the options (the cursor row in the accent colour, marked
        // `›`), a blank row and the hint, all two cells in.
        let mut pane = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::new(String::new()));
        if let Some(title) = &body.title {
            pane = pane
                .child(inset(Text::new(title.clone()).style(tokens.bold())))
                .child(Text::new(String::new()));
        }
        for line in &body.header {
            pane = pane.child(inset(Text::new(line.clone()).wrap(TextWrap::Word)));
        }
        // An approval's rows start at the edge; a form's are inset like its text.
        let options = self.options.view(
            Rc::new(rows_of(&body)),
            OptionStyles {
                cursor: tokens.accent().attrs(Attrs::BOLD),
                label: Style::new(),
                description: Style::new(),
            },
            if body.form { 2 } else { 0 },
        );
        pane = pane
            .child(options)
            .child(Text::new(String::new()))
            .child(inset(Text::new(body.hint.clone()).style(tokens.dim())));
        let mut focus = Focus::new(Dialog::bare(BoxWidget::new().style(Style::new()).child(pane)))
            .can_request_focus(false)
            .bind(rtwk::focus::KeyPress::ESC, cx.send(Msg::Cancel))
            .bind(rtwk::focus::KeyPress::UP, cx.send(Msg::Move(-1)))
            .bind(rtwk::focus::KeyPress::DOWN, cx.send(Msg::Move(1)))
            .bind(rtwk::focus::KeyPress::ENTER, cx.send(Msg::Enter));
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

/// The option rows: number and label (and the shortcut shown after it), and
/// the description column of a form.
fn rows_of(body: &Body) -> Vec<OptionRow> {
    body.options
        .iter()
        .enumerate()
        .map(|(index, option)| OptionRow {
            label: match (&option.key_label, body.form) {
                (Some(key), false) => format!("{}. {} ({key})", index + 1, option.label),
                _ => format!("{}. {}", index + 1, option.label),
            },
            description: option.description.clone(),
        })
        .collect()
}

fn inset(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child).into()
}
