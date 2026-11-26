//! The tool row of the virtualized transcript: the shell of the tool cells
//! (`cells/exec.rs`, `tool_call.rs`, `computer_activity.rs`). Flutter's
//! `ExpansionTile` state: the row keeps `expanded`, draws the bullet and the
//! shared control, the cell file says what its header, body and control are.

use std::sync::Arc;

use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row};
use rtwk::style::{Attrs, Style};
use rtwk::text::DefaultTextStyle;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_rich::highlight::SyntaxTheme;

use codex_tui_core::threads::{ItemKey, McpCall, ToolActivity, ToolKind, Transcript};

use crate::app::theme::Tokens;

use super::cells::disclosure::{self, disclosing};
use super::cells::tool_rows::{BULLET, Env, Parts};
use super::cells::{computer_activity, exec, tool_call};

#[derive(Clone, PartialEq)]
pub(super) struct ToolRow {
    pub first: Arc<ToolActivity>,
    pub rest: Vec<Arc<ToolActivity>>,
    pub output_lines: Option<usize>,
    pub detailed: bool,
}

/// Whether the tool row draws a closed control, which makes it an F4 stop.
pub(super) fn has_control(first: &ToolActivity) -> bool {
    match first.kind {
        ToolKind::Shell { .. } => exec::has_control(first),
        ToolKind::Mcp(_) | ToolKind::Dynamic(_) => true,
    }
}

/// Inputs that can change F4 stops for a fixed layout. Growing stdout only
/// changes the walk when a running shell gains an output control.
#[derive(Clone, PartialEq)]
pub(super) struct ActivityStamp {
    metadata: u64,
    output_stops: Vec<ItemKey>,
}

impl ActivityStamp {
    pub(super) fn of(transcript: &Transcript) -> Self {
        let mut output_stops: Vec<_> = transcript
            .streamed_shell_output_lines()
            .filter(|(_, tool, initial, current)| {
                exec::gained_output_control(tool, *initial, *current)
            })
            .map(|(key, _, _, _)| key.clone())
            .collect();
        output_stops.sort_unstable();
        Self {
            metadata: transcript.activity_revision(),
            output_stops,
        }
    }
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
        let tokens = Tokens::of(cx);
        let strings = super::cells::strings::of(cx);
        let syntax = SyntaxTheme::of(cx);
        let expanded = widget.detailed || disclosure::expanded(cx, self.expanded);
        let env = Env {
            strings,
            tokens: &tokens,
            syntax: &syntax,
            detailed: widget.detailed,
            expanded,
        };
        let calls: Vec<&ToolActivity> = widget.calls().collect();
        let parts = match &calls[0].kind {
            ToolKind::Shell { .. } => exec::parts(&calls, &env, widget.output_lines),
            ToolKind::Mcp(_) if calls[0].is_computer_activity() => {
                let group: Vec<&McpCall> = calls
                    .iter()
                    .filter_map(|call| match &call.kind {
                        ToolKind::Mcp(call) if call.is_computer_activity() => Some(call),
                        _ => None,
                    })
                    .collect();
                computer_activity::parts(&group, &env)
            }
            ToolKind::Mcp(call) => tool_call::mcp(call, &env),
            ToolKind::Dynamic(call) => tool_call::dynamic(call, &env),
        };
        let control = tokens.dim().attrs(Attrs::DIM);
        match parts {
            Parts::Plain(content, label) => match label.filter(|_| !widget.detailed) {
                Some(label) => disclosing(cx, content, label, control, expanded),
                None => content,
            },
            Parts::Rows(rows) => {
                // the cell the reader has the focus on (F4) has its first row underlined (oracle
                // `render_disclosure`)
                let header = if disclosure::focused(cx) && !widget.detailed {
                    DefaultTextStyle::new(Style::new().attrs(Attrs::UNDERLINE), rows.header).into()
                } else {
                    rows.header
                };
                let mut content = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(header);
                for row in rows.body {
                    content = content.child(row);
                }
                // The control hangs off the cell's left edge, the bullet's column included.
                let cell: AnyWidget = Row::new()
                    .spacing(1)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(ExcludeSelection::new(Text::line(BULLET).style(rows.mark)))
                    .child(Expanded::new(1, content))
                    .into();
                match rows.disclosure.filter(|_| !widget.detailed) {
                    Some(label) => disclosing(cx, cell, label, control, expanded),
                    None => cell,
                }
            }
        }
    }
}

impl_widget!(stateful eq ToolRow);
