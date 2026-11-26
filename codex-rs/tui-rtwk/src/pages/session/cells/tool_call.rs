//! MCP and dynamic tool calls (oracle `history_cell/{mcp,mcp_preview,
//! mcp_result,dynamic}.rs`): the collapsed form is the header and the last
//! three result rows (`compact_hyperlink_lines`), the expanded one the whole
//! invocation and result (`transcript_hyperlink_lines`). Flutter: an
//! `ExpansionTile` whose title is the header and whose children are the rows;
//! the bullet, the control and the flag are the row's.

use std::borrow::Cow;

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{TextOverflow, segmenter_of};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::threads::{
    BlockBody, DynamicCall, DynamicOutput, McpCall, McpResultKind, ToolStatus,
};

use crate::app::theme::Tokens;

use super::strings::{INVOCATION_CLOSE, INVOCATION_DOT, INVOCATION_OPEN, Strings};
use super::text::{Env as TextEnv, Part, Room};
use super::tool_rows::{
    BRANCH, BULLET, Env, Parts, Rows, branch_row, detail_row, result_row, source_lines,
};
use crate::pages::session::strings::INLINE_GAP;

const PREVIEW_LINES: usize = 3;
const PREVIEW_LINE_BYTES: usize = 16 * 1024;

/// The cell of one MCP call.
pub fn mcp(call: &McpCall, env: &Env<'_>) -> Parts {
    let Env {
        strings,
        tokens,
        expanded,
        ..
    } = *env;
    let success = call.success();
    let mark = mcp_mark(call, tokens);
    let verb = match success {
        Some(true) => strings.called_tool,
        Some(false) => strings.failed_tool,
        None => strings.calling_tool,
    };
    let disclosure = Some(
        if expanded {
            strings.hide_details
        } else {
            strings.show_details
        }
        .to_owned(),
    );
    if expanded {
        return Parts::Rows(Rows {
            mark,
            header: transcript(call.clone(), strings, tokens.clone()),
            body: Vec::new(),
            disclosure,
        });
    }
    let header = Text::rich(vec![
        Span::styled(verb, tokens.bold()),
        Span::new(INLINE_GAP),
        Span::styled(call.title(), tokens.accent()),
    ])
    .max_lines(Some(1))
    .overflow(TextOverflow::Ellipsis)
    .into();
    Parts::Rows(Rows {
        mark,
        header,
        body: preview(call, strings)
            .into_iter()
            .enumerate()
            .map(|(index, text)| result_row(index, text, tokens))
            .collect(),
        disclosure,
    })
}

/// The bullet's style of an MCP call: running, called, failed.
pub fn mcp_mark(call: &McpCall, tokens: &Tokens) -> Style {
    match call.success() {
        Some(true) => tokens.success(),
        Some(false) => tokens.error(),
        None => tokens.accent(),
    }
    .attrs(Attrs::BOLD)
}

/// The result rows of the collapsed form, from the end (oracle
/// `compact_mcp_lines`): at most three, a long line cut with `…`.
fn preview(call: &McpCall, strings: &Strings) -> Vec<String> {
    let mut details: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let push_tail = |details: &mut std::collections::VecDeque<String>, text: &str| {
        for line in text.lines().rev().take(PREVIEW_LINES - details.len()) {
            let bounded = &line[..line.floor_char_boundary(PREVIEW_LINE_BYTES)];
            let mut detail = bounded.to_owned();
            if bounded.len() < line.len() {
                detail.push('…');
            }
            details.push_front(detail);
        }
    };
    match &call.result {
        Some(Ok(result)) => {
            for block in result.content.iter().rev() {
                push_tail(&mut details, &body_text(&call.preview_text(block), strings));
                // A code-mode image can also carry text; its marker precedes that output.
                if block.is_image && block.text().is_some() && details.len() < PREVIEW_LINES {
                    details.push_front(strings.tool_result_image.to_owned());
                }
                if details.len() == PREVIEW_LINES {
                    break;
                }
            }
        }
        Some(Err(error)) => {
            push_tail(&mut details, error);
            if error.lines().take(PREVIEW_LINES + 1).count() <= PREVIEW_LINES {
                if let Some(first) = details.front_mut() {
                    first.insert_str(0, strings.tool_error);
                } else {
                    details.push_front(strings.tool_error.to_owned());
                }
            }
        }
        None => {}
    }
    details.into()
}

