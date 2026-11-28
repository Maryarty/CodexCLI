//! The scrollback spelling of a shell call (oracle `exec_cell/render.rs`
//! `command_display_lines`): the command wrapped under `  │ `, its output
//! under `  └ `. An agent's call shows the first three output rows and the
//! count of the lines behind them; a call the user ran shows up to fifty rows
//! of its output (the head and the tail, the middle cut by whole screen rows).
//! The inline viewport draws every command this way, the chat only the
//! user's own (the oracle's `compact_hyperlink_lines` is `display_lines` for
//! them).
//!
//! One [`Plan`] holds the rows; the cell paints it with the theme (`parts`) and
//! the text contract reads it without one (`texts`).

use std::borrow::Cow;
use std::ops::Range;

use rtwk::AnyWidget;
use rtwk::style::{Attrs, Span};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk_render::objects::text::{TextWrap, break_lines};
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use codex_tui_core::threads::{ToolActivity, ToolKind, ToolStatus};

use crate::app::theme::Tokens;
use crate::pages::session::strings::INLINE_GAP;

use super::super::strings::Strings;
use super::super::text::{Part, Room};
use super::super::tool_call::text_room;
use super::super::tool_rows::{BRANCH, BULLET, Rows, glyph_row, plain_text};
use super::{Headline, PREVIEW_LINE_BYTES, PREVIEW_LINES, retained_line};

/// Lines of the command under its header (oracle `command_continuation_max_lines`).
const CONTINUATION_LINES: usize = 2;
/// Output rows of a call the user ran (oracle `USER_SHELL_TOOL_CALL_MAX_LINES`).
const USER_SHELL_LINES: usize = 50;
const CONTINUATION: &str = "\u{2502} ";
const BLANK: &str = "  ";
/// The cells of a body row's glyph and the bullet's column before it.
const GUTTER: i32 = 4;

/// Whether a shell call draws as `command_display_lines`.
pub(super) fn applies(head: &Headline<'_>, tool: &ToolActivity, inline: bool) -> bool {
    !head.exploring && (inline || user_shell(tool))
}

fn user_shell(tool: &ToolActivity) -> bool {
    matches!(
        tool.kind,
        ToolKind::Shell {
            user_shell: true,
            ..
        }
    )
}

/// Where the text of a row comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Src {
    /// A line of the command (the styled text is sliced from its highlighting).
    Command(usize),
    /// A line of the output (the styled text is sliced from its colour codes).
    Output(usize),
    /// A hint or a notice: dim plain text.
    Note,
}

/// One drawn row after its glyph.
pub(super) struct Line {
    pub src: Src,
    pub range: Range<usize>,
    /// What the row says, as plain text.
    pub text: String,
}

pub(super) struct Plan {
    pub title: &'static str,
    pub header: Line,
    pub continuation: Vec<Line>,
    pub output: Vec<Line>,
    /// The output lines that rows come from, colour codes in.
    pub raw: Vec<String>,
    /// Something is cut: the command's lines or the output's.
    pub hidden: bool,
}

/// The ranges of `text` as rows `width` cells wide (a blank line is one empty row).
fn split_rows(room: Room, text: &str, width: i32) -> Vec<Range<usize>> {
    if text.trim().is_empty() {
        return std::iter::once(0..0).collect();
    }
    break_lines(&room.segmenter, text, width.max(1), TextWrap::Word)
        .into_iter()
        .map(|(from, to)| from..to)
        .collect()
}

fn note(text: String) -> Line {
    Line {
        src: Src::Note,
        range: 0..text.len(),
        text,
    }
}

/// `… +N lines` with the transcript hint after it (oracle `output_ellipsis_text`).
fn omitted_hint(strings: &Strings, omitted: usize) -> String {
    let count = if omitted == 1 {
        strings.omitted_output_line.to_owned()
    } else {
        strings.label(strings.omitted_output_lines, &omitted.to_string())
    };
    strings.label(strings.inline_repl_hidden, &count)
}

/// `+N lines` with the transcript hint after it (oracle `ToolOutputPreview::finish`).
fn preview_hint(strings: &Strings, omitted: usize) -> String {
    let count = if omitted == 1 {
        strings.repl_hidden_line.to_owned()
    } else {
        strings.label(strings.repl_hidden_lines, &omitted.to_string())
    };
    strings.label(strings.inline_repl_hidden, &count)
}

