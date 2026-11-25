//! What the user sent while a turn runs and has not gone out yet, above the
//! composer (the oracle's `bottom_pane/pending_input_preview.rs`): the steers
//! the server has not echoed, the steers it refused, then the queued messages,
//! each section a `• ` header over `↳` rows (at most three rows of a message,
//! then a row with `…`), and the edit hint with the bound key under the queue.
//! The thread model owns all three lists; the widget keeps nothing.
//!
//! Flutter: a `Column` of sections; a section is a `Row` of a hanging prefix
//! and an `Expanded` `Text`; the rows a message takes are measured under a
//! `LayoutBuilder` (`TextPainter.computeLineMetrics`).

use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::provider::select;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, StatelessWidget, impl_widget};

use codex_tui_core::features::async_questions::reply;
use codex_tui_core::keymap::actions::{CHAT_EDIT_QUEUED_MESSAGE, CHAT_INTERRUPT_TURN};
use codex_tui_core::threads::ThreadModel;

use crate::widgets::measure;

use super::strings::{self, Strings};

/// The rows of one message the preview shows; more end in a row with `…`.
const PREVIEW_LINE_LIMIT: usize = 3;

/// The lines of one message the preview reads (`take(PREVIEW_LINE_LIMIT + 1)`).
const SOURCE_LINES: usize = PREVIEW_LINE_LIMIT + 1;

/// The three lists of the preview, as texts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::pages::session) struct Pending {
    steers: Vec<String>,
    rejected: Vec<String>,
    queued: Vec<String>,
}

impl Pending {
    pub(in crate::pages::session) fn of(thread: &ThreadModel) -> Pending {
        Pending {
            steers: thread
                .pending_steers()
                .iter()
                .map(|steer| shown(&steer.message().text))
                .collect(),
            rejected: thread
                .rejected_steers()
                .iter()
                .map(|message| shown(&message.text))
                .collect(),
            queued: thread
                .queue()
                .iter()
                .map(|message| shown(&message.text))
                .collect(),
        }
    }

    pub(in crate::pages::session) fn is_empty(&self) -> bool {
        self.steers.is_empty() && self.rejected.is_empty() && self.queued.is_empty()
    }
}

/// What the preview shows of a message: an answer to an async question as the question quoted
/// over the answer.
fn shown(text: &str) -> String {
    reply::display_text(text).unwrap_or_else(|| text.to_owned())
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::pages::session) struct PendingInput {
    /// Questions wait: the queue's header stays on show, and the edit hint gives way to theirs.
    pub questions: bool,
}

impl_widget!(stateless eq PendingInput);

impl StatelessWidget for PendingInput {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let pending = select::<ThreadModel, Pending, _>(cx, Pending::of);
        if pending.is_empty() && !self.questions {
            return SizedBox::shrink().into();
        }
        let questions = self.questions;
        let strings = strings::of(cx);
        let keymap = crate::keymap::keymap(cx);
        let interrupt = crate::keymap::label(&keymap, CHAT_INTERRUPT_TURN);
        let edit = crate::keymap::label(&keymap, CHAT_EDIT_QUEUED_MESSAGE);
        LayoutBuilder::new(move |cx, constraints| {
            // the oracle draws nothing under four columns
            if constraints.max_width < 4 {
                return SizedBox::shrink().into();
            }
            let seg = segmenter_of(cx);
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let mut column = Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch);
            let dim = super::dim();
            let mut first = true;
            let mut section = |column: Column, header: Vec<Span>, rows: Vec<AnyWidget>| {
                let mut column = column;
                if !first {
                    column = column.child(SizedBox::new(None, Some(1), None));
                }
                first = false;
                column = column.child(header_row(strings, &dim, header));
                for row in rows {
                    column = column.child(row);
                }
                column
            };
            if !pending.steers.is_empty() {
                let mut header = vec![Span::styled(strings.steers_header, Style::new())];
                if let Some(key) = &interrupt {
                    header.push(Span::styled(strings.steers_press, dim.clone()));
                    header.push(Span::styled(key.clone(), super::bold()));
                    header.push(Span::styled(strings.steers_interrupt, dim.clone()));
                }
                let rows = pending
                    .steers
                    .iter()
                    .map(|text| message_rows(seg, width, strings, text, dim.clone(), dim.clone()))
                    .collect();
                column = section(column, header, rows);
            }
            if !pending.rejected.is_empty() {
                let header = vec![Span::styled(strings.rejected_header, Style::new())];
                let rows = pending
                    .rejected
                    .iter()
                    .map(|text| message_rows(seg, width, strings, text, dim.clone(), dim.clone()))
                    .collect();
                column = section(column, header, rows);
            }
            if !pending.queued.is_empty() || questions {
                let header = vec![Span::styled(strings.queued_header, Style::new())];
                let italic = dim.clone().attrs(Attrs::ITALIC);
                let rows = pending
                    .queued
                    .iter()
                    .map(|text| {
                        message_rows(seg, width, strings, text, dim.clone(), italic.clone())
                    })
                    .collect();
                column = section(column, header, rows);
                if let Some(key) = edit.as_ref().filter(|_| !questions) {
                    column = column.child(Row::new().child(Text::rich_line(vec![
                        Span::styled(strings.item_hang, Style::new()),
                        Span::styled(key.clone(), super::bold()),
                        Span::styled(strings.edit_rest, dim.clone()),
                    ])));
                }
            }
            column.into()
        })
        .into()
    }
}

/// `• ` and the header, which wraps under a two-column indent (the oracle's
/// `subsequent_indent`).
fn header_row(strings: &Strings, dim: &Style, header: Vec<Span>) -> AnyWidget {
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(strings.bullet.trim_end()).style(dim.clone()))
        .child(SizedBox::new(Some(1), None, None))
        .child(Expanded::new(1, Text::rich(header).wrap(TextWrap::Word)))
        .into()
}

/// One message: `  ↳ ` and its first lines, wrapped under a four-column indent,
/// at most three rows, then `    …` when more were cut.
fn message_rows(
    seg: measure::Measure,
    width: usize,
    strings: &Strings,
    text: &str,
    prefix: Style,
    style: Style,
) -> AnyWidget {
    let hang = strings.item_hang.len();
    let room = width.saturating_sub(hang).max(1);
    let lines: Vec<&str> = text.lines().take(SOURCE_LINES).collect();
    let rows: usize = lines
        .iter()
        .map(|line| measure::rows(seg, line, room).len().max(1))
        .sum();
    let shown = lines.join("\n");
    // the mark sits on the first row only; the rows under it are indent
    let mark = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .child(Text::line(strings.item_prefix.trim_end()).style(prefix));
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(SizedBox::new(
                    Some(i32::try_from(hang).unwrap_or(4)),
                    None,
                    Some(mark.into()),
                ))
                .child(Expanded::new(
                    1,
                    Text::new(shown)
                        .style(style.clone())
                        .wrap(TextWrap::Word)
                        .max_lines(Some(PREVIEW_LINE_LIMIT))
                        .overflow(TextOverflow::Clip),
                )),
        );
    if rows > PREVIEW_LINE_LIMIT {
        column = column
            .child(Text::line(format!("{}{}", strings.item_hang, strings.ellipsis)).style(style));
    }
    column.into()
}