/// Everything of an MCP call: the invocation inline after the verb when it
/// fits the room, else under it (oracle `render_hyperlink_lines`, transcript
/// mode). `LayoutBuilder` + `segmenter_of(cx)` pick the spelling.
pub fn transcript(call: McpCall, strings: &'static Strings, tokens: Tokens) -> AnyWidget {
    LayoutBuilder::new(move |cx, room| {
        let tokens = &tokens;
        let verb = expanded_verb(&call, strings);
        let args = call.arguments.to_string();
        let invocation = vec![
            Span::styled(call.server.clone(), tokens.accent()),
            Span::new(INVOCATION_DOT),
            Span::styled(call.tool.clone(), tokens.accent()),
            Span::new(INVOCATION_OPEN),
            Span::styled(args.clone(), tokens.dim().attrs(Attrs::DIM)),
            Span::new(INVOCATION_CLOSE),
        ];
        let inline = invocation_inline(
            &call,
            &args,
            verb,
            Room {
                width: room.max_width,
                segmenter: segmenter_of(cx),
            },
        );
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        if inline {
            let mut spans = vec![Span::styled(verb, tokens.bold()), Span::new(INLINE_GAP)];
            spans.extend(invocation);
            column = column.child(Text::rich(spans));
        } else {
            column = column
                .child(Text::line(verb).style(tokens.bold()))
                .child(branch_row(
                    0,
                    Text::rich(invocation).into(),
                    tokens.dim().attrs(Attrs::DIM),
                ));
        }
        for (index, row) in detail_rows(&call, strings).into_iter().enumerate() {
            let index = if inline { index } else { index + 1 };
            column = column.child(detail_row(index, row, tokens));
        }
        column.into()
    })
    .into()
}

/// What a block says, worded (oracle `render_full`: media and embedded
/// resources are markers).
pub(super) fn body_text<'a>(body: &BlockBody<'a>, strings: &Strings) -> Cow<'a, str> {
    match body {
        BlockBody::Text(text) => text.clone(),
        BlockBody::Image => Cow::Borrowed(strings.tool_result_image),
        BlockBody::Audio => Cow::Borrowed(strings.tool_result_audio),
        BlockBody::Resource(uri) => Cow::Owned(strings.label(strings.tool_resource, uri)),
        BlockBody::Link(uri) => Cow::Owned(strings.label(strings.tool_link, uri)),
    }
}

/// The rows of the whole result (oracle `render_detail_lines`, transcript mode).
pub(super) fn detail_rows(call: &McpCall, strings: &Strings) -> Vec<String> {
    let mut rows = Vec::new();
    match &call.result {
        Some(Ok(result)) => {
            for block in &result.content {
                if call.result_kind() == McpResultKind::NodeRepl
                    && block.is_image
                    && block.text().is_some()
                {
                    rows.push(strings.tool_result_image.to_owned());
                }
                rows.extend(
                    body_text(&block.render_full(), strings)
                        .lines()
                        .map(str::to_owned),
                );
            }
        }
        Some(Err(error)) => {
            rows.extend(
                format!("{}{error}", strings.tool_error)
                    .lines()
                    .map(str::to_owned),
            );
        }
        None => {}
    }
    rows
}

