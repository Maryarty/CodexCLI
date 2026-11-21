//! Tool previews and turn timing in the virtualized transcript.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local, TimeZone};
use codex_utils_elapsed::format_duration;
use rtwk::controls::Pressable;
use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::highlight::{Highlighter, SyntaxTheme, language_named};

use codex_tui_core::threads::{
    ExplorationAction, ToolActivity, ToolKind, ToolMedia, ToolStatus, TurnTiming,
};

use crate::app::theme::Tokens;

use super::strings;

const PREVIEW_LINES: usize = 3;
const PREVIEW_LINE_BYTES: usize = 16 * 1024;

#[derive(Clone, PartialEq)]
pub(super) struct ToolRow {
    pub first: Arc<ToolActivity>,
    pub rest: Vec<Arc<ToolActivity>>,
    pub detailed: bool,
}

impl ToolRow {
    fn calls(&self) -> impl Iterator<Item = &ToolActivity> {
        std::iter::once(self.first.as_ref()).chain(self.rest.iter().map(Arc::as_ref))
    }
}

#[derive(Default)]
pub(super) struct ToolRowState {
    expanded: bool,
}

impl StatefulWidget for ToolRow {
    type State = ToolRowState;

    fn create_state(&self) -> ToolRowState {
        ToolRowState::default()
    }
}

impl State for ToolRowState {
    type Widget = ToolRow;
    type Msg = bool;

    fn update(&mut self, expanded: bool, _: &ToolRow, _: &mut Cx<'_, Self>) -> Rebuild {
        self.expanded = expanded;
        Rebuild::Yes
    }

