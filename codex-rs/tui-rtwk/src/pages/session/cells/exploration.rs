//! The lines of an exploration cell (oracle `exec_cell/render.rs` `exploring_display_lines`, cut by
//! `compact_hyperlink_lines`): one line per read, list or search the calls made, neighbouring reads
//! of calls that did not fail merged into one, the title in the accent colour, the exit code of a
//! failed call behind its last line.
//!
//! The oracle wraps a line at the cell's whole width (the title counted as its indent) and only then
//! puts the `  └ ` gutter in front, then clips each row at the width with `…`: a line wider than the
//! room loses its tail behind the cut and goes on in the next row. `LayoutBuilder` + `segmenter_of(cx)`
//! (`widgets/measure.rs`' break engine) say the width; the rows are built from it.

use rtwk::AnyWidget;
use rtwk::LayoutBuilder;
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Span, Style};
use rtwk::text::segmenter_of;
use rtwk_render::objects::text::{TextWrap, break_lines};

use codex_tui_core::threads::{ExplorationAction, ToolActivity, ToolKind, ToolStatus};

use super::strings::Strings;
use super::tool_rows::{BULLET_COLUMNS, branch_row, clipped};

/// The rows the compact cell keeps under its header (oracle `DETAIL_PREVIEW_LINES`).
pub const PREVIEW_ROWS: usize = 3;

/// How a piece of a line is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Paint {
    Plain,
    Dim,
    Red,
}

/// One line of the cell: a title and the pieces behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    title: &'static str,
    pieces: Vec<(String, Paint)>,
}

/// What a call of the cell contributes.
struct Call<'a> {
    actions: &'a [ExplorationAction],
    compound: bool,
    failed: Option<i32>,
}

impl Call<'_> {
    fn reads_only(&self) -> bool {
        self.actions
            .iter()
            .all(|action| matches!(action, ExplorationAction::Read(_)))
    }
}

/// The exit code of a call that ended without success (oracle `CommandOutput::from_execution`: a
/// status other than completed is exit code 1 unless the server said another); `None` while it runs.
pub fn failure(tool: &ToolActivity) -> Option<i32> {
    let ToolKind::Shell { exit_code, .. } = &tool.kind else {
        return None;
    };
    match tool.status {
        ToolStatus::Running => None,
        ToolStatus::Completed => exit_code.filter(|code| *code != 0),
        ToolStatus::Failed | ToolStatus::Declined => Some(exit_code.filter(|code| *code != 0).unwrap_or(1)),
    }
}

/// The calls that failed (the header says how many).
pub fn failures(calls: &[&ToolActivity]) -> usize {
    calls.iter().filter(|call| failure(call).is_some()).count()
}

/// The lines of `calls` (oracle `exploring_display_lines`).
pub fn steps(calls: &[&ToolActivity], strings: &Strings) -> Vec<Step> {
    let calls: Vec<Call<'_>> = calls
        .iter()
        .filter_map(|tool| match &tool.kind {
            ToolKind::Shell {
                actions, compound, ..
            } => Some(Call {
                actions,
                compound: *compound,
                failed: failure(tool),
            }),
            _ => None,
        })
        .collect();
    let mut out = Vec::new();
    let mut at = 0;
    while at < calls.len() {
        let call = &calls[at];
        // Reads of calls that did not fail are one line; a failed call stands on its own.
        let group = if call.reads_only() && call.failed.is_none() {
            1 + calls[at + 1..]
                .iter()
                .take_while(|next| next.failed.is_none() && next.reads_only())
                .count()
        } else {
            1
        };
        let members = &calls[at..at + group];
        at += group;
        let mut lines: Vec<Step> = if call.reads_only() {
            let mut names: Vec<&str> = Vec::new();
            for action in members.iter().flat_map(|member| member.actions) {
                if let ExplorationAction::Read(read) = action {
                    for name in read {
                        if !names.contains(&name.as_str()) {
                            names.push(name);
                        }
                    }
                }
            }
            vec![read_step(&names, strings)]
        } else {
            call.actions
                .iter()
                .map(|action| step_of(action, strings))
                .collect()
        };
        if let Some(code) = call.failed
            && let Some(last) = lines.last_mut()
        {
            // A compound command has one exit code, not an outcome for each action; a search that
            // exits 1 may have found nothing: its code is told without calling it a failure.
            let template = if call.compound {
                strings.explore_command_exit
            } else {
                strings.explore_exit
            };
            let search = call
                .actions
                .iter()
                .any(|action| matches!(action, ExplorationAction::Search { .. }));
            let paint = if code == 1 && search {
                Paint::Dim
            } else {
                Paint::Red
            };
            last.pieces
                .push((strings.label(template, &code.to_string()), paint));
        }
        out.extend(lines);
    }
    out
}