/// The cell of one dynamic call.
pub fn dynamic(call: &DynamicCall, env: &Env<'_>) -> Parts {
    let Env {
        strings,
        tokens,
        expanded,
        ..
    } = *env;
    let mark: Style = match call.status {
        ToolStatus::Running => tokens.dim(),
        ToolStatus::Completed => tokens.success(),
        ToolStatus::Failed | ToolStatus::Declined => tokens.error().attrs(Attrs::BOLD),
    };
    let (verb, duration) = dynamic_heading(call, strings);
    let mut header = vec![
        Span::styled(verb, tokens.bold()),
        Span::new(INLINE_GAP),
        Span::styled(call.name.clone(), tokens.accent()),
    ];
    if let Some(duration) = duration {
        header.push(Span::styled(duration, tokens.dim().attrs(Attrs::DIM)));
    }
    let header = Text::rich(header);
    let disclosure = Some(
        if expanded {
            strings.hide_details
        } else {
            strings.show_details
        }
        .to_owned(),
    );
    if expanded {
        return Parts::Rows(Rows {
            mark,
            header: header.into(),
            body: dynamic_details(call, strings)
                .into_iter()
                .enumerate()
                .map(|(index, row)| detail_row(index, row, tokens))
                .collect(),
            disclosure,
        });
    }
    let (notice, rows) = dynamic_preview(call, strings);
    let mut body = Vec::new();
    if let Some(notice) = notice {
        body.push(
            Text::rich(vec![Span::styled(notice, tokens.dim().attrs(Attrs::DIM))])
                .max_lines(Some(1))
                .overflow(TextOverflow::Ellipsis)
                .into(),
        );
    }
    for (index, row) in rows.into_iter().enumerate() {
        body.push(result_row(index, row, tokens));
    }
    Parts::Rows(Rows {
        mark,
        header: header
            .max_lines(Some(1))
            .overflow(TextOverflow::Ellipsis)
            .into(),
        body,
        disclosure,
    })
}

fn expanded_verb(call: &McpCall, strings: &Strings) -> &'static str {
    if call.success().is_some() {
        strings.called_tool
    } else {
        strings.calling_tool
    }
}

fn invocation_inline(call: &McpCall, args: &str, verb: &str, room: Room) -> bool {
    let invocation = format!("{}.{}({args})", call.server, call.tool);
    let reserved = room.segmenter.width(verb) + 1;
    i32::try_from(room.segmenter.width(&invocation)).unwrap_or(i32::MAX)
        <= room
            .width
            .saturating_sub(i32::try_from(reserved).unwrap_or(i32::MAX))
}

pub(super) fn text_room(room: Room) -> Room {
    room.after(BULLET).after(" ")
}

/// One detail row's glyph and text. Clipped previews also mount a separate ellipsis.
pub(super) fn result_parts(index: usize, text: String, room: Option<Room>, parts: &mut Vec<Part>) {
    let branch = if index == 0 { BRANCH } else { "  " };
    let cut = room.is_some_and(|room| room.after(branch).cuts(&text));
    parts.push(Part::gutter(branch));
    parts.push(Part::text(text));
    if cut {
        parts.push(Part::gutter("…"));
    }
}

/// The same invocation and result node order as the expanded call's LayoutBuilder.
pub(super) fn transcript_parts(call: &McpCall, s: &Strings, room: Room) -> Vec<Part> {
    let verb = expanded_verb(call, s);
    let args = call.arguments.to_string();
    let invocation = format!(
        "{}{INVOCATION_DOT}{}{INVOCATION_OPEN}{args}{INVOCATION_CLOSE}",
        call.server, call.tool,
    );
    let inline = invocation_inline(call, &args, verb, room);
    let mut parts = if inline {
        vec![Part::text(format!("{verb}{INLINE_GAP}{invocation}"))]
    } else {
        vec![
            Part::text(verb),
            Part::gutter(BRANCH),
            Part::text(invocation),
        ]
    };
    for (index, row) in detail_rows(call, s).into_iter().enumerate() {
        result_parts(index + usize::from(!inline), row, None, &mut parts);
    }
    parts
}

