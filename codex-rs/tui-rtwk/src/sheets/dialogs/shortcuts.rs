//! The shortcut overlay: three columns of bindings (the list page) or the
//! keyboard diagram (the keyboard page). Oracle: `bottom_pane/shortcut_overlay.rs`
//! (`lines`, column padding) and `shortcut_overlay/keyboard.rs`.

use crate::widgets::measure::Measure;
use rtwk::LayoutBuilder;
use rtwk::focus::Focus;
use rtwk::layout::{
    Axis, Column, CrossAxisAlignment, Expanded, Flexible, Grid, GridPlacement, GridTrack,
    MainAxisSize, Row, SizedBox,
};
use rtwk::overlay::Dialog;
use rtwk::scroll::{NeverScrollableScrollPhysics, SingleChildScrollView};
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};
use std::rc::Rc;

use super::dialog::{Msg, SmallDialogState};
use super::strings::{Keys, Strings};
use crate::app::theme::Tokens;
use crate::keymap::{bind_fixed, label, stroke_for, stroke_label};
use codex_tui_core::keymap::fixed;
use codex_tui_core::keymap::actions::GLOBAL_OPEN_AGENTS;
use codex_tui_core::keymap::{Keymap, Stroke};

/// The two pages of the shortcut overlay (`bottom_pane/shortcut_overlay.rs`: `ShortcutView`):
/// the list of bindings and the keyboard diagram. Drawn state: Tab swaps them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ShortcutView {
    #[default]
    List,
    Keyboard,
}

impl ShortcutView {
    pub(super) fn other(self) -> Self {
        match self {
            ShortcutView::List => ShortcutView::Keyboard,
            ShortcutView::Keyboard => ShortcutView::List,
        }
    }
}

/// The dialog of the page `view`: Tab swaps the pages, `?` and Esc close.
pub(super) fn build(view: ShortcutView, s: &Strings, cx: &mut Cx<'_, SmallDialogState>) -> Focus {
    let tokens = Tokens::of(cx);
    let (title, rows) = match view {
        ShortcutView::List => (s.shortcuts_list_title, list_rows(s, &tokens)),
        ShortcutView::Keyboard => (s.shortcuts_keyboard_title, keyboard_rows(s, &tokens)),
    };
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::new(String::new()));
    for row in rows {
        column = column.child(row);
    }
    let column = column
        .child(Text::line(s.keymap_customize).style(tokens.dim()))
        .child(Text::new(String::new()))
        .child(Text::line(s.shortcuts_close).style(tokens.dim()));
    let focus =
        Focus::new(Dialog::new(column).title(title).max_width(None)).can_request_focus(false);
    let focus = bind_fixed(focus, fixed::DISMISS, cx.send(Msg::Close));
    bind_fixed(
        bind_fixed(focus, fixed::OPEN_SHORTCUTS, cx.send(Msg::Close)),
        fixed::TAB,
        cx.send(Msg::Toggle),
    )
}

/// The overlay as the oracle's bottom pane draws it (`lines`, `render`): the
/// bold title, the groups and `/keymap customize`, two cells in, to stand above
/// the composer in a `Flexible` of the bottom pane's column; the close hint is
/// the footer's (`close_hint`). Given less room than it has rows it keeps
/// `/keymap customize` and says `… resize to see all` over it: Flutter's
/// `Column(min)[Flexible(SingleChildScrollView(body)), note, customize]`, the
/// body's rows what its own arrangement holds (`groups`).
pub(super) fn panel(
    s: &'static Strings,
    tokens: &Tokens,
    sections: Vec<Resolved>,
    flexible: bool,
) -> AnyWidget {
    let tokens = tokens.clone();
    let inner = LayoutBuilder::new(move |cx, constraints| {
        let seg = segmenter_of(cx);
        let (groups, group_rows) = groups(seg, &tokens, &sections, constraints.max_width);
        // the title, a blank, the groups, a blank
        let body_rows = 3 + group_rows;
        let body = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::line(s.shortcuts_title).style(tokens.bold()))
            .child(Text::new(String::new()))
            .child(groups)
            .child(Text::new(String::new()));
        // `/keymap` in the accent, the rest dim.
        let (command, rest) = s
            .keymap_customize
            .split_at(s.keymap_customize.find(' ').unwrap_or(0));
        let customize = Row::new()
            .child(Text::line(command).style(tokens.accent()))
            .child(Text::line(rest).style(tokens.dim()));
        let room = constraints.max_height;
        if room <= 0 {
            // no row left: the oracle renders into an empty area
            return SizedBox::fixed(0, 0).into();
        }
        if room > body_rows {
            return Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(body)
                .child(customize)
                .into();
        }
        let mut clipped = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Flexible::new(
                1,
                SingleChildScrollView::new(body)
                    .physics(Rc::new(NeverScrollableScrollPhysics))
                    .can_request_focus(false),
            ));
        if room >= 2 {
            clipped = clipped.child(Text::line(s.shortcuts_resize).style(tokens.dim()));
        }
        clipped.child(customize).into()
    });
    let row = Row::new()
        .child(SizedBox::new(Some(2), None, None))
        .child(Expanded::new(1, inner));
    if flexible {
        Flexible::new(OUTWEIGHS_THE_GAP, row).into()
    } else {
        row.into()
    }
}