    fn build(&self, widget: &ToolRow, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tool = &widget.first;
        let status = if widget
            .rest
            .iter()
            .any(|call| call.status == ToolStatus::Running)
        {
            ToolStatus::Running
        } else {
            tool.status
        };
        let tokens = Tokens::of(cx);
        let strings = strings::of(cx);
        let syntax = SyntaxTheme::of(cx);
        if widget.detailed && matches!(tool.kind, ToolKind::Shell { .. }) {
            return detailed_shells(widget, strings, &tokens, &syntax);
        }
        let expanded = widget.detailed || self.expanded;
        let toggle = cx.send(!expanded);
        let tap = cx.map(move |_| !expanded);
        let (title, command, actions, media) = match &tool.kind {
            ToolKind::Shell {
                command,
                actions,
                exit_code,
                user_shell,
                ..
            } => {
                let title = if !actions.is_empty() {
                    Cow::Borrowed(if status == ToolStatus::Running {
                        strings.exploring
                    } else {
                        strings.explored
                    })
                } else if status == ToolStatus::Running {
                    Cow::Borrowed(strings.running_command)
                } else if let Some(code) = exit_code.filter(|&code| code != 0) {
                    Cow::Owned(strings.transcript_label(strings.command_exited, &code.to_string()))
                } else {
                    Cow::Borrowed(if *user_shell {
                        strings.user_ran_command
                    } else {
                        strings.ran_command
                    })
                };
                (title, command_text(command), actions.as_slice(), &[][..])
            }
            ToolKind::Call {
                namespace,
                name,
                media,
                ..
            } => {
                let name = namespace
                    .as_ref()
                    .map_or_else(|| name.clone(), |ns| format!("{ns}.{name}"));
                let verb = match tool.status {
                    ToolStatus::Running => strings.calling_tool,
                    ToolStatus::Completed => strings.called_tool,
                    ToolStatus::Failed | ToolStatus::Declined => strings.failed_tool,
                };
                (
                    Cow::Borrowed(verb),
                    Cow::Owned(name),
                    &[][..],
                    media.as_slice(),
                )
            }
        };
        let is_shell = matches!(tool.kind, ToolKind::Shell { .. });
        let mut command_lines = command.lines();
        let first = command_lines.next().unwrap_or_default();
        let multiline = command_lines.next().is_some();
        let mut header = vec![Span::styled(title.into_owned(), tokens.bold())];
        if actions.is_empty() {
            header.push(Span::new(strings::INLINE_GAP));
            if is_shell {
                let lexer = Highlighter::new(language_named("bash"), first);
                header.extend(
                    syntax
                        .spans(first, lexer.tokens(0))
                        .into_iter()
                        .map(|span| Span::styled(span.text, span.style)),
                );
            } else {
                header.push(Span::styled(first.to_owned(), tokens.info()));
            }
            if multiline {
                header.push(Span::new(strings::MORE));
            }
        }
        let header = Text::rich(header)
            .max_lines(Some(1))
            .overflow(TextOverflow::Ellipsis);
        let output = tool
            .output
            .as_deref()
            .unwrap_or_default()
            .trim_end_matches(['\r', '\n']);
        let mut preview = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let mut hidden = 0;
        let mut shown = 0;
        if actions.is_empty() {
            let text_lines = PREVIEW_LINES.saturating_sub(media.len());
            let tail: Vec<_> = output.lines().rev().take(text_lines).collect();
            if is_shell && tail.len() == PREVIEW_LINES {
                hidden = output.lines().count() - PREVIEW_LINES;
            }
            for (index, line) in tail.into_iter().rev().enumerate() {
                preview = preview.child(output_row(index, line, &tokens, false));
                shown += 1;
            }
            for item in media.iter().rev().take(PREVIEW_LINES).rev() {
                preview = preview.child(output_row(
                    shown,
                    media_label(*item, strings),
                    &tokens,
                    false,
                ));
                shown += 1;
            }
            if output.is_empty() && is_shell && tool.status != ToolStatus::Running {
                preview = preview.child(output_row(0, strings.tool_no_output, &tokens, false));
            }
        } else {
            for (index, action) in exploration_actions(widget)
                .iter()
                .take(PREVIEW_LINES)
                .enumerate()
            {
                let line = match action {
                    ExplorationAction::Read(names) => {
                        strings.transcript_label(strings.explored_read, &names.join(", "))
                    }
                    ExplorationAction::List(path) => {
                        strings.transcript_label(strings.explored_list, path)
                    }
                    ExplorationAction::Search {
                        query,
                        path: Some(path),
                    } => strings.search_in(query, path),
                    ExplorationAction::Search { query, path: None } => {
                        strings.transcript_label(strings.explored_search, query)
                    }
                };
                preview = preview.child(branch_row(
                    index,
                    Text::line(line).overflow(TextOverflow::Ellipsis),
                    tokens.dim().attrs(Attrs::DIM),
                ));
            }
        }
        let call_arguments = match &tool.kind {
            ToolKind::Call { arguments, .. } if !arguments.is_null() => Some(arguments),
            _ => None,
        };
        let has_details = is_shell || shown > 0 || call_arguments.is_some();
        let disclose = if widget.detailed {
            None
        } else if expanded {
            Some(strings.hide_tool_details.to_owned())
        } else if is_shell && hidden > 0 {
            Some(if hidden == 1 {
                strings.hidden_output_line.to_owned()
            } else {
                strings.transcript_label(strings.hidden_output_lines, &hidden.to_string())
            })
        } else if multiline || !actions.is_empty() || !is_shell && has_details {
            Some(strings.show_tool_details.to_owned())
        } else {
            None
        };
        let mut content = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(header);
        if expanded {
            for call in widget.calls() {
                content = content.child(call_details(call, strings, &tokens));
            }
        } else {
            content = content.child(preview);
        }
        if let Some(label) = disclose {
            content = content.child(Padding::new(
                EdgeInsets::only(2, 0, 0, 0),
                Text::new(label).style(tokens.dim().attrs(Attrs::DIM)),
            ));
        }
        let content: AnyWidget = if !widget.detailed && (has_details || expanded) {
            let content: AnyWidget = content.into();
            Pressable::new(move |_, _| content.clone())
                .bindings([("enter", Some(toggle.clone())), ("space", Some(toggle))])
                .on_tap(tap)
                .into()
        } else {
            content.into()
        };
        let mark_style = match status {
            ToolStatus::Running => tokens.accent(),
            ToolStatus::Completed if !actions.is_empty() => tokens.dim().attrs(Attrs::DIM),
            ToolStatus::Completed => tokens.success(),
            ToolStatus::Failed | ToolStatus::Declined => tokens.error(),
        }
        .attrs(Attrs::BOLD);
        Row::new()
            .spacing(1)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::line("•").style(mark_style))
            .child(Expanded::new(1, content))
            .into()
    }
}