/// The retained lines of the call's output (oracle `CommandOutput::lines`).
fn lines<'a>(
    tool: &'a ToolActivity,
    strings: &Strings,
) -> Box<dyn DoubleEndedIterator<Item = Cow<'a, str>> + 'a> {
    if let Some(retained) = tool.retained_output() {
        // collected: the labels borrow `strings`, which does not live as long as the tool
        #[allow(clippy::needless_collect)]
        let labelled: Vec<Cow<'a, str>> = retained
            .lines()
            .map(|line| Cow::Owned(retained_line(line, strings)))
            .collect();
        return Box::new(labelled.into_iter());
    }
    Box::new(
        tool.output
            .as_deref()
            .unwrap_or_default()
            .lines()
            .map(Cow::Borrowed),
    )
}

/// `(total, retained)` lines of the call's output (oracle `CommandOutput::line_counts`).
fn counts(tool: &ToolActivity) -> (usize, usize) {
    match tool.retained_output() {
        Some(retained) => (
            retained.retained_lines() + retained.omitted_lines(),
            retained.retained_lines(),
        ),
        None => {
            let total = tool.output.as_deref().unwrap_or_default().lines().count();
            (total, total)
        }
    }
}

/// Whether the call reports an output at all: a finished call always does.
fn has_output(tool: &ToolActivity) -> bool {
    tool.status != ToolStatus::Running || tool.output.is_some() || tool.retained_output().is_some()
}

pub(super) fn plan(
    tool: &ToolActivity,
    head: &Headline<'_>,
    strings: &Strings,
    room: Room,
) -> Plan {
    let width = room.width.max(1);
    let title = if head.status == ToolStatus::Running {
        strings.running_command
    } else if user_shell(tool) {
        strings.user_ran_command
    } else {
        strings.ran_command
    };
    let command: Vec<&str> = head.command.lines().collect();
    let prefix = 2 + i32::try_from(room.segmenter.width(title)).unwrap_or(0) + 1;
    let first = command.first().copied().unwrap_or_default();
    let mut first_rows = split_rows(room, first, width - prefix).into_iter();
    let header_range = first_rows.next().unwrap_or(0..0);
    let header = Line {
        src: Src::Command(0),
        text: first[header_range.clone()].to_owned(),
        range: header_range,
    };
    // the rest of the first line stays as wrapped beside the title; the other lines wrap
    // under the continuation's own gutter
    let mut continuation: Vec<Line> = first_rows
        .map(|range| Line {
            src: Src::Command(0),
            text: first[range.clone()].to_owned(),
            range,
        })
        .collect();
    for (index, line) in command.iter().enumerate().skip(1) {
        for range in split_rows(room, line, width - GUTTER) {
            continuation.push(Line {
                src: Src::Command(index),
                text: line[range.clone()].to_owned(),
                range,
            });
        }
    }
    let hidden = continuation.len() > CONTINUATION_LINES;
    if hidden {
        let cut = continuation.len() - CONTINUATION_LINES;
        continuation.truncate(CONTINUATION_LINES);
        continuation.push(note(if cut == 1 {
            strings.omitted_output_line.to_owned()
        } else {
            strings.label(strings.omitted_output_lines, &cut.to_string())
        }));
    }
    let mut plan = Plan {
        title,
        header,
        continuation,
        output: Vec::new(),
        raw: Vec::new(),
        hidden,
    };
    if !has_output(tool) {
        return plan;
    }
    if user_shell(tool) {
        user_output(tool, strings, room, &mut plan);
    } else {
        preview_output(tool, strings, room, &mut plan);
    }
    // the hint row is the only note beside "(no output)", which has no lines behind it
    let cut_output = !plan.raw.is_empty() && plan.output.iter().any(|row| row.src == Src::Note);
    plan.hidden = hidden || cut_output;
    plan
}