/// The panel's flex against the reader's gap (flex 1) in the bottom pane's
/// column: the panel takes the rows it needs and the gap keeps its one from
/// a free height of nine rows on.
const OUTWEIGHS_THE_GAP: i32 = 8;

/// A column of the overlay with its keys resolved: what the user's keymap and
/// the running turn make of the table's rows.
pub(super) struct Resolved {
    heading: &'static str,
    rows: Vec<(String, &'static str)>,
}

/// The columns of `bindings` (`shortcut_overlay.rs`): a row whose action is
/// unbound is left out, `Send message` and `Quit` turn into `Queue message`
/// and `Interrupt` while a turn runs.
pub(super) fn resolve(
    s: &Strings,
    keymap: &Keymap,
    running: bool,
    agents: bool,
) -> Vec<Resolved> {
    let spell = |stroke: &Stroke| stroke_label(stroke);
    s.shortcut_sections
        .iter()
        .map(|section| {
            let rows = section
                .rows
                .iter()
                .filter_map(|entry| {
                    let keys = match entry.keys {
                        Keys::Plain(text) => text.to_string(),
                        Keys::Fixed(id) => spell(&stroke_for(id)?),
                        Keys::FixedPair(a, b) => format!(
                            "{}{}{}",
                            spell(&stroke_for(a)?),
                            s.shortcut_either,
                            spell(&stroke_for(b)?)
                        ),
                        // the empty prompt's `←` stands for the agents page when it is reachable
                        Keys::Bound(id) if agents && id == GLOBAL_OPEN_AGENTS => {
                            stroke_label(&stroke_for(fixed::OPEN_AGENTS_FROM_LEFT)?)
                        }
                        Keys::Bound(id) => label(keymap, id)?,
                    };
                    let action = match (running, entry.running) {
                        (true, Some(running)) => running,
                        _ => entry.label,
                    };
                    Some((keys, action))
                })
                .collect();
            Resolved {
                heading: section.heading,
                rows,
            }
        })
        .collect()
}

/// Cells between the columns (`shortcut_help.rs`, `COLUMN_GAP`).
const COLUMN_GAP: i32 = 4;

/// The groups in as many columns as `room` holds (`group_lines`): three; the
/// first and the other two stacked; or all stacked.
fn groups(seg: Measure, tokens: &Tokens, sections: &[Resolved], room: i32) -> (AnyWidget, i32) {
    let cells = |text: &str| seg.width(text) as i32;
    let widths: Vec<i32> = sections
        .iter()
        .map(|group| {
            let keys = group
                .rows
                .iter()
                .map(|(key, _)| cells(key))
                .max()
                .unwrap_or(0);
            let rows = group.rows.iter().map(|(_, label)| keys + 2 + cells(label));
            rows.chain([cells(group.heading)]).max().unwrap_or(0)
        })
        .collect();
    let section = |index: usize| -> AnyWidget {
        let group = &sections[index];
        let mut grid = Grid::new()
            .columns([GridTrack::content(), GridTrack::content()])
            .column_gap(2)
            .child(GridPlacement::new(Text::line(group.heading).style(tokens.bold())).span(1, 2));
        for (key, label) in &group.rows {
            grid = grid
                .child(Text::line(key.clone()).style(tokens.accent()))
                .child(Text::line(*label));
        }
        grid.into()
    };
    let stack = |indexes: &[usize]| -> AnyWidget {
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start);
        for (at, index) in indexes.iter().enumerate() {
            if at > 0 {
                column = column.child(Text::new(String::new()));
            }
            column = column.child(section(*index));
        }
        column.into()
    };
    let side = |columns: Vec<AnyWidget>| -> AnyWidget {
        let mut row = Row::new().cross_axis_alignment(CrossAxisAlignment::Start);
        for (at, column) in columns.into_iter().enumerate() {
            if at > 0 {
                row = row.child(SizedBox::new(Some(COLUMN_GAP), None, None));
            }
            row = row.child(column);
        }
        row.into()
    };
    // a section is its heading and its rows; stacked ones have a blank between
    let heights: Vec<i32> = sections
        .iter()
        .map(|group| 1 + group.rows.len() as i32)
        .collect();
    if widths.iter().sum::<i32>() + COLUMN_GAP * 2 <= room {
        let rows = heights.iter().copied().max().unwrap_or(0);
        (side(vec![stack(&[0]), stack(&[1]), stack(&[2])]), rows)
    } else if widths[0] + COLUMN_GAP + widths[1].max(widths[2]) <= room {
        let rows = heights[0].max(heights[1] + 1 + heights[2]);
        (side(vec![stack(&[0]), stack(&[1, 2])]), rows)
    } else {
        // Narrower than the widest row the oracle wraps by word (`word_wrap_lines`);
        // here the rows are clipped at the right edge (a horizontal view that does
        // not scroll), one row each.
        let rows = heights.iter().sum::<i32>() + 2;
        let clipped = SingleChildScrollView::new(stack(&[0, 1, 2]))
            .scroll_direction(Axis::Horizontal)
            .physics(Rc::new(NeverScrollableScrollPhysics))
            .can_request_focus(false);
        (clipped.into(), rows)
    }
}