fn exploration_actions(widget: &ToolRow) -> Vec<ExplorationAction> {
    let mut merged = Vec::new();
    let mut previous_reads = false;
    for call in widget.calls() {
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

fn detailed_shells(
    widget: &ToolRow,
    strings: &strings::Strings,
    tokens: &Tokens,
    syntax: &SyntaxTheme,
) -> AnyWidget {
    let mut rows = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .spacing(1);
    for call in widget.calls() {
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
                spans.push(Span::new(strings::LINE_BREAK));
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
                    Span::new(strings.transcript_label(strings.exit_status, &code.to_string())),
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

fn call_details(tool: &ToolActivity, strings: &strings::Strings, tokens: &Tokens) -> Column {
    let (full, media) = match &tool.kind {
        ToolKind::Shell { command, .. } => (command_text(command).into_owned(), &[][..]),
        ToolKind::Call {
            namespace,
            name,
            arguments,
            media,
        } => {
            let name = namespace
                .as_ref()
                .map_or_else(|| name.clone(), |namespace| format!("{namespace}.{name}"));
            let full = if arguments.is_null() {
                name
            } else {
                format!("{name}({arguments})")
            };
            (full, media.as_slice())
        }
    };
    let mut details = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Padding::new(EdgeInsets::only(2, 0, 0, 0), Text::new(full)));
    let output = tool
        .output
        .as_deref()
        .unwrap_or_default()
        .trim_end_matches(['\r', '\n']);
    if !output.is_empty() {
        details = details.child(output_row(0, output, tokens, true));
    }
    for (index, item) in media.iter().enumerate() {
        let line = index + usize::from(!output.is_empty());
        details = details.child(output_row(line, media_label(*item, strings), tokens, true));
    }
    details
}

fn command_text(command: &str) -> Cow<'_, str> {
    let Some(argv) = shlex::split(command) else {
        return Cow::Borrowed(command);
    };
    codex_shell_command::parse_command::extract_shell_command(&argv).map_or_else(
        || Cow::Borrowed(command),
        |(_, script)| Cow::Owned(script.to_owned()),
    )
}

fn media_label(media: ToolMedia, strings: &strings::Strings) -> &'static str {
    match media {
        ToolMedia::Image => strings.tool_result_image,
        ToolMedia::Audio => strings.tool_result_audio,
    }
}

fn output_row(index: usize, line: &str, tokens: &Tokens, wrap: bool) -> Row {
    let dim = tokens.dim().attrs(Attrs::DIM);
    let line = if !wrap && line.len() > PREVIEW_LINE_BYTES {
        let prefix = &line[..line.floor_char_boundary(PREVIEW_LINE_BYTES)];
        format!("{prefix}{}", strings::MORE)
    } else {
        line.to_owned()
    };
    // oracle exec_cell/render.rs `dimmed_output_line`: the output's own SGR colours, every span dimmed
    let mut spans: Vec<Span> = Vec::new();
    for (index, parsed) in rtwk::ansi::parse_ansi(&line).into_iter().enumerate() {
        if index != 0 {
            spans.push(Span::new(strings::LINE_BREAK));
        }
        spans.extend(parsed.into_iter().map(|mut span| {
            span.style.attrs = span.style.attrs | Attrs::DIM;
            span
        }));
    }
    let line = Text::rich(spans);
    branch_row(
        index,
        if wrap {
            line
        } else {
            line.max_lines(Some(1)).overflow(TextOverflow::Ellipsis)
        },
        dim,
    )
}

fn branch_row(index: usize, body: Text, marker: Style) -> Row {
    let branch = if index == 0 { "└ " } else { "  " };
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(branch).style(marker))
        .child(Expanded::new(1, body))
}

impl_widget!(stateful eq ToolRow);

pub(super) fn completion(
    timing: TurnTiming,
    now: i64,
    strings: &strings::Strings,
    tokens: &Tokens,
) -> AnyWidget {
    let mut parts = Vec::new();
    if let Some(millis) = timing.duration_ms {
        let elapsed = if millis < 1000 {
            "<1s".to_string()
        } else {
            super::activity::compact_elapsed((millis / 1000) as u64)
        };
        parts.push(strings.worked_for(&elapsed).to_string());
    }
    if let Some(at) = timing
        .completed_at
        .and_then(|at| Local.timestamp_opt(at, 0).single())
    {
        let today = Local.timestamp_opt(now, 0).single();
        let format = match today {
            Some(today) if today.date_naive() == at.date_naive() => "%H:%M",
            Some(today) if today.year() == at.year() => "%b %-d %H:%M",
            _ => "%Y-%m-%d %H:%M",
        };
        parts.push(at.format(format).to_string());
    }
    Padding::new(
        EdgeInsets::only(2, 1, 1, 0),
        Text::new(parts.join(" • ")).style(tokens.dim().attrs(Attrs::DIM)),
    )
    .into()
}
