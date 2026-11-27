//! A run of adjacent computer calls as one cell (oracle
//! `history_cell/computer_activity.rs`): `• Using|Used computer · N
//! action(s)[ · N failed]` and the calls worth showing under it. The preview
//! favours a failure and an image over order but keeps the calls' own order;
//! the owned transcript has a shared disclosure, and the inline viewport
//! reports its hidden calls with the scrollback hint.

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
use super::text::{Env as TextEnv, Part, Room};
use super::tool_call::{mcp_mark, text_room, transcript, transcript_parts};
use super::tool_rows::{BULLET, BULLET_COLUMNS, Env, Parts, Rows, glyph_row};

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
        inline,
        ..
    } = *env;
    let disclosure = (expanded || !inline).then(|| {
        if expanded {
            strings.hide_details
        } else {
            strings.show_details
        }
        .to_owned()
    });
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
    let Compact {
        label,
        counted,
        failed,
        shown,
        remaining,
    } = compact(calls, strings);
    let mut header = vec![
        Span::styled(label, tokens.bold()),
        Span::styled(counted, tokens.dim().attrs(Attrs::DIM)),
    ];
    if let Some(failed) = failed {
        header.push(Span::styled(failed, tokens.error()));
    }
    let tail = inline && remaining > 0;
    let mut body: Vec<_> = shown
        .iter()
        .enumerate()
        .map(|(row, &index)| {
            summary(
                calls[index],
                !tail && row + 1 == shown.len(),
                strings,
                tokens,
            )
        })
        .collect();
    if tail {
        let dim = tokens.dim().attrs(Attrs::DIM);
        let hint = strings.label(strings.computer_more, &remaining.to_string());
        body.push(glyph_row("└ ", dots_end(hint, dim.clone()), dim));
    }
    Parts::Rows(Rows {
        mark: tokens.dim().attrs(Attrs::DIM),
        header: Text::rich(header).into(),
        body,
        disclosure,
    })
}

struct Compact {
    label: &'static str,
    counted: String,
    failed: Option<String>,
    shown: Vec<usize>,
    remaining: usize,
}

fn compact(calls: &[&McpCall], strings: &Strings) -> Compact {
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
    let failed =
        (failures > 0).then(|| strings.label(strings.computer_failed_count, &failures.to_string()));
    let shown = match active {
        Some(index) => vec![index],
        None => {
            let mut indices: Vec<_> = (0..count).collect();
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
            indices.sort_unstable();
            indices
        }
    };
    Compact {
        label,
        counted,
        failed,
        remaining: if active.is_none() {
            count - shown.len()
        } else {
            0
        },
        shown,
    }
}

fn label(call: &McpCall, strings: &Strings) -> String {
    one_line(
        call.action_title()
            .unwrap_or(strings.computer_default_title),
    )
}

fn preview(call: &McpCall, title: &str, strings: &Strings) -> String {
    if call.has_image() {
        strings.label(strings.computer_screenshot, title)
    } else {
        title.to_owned()
    }
}

pub(super) fn texts(calls: &[&McpCall], env: &TextEnv<'_>, room: Room, inline: bool) -> Vec<Part> {
    let strings = env.cells;
    let expanded = env.expanded || env.detailed;
    let mut parts = Vec::new();
    let room = text_room(room);
    if expanded {
        for call in calls {
            parts.push(Part::text(BULLET));
            parts.extend(transcript_parts(call, strings, room));
        }
    } else {
        let Compact {
            label,
            counted,
            failed,
            shown,
            remaining,
        } = compact(calls, strings);
        let tail = inline && remaining > 0;
        parts.push(Part::gutter(BULLET));
        parts.push(Part::text(format!(
            "{label}{counted}{}",
            failed.as_deref().unwrap_or_default()
        )));
        for (position, &index) in shown.iter().enumerate() {
            let branch = if !tail && position + 1 == shown.len() {
                "└ "
            } else {
                "├ "
            };
            parts.push(Part::gutter(branch));
            parts.extend(summary_parts(calls[index], strings, room.after(branch)));
        }
        if tail {
            let hint = strings.label(strings.computer_more, &remaining.to_string());
            parts.extend([Part::gutter("└ "), Part::text(hint.clone())]);
            if room.after("└ ").cuts(&hint) {
                parts.push(Part::gutter("…"));
            }
        }
    }
    if !env.detailed && (expanded || !inline) {
        parts.push(Part::gutter(if expanded {
            strings.hide_details
        } else {
            strings.show_details
        }));
    }
    parts
}

fn summary_parts(call: &McpCall, strings: &Strings, room: Room) -> Vec<Part> {
    fn line(parts: &mut Vec<Part>, text: String, room: Room) {
        let cut = room.cuts(&text);
        parts.push(Part::text(text));
        if cut {
            parts.push(Part::gutter("…"));
        }
    }

    let title = label(call, strings);
    let mut parts = Vec::new();
    if call.success() != Some(false) {
        line(&mut parts, preview(call, &title, strings), room);
        return parts;
    }
    let error = call.error_preview().map(one_line);
    let width = room.width + BRANCH_COLUMNS + BULLET_COLUMNS;
    let prefix = strings.computer_failed_prefix;
    if let Some(error) = error.as_ref().filter(|_| width >= WIDE_FAILURE_FROM) {
        let title_room = Room {
            width: width / 3,
            ..room
        };
        let used = title_room
            .width
            .min(i32::try_from(room.segmenter.width(&title)).unwrap_or(i32::MAX));
        let gap = strings.computer_title_error_gap;
        parts.push(Part::text(prefix));
        line(&mut parts, title, title_room);
        parts.push(Part::text(gap));
        line(
            &mut parts,
            error.clone(),
            Room {
                width: room.after(prefix).after(gap).width.saturating_sub(used),
                ..room
            },
        );
    } else {
        line(
            &mut parts,
            format!("{prefix}{}", error.as_deref().unwrap_or(&title)),
            room,
        );
    }
    parts
}

/// One shown call: its title, or why it failed, on one row (oracle `preview`:
/// the cut keeps the blanks before its `…`, `dots_end`).
fn summary(call: &McpCall, last: bool, strings: &'static Strings, tokens: &Tokens) -> AnyWidget {
    let title = label(call, strings);
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
    } else {
        dots_end(preview(call, &title, strings), dim.clone())
    };
    glyph_row(if last { "└ " } else { "├ " }, body, dim)
}

/// Whitespace as single blanks (oracle `preview`'s normalisation).
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
