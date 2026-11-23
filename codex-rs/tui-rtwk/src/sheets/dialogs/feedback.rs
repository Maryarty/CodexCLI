//! The feedback flow's two selection steps: "How was this?" and "Include logs
//! and session history?". Oracle: `bottom_pane/feedback_view.rs`
//! (`feedback_selection_params`, `feedback_consent_params`).

use std::rc::Rc;

use rtwk::controls::SelectionList;
use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::overlay::Dialog;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::screens::dialogs::{Attachments, FeedbackCategory};

use super::dialog::{Msg, SmallDialogState};
use super::strings::Strings;
use crate::app::theme::Tokens;

/// A step of the feedback flow: drawn state of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// "How was this?"
    Category,
    /// "Include logs and session history?"
    Consent(FeedbackCategory),
}

/// The category of the picker's row `index`.
pub(super) fn category(index: usize) -> Option<FeedbackCategory> {
    FeedbackCategory::ALL.get(index).copied()
}

pub(super) fn build(
    step: Step,
    attachments: &Attachments,
    s: &Strings,
    cx: &mut Cx<'_, SmallDialogState>,
) -> Focus {
    let tokens = Tokens::of(cx);
    match step {
        Step::Category => {
            let rows = FeedbackCategory::ALL
                .iter()
                .zip(s.feedback_categories.iter())
                .map(|(_, (name, description))| ((*name).to_string(), (*description).to_string()))
                .collect();
            let column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .child(Text::new(String::new()))
                .child(list(rows, cx, Msg::Category))
                .child(Text::new(String::new()))
                .child(Text::line(s.popup_hint).style(tokens.dim()));
            frame(Dialog::new(column).title(s.feedback_title), 5, cx)
        }
        Step::Consent(_) => {
            let rows = vec![
                (
                    s.consent_yes.to_string(),
                    s.consent_yes_description.to_string(),
                ),
                (
                    s.consent_no.to_string(),
                    s.consent_no_description.to_string(),
                ),
            ];
            let disclosure = if attachments.can_save {
                s.consent_disclosure_save
            } else {
                s.consent_disclosure
            };
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Text::new(String::new()))
                .child(Text::new(disclosure.to_string()).style(tokens.dim()));
            for file in files(attachments, s) {
                column = column.child(Text::line(format!("{}{file}", s.consent_bullet)));
            }
            let column = column
                .child(Text::new(String::new()))
                .child(list(rows, cx, |index| Msg::Consent(index == 0)))
                .child(Text::new(String::new()))
                .child(Text::line(s.popup_hint).style(tokens.dim()));
            frame(Dialog::new(column).title(s.consent_title), 2, cx)
        }
    }
}

/// The files the consent step lists, in the oracle's order.
fn files(attachments: &Attachments, s: &Strings) -> Vec<String> {
    let mut files: Vec<String> = s
        .consent_files
        .iter()
        .map(|file| (*file).to_string())
        .collect();
    if attachments.windows_sandbox_log {
        files.push(s.consent_windows_log.to_string());
    }
    files.extend(attachments.rollout.clone());
    files.extend(attachments.auto_review.clone());
    files
}

/// The dialog around a step: Esc closes, the digits pick a row.
fn frame(dialog: Dialog, count: usize, cx: &mut Cx<'_, SmallDialogState>) -> Focus {
    let consent = count == 2;
    let mut focus = Focus::new(dialog)
        .can_request_focus(false)
        .bind(KeyPress::ESC, cx.send(Msg::Close));
    for index in 0..count.min(9) {
        if let Some(digit) = char::from_digit(u32::try_from(index + 1).unwrap_or(0), 10) {
            let msg = if consent {
                Msg::Consent(index == 0)
            } else {
                Msg::Category(index)
            };
            focus = focus.bind(digit, cx.send(msg));
        }
    }
    focus
}

/// The rows `N. name  description`, the descriptions in one wrapped column.
fn list(
    rows: Vec<(String, String)>,
    cx: &mut Cx<'_, SmallDialogState>,
    msg: fn(usize) -> Msg,
) -> AnyWidget {
    let tokens = Tokens::of(cx);
    let dim = tokens.dim();
    let labels: Vec<(String, String)> = rows
        .into_iter()
        .enumerate()
        .map(|(index, (name, description))| (format!("{}. {name}", index + 1), description))
        .collect();
    let name_width = labels
        .iter()
        .map(|row| row.0.chars().count())
        .max()
        .unwrap_or(0);
    let count = labels.len();
    let labels = Rc::new(labels);
    SizedBox::new(
        None,
        Some(i32::try_from(count * 3).unwrap_or(i32::MAX)),
        Some(
            SelectionList::new(
                count,
                |index| index,
                move |index| match labels.get(index) {
                    Some((label, description)) => Row::new()
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .child(SizedBox::new(
                            Some(i32::try_from(name_width + 2).unwrap_or(i32::MAX)),
                            None,
                            Some(Text::new(label.clone()).into()),
                        ))
                        .child(Expanded::new(
                            1,
                            Text::new(description.clone()).style(dim.clone()),
                        ))
                        .into(),
                    None => AnyWidget::default(),
                },
            )
            .on_submitted(cx.map(msg))
            .autofocus(true)
            .into(),
        ),
    )
    .into()
}