/// The headings and the rows of the three columns, the keys in bold: a
/// `Grid` of a key track and a label track per group, rtwk sizes the tracks
/// to their content; a group is two cells (the gap) from the next.
fn list_rows(s: &Strings, tokens: &Tokens) -> Vec<AnyWidget> {
    let groups = &s.shortcut_groups;
    let mut tracks = Vec::new();
    for index in 0..groups.len() {
        tracks.extend([GridTrack::content(), GridTrack::content()]);
        if index + 1 < groups.len() {
            tracks.push(GridTrack::fixed(0));
        }
    }
    let mut grid = Grid::new().columns(tracks).column_gap(2);
    let between = |grid: Grid, index: usize| {
        if index + 1 < groups.len() {
            grid.child(SizedBox::fixed(0, 1))
        } else {
            grid
        }
    };
    for (index, group) in groups.iter().enumerate() {
        grid = grid
            .child(GridPlacement::new(Text::line(group.heading).style(tokens.bold())).span(1, 2));
        grid = between(grid, index);
    }
    let height = groups
        .iter()
        .map(|group| group.rows.len())
        .max()
        .unwrap_or(0);
    for row in 0..height {
        for (index, group) in groups.iter().enumerate() {
            grid = match group.rows.get(row) {
                Some((key, label)) => grid
                    .child(Text::line(*key).style(tokens.accent()))
                    .child(Text::line(*label)),
                None => grid
                    .child(SizedBox::fixed(0, 1))
                    .child(SizedBox::fixed(0, 1)),
            };
            grid = between(grid, index);
        }
    }
    vec![grid.into(), Text::new(String::new()).into()]
}

/// The keyboard diagram, as the oracle draws the base layout.
fn keyboard_rows(s: &Strings, tokens: &Tokens) -> Vec<AnyWidget> {
    let mut out: Vec<AnyWidget> = s
        .keyboard
        .iter()
        .map(|line| Text::line(*line).style(tokens.dim()).into())
        .collect();
    out.push(Text::new(String::new()).into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rtwk::prelude::ThemeData;
    use rtwk_core::unicode::TextSegmenter;

    fn sections() -> Vec<Resolved> {
        // the first group's label is `ｶﾞ`: 4 cells clustered, 5 in legacy, as wide as
        // the others' rows, so three columns need 4 + 4 + 4 + 2 gaps of 4 = 20
        let group = |label: &'static str| Resolved {
            heading: "h",
            rows: vec![("a".to_string(), label)],
        };
        vec![group("\u{ff76}\u{ff9e}"), group("x"), group("x")]
    }

    /// The rows the arrangement takes: the three columns are as tall as the tallest group,
    /// the first and the other two stacked are the second's and third's and a blank.
    fn rows(seg: TextSegmenter) -> i32 {
        let tokens = Tokens::new(Rc::new(ThemeData::DARK));
        groups(seg, &tokens, &sections(), 20).1
    }

    #[test]
    fn the_groups_are_arranged_by_the_terminals_count() {
        assert_eq!(rows(TextSegmenter::default()), 2);
        assert_eq!(rows(TextSegmenter::legacy()), 5);
    }
}
