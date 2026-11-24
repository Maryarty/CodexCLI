// cell: Item::PlanUpdate(_)
//! A plan update (oracle history_cell/plans.rs `PlanUpdateCell`). Compact it
//! is the header with the steps done, then the first three steps (the running
//! one first, the done ones last), each one row clipped with an ellipsis; the
//! control (transcript_view/layout.rs `activity_layout`) appears when the
//! oracle's `activity_disclosure` says so: a note, more than three steps, or a
//! step wider than the row. Revealed (and in the detailed transcript) it is
//! the note and every step in plan order, each wrapped under a hanging
//! indent.
//!
//! Flutter: a `StatefulWidget` holding `expanded` (as an `ExpansionTile`), a
//! `LayoutBuilder` choosing between the cell with and without its control from
//! the measured width, the control from `disclosure.rs`, rows of `Text` in
//! `Row`/`Expanded`/`SizedBox`.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::threads::{PlanStep, PlanUpdate, StepStatus};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::one_row::one_row;
use super::strings::Strings;
use crate::widgets::measure::Measure;
use codex_tui_core::threads::Item;

/// How many steps the compact cell lists (oracle `DETAIL_PREVIEW_LINES`).
const PREVIEW_STEPS: usize = 3;
/// The compact row reserves four columns of indent and two of marker.
const STEP_RESERVE: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub struct PlanUpdateCell {
    pub update: Arc<PlanUpdate>,
    /// The detailed transcript shows everything and no control.
    pub detailed: bool,
}

#[derive(Default)]
pub struct PlanUpdateCellState {
    expanded: bool,
}

impl StatefulWidget for PlanUpdateCell {
    type State = PlanUpdateCellState;

    fn create_state(&self) -> PlanUpdateCellState {
        PlanUpdateCellState::default()
    }
}

impl_widget!(stateful eq PlanUpdateCell);

impl State for PlanUpdateCellState {
    type Widget = PlanUpdateCell;
    type Msg = bool;

    fn update(&mut self, expanded: bool, _: &PlanUpdateCell, _: &mut Cx<'_, Self>) -> Rebuild {
        self.expanded = expanded;
        Rebuild::Yes
    }

    fn build(&self, widget: &PlanUpdateCell, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let update = &widget.update;
        if widget.detailed {
            return revealed(update, &tokens, strings);
        }
        let seg = segmenter_of(cx);
        let compact = compact(update, &tokens, strings);
        let shown = if self.expanded {
            revealed(update, &tokens, strings)
        } else {
            compact.clone()
        };
        let label = if self.expanded {
            strings.hide_details
        } else {
            strings.show_details
        };
        let with_control = super::disclosure::disclosing(
            cx,
            shown,
            label,
            tokens.dim().attrs(Attrs::DIM),
            self.expanded,
        );
        let update = update.clone();
        LayoutBuilder::new(move |_, room| {
            let width = usize::try_from(room.max_width).unwrap_or(0);
            if discloses(&update, seg, width) {
                with_control.clone()
            } else {
                compact.clone()
            }
        })
        .into()
    }
}

/// The oracle's `activity_disclosure`: a note, more steps than the preview
/// holds, or a step the compact row would clip.
fn discloses(update: &PlanUpdate, seg: Measure, width: usize) -> bool {
    update.note().is_some()
        || update.steps.len() > PREVIEW_STEPS
        || update
            .steps
            .iter()
            .any(|step| seg.width(&one_line(&step.text)) + STEP_RESERVE > width)
}

/// A step on one line: runs of white space are one blank.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The marker and the style of a step by its status.
fn marker(step: &PlanStep, tokens: &Tokens) -> (&'static str, Style) {
    match step.status {
        StepStatus::Completed => ("✔ ", tokens.dim().attrs(Attrs::STRIKE | Attrs::DIM)),
        StepStatus::InProgress => ("□ ", tokens.accent().attrs(Attrs::BOLD)),
        StepStatus::Pending => ("□ ", tokens.dim().attrs(Attrs::DIM)),
    }
}

/// `compact_hyperlink_lines`: the header and the first steps, a row each.
fn compact(update: &PlanUpdate, tokens: &Tokens, strings: &Strings) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(one_row(vec![
            Span::styled(strings.bullet, tokens.dim().attrs(Attrs::DIM)),
            Span::styled(strings.updated_plan, tokens.bold()),
            Span::styled(progress(update, strings), tokens.dim().attrs(Attrs::DIM)),
        ]));
    let mut steps: Vec<&PlanStep> = update.steps.iter().collect();
    // In progress, then pending, then completed; the order inside each kept.
    steps.sort_by_key(|step| match step.status {
        StepStatus::InProgress => 0,
        StepStatus::Pending => 1,
        StepStatus::Completed => 2,
    });
    for (index, step) in steps.into_iter().take(PREVIEW_STEPS).enumerate() {
        let (mark, style) = marker(step, tokens);
        column = column.child(one_row(vec![
            Span::styled(branch(index == 0, strings), tokens.dim().attrs(Attrs::DIM)),
            Span::styled(mark, style.clone()),
            Span::styled(one_line(&step.text), style),
        ]));
    }
    if update.steps.is_empty() {
        column = column.child(one_row(vec![
            Span::styled(strings.plan_branch, tokens.dim().attrs(Attrs::DIM)),
            Span::styled(strings.no_steps, tokens.dim().attrs(Attrs::DIM)),
        ]));
    }
    column.into()
}

/// ` · 1/4 complete`: the steps done of all.
fn progress(update: &PlanUpdate, strings: &Strings) -> String {
    let (lead, of, tail) = strings.plan_progress;
    [
        lead,
        &update.completed().to_string(),
        of,
        &update.steps.len().to_string(),
        tail,
    ]
    .concat()
}

/// The branch mark of a row: the first row of the block carries `└`.
fn branch(first: bool, strings: &Strings) -> &'static str {
    if first {
        strings.plan_branch
    } else {
        strings.plan_indent
    }
}

/// `transcript_hyperlink_lines`: the header, then the note and every step
/// under one branch, wrapped at the width; a step's marker hangs.
fn revealed(update: &PlanUpdate, tokens: &Tokens, strings: &Strings) -> AnyWidget {
    let mut body = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    if let Some(note) = update.note() {
        body = body.child(
            Text::new(note.to_owned()).style(tokens.dim().attrs(Attrs::DIM | Attrs::ITALIC)),
        );
    }
    if update.steps.is_empty() {
        body = body.child(
            Text::new(strings.no_steps).style(tokens.dim().attrs(Attrs::DIM | Attrs::ITALIC)),
        );
    }
    for step in &update.steps {
        let (mark, style) = marker(step, tokens);
        body = body.child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(SizedBox::new(
                    Some(2),
                    None,
                    Some(Text::new(mark).style(style.clone()).into()),
                ))
                .child(Expanded::new(1, Text::new(step.text.clone()).style(style))),
        );
    }
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::rich(vec![
            Span::styled(strings.bullet, tokens.dim().attrs(Attrs::DIM)),
            Span::styled(strings.updated_plan, tokens.bold()),
        ]))
        .child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(SizedBox::new(
                    Some(4),
                    None,
                    Some(
                        Text::new(strings.plan_branch)
                            .style(tokens.dim().attrs(Attrs::DIM))
                            .into(),
                    ),
                ))
                .child(Expanded::new(1, body)),
        )
        .into()
}

/// An `update_plan` report.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::PlanUpdate(update) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        PlanUpdateCell {
            update: update.clone(),
            detailed: env.detailed,
        }
        .into(),
    )
}
