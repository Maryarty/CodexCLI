//! The shell calls (oracle `exec_cell/render.rs`, `history_cell/exec.rs`):
//! a command with its output tail, and a run of exploration calls merged into
//! one `Explored` cell. The bullet, the control and the expanded flag are the
//! row's (`transcript_activity.rs`).

use std::borrow::Cow;
use std::time::Duration;

use codex_utils_elapsed::format_duration;
use rtwk::AnyWidget;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row};
use rtwk::style::{Attrs, Span};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use codex_tui_core::threads::{ExplorationAction, ToolActivity, ToolKind, ToolStatus};

use crate::app::theme::Tokens;
use crate::pages::session::strings::{INLINE_GAP, LINE_BREAK, MORE};

use super::strings::Strings;
use super::tool_rows::{Env, Parts, Rows, ansi_row, branch_row};

const PREVIEW_LINES: usize = 3;
const PREVIEW_LINE_BYTES: usize = 16 * 1024;

/// The shell calls of one row (`calls[0]` is the row's first item).
pub fn parts(calls: &[&ToolActivity], env: &Env<'_>) -> Parts {
    let Env {
        strings,
        tokens,
        syntax,
        detailed,
        expanded,
    } = *env;
    if detailed {
        return Parts::Plain(detailed_shells(calls, strings, tokens, syntax), None);
    }
    let tool = calls[0];
    let status = if calls[1..]
        .iter()
        .any(|call| call.status == ToolStatus::Running)
    {
        ToolStatus::Running
    } else {
        tool.status
    };
    let ToolKind::Shell {
        command,
        actions,
        exit_code,
        user_shell,
        ..
    } = &tool.kind
    else {
        return Parts::Plain(AnyWidget::default(), None);
    };
    let title = if !actions.is_empty() {
        Cow::Borrowed(if status == ToolStatus::Running {
            strings.exploring
        } else {
            strings.explored
        })
    } else if status == ToolStatus::Running {
        Cow::Borrowed(strings.running_command)
    } else if let Some(code) = exit_code.filter(|&code| code != 0) {
        Cow::Owned(strings.label(strings.command_exited, &code.to_string()))
    } else {
        Cow::Borrowed(if *user_shell {
            strings.user_ran_command
        } else {
            strings.ran_command
        })
    };
    let command = command_text(command);
    let mut command_lines = command.lines();
    let first = command_lines.next().unwrap_or_default();
    let multiline = command_lines.next().is_some();
    let mut header = vec![Span::styled(title.into_owned(), tokens.bold())];
    if actions.is_empty() {
        header.push(Span::new(INLINE_GAP));
        let lexer = Highlighter::new(language_named("bash"), first);
        header.extend(
            syntax
                .spans(first, lexer.tokens(0))
                .into_iter()
                .map(|span| Span::styled(span.text, span.style)),
        );
        if multiline {
            header.push(Span::new(MORE));
        }
    }
    let header: AnyWidget = Text::rich(header)
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .into();
    let output = tool
        .output
        .as_deref()
        .unwrap_or_default()
        .trim_end_matches(['\r', '\n']);
    let mut body: Vec<AnyWidget> = Vec::new();
    let mut hidden = 0;
    if expanded {
        for call in calls {
            body.push(call_details(call, tokens));
        }
    } else if actions.is_empty() {
        let tail: Vec<_> = output.lines().rev().take(PREVIEW_LINES).collect();
        if tail.len() == PREVIEW_LINES {
            hidden = output.lines().count() - PREVIEW_LINES;
        }
        for (index, line) in tail.into_iter().rev().enumerate() {
            body.push(ansi_row(index, line, tokens, false, PREVIEW_LINE_BYTES));
        }
        if output.is_empty() && tool.status != ToolStatus::Running {
            body.push(ansi_row(
                0,
                strings.tool_no_output,
                tokens,
                false,
                PREVIEW_LINE_BYTES,
            ));
        }
    } else {
        for (index, action) in exploration_actions(calls)
            .iter()
            .take(PREVIEW_LINES)
            .enumerate()
        {
            let line = match action {
                ExplorationAction::Read(names) => {
                    strings.label(strings.explored_read, &names.join(", "))
                }
                ExplorationAction::List(path) => strings.label(strings.explored_list, path),
                ExplorationAction::Search {
                    query,
                    path: Some(path),
                } => strings
                    .explored_search_in
                    .replace("{0}", query)
                    .replace("{1}", path),
                ExplorationAction::Search { query, path: None } => {
                    strings.label(strings.explored_search, query)
                }
            };
            body.push(branch_row(
                index,
                Text::line(line).overflow(TextOverflow::Ellipsis).into(),
                tokens.dim().attrs(Attrs::DIM),
            ));
        }
    }
    let disclosure = if expanded {
        Some(strings.hide_details.to_owned())
    } else if hidden > 0 {
        Some(if hidden == 1 {
            strings.hidden_output_line.to_owned()
        } else {
            strings.label(strings.hidden_output_lines, &hidden.to_string())
        })
    } else if multiline || !actions.is_empty() {
        Some(strings.show_details.to_owned())
    } else {
        None
    };
    let mark = match status {
        ToolStatus::Running => tokens.accent(),
        ToolStatus::Completed if !actions.is_empty() => tokens.dim().attrs(Attrs::DIM),
        ToolStatus::Completed => tokens.success(),
        ToolStatus::Failed | ToolStatus::Declined => tokens.error(),
    }
    .attrs(Attrs::BOLD);
    Parts::Rows(Rows {
        mark,
        header,
        body,
        disclosure,
    })
}

