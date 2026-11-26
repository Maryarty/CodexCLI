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
use rtwk::widgets::Text;
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use codex_tui_core::threads::{ToolActivity, ToolKind, ToolStatus};

use crate::app::theme::Tokens;
use crate::pages::session::strings::{INLINE_GAP, LINE_BREAK, MORE};
use crate::widgets::dots::dots_spans_kept;

use super::exploration;
use super::strings::Strings;
use super::text::{self, Part, Room};
use super::tool_call::text_room;
use super::tool_rows::{BRANCH, BULLET, Env, Parts, Rows, ansi_row, compact_output, plain_text};

const PREVIEW_LINES: usize = 3;
const PREVIEW_LINE_BYTES: usize = 16 * 1024;

/// The shell calls of one row (`calls[0]` is the row's first item).
pub fn parts(calls: &[&ToolActivity], env: &Env<'_>, output_lines: Option<usize>) -> Parts {
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
    let Some(head) = Headline::of(calls, strings) else {
        return Parts::Plain(AnyWidget::default(), None);
    };
    let (status, exploring, failed) = (head.status, head.exploring, head.failed);
    let tool = calls[0];
    let mut header = vec![Span::styled(head.title.to_string(), tokens.bold())];
    if exploring {
        if let Some(exits) = &head.exits {
            header.push(Span::styled(exits.clone(), tokens.bold()));
        }
    } else {
        header.push(Span::new(INLINE_GAP));
        let first = head.first_line();
        let lexer = Highlighter::new(language_named("bash"), first);
        header.extend(
            syntax
                .spans(first, lexer.tokens(0))
                .into_iter()
                .map(|span| Span::styled(span.text, span.style)),
        );
        if head.multiline() {
            header.push(Span::new(MORE));
        }
    }
    let header = dots_spans_kept(header);
    let output = preview_output(tool);
    let mut body: Vec<AnyWidget> = Vec::new();
    let mut hidden = 0;
    if expanded {
        for call in calls {
            body.push(call_details(call, tokens));
        }
    } else if !exploring {
        let tail: Vec<_> = output.lines().rev().take(PREVIEW_LINES).collect();
        if tail.len() == PREVIEW_LINES {
            hidden = hidden_output_lines(output_lines.unwrap_or_else(|| output.lines().count()));
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
        body.push(exploration::body(
            exploration::steps(calls, strings),
            tokens.accent(),
            tokens.dim().attrs(Attrs::DIM),
            tokens.error(),
        ));
    }
    let disclosure = if expanded {
        Some(strings.hide_details.to_owned())
    } else {
        closed_disclosure(&head, hidden, strings)
    };
    // Oracle `compact_command_lines`: a non-zero exit draws a plain bold bullet (no red); the
    // exploration header's is a dim one.
    let mark = match status {
        ToolStatus::Running => tokens.accent().attrs(Attrs::BOLD),
        _ if exploring => tokens.dim().attrs(Attrs::DIM),
        _ if failed.is_some() => tokens.bold(),
        ToolStatus::Completed => tokens.success().attrs(Attrs::BOLD),
        ToolStatus::Failed | ToolStatus::Declined => tokens.bold(),
    };
    Parts::Rows(Rows {
        mark,
        header,
        body,
        disclosure,
    })
}

fn closed_disclosure(head: &Headline<'_>, hidden: usize, strings: &Strings) -> Option<String> {
    if hidden == 1 {
        Some(strings.hidden_output_line.to_owned())
    } else if hidden > 1 {
        Some(strings.label(strings.hidden_output_lines, &hidden.to_string()))
    } else if head.has_command_control() {
        Some(strings.show_details.to_owned())
    } else {
        None
    }
}

fn preview_output(tool: &ToolActivity) -> &str {
    tool.output
        .as_deref()
        .unwrap_or_default()
        .trim_end_matches(['\r', '\n'])
}

fn hidden_output_lines(lines: usize) -> usize {
    lines.saturating_sub(PREVIEW_LINES)
}

fn multiline(command: &str) -> bool {
    command.lines().nth(1).is_some()
}

/// Whether a shell row has a control independent of its output.
fn has_command_control(tool: &ToolActivity) -> bool {
    let ToolKind::Shell {
        command, actions, ..
    } = &tool.kind
    else {
        return false;
    };
    !actions.is_empty() || multiline(&command_text(command))
}

/// Whether the first call draws a control while it is closed.
pub(in crate::pages::session) fn has_control(tool: &ToolActivity) -> bool {
    has_command_control(tool)
        || hidden_output_lines(preview_output(tool).lines().take(PREVIEW_LINES + 1).count()) > 0
}

/// Whether a delta has added an output control to a shell whose original
/// output and command did not already make it a stop.
pub(in crate::pages::session) fn gained_output_control(
    tool: &ToolActivity,
    initial: usize,
    current: usize,
) -> bool {
    hidden_output_lines(initial) == 0
        && hidden_output_lines(current) > 0
        && !has_command_control(tool)
}

/// What the header of a shell row says, in words: the one place the title, the count of failures
/// and the command line are decided, for the row that draws them (`parts`) and for the text Find
/// reads (`texts`).
struct Headline<'a> {
    /// A call of the run is still running.
    status: ToolStatus,
    exploring: bool,
    /// The exit code of a failed call.
    failed: Option<i32>,
    title: Cow<'a, str>,
    /// An exploration's count of non-zero exits.
    exits: Option<String>,
    /// The script of the row's first call.
    command: Cow<'a, str>,
}

