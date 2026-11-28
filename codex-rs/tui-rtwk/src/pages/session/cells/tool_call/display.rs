//! The scrollback spelling of a standard MCP call (oracle `mcp.rs`
//! `render_hyperlink_lines`, display mode): `Called` and the whole invocation
//! on one row when it fits the room, else `Called` alone and the invocation
//! wrapped under `└ ` with a four-column hanging indent; under it the result's
//! bounded preview (three rows, then the count of the lines behind them and
//! the transcript hint). A code-mode call keeps its compact title
//! (`tool_call.rs` `inline_preview`). The inline viewport draws every call
//! this way, the owned transcript the compact preview.
//!
//! One [`Plan`] holds the rows; the cell paints it with the theme (`rows`)
//! and the text contract reads it without one (`texts`).

use std::ops::Range;

use rtwk::AnyWidget;
use rtwk::style::{Attrs, Span};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk_render::objects::text::{TextWrap, break_lines};

use codex_tui_core::threads::{McpCall, McpResultKind};

use crate::app::theme::Tokens;
use crate::pages::session::strings::INLINE_GAP;

use super::super::strings::{INVOCATION_CLOSE, INVOCATION_DOT, INVOCATION_OPEN, Strings};
use super::super::text::{Part, Room};
use super::super::tool_rows::{BRANCH, BULLET, Rows, glyph_row, result_row};
use super::{expanded_verb, inline_preview, invocation_inline, result_parts};

/// The blanks before a wrapped invocation row: the branch's two cells and the indent's four.
const HANGING: &str = "      ";
/// The cells of the branch glyph, and of it and the indent.
const BRANCH_CELLS: i32 = 2;
const HANGING_CELLS: i32 = 6;

/// Whether a call draws as the display form: any standard call of the inline viewport.
pub(in crate::pages::session) fn applies(call: &McpCall, inline: bool) -> bool {
    inline && call.result_kind() == McpResultKind::Standard
}

pub(super) struct Plan {
    verb: &'static str,
    /// `server.tool(arguments)`.
    invocation: String,
    /// The invocation shares the verb's row.
    beside: bool,
    /// The rows of the invocation under the verb (empty when it is beside it).
    under: Vec<Range<usize>>,
    /// The bounded preview of the result, one string per row.
    result: Vec<String>,
}

/// The rows of `call` in `room` (the room inside the bullet's column).
pub(super) fn plan(call: &McpCall, strings: &Strings, room: Room) -> Plan {
    let verb = expanded_verb(call, strings);
    let args = call.arguments.to_string();
    let invocation = format!(
        "{}{INVOCATION_DOT}{}{INVOCATION_OPEN}{args}{INVOCATION_CLOSE}",
        call.server, call.tool,
    );
    let beside = invocation_inline(call, &args, verb, room);
    let under = if beside {
        Vec::new()
    } else {
        rows_under(&invocation, room)
    };
    Plan {
        verb,
        invocation,
        beside,
        under,
        result: inline_preview(call, strings, room.after(BRANCH)).rows,
    }
}

/// The rows of `text` as `adaptive_wrap_hyperlink_lines` cuts it under the verb: the first one
/// after `└ `, the others after a hanging indent.
fn rows_under(text: &str, room: Room) -> Vec<Range<usize>> {
    let mut rows = Vec::new();
    let mut from = 0;
    let mut width = room.width.saturating_sub(BRANCH_CELLS).max(1);
    while from < text.len() {
        let Some((start, end)) = break_lines(&room.segmenter, &text[from..], width, TextWrap::Word)
            .into_iter()
            .next()
        else {
            break;
        };
        rows.push(from + start..from + end);
        let rest = &text[from + end..];
        let next = text.len() - rest.trim_start().len();
        if next <= from {
            break;
        }
        from = next;
        width = room.width.saturating_sub(HANGING_CELLS).max(1);
    }
    if rows.is_empty() {
        rows.push(0..0);
    }
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

/// The invocation, painted: the server and the tool in the accent, the arguments dim.
fn invocation_spans(call: &McpCall, tokens: &Tokens) -> Vec<Span> {
    vec![
        Span::styled(call.server.clone(), tokens.accent()),
        Span::new(INVOCATION_DOT),
        Span::styled(call.tool.clone(), tokens.accent()),
        Span::new(INVOCATION_OPEN),
        Span::styled(call.arguments.to_string(), tokens.dim().attrs(Attrs::DIM)),
        Span::new(INVOCATION_CLOSE),
    ]
}

/// The cell's rows, painted; no control (the inline viewport has none).
pub(super) fn rows(call: &McpCall, plan: &Plan, tokens: &Tokens) -> Rows {
    let dim = tokens.dim().attrs(Attrs::DIM);
    let spans = invocation_spans(call, tokens);
    let mut body: Vec<AnyWidget> = Vec::new();
    let header: AnyWidget = if plan.beside {
        let mut header = vec![
            Span::styled(plan.verb, tokens.bold()),
            Span::new(INLINE_GAP),
        ];
        header.extend(spans);
        Text::rich(header).into()
    } else {
        for (index, range) in plan.under.iter().enumerate() {
            let glyph = if index == 0 { BRANCH } else { HANGING };
            body.push(glyph_row(
                glyph,
                single_row(slice(&spans, range)),
                dim.clone(),
            ));
        }
        Text::line(plan.verb).style(tokens.bold()).into()
    };
    let first = usize::from(!plan.beside);
    for (index, text) in plan.result.iter().enumerate() {
        body.push(result_row(index + first, text.clone(), tokens));
    }
    Rows {
        mark: super::mcp_mark(call, tokens),
        header,
        body,
        disclosure: None,
    }
}

/// The parts of the cell as [`rows`] mounts them, in tree order (`text_contract`).
pub(super) fn texts(plan: &Plan, room: Room) -> Vec<Part> {
    let mut parts = vec![Part::gutter(BULLET)];
    if plan.beside {
        parts.push(Part::text(format!(
            "{}{INLINE_GAP}{}",
            plan.verb, plan.invocation
        )));
    } else {
        parts.push(Part::text(plan.verb));
        for (index, range) in plan.under.iter().enumerate() {
            parts.push(Part::gutter(if index == 0 { BRANCH } else { HANGING }));
            parts.push(Part::text(plan.invocation[range.clone()].to_owned()));
        }
    }
    let first = usize::from(!plan.beside);
    for (index, text) in plan.result.iter().enumerate() {
        result_parts(index + first, text.clone(), Some(room), &mut parts);
    }
    parts
}
