//! Raw output mode (oracle `HistoryRenderMode::Raw`, `HistoryCell::raw_lines`,
//! toggled by alt+r and `/raw`): a cell draws as its source text, one
//! unstyled row per source line, with no prefix, bullet, surface or
//! Markdown, so the terminal's own selection copies clean text.
//!
//! The rows of an item are [`lines`]; an item with no raw form here (its
//! rows are not text of its own: the patch, plan-update, collab and
//! approval cells, the notices) draws as it does in rich mode.
//!
//! Flutter: the cell is picked by a flag the way a `ListView.builder` picks
//! between two item builders; the raw one is a `Column` of plain `Text`s.

use std::sync::Arc;

use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::widgets::Text;

use codex_tui_core::threads::{Item, McpCall, ToolActivity, ToolKind};

use super::strings::{INVOCATION_CLOSE, INVOCATION_DOT, INVOCATION_OPEN, Strings};
use super::tool_call::{body_text, detail_rows};

/// The lines of `source` (oracle `raw_lines_from_source`): split at each
/// newline, a trailing newline leaving no last line, an empty source none.
pub fn source_lines(source: &str) -> Vec<String> {
    if source.is_empty() {
        return Vec::new();
    }
    let mut parts: Vec<&str> = source.split('\n').collect();
    if source.ends_with('\n') {
        parts.pop();
    }
    parts.into_iter().map(str::to_owned).collect()
}

/// The raw lines of an MCP call (oracle `McpToolCallCell::raw_lines`): the
/// verb and the invocation, then the whole result.
fn mcp(call: &McpCall, strings: &Strings) -> Vec<String> {
    let verb = if call.success().is_some() {
        strings.called_tool
    } else {
        strings.calling_tool
    };
    let mut lines = vec![format!(
        "{verb} {}{INVOCATION_DOT}{}{INVOCATION_OPEN}{}{INVOCATION_CLOSE}",
        call.server, call.tool, call.arguments
    )];
    match &call.result {
        Some(Ok(result)) => {
            for block in &result.content {
                lines.extend(source_lines(&body_text(&block.render_full(), strings)));
            }
        }
        Some(Err(error)) => lines.push([strings.tool_error, error].concat()),
        None => {}
    }
    lines
}

/// The raw lines of `item`, `None` when it has no raw form of its own.
pub fn lines(item: &Item, strings: &Strings) -> Option<Vec<String>> {
    Some(match item {
        // the prompt's text (control characters removed), then the labels of
        // the images it does not name, a blank row apart
        Item::User(prompt) => {
            let shown = prompt.shown();
            let mut lines = source_lines(shown.text.trim_end_matches(['\r', '\n']));
            if !prompt.images.is_empty() {
                if !lines.is_empty() {
                    lines.push(String::new());
                }
                lines.extend(
                    prompt
                        .images
                        .iter()
                        .map(|number| strings.label(strings.image_label, &number.to_string())),
                );
            }
            lines
        }
        Item::Agent { text, .. } => source_lines(text),
        Item::Plan { text } => source_lines(text),
        // the summary (`transcript_only` cells have no raw form in the
        // scrollback, and the detailed transcript is not raw)
        Item::Reasoning { summary, .. } => source_lines(summary.trim()),
        Item::Delegated(message) => {
            let mut lines = vec![strings.label(strings.delegated_attribution, &message.source)];
            lines.extend(message.markdown.lines().map(str::to_owned));
            lines
        }
        Item::Tool(tool) => match &tool.kind {
            ToolKind::Mcp(call) => mcp(call, strings),
            _ => return None,
        },
        _ => return None,
    })
}

/// The raw lines of a run of computer calls (oracle `ComputerActivityCell::raw_lines`: the
/// calls' own detailed lines, each call its bullet, verb and invocation and the whole result
/// under a branch); `item` is the first call and `rest` the others of the run. Any other item is
/// [`lines`].
pub fn run_lines(
    item: &Item,
    rest: &[Arc<ToolActivity>],
    strings: &Strings,
) -> Option<Vec<String>> {
    let Item::Tool(first) = item else {
        return lines(item, strings);
    };
    // the shell calls of the run: the cell's own detailed rows, plain
    if matches!(first.kind, ToolKind::Shell { .. }) {
        let calls: Vec<&ToolActivity> = std::iter::once(first)
            .chain(rest)
            .map(Arc::as_ref)
            .collect();
        return Some(super::exec::raw_lines(&calls, strings));
    }
    if !first.is_computer_activity() {
        return lines(item, strings);
    }
    let mut out = Vec::new();
    for tool in std::iter::once(first).chain(rest) {
        let ToolKind::Mcp(call) = &tool.kind else {
            continue;
        };
        if !call.is_computer_activity() {
            continue;
        }
        let verb = if call.success().is_some() {
            strings.called_tool
        } else {
            strings.calling_tool
        };
        out.push(format!(
            "\u{2022} {verb} {}{INVOCATION_DOT}{}{INVOCATION_OPEN}{}{INVOCATION_CLOSE}",
            call.server, call.tool, call.arguments
        ));
        for (index, row) in detail_rows(call, strings).into_iter().enumerate() {
            out.push(format!(
                "{}{row}",
                if index == 0 { "  \u{2514} " } else { "    " }
            ));
        }
    }
    Some(out)
}

/// The cell of raw `lines`: a plain `Text` per line, a blank row for an
/// empty one.
pub fn cell(lines: Vec<String>, strings: &Strings) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for line in lines {
        column = column.child(Text::new(if line.is_empty() {
            strings.gap.to_owned()
        } else {
            line
        }));
    }
    column.into()
}
