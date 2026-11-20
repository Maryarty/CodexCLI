//! Tool previews and turn timing in the virtualized transcript.

use std::sync::Arc;

use chrono::{Datelike, Local, TimeZone};
use rtwk::controls::ExpansionTile;
use rtwk::layout::{CrossAxisAlignment, EdgeInsets, Expanded, Padding, Row};
use rtwk::style::Span;
use rtwk::text::TextOverflow;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::threads::{ToolActivity, ToolKind, ToolStatus, TurnTiming};

use crate::app::theme::Tokens;

use super::strings;

#[derive(Clone, PartialEq)]
pub(super) struct ToolRow(pub Arc<ToolActivity>);

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
        let tool = &widget.0;
        let tokens = Tokens::of(cx);
        let strings = strings::of(cx);
        let (verb, description) = match &tool.kind {
            ToolKind::Shell { command } => (
                if tool.status == ToolStatus::Running {
                    strings.running_command
                } else {
                    strings.ran_command
                },
                command.clone(),
            ),
            ToolKind::Call {
                namespace,
                name,
                arguments,
            } => {
                let name = namespace
                    .as_ref()
                    .map_or_else(|| name.clone(), |ns| format!("{ns}.{name}"));
                let args = if arguments.is_null() {
                    String::new()
                } else {
                    arguments.to_string()
                };
                (
                    if tool.status == ToolStatus::Running {
                        strings.calling_tool
                    } else {
                        strings.called_tool
                    },
                    format!("{name}({args})"),
                )
            }
        };
        let header = Text::rich(vec![
            Span::styled(verb, tokens.bold()),
            Span::new(strings::INLINE_GAP),
            Span::styled(description, tokens.info()),
        ])
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis);
        let mark_style = match tool.status {
            ToolStatus::Running => tokens.accent(),
            ToolStatus::Completed => tokens.success(),
            ToolStatus::Failed | ToolStatus::Declined => tokens.error(),
        };
        let content: AnyWidget = if let Some(output) = tool
            .output
            .as_deref()
            .filter(|text| !text.trim().is_empty())
        {
            let details: AnyWidget = if self.expanded {
                Text::new(output.trim_end().to_owned())
                    .style(tokens.dim())
                    .into()
            } else {
                AnyWidget::default()
            };
            let mut tile = ExpansionTile::new(header, details)
                .on_expansion_changed(cx.map(std::convert::identity));
            if !self.expanded {
                let preview = output.lines().take(3).collect::<Vec<_>>().join("\n");
                tile = tile.subtitle(
                    Text::new(preview)
                        .style(tokens.dim())
                        .max_lines(Some(3))
                        .overflow(TextOverflow::Ellipsis),
                );
            }
            tile.into()
        } else {
            header.into()
        };
        Row::new()
            .spacing(1)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::line("•").style(mark_style))
            .child(Expanded::new(1, content))
            .into()
    }
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
        Text::new(parts.join(" • ")).style(tokens.dim()),
    )
    .into()
}