pub(super) fn mcp_texts(call: &McpCall, env: &TextEnv<'_>, room: Room) -> Vec<Part> {
    let s = env.cells;
    let room = text_room(room);
    let expanded = env.detailed || env.expanded;
    let mut parts = vec![Part::gutter(BULLET)];
    if expanded {
        parts.extend(transcript_parts(call, s, room));
    } else {
        let verb = match call.success() {
            Some(true) => s.called_tool,
            Some(false) => s.failed_tool,
            None => s.calling_tool,
        };
        parts.push(Part::text(format!("{verb}{INLINE_GAP}{}", call.title())));
        for (index, row) in preview(call, s).into_iter().enumerate() {
            result_parts(index, row, Some(room), &mut parts);
        }
    }
    if !env.detailed {
        parts.push(Part::gutter(if expanded {
            s.hide_details
        } else {
            s.show_details
        }));
    }
    parts
}

fn dynamic_heading(call: &DynamicCall, strings: &Strings) -> (&'static str, Option<String>) {
    let verb = match call.status {
        _ if call.interrupted => strings.interrupted_tool,
        ToolStatus::Running => strings.calling_tool,
        ToolStatus::Completed => strings.called_tool,
        ToolStatus::Failed | ToolStatus::Declined => strings.failed_tool,
    };
    let duration = call
        .duration_ms
        .filter(|millis| *millis >= 0)
        .map(|millis| strings.label(strings.tool_duration, &millis.to_string()));
    (verb, duration)
}
fn dynamic_output(call: &DynamicCall, strings: &Strings, all: bool) -> Vec<String> {
    match &call.output {
        Some(output) if output.is_empty() => vec![strings.tool_no_output.to_owned()],
        Some(output) => output
            .iter()
            .map(|item| match item {
                DynamicOutput::Text(text) => text.as_str(),
                DynamicOutput::Image => strings.dynamic_image,
                DynamicOutput::Audio => strings.dynamic_audio,
            })
            .flat_map(|text| {
                if all {
                    source_lines(text)
                } else {
                    text.split_terminator('\n').collect()
                }
            })
            .map(str::to_owned)
            .collect(),
        None if call.status == ToolStatus::Running => Vec::new(),
        None => vec![strings.output_unavailable.to_owned()],
    }
}

fn dynamic_details(call: &DynamicCall, strings: &Strings) -> Vec<String> {
    let arguments = serde_json::to_string_pretty(&call.arguments)
        .unwrap_or_else(|_| call.arguments.to_string());
    let arguments = strings.label(strings.tool_arguments, &arguments);
    source_lines(&arguments)
        .into_iter()
        .map(str::to_owned)
        .chain(dynamic_output(call, strings, true))
        .collect()
}

fn dynamic_preview(call: &DynamicCall, strings: &Strings) -> (Option<String>, Vec<String>) {
    let mut rows = dynamic_output(call, strings, false);
    let real = matches!(&call.output, Some(output) if !output.is_empty());
    let skipped = if real {
        rows.len().saturating_sub(PREVIEW_LINES)
    } else {
        0
    };
    let notice =
        (skipped > 0).then(|| strings.label(strings.tool_earlier_lines, &skipped.to_string()));
    rows.drain(..skipped);
    if real {
        for row in &mut rows {
            if row.len() > PREVIEW_LINE_BYTES {
                row.truncate(row.floor_char_boundary(PREVIEW_LINE_BYTES));
                row.push('…');
            }
        }
    }
    (notice, rows)
}

pub(super) fn dynamic_texts(call: &DynamicCall, env: &TextEnv<'_>, room: Room) -> Vec<Part> {
    let s = env.cells;
    let (verb, duration) = dynamic_heading(call, s);
    let mut parts = vec![
        Part::gutter(BULLET),
        Part::text(format!(
            "{verb}{INLINE_GAP}{}{}",
            call.name,
            duration.as_deref().unwrap_or_default()
        )),
    ];
    let expanded = env.detailed || env.expanded;
    let (rows, room) = if expanded {
        (dynamic_details(call, s), None)
    } else {
        let (notice, rows) = dynamic_preview(call, s);
        parts.extend(notice.map(Part::text));
        (rows, Some(text_room(room)))
    };
    for (index, row) in rows.into_iter().enumerate() {
        result_parts(index, row, room, &mut parts);
    }
    if !env.detailed {
        parts.push(Part::gutter(if expanded {
            s.hide_details
        } else {
            s.show_details
        }));
    }
    parts
}
