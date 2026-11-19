//! The dialog of a presented request (S13): the question, the header, the
//! options and the hint, drawn from `present::body` of the prompt the model
//! holds for the request. Enter, the digits and each option's shortcut answer
//! it; Esc cancels (the oracle's `cancel_current_request`). A request whose
//! prompt the model does not hold gets the skeleton's dismiss dialog.

use std::rc::Rc;

use rtwk::data::{Table, TableCell, TableColumn, TableController, TableSelection};
use rtwk::focus::Focus;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{
    Align, Alignment, BoxWidget, Column, CrossAxisAlignment, EdgeInsets, GridTrack, MainAxisSize, Padding, SizedBox,
};
use rtwk::overlay::{Dialog, close_dialog};
use rtwk::provider::{read, select};
use rtwk::style::{Attrs, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Key, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::RequestQueue;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::approvals::{Decision, Intent, Prompt};

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
    table: TableController,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Answer(usize),
    Cancel,
}

impl StatefulWidget for RequestDialog {
    type State = RequestDialogState;

    fn create_state(&self) -> RequestDialogState {
        RequestDialogState {
            table: TableController::new().with_selected_key(Key::value(0usize)),
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
        let count = i32::try_from(body.options.len()).unwrap_or(i32::MAX);
        let options = self.options(&body, &tokens, cx);
        // An approval's rows start at the edge; a form's are inset like its text.
        let options = if body.form { inset(options) } else { options };
        pane = pane
            .child(SizedBox::new(None, Some(count), Some(options)))
            .child(Text::new(String::new()))
            .child(inset(Text::new(body.hint.clone()).style(tokens.dim())));
        let mut focus = Focus::new(Dialog::bare(Align::new(
            Alignment::BOTTOM_CENTER,
            BoxWidget::new().style(Style::new()).child(pane),
        )))
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

/// One option row: its number and label (and the shortcut shown after it),
/// and the description column of a form.
#[derive(Clone)]
struct Row {
    index: usize,
    text: String,
    description: Option<String>,
}

impl RequestDialogState {
    /// The options as a `Table` of a label column and a description column
    /// (rtwk sizes the columns and the gap); the cursor row is the table's.
    fn options(
        &self,
        body: &Body,
        tokens: &theme::Tokens,
        cx: &mut Cx<'_, Self>,
    ) -> AnyWidget {
        let rows: Vec<Row> = body
            .options
            .iter()
            .enumerate()
            .map(|(index, option)| Row {
                index,
                text: match (&option.key_label, body.form) {
                    (Some(key), false) => format!("{}. {} ({key})", index + 1, option.label),
                    _ => format!("{}. {}", index + 1, option.label),
                },
                description: option.description.clone(),
            })
            .collect();
        let (accent, watch) = (tokens.accent().attrs(Attrs::BOLD), self.table.clone());
        // The cursor mark is a column of its own, two cells wide.
        let mark = TableColumn::new("", move |row: &Row| {
            let (watch, accent, index) = (watch.clone(), accent.clone(), row.index);
            TableCell::widget(ListenableBuilder::new(Rc::new(watch.clone()), move |_, _| {
                let on = watch.selected_key() == Some(Key::value(index));
                Text::new(if on { "\u{203a} " } else { "  " }).style(accent.clone()).into()
            }))
        })
        .width(GridTrack::fixed(2));
        let label = TableColumn::new("", |row: &Row| TableCell::new(row.text.clone()))
            .width(GridTrack::content());
        let description = TableColumn::new("", |row: &Row| {
            TableCell::widget(Padding::new(
                EdgeInsets::only(2, 0, 0, 0),
                Text::new(row.description.clone().unwrap_or_default()),
            ))
        })
        .width(GridTrack::fr(1));
        Table::new(rows, [mark, label, description])
            .row_key(|row| Key::value(row.index))
            .controller(self.table.clone())
            .selection(TableSelection::Row)
            .show_header(false)
            .column_gap(0)
            .cursor_style(tokens.accent().attrs(Attrs::BOLD))
            .selected_style(Style::new())
            .autofocus(true)
            .on_activated(cx.map(|row: Row| Msg::Answer(row.index)))
            .into()
    }
}

fn inset(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child).into()
}