/// An agent's output: three screen rows and what is left (oracle `tool_output_hyperlink_preview`).
fn preview_output(tool: &ToolActivity, strings: &Strings, room: Room, plan: &mut Plan) {
    let width = room.width - GUTTER;
    let (total, _) = counts(tool);
    let mut omitted = 0;
    let mut consumed = 0;
    let mut full = false;
    for line in lines(tool, strings).take(PREVIEW_LINES) {
        consumed += 1;
        let remaining = PREVIEW_LINES - plan.output.len();
        if full || remaining == 0 {
            omitted += 1;
            continue;
        }
        let bounded = &line[..line.floor_char_boundary(PREVIEW_LINE_BYTES)];
        let plain = plain_text(bounded);
        let ranges = split_rows(room, &plain, width);
        if bounded.len() < line.len() || ranges.len() > remaining {
            omitted += 1;
            full = true;
        }
        let at = plan.raw.len();
        plan.raw.push(bounded.to_owned());
        for range in ranges.into_iter().take(remaining) {
            plan.output.push(Line {
                src: Src::Output(at),
                text: plain[range.clone()].to_owned(),
                range,
            });
        }
    }
    omitted += total.saturating_sub(consumed);
    if omitted > 0 {
        plan.output.push(note(preview_hint(strings, omitted)));
    } else if plan.output.is_empty() {
        plan.output.push(note(strings.tool_no_output.to_owned()));
    }
}

/// A user's output: the head and the tail of up to fifty lines each, wrapped, the middle cut
/// by screen rows (oracle `output_lines` and `truncate_lines_middle`).
fn user_output(tool: &ToolActivity, strings: &Strings, room: Room, plan: &mut Plan) {
    let width = room.width - GUTTER;
    let (total, retained) = counts(tool);
    if total == 0 {
        plan.output.push(note(strings.tool_no_output.to_owned()));
        return;
    }
    let head_end = total.min(USER_SHELL_LINES).min(retained);
    let tail_len = total
        .saturating_sub(head_end)
        .min(USER_SHELL_LINES)
        .min(retained.saturating_sub(head_end));
    let omitted = total.saturating_sub(head_end + tail_len);
    let omitted = (omitted > 0).then_some(omitted);
    let mut tail: Vec<Cow<'_, str>> = lines(tool, strings).rev().take(tail_len).collect();
    tail.reverse();
    let mut rows: Vec<Line> = Vec::new();
    let push = |plan: &mut Plan, rows: &mut Vec<Line>, line: &str| {
        let plain = plain_text(line);
        let at = plan.raw.len();
        plan.raw.push(line.to_owned());
        for range in split_rows(room, &plain, width) {
            rows.push(Line {
                src: Src::Output(at),
                text: plain[range.clone()].to_owned(),
                range,
            });
        }
    };
    for line in lines(tool, strings).take(head_end) {
        push(plan, &mut rows, &line);
    }
    if let Some(omitted) = omitted {
        let text = omitted_hint(strings, omitted);
        for range in split_rows(room, &text, width) {
            rows.push(note(text[range].to_owned()));
        }
    }
    for line in &tail {
        push(plan, &mut rows, line);
    }
    plan.output = cut_middle(rows, strings, room, omitted);
}

/// Whole screen rows, a head and a tail around one hint (oracle `truncate_lines_middle`).
fn cut_middle(
    rows: Vec<Line>,
    strings: &Strings,
    room: Room,
    omitted_hint_count: Option<usize>,
) -> Vec<Line> {
    if rows.len() <= USER_SHELL_LINES {
        return rows;
    }
    let width = room.width - GUTTER;
    let had = usize::from(omitted_hint_count.is_some());
    let estimated = omitted_hint_count.unwrap_or(0) + rows.len().saturating_sub(had);
    // the hint wraps like the output beside it, and the rows it takes come off the budget
    let hint = |count: usize| -> Vec<Line> {
        let text = omitted_hint(strings, count);
        split_rows(room, &text, width)
            .into_iter()
            .map(|range| note(text[range].to_owned()))
            .collect()
    };
    let cost = hint(estimated).len().max(1);
    if cost >= USER_SHELL_LINES {
        return hint(estimated);
    }
    let available = USER_SHELL_LINES - cost;
    let head = available / 2;
    let tail = available - head;
    let head = head.min(rows.len());
    let tail = tail.min(rows.len() - head);
    let additional = rows.len().saturating_sub(head + tail).saturating_sub(had);
    let split = rows.len() - tail;
    let mut rows = rows;
    let tail_rows = rows.split_off(split);
    rows.truncate(head);
    rows.extend(hint(omitted_hint_count.unwrap_or(0) + additional));
    rows.extend(tail_rows);
    rows
}