impl<'a> Headline<'a> {
    /// The header of the row of `calls`; `None` when its first call is not a shell call.
    fn of(calls: &[&'a ToolActivity], strings: &Strings) -> Option<Headline<'a>> {
        let tool = *calls.first()?;
        let ToolKind::Shell {
            command,
            actions,
            user_shell,
            ..
        } = &tool.kind
        else {
            return None;
        };
        let status = if calls[1..]
            .iter()
            .any(|call| call.status == ToolStatus::Running)
        {
            ToolStatus::Running
        } else {
            tool.status
        };
        let exploring = !actions.is_empty();
        // Oracle `nonzero_exit`: the call's output says a code other than 0.
        let failed = exploration::failure(tool);
        let title = if exploring {
            Cow::Borrowed(if status == ToolStatus::Running {
                strings.exploring
            } else {
                strings.explored
            })
        } else if let Some(code) = failed {
            Cow::Owned(strings.label(strings.command_exited, &code.to_string()))
        } else if status == ToolStatus::Running {
            Cow::Borrowed(strings.running_command)
        } else {
            Cow::Borrowed(if *user_shell {
                strings.user_ran_command
            } else {
                strings.ran_command
            })
        };
        let exits = exploring
            .then(|| exploration::failures(calls))
            .filter(|count| *count > 0)
            .map(|count| {
                let template = if count == 1 {
                    strings.nonzero_exit
                } else {
                    strings.nonzero_exits
                };
                strings.label(template, &count.to_string())
            });
        Some(Headline {
            status,
            exploring,
            failed,
            title,
            exits,
            command: command_text(command),
        })
    }

    /// The command line the header shows.
    fn first_line(&self) -> &str {
        self.command.lines().next().unwrap_or_default()
    }

    /// The command has more lines than the header shows.
    fn multiline(&self) -> bool {
        multiline(&self.command)
    }

    fn has_command_control(&self) -> bool {
        self.exploring || self.multiline()
    }

    /// The header as one string: what its `Text` holds (an exploration names no command).
    fn text(&self) -> String {
        let mut text = self.title.to_string();
        if self.exploring {
            text.extend(self.exits.as_deref());
        } else {
            text.push_str(INLINE_GAP);
            text.push_str(self.first_line());
            if self.multiline() {
                text.push_str(MORE);
            }
        }
        text
    }
}

/// The texts of the shell row of `calls`, one part for each `Text` its cell builds, in tree order
/// (`text_contract` holds them to the mounted cell): the detailed transcript's form, or the
/// expanded one (what Find reads of a cell it opens), or the compact shell output tail.
pub fn texts(calls: &[&ToolActivity], env: &text::Env<'_>) -> Option<Vec<Part>> {
    texts_at(calls, env, Room::default())
}

pub(super) fn texts_at(
    calls: &[&ToolActivity],
    env: &text::Env<'_>,
    room: Room,
) -> Option<Vec<Part>> {
    let strings = env.cells;
    let head = Headline::of(calls, strings)?;
    if env.detailed {
        return Some(detailed_texts(calls, strings));
    }
    // the `…` of the header's cut is a node whether the room cuts it or not (`dots_spans_kept`)
    let mut parts = vec![
        Part::gutter(BULLET),
        Part::text(head.text()),
        Part::gutter("\u{2026}"),
    ];
    if !env.expanded {
        if head.exploring {
            parts.extend(exploration::texts(calls, strings, text_room(room)));
            parts.push(Part::gutter(strings.show_details));
            return Some(parts);
        }
        let output = preview_output(calls[0]);
        let tail: Vec<_> = output.lines().rev().take(PREVIEW_LINES).collect();
        for (index, line) in tail.into_iter().rev().enumerate() {
            parts.push(Part::gutter(if index == 0 { BRANCH } else { "  " }));
            parts.push(Part::text(plain_text(&compact_output(
                line,
                PREVIEW_LINE_BYTES,
            ))));
        }
        if output.is_empty() && calls[0].status != ToolStatus::Running {
            parts.push(Part::gutter(BRANCH));
            parts.push(Part::text(strings.tool_no_output));
        }
        if let Some(disclosure) =
            closed_disclosure(&head, hidden_output_lines(output.lines().count()), strings)
        {
            parts.push(Part::gutter(disclosure));
        }
        return Some(parts);
    }
    for call in calls {
        let ToolKind::Shell { command, .. } = &call.kind else {
            continue;
        };
        parts.push(Part::text(command_text(command)));
        let output = call
            .output
            .as_deref()
            .unwrap_or_default()
            .trim_end_matches(['\r', '\n']);
        if !output.is_empty() {
            parts.push(Part::gutter(BRANCH));
            parts.push(Part::text(plain_text(output)));
        }
    }
    parts.push(Part::gutter(strings.hide_details));
    Some(parts)
}

/// The parts of [`detailed_shells`].
fn detailed_texts(calls: &[&ToolActivity], strings: &Strings) -> Vec<Part> {
    let mut parts = Vec::new();
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
        if !parts.is_empty() {
            parts.push(Part::gap());
        }
        let source = command_text(command);
        let code = Highlighter::new(language_named("bash"), &source);
        parts.push(Part::text(strings.shell_prompt));
        parts.push(Part::text(
            (0..code.line_count())
                .map(|index| code.line(index))
                .collect::<Vec<_>>()
                .join(LINE_BREAK),
        ));
        if let Some(output) = call.output.as_deref().filter(|output| !output.is_empty()) {
            parts.push(Part::text(output.trim_end_matches(['\r', '\n'])));
        }
        if let Some(code) = *exit_code {
            let mut result = if code == 0 {
                strings.exit_ok.to_owned()
            } else {
                format!(
                    "{}{}",
                    strings.exit_failed,
                    strings.label(strings.exit_status, &code.to_string())
                )
            };
            if let Some(millis) = *duration_ms {
                result.push_str(&format!(
                    " \u{2022} {}",
                    format_duration(Duration::from_millis(millis))
                ));
            }
            parts.push(Part::text(result));
        }
    }
    parts
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
