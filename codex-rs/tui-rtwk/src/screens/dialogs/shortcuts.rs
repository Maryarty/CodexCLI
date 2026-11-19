//! The shortcut overlay: three columns of bindings (the list page) or the
//! keyboard diagram (the keyboard page). Oracle: `bottom_pane/shortcut_overlay.rs`
//! (`lines`, column padding) and `shortcut_overlay/keyboard.rs`.

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{
    Column, CrossAxisAlignment, Expanded, Grid, GridPlacement, GridTrack, MainAxisSize, Row, SizedBox,
};
use rtwk::LayoutBuilder;
use rtwk::overlay::Dialog;
use rtwk_core::unicode::{TextMode, string_width};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::screens::dialogs::ShortcutView;

use super::dialog::{Msg, SmallDialogState};
use super::strings::Strings;
use crate::app::theme::Tokens;

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
    Focus::new(Dialog::new(column).title(title).max_width(None))
        .can_request_focus(false)
        .bind(KeyPress::ESC, cx.send(Msg::Close))
        .bind('?', cx.send(Msg::Close))
        .bind(KeyPress::TAB, cx.send(Msg::Toggle))
}

/// The overlay as the oracle's bottom pane draws it (`lines`): the bold
/// title, the groups and `/keymap customize`, two cells in, to stand above
/// the composer; the close hint is the footer's (`close_hint`).
pub(super) fn panel(s: &'static Strings, tokens: &Tokens) -> AnyWidget {
    let tokens = tokens.clone();
    let inner = LayoutBuilder::new(move |_, constraints| {
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(Text::line(s.shortcuts_title).style(tokens.bold()))
            .child(Text::new(String::new()));
        column = column.child(groups(s, &tokens, constraints.max_width));
        // `/keymap` in the accent, the rest dim.
        let (command, rest) = s
            .keymap_customize
            .split_at(s.keymap_customize.find(' ').unwrap_or(0));
        let customize = Row::new()
            .child(Text::line(command).style(tokens.accent()))
            .child(Text::line(rest).style(tokens.dim()));
        column.child(Text::new(String::new())).child(customize).into()
    });
    Row::new()
        .child(SizedBox::new(Some(2), None, None))
        .child(Expanded::new(1, inner))
        .into()
}

/// Cells between the columns (`shortcut_help.rs`, `COLUMN_GAP`).
const COLUMN_GAP: i32 = 4;

/// The groups in as many columns as `room` holds (`group_lines`): three; the
/// first and the other two stacked; or all stacked.
fn groups(s: &Strings, tokens: &Tokens, room: i32) -> AnyWidget {
    let cells = |text: &str| string_width(text, TextMode::Graphemes, 1) as i32;
    let widths: Vec<i32> = s
        .shortcut_groups
        .iter()
        .map(|group| {
            let keys = group.rows.iter().map(|(key, _)| cells(key)).max().unwrap_or(0);
            let rows = group.rows.iter().map(|(_, label)| keys + 2 + cells(label));
            rows.chain([cells(group.heading)]).max().unwrap_or(0)
        })
        .collect();
    let section = |index: usize| -> AnyWidget {
        let group = &s.shortcut_groups[index];
        let mut grid = Grid::new()
            .columns([GridTrack::content(), GridTrack::content()])
            .column_gap(2)
            .child(GridPlacement::new(Text::line(group.heading).style(tokens.bold())).span(1, 2));
        for (key, label) in group.rows {
            grid = grid
                .child(Text::line(*key).style(tokens.accent()))
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
    if widths.iter().sum::<i32>() + COLUMN_GAP * 2 <= room {
        side(vec![stack(&[0]), stack(&[1]), stack(&[2])])
    } else if widths[0] + COLUMN_GAP + widths[1].max(widths[2]) <= room {
        side(vec![stack(&[0]), stack(&[1, 2])])
    } else {
        stack(&[0, 1, 2])
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
        grid = grid.child(
            GridPlacement::new(Text::line(group.heading).style(tokens.bold())).span(1, 2),
        );
        grid = between(grid, index);
    }
    let height = groups.iter().map(|group| group.rows.len()).max().unwrap_or(0);
    for row in 0..height {
        for (index, group) in groups.iter().enumerate() {
            grid = match group.rows.get(row) {
                Some((key, label)) => grid
                    .child(Text::line(*key).style(tokens.accent()))
                    .child(Text::line(*label)),
                None => grid.child(SizedBox::fixed(0, 1)).child(SizedBox::fixed(0, 1)),
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
