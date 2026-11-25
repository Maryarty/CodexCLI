//! The feedback flow's two selection steps: "How was this?" and "Include logs
//! and session history?". Oracle: `bottom_pane/feedback_view.rs`
//! (`feedback_selection_params`, `feedback_consent_params`). The words are the dialogs' table.

use std::rc::Rc;

use rtwk::LayoutBuilder;
use rtwk::controls::SelectionList;
use rtwk::focus::Focus;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::overlay::Dialog;
use rtwk::text::segmenter_of;
use rtwk::style::Span;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::feedback;
use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::dialogs::{Attachments, FeedbackCategory};

use super::sheet::{Msg, SheetState};
use crate::app::theme::Tokens;
use crate::sheets::dialogs::strings::Strings;
use crate::keymap::bind_fixed;
use crate::widgets::measure;

/// A step of the feedback flow: drawn state of the sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// "How was this?"
    Category,
    /// "Include logs and session history?"
    Consent(FeedbackCategory),
    /// "Tell us more": the note, with or without the logs.
    Note {
        category: FeedbackCategory,
        include_logs: bool,
    },
}

/// The category of the picker's row `index`.
pub(super) fn category(index: usize) -> Option<FeedbackCategory> {
    FeedbackCategory::ALL.get(index).copied()
}

pub(super) fn build(
    step: Step,
    attachments: &Attachments,
    s: &Strings,
    cx: &mut Cx<'_, SheetState>,
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
        Step::Note { .. } => Focus::new(Text::new(String::new())),
        Step::Consent(category) => {
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
            if feedback::shows_connectivity(category, attachments.diagnostics.len()) {
                column = column
                    .child(Text::new(String::new()))
                    .child(Text::rich(vec![
                        Span::new(s.consent_connectivity_title).bold(),
                    ]));
                for diagnostic in &attachments.diagnostics {
                    column = column.child(Text::line(format!(
                        "{}{}",
                        s.consent_headline_bullet, diagnostic.headline
                    )));
                    for detail in &diagnostic.details {
                        column = column.child(Text::rich(vec![
                            Span::styled(s.consent_detail_bullet, tokens.dim()),
                            Span::new(detail.clone()),
                        ]));
                    }
                }
            }
            // two blank rows between the files and the list, as the oracle's header and its list
            // leave them
            let column = column
                .child(Text::new(String::new()))
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
    if !attachments.diagnostics.is_empty() {
        files.push(s.consent_diagnostics_file.to_string());
    }
    files
}

/// The dialog around a step: Esc closes, the digits pick a row.
fn frame(dialog: Dialog, count: usize, cx: &mut Cx<'_, SheetState>) -> Focus {
    let consent = count == 2;
    let mut focus = bind_fixed(
        Focus::new(dialog).can_request_focus(false),
        fixed::DISMISS,
        cx.send(Msg::Close),
    );
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
    cx: &mut Cx<'_, SheetState>,
    msg: fn(usize) -> Msg,
) -> AnyWidget {
    let tokens = Tokens::of(cx);
    let dim = tokens.dim();
    let labels: Vec<(String, String)> = rows
        .into_iter()
        .enumerate()
        .map(|(index, (name, description))| (format!("{}. {name}", index + 1), description))
        .collect();
    let segmenter = segmenter_of(cx);
    let name_width = labels
        .iter()
        .map(|row| segmenter.width(&row.0))
        .max()
        .unwrap_or(0);
    let count = labels.len();
    let labels = Rc::new(labels);
    let on_submitted = cx.map(msg);
    // as tall as its rows are: a description that wraps is more than one (the oracle's rows are
    // as tall as their text), so the hint under the list follows it
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let room = usize::try_from(constraints.max_width)
            .unwrap_or(1)
            .saturating_sub(1 + name_width + 2)
            .max(1);
        let height: usize = labels
            .iter()
            .map(|(_, description)| measure::rows(seg, description, room).len().max(1))
            .sum();
        let (labels, dim) = (labels.clone(), dim.clone());
        SizedBox::new(
            None,
            Some(i32::try_from(height).unwrap_or(i32::MAX)),
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
                .on_submitted(on_submitted.clone())
                .autofocus(true)
                .into(),
            ),
        )
        .into()
    })
    .into()
}
