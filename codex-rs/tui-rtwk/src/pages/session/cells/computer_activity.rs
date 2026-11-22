//! A run of adjacent computer calls as one cell (oracle
//! `history_cell/computer_activity.rs`): `• Using|Used computer · N
//! action(s)[ · N failed]` and the calls worth showing under it. The preview
//! favours a failure and an image over order but keeps the calls' own order;
//! the shared control (not the oracle's `N more · ⌃t` row) reveals the rest.

use rtwk::layout::{
    BoxConstraints, Column, ConstrainedBox, CrossAxisAlignment, Expanded, MainAxisSize, Row,
    UNBOUNDED,
};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, LayoutBuilder};

use codex_tui_core::threads::McpCall;

use crate::app::theme::Tokens;
use crate::widgets::dots::dots_end;

use super::strings::Strings;
use super::tool_call::{mcp_mark, transcript};
use super::tool_rows::{BULLET_COLUMNS, Env, Parts, Rows, glyph_row};

/// The calls shown when more than three are done (oracle: two and the
/// `N more` row).
const SHOWN_WHEN_MANY: usize = 2;
const SHOWN: usize = 3;
/// The summary room of a row under a `├ ` / `└ ` branch (oracle: `width - 4`).
const BRANCH_COLUMNS: i32 = 2;
/// Below this width a failed row drops its title (oracle `width < 60`).
const WIDE_FAILURE_FROM: i32 = 60;

/// The group of `calls` (all of them computer calls).
pub fn parts(calls: &[&McpCall], env: &Env<'_>) -> Parts {
    let Env {
        strings,
        tokens,
        expanded,
        ..
    } = *env;
    let disclosure = Some(
        if expanded {
            strings.hide_details
        } else {
            strings.show_details
        }
        .to_owned(),
    );
    if expanded {
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for call in calls {
            let mark = mcp_mark(call, tokens);
            column = column.child(
                Row::new()
                    .spacing(1)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(Text::line("•").style(mark))
                    .child(Expanded::new(
                        1,
                        transcript((*call).clone(), strings, tokens.clone()),
                    )),
            );
        }
        return Parts::Plain(column.into(), disclosure);
    }
    let active = calls.iter().rposition(|call| call.result.is_none());
    let failures = calls
        .iter()
        .filter(|call| call.success() == Some(false))
        .count();
    let count = calls.len();
    let label = if active.is_some() {
        strings.computer_using
    } else {
        strings.computer_used
    };
    let counted = strings.label(
        if count == 1 {
            strings.computer_action_one
        } else {
            strings.computer_action_many
        },
        &count.to_string(),
    );
    let mut header = vec![
        Span::styled(label, tokens.bold()),
        Span::styled(counted, tokens.dim().attrs(Attrs::DIM)),
    ];
    if failures > 0 {
        header.push(Span::styled(
            strings.label(strings.computer_failed_count, &failures.to_string()),
            tokens.error(),
        ));
    }
    let shown = match active {
        Some(index) => vec![index],
        None => {
            let mut indices: Vec<usize> = (0..count).collect();
            // A failure first, then an image, then the later call.
            indices.sort_by_key(|&index| {
                let call = calls[index];
                std::cmp::Reverse((call.success() == Some(false), call.has_image(), index))
            });
            indices.truncate(if count > SHOWN {
                SHOWN_WHEN_MANY
            } else {
                SHOWN
            });
            indices
        }
    };
    let mut shown = shown;
    shown.sort_unstable();
    let last = shown.len().saturating_sub(1);
    let body = shown
        .iter()
        .enumerate()
        .map(|(row, &index)| summary(calls[index], row == last, strings, tokens))
        .collect();
    Parts::Rows(Rows {
        mark: tokens.dim().attrs(Attrs::DIM),
        header: Text::rich(header).into(),
        body,
        disclosure,
    })
}

/// One shown call: its title, or why it failed, on one row (oracle `preview`:
/// the cut keeps the blanks before its `…`, `dots_end`).
fn summary(call: &McpCall, last: bool, strings: &'static Strings, tokens: &Tokens) -> AnyWidget {
    let title = one_line(
        call.action_title()
            .unwrap_or(strings.computer_default_title),
    );
    let dim = tokens.dim().attrs(Attrs::DIM);
    let failed = call.success() == Some(false);
    let body: AnyWidget = if failed {
        let error = call.error_preview().map(one_line);
        let red = tokens.error();
        LayoutBuilder::new(move |_, room| {
            // The oracle's `width` is the cell's: the summary room, its branch and the bullet.
            let width = room.max_width + BRANCH_COLUMNS + BULLET_COLUMNS;
            let prefix = || Text::line(strings.computer_failed_prefix).style(red.clone());
            match &error {
                Some(error) if width >= WIDE_FAILURE_FROM => Row::new()
                    .child(prefix())
                    .child(ConstrainedBox::new(
                        BoxConstraints::new(0, width / 3, 0, UNBOUNDED),
                        dots_end(title.clone(), red.clone()),
                    ))
                    .child(Text::line(strings.computer_title_error_gap).style(red.clone()))
                    .child(Expanded::new(1, dots_end(error.clone(), red.clone())))
                    .into(),
                Some(error) => dots_end(
                    format!("{}{error}", strings.computer_failed_prefix),
                    red.clone(),
                ),
                None => dots_end(
                    format!("{}{title}", strings.computer_failed_prefix),
                    red.clone(),
                ),
            }
        })
        .into()
    } else if call.has_image() {
        dots_end(
            strings.label(strings.computer_screenshot, &title),
            dim.clone(),
        )
    } else {
        dots_end(title, dim.clone())
    };
    glyph_row(if last { "└ " } else { "├ " }, body, dim)
}

/// Whitespace as single blanks (oracle `preview`'s normalisation).
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
