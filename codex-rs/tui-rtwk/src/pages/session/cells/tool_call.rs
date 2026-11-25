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
use super::tool_rows::{Env, Parts, Rows, branch_row, detail_row, result_row, source_lines};
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
        body: preview(call, env)
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
fn preview(call: &McpCall, env: &Env<'_>) -> Vec<String> {
    let strings = env.strings;
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
        let verb = if call.success().is_some() {
            strings.called_tool
        } else {
            strings.calling_tool
        };
        let args = call.arguments.to_string();
        let invocation = vec![
            Span::styled(call.server.clone(), tokens.accent()),
            Span::new(INVOCATION_DOT),
            Span::styled(call.tool.clone(), tokens.accent()),
            Span::new(INVOCATION_OPEN),
            Span::styled(args.clone(), tokens.dim().attrs(Attrs::DIM)),
            Span::new(INVOCATION_CLOSE),
        ];
        let text_width = segmenter_of(cx).width(&format!("{}.{}({args})", call.server, call.tool));
        let reserved = segmenter_of(cx).width(verb) + 1;
        let inline = text_width as i32 <= room.max_width - reserved as i32;
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
fn detail_rows(call: &McpCall, strings: &Strings) -> Vec<String> {
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
    let verb = match call.status {
        _ if call.interrupted => strings.interrupted_tool,
        ToolStatus::Running => strings.calling_tool,
        ToolStatus::Completed => strings.called_tool,
        ToolStatus::Failed | ToolStatus::Declined => strings.failed_tool,
    };
    let mut header = vec![
        Span::styled(verb, tokens.bold()),
        Span::new(INLINE_GAP),
        Span::styled(call.name.clone(), tokens.accent()),
    ];
    if let Some(millis) = call.duration_ms.filter(|millis| *millis >= 0) {
        header.push(Span::styled(
            strings.label(strings.tool_duration, &millis.to_string()),
            tokens.dim().attrs(Attrs::DIM),
        ));
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
    let output_rows = |all: bool| -> Vec<String> {
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
    };
    if expanded {
        let arguments = serde_json::to_string_pretty(&call.arguments)
            .unwrap_or_else(|_| call.arguments.to_string());
        let arguments = strings.label(strings.tool_arguments, &arguments);
        return Parts::Rows(Rows {
            mark,
            header: header.into(),
            body: source_lines(&arguments)
                .into_iter()
                .map(str::to_owned)
                .chain(output_rows(true))
                .enumerate()
                .map(|(index, row)| detail_row(index, row, tokens))
                .collect(),
            disclosure,
        });
    }
    let mut body = Vec::new();
    let mut rows = output_rows(false);
    // The collapsed form keeps the last three rows of a real output.
    let real = matches!(&call.output, Some(output) if !output.is_empty());
    let mut skipped = 0;
    if real && rows.len() > PREVIEW_LINES {
        skipped = rows.len() - PREVIEW_LINES;
        rows.drain(..skipped);
    }
    if skipped > 0 {
        body.push(
            Text::rich(vec![Span::styled(
                strings.label(strings.tool_earlier_lines, &skipped.to_string()),
                tokens.dim().attrs(Attrs::DIM),
            )])
            .max_lines(Some(1))
            .overflow(TextOverflow::Ellipsis)
            .into(),
        );
    }
    for (index, mut row) in rows.into_iter().enumerate() {
        if real && row.len() > PREVIEW_LINE_BYTES {
            row.truncate(row.floor_char_boundary(PREVIEW_LINE_BYTES));
            row.push('…');
        }
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