/// The spans of `range` of the text `spans` make up.
fn slice(spans: &[Span], range: &Range<usize>) -> Vec<Span> {
    let mut out = Vec::new();
    let mut at = 0;
    for span in spans {
        let end = at + span.text.len();
        let (from, to) = (range.start.max(at), range.end.min(end));
        if from < to {
            out.push(Span::styled(
                span.text[from - at..to - at].to_owned(),
                span.style.clone(),
            ));
        }
        at = end;
    }
    out
}

fn single_row(spans: Vec<Span>) -> AnyWidget {
    Text::rich(spans)
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into()
}

/// The cell's rows, painted.
pub(super) fn rows(
    plan: &Plan,
    head: &Headline<'_>,
    tool: &ToolActivity,
    tokens: &Tokens,
    syntax: &SyntaxTheme,
    disclosure: Option<String>,
) -> Rows {
    let code = Highlighter::new(language_named("bash"), &head.command);
    let command_spans = |index: usize| -> Vec<Span> {
        syntax
            .spans(code.line(index), code.tokens(index))
            .into_iter()
            .map(|span| Span::styled(span.text, span.style))
            .collect()
    };
    let dim = tokens.dim().attrs(Attrs::DIM);
    let raw: Vec<Vec<Span>> = plan
        .raw
        .iter()
        .map(|line| {
            rtwk::ansi::parse_ansi(line)
                .into_iter()
                .flatten()
                .map(|mut span| {
                    span.style.attrs |= Attrs::DIM;
                    span
                })
                .collect()
        })
        .collect();
    let paint = |line: &Line| -> Vec<Span> {
        match line.src {
            Src::Command(index) => slice(&command_spans(index), &line.range),
            Src::Output(index) => slice(&raw[index], &line.range),
            Src::Note => vec![Span::styled(line.text.clone(), dim.clone())],
        }
    };
    let mut header = vec![
        Span::styled(plan.title.to_owned(), tokens.bold()),
        Span::new(INLINE_GAP),
    ];
    header.extend(paint(&plan.header));
    let mut body: Vec<AnyWidget> = Vec::new();
    for line in &plan.continuation {
        body.push(glyph_row(
            CONTINUATION,
            single_row(paint(line)),
            dim.clone(),
        ));
    }
    for (index, line) in plan.output.iter().enumerate() {
        let glyph = if index == 0 { BRANCH } else { BLANK };
        let row = if line.src == Src::Note {
            crate::widgets::dots::dots_end(line.text.clone(), dim.clone())
        } else {
            single_row(paint(line))
        };
        body.push(glyph_row(glyph, row, dim.clone()));
    }
    let mark = match head.status {
        ToolStatus::Running => tokens.accent().attrs(Attrs::BOLD),
        _ if head.failed.is_some() || tool_failed(tool) => tokens.error().attrs(Attrs::BOLD),
        ToolStatus::Completed => tokens.success().attrs(Attrs::BOLD),
        ToolStatus::Failed | ToolStatus::Declined => tokens.error().attrs(Attrs::BOLD),
    };
    Rows {
        mark,
        header: single_row(header),
        body,
        disclosure,
    }
}

fn tool_failed(tool: &ToolActivity) -> bool {
    matches!(tool.status, ToolStatus::Failed | ToolStatus::Declined)
}

/// The parts of the cell as [`rows`] mounts them, in tree order (`text_contract`).
pub(super) fn texts(plan: &Plan, room: Room, disclosure: Option<String>) -> Vec<Part> {
    let mut parts = vec![
        Part::gutter(BULLET),
        Part::text(format!("{}{INLINE_GAP}{}", plan.title, plan.header.text)),
    ];
    for line in &plan.continuation {
        parts.push(Part::gutter(CONTINUATION));
        parts.push(Part::text(line.text.clone()));
    }
    let room = text_room(room);
    for (index, line) in plan.output.iter().enumerate() {
        let glyph = if index == 0 { BRANCH } else { BLANK };
        parts.push(Part::gutter(glyph));
        parts.push(Part::text(line.text.clone()));
        if line.src == Src::Note && room.after(glyph).cuts(&line.text) {
            parts.push(Part::gutter("\u{2026}"));
        }
    }
    parts.extend(disclosure.map(Part::gutter));
    parts
}
