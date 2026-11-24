//! A finished hook run (oracle history_cell/hook_cell.rs `HookCell::output_lines`).
//! A completed run with a system message (its first warning entry) is that
//! message after a dim `↳ Hook · `; any other run is a header (a green bullet
//! for a completed one, a red one else) with the system message and the other
//! output entries under a `└` branch. Output is drawn as the hook wrote it:
//! no clipping, and SGR codes in the system message read as styles.
//!
//! Flutter: a stateless widget, a `Column` of one `Text.rich` per row.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::{HookEntryKind, HookRun, HookStatus};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookCell {
    pub run: Arc<HookRun>,
}

impl_widget!(stateless eq HookCell);

impl StatelessWidget for HookCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for row in rows(&self.run, &tokens, strings) {
            column = column.child(if row.is_empty() {
                Text::new(strings.gap)
            } else {
                Text::rich(row)
            });
        }
        column.into()
    }
}

/// The texts of the cell: one per row, the gap for a blank one. The text does
/// not depend on the theme: any tokens give the rows.
pub fn texts(run: &HookRun, strings: &Strings) -> Vec<String> {
    let tokens = Tokens::new(std::rc::Rc::new(rtwk::prelude::ThemeData::DARK));
    rows(run, &tokens, strings)
        .into_iter()
        .map(|row| {
            if row.is_empty() {
                strings.gap.to_owned()
            } else {
                row.iter().map(|span| span.text.as_str()).collect()
            }
        })
        .collect()
}

/// The rows, each its spans; a blank row has none.
fn rows(run: &HookRun, tokens: &Tokens, strings: &Strings) -> Vec<Vec<Span>> {
    let mut rows: Vec<Vec<Span>> = Vec::new();
    let mut system = run
        .entries
        .iter()
        .find(|entry| entry.kind == HookEntryKind::Warning)
        .map(|entry| message_lines(&entry.text))
        .map(Vec::into_iter);
    let first = system.as_mut().and_then(Iterator::next);
    match (run.status, first) {
        (HookStatus::Completed, Some(first)) => {
            rows.push(
                [
                    vec![Span::styled(
                        strings.hook_provenance,
                        tokens.dim().attrs(Attrs::DIM),
                    )],
                    first,
                ]
                .concat(),
            );
        }
        (status, first) => {
            let (bullet, header) = match status {
                HookStatus::Completed => (tokens.success(), strings.hook_completed),
                HookStatus::Failed => (tokens.error(), strings.hook_failed),
                HookStatus::Blocked => (tokens.error(), strings.hook_blocked),
                HookStatus::Stopped => (tokens.error(), strings.hook_stopped),
            };
            rows.push(vec![
                Span::styled(strings.hook_bullet, bullet.attrs(Attrs::BOLD)),
                Span::new(strings.hook_after_bullet),
                Span::new(header),
            ]);
            if let Some(first) = first {
                rows.push([vec![Span::new(strings.hook_branch)], first].concat());
            }
        }
    }
    for line in system.into_iter().flatten() {
        rows.push(if line.is_empty() {
            Vec::new()
        } else {
            [vec![Span::new(strings.hook_body_indent)], line].concat()
        });
    }
    for entry in &run.entries {
        if entry.kind == HookEntryKind::Warning {
            continue;
        }
        let mut lines = entry.text.split('\n');
        if let Some(first) = lines.next() {
            rows.push(vec![
                Span::new(strings.hook_branch),
                Span::new(first.to_owned()),
            ]);
        }
        for line in lines {
            rows.push(if line.is_empty() {
                Vec::new()
            } else {
                vec![
                    Span::new(strings.hook_body_indent),
                    Span::new(line.to_owned()),
                ]
            });
        }
    }
    rows
}

/// A system message's lines with their SGR styles; a final newline leaves
/// a blank row (oracle `ansi_escape(..)` plus the pushed blank line).
fn message_lines(text: &str) -> Vec<Vec<Span>> {
    let mut lines = rtwk::ansi::parse_ansi(text.trim_end_matches('\n'));
    if text.ends_with('\n') {
        lines.push(Vec::new());
    }
    lines
}

/// A finished hook run.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, _env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Hook(run) = item else {
        return AnyWidget::default();
    };
    below(place.separator, HookCell { run: run.clone() }.into())
}
