//! The shortcut overlay: three columns of bindings (the list page) or the
//! keyboard diagram (the keyboard page). Oracle: `bottom_pane/shortcut_overlay.rs`
//! (`lines`, column padding) and `shortcut_overlay/keyboard.rs`.

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{
    Column, CrossAxisAlignment, Grid, GridPlacement, GridTrack, MainAxisSize, SizedBox,
};
use rtwk::overlay::Dialog;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::screens::dialogs::ShortcutView;

use super::dialog::{Msg, SmallDialogState};
use super::strings::Strings;
use crate::theme::Tokens;

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