/// The reads of a run of exploration calls, neighbouring ones merged (oracle
/// `ExecCell::coalesce`).
fn exploration_actions(calls: &[&ToolActivity]) -> Vec<ExplorationAction> {
    let mut merged = Vec::new();
    let mut previous_reads = false;
    for call in calls {
        let ToolKind::Shell {
            actions, exit_code, ..
        } = &call.kind
        else {
            break;
        };
        let mergeable = exit_code.is_none_or(|code| code == 0);
        if mergeable
            && previous_reads
            && let [ExplorationAction::Read(next)] = actions.as_slice()
            && let Some(ExplorationAction::Read(names)) = merged.last_mut()
        {
            for name in next {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
        } else {
            merged.extend(actions.iter().cloned());
        }
        previous_reads = mergeable && matches!(actions.as_slice(), [ExplorationAction::Read(_)]);
    }
    merged
}

/// The detailed transcript's shell calls: the command after its prompt, the
/// whole output, the exit line.
fn detailed_shells(
    calls: &[&ToolActivity],
    strings: &Strings,
    tokens: &Tokens,
    syntax: &SyntaxTheme,
) -> AnyWidget {
    let mut rows = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .spacing(1);
    for call in calls {
        let ToolKind::Shell {
            command,
            exit_code,
            duration_ms,
            ..
        } = &call.kind
        else {
            continue;
        };
        let source = command_text(command);
        let code = Highlighter::new(language_named("bash"), &source);
        let mut spans = Vec::new();
        for index in 0..code.line_count() {
            if index != 0 {
                spans.push(Span::new(LINE_BREAK));
            }
            spans.extend(
                syntax
                    .spans(code.line(index), code.tokens(index))
                    .into_iter()
                    .map(|span| Span::styled(span.text, span.style)),
            );
        }
        let mut detail = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(
                Row::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .spacing(1)
                    .child(Text::line(strings.shell_prompt).style(tokens.accent()))
                    .child(Expanded::new(1, Text::rich(spans))),
            );
        if let Some(output) = call.output.as_deref().filter(|output| !output.is_empty()) {
            detail = detail.child(Text::new(output.trim_end_matches(['\r', '\n']).to_owned()));
        }
        if let Some(code) = *exit_code {
            let mut result = if code == 0 {
                vec![Span::styled(
                    strings.exit_ok,
                    tokens.success().attrs(Attrs::BOLD),
                )]
            } else {
                vec![
                    Span::styled(strings.exit_failed, tokens.error().attrs(Attrs::BOLD)),
                    Span::new(strings.label(strings.exit_status, &code.to_string())),
                ]
            };
            if let Some(millis) = *duration_ms {
                result.push(Span::styled(
                    format!(" • {}", format_duration(Duration::from_millis(millis))),
                    tokens.dim().attrs(Attrs::DIM),
                ));
            }
            detail = detail.child(Text::rich(result));
        }
        rows = rows.child(detail);
    }
    rows.into()
}

/// The expanded call: the whole command, the whole output.
fn call_details(tool: &ToolActivity, tokens: &Tokens) -> AnyWidget {
    let ToolKind::Shell { command, .. } = &tool.kind else {
        return AnyWidget::default();
    };
    let mut details = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Padding::new(
            EdgeInsets::only(2, 0, 0, 0),
            Text::new(command_text(command).into_owned()),
        ));
    let output = tool
        .output
        .as_deref()
        .unwrap_or_default()
        .trim_end_matches(['\r', '\n']);
    if !output.is_empty() {
        details = details.child(ansi_row(0, output, tokens, true, PREVIEW_LINE_BYTES));
    }
    details.into()
}

/// The script of a `bash -lc` command line, the line itself otherwise.
pub fn command_text(command: &str) -> Cow<'_, str> {
    let Some(argv) = shlex::split(command) else {
        return Cow::Borrowed(command);
    };
    codex_shell_command::parse_command::extract_shell_command(&argv).map_or_else(
        || Cow::Borrowed(command),
        |(_, script)| Cow::Owned(script.to_owned()),
    )
}