fn read_step(names: &[&str], strings: &Strings) -> Step {
    let mut pieces = Vec::new();
    for (index, name) in names.iter().enumerate() {
        if index != 0 {
            pieces.push((strings.explore_names_separator.to_owned(), Paint::Dim));
        }
        pieces.push(((*name).to_owned(), Paint::Plain));
    }
    Step {
        title: strings.explore_read,
        pieces,
    }
}

fn step_of(action: &ExplorationAction, strings: &Strings) -> Step {
    match action {
        ExplorationAction::Read(names) => {
            read_step(&names.iter().map(String::as_str).collect::<Vec<_>>(), strings)
        }
        ExplorationAction::List(path) => Step {
            title: strings.explore_list,
            pieces: vec![(path.clone(), Paint::Plain)],
        },
        ExplorationAction::Search { query, path } => {
            let mut pieces = vec![(query.clone(), Paint::Plain)];
            if let Some(path) = path {
                pieces.push((strings.explore_in.to_owned(), Paint::Dim));
                pieces.push((path.clone(), Paint::Plain));
            }
            Step {
                title: strings.explore_search,
                pieces,
            }
        }
    }
}

/// The body of the compact cell: the lines wrapped at the cell's width, the first `PREVIEW_ROWS` rows
/// behind their gutter.
pub fn body(steps: Vec<Step>, accent: Style, dim: Style, red: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        // The cell's width: the room of the body and the bullet's column before it.
        let width = constraints.max_width.saturating_add(BULLET_COLUMNS).max(1);
        let paint = |paint: Paint| match paint {
            Paint::Plain => Style::default(),
            Paint::Dim => dim.clone(),
            Paint::Red => red.clone(),
        };
        let mut rows: Vec<Vec<Span>> = Vec::new();
        for step in &steps {
            let indent = i32::try_from(seg.width(step.title)).unwrap_or(i32::MAX).saturating_add(1);
            let text: String = step.pieces.iter().map(|(piece, _)| piece.as_str()).collect();
            let room = width.saturating_sub(indent).max(1);
            let chunks = if text.is_empty() {
                vec![(0, 0)]
            } else {
                break_lines(&seg, &text, room, TextWrap::Word)
            };
            for (index, (from, to)) in chunks.into_iter().enumerate() {
                let mut spans = vec![if index == 0 {
                    Span::styled(format!("{} ", step.title), accent.clone())
                } else {
                    Span::new(" ".repeat(usize::try_from(indent).unwrap_or(0)))
                }];
                // The chunk's slice of each piece.
                let mut start = 0;
                for (piece, style) in &step.pieces {
                    let end = start + piece.len();
                    let (a, b) = (from.max(start), to.min(end));
                    if a < b {
                        let shown = if end >= to { piece[a - start..b - start].trim_end() } else { &piece[a - start..b - start] };
                        spans.push(Span::styled(shown.to_owned(), paint(*style)));
                    }
                    start = end;
                }
                rows.push(spans);
            }
        }
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, spans) in rows.into_iter().take(PREVIEW_ROWS).enumerate() {
            column = column.child(branch_row(index, clipped(spans), dim.clone()));
        }
        column.into()
    })
    .into()
}
