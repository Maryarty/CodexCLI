//! The shortcut overlay: three columns of bindings (the list page) or the
//! keyboard diagram (the keyboard page). Oracle: `bottom_pane/shortcut_overlay.rs`
//! (`lines`, column padding) and `shortcut_overlay/keyboard.rs`.

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::overlay::Dialog;
use rtwk::style::Span;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::screens::dialogs::ShortcutView;

use super::dialog::{Msg, SmallDialogState};
use super::strings::{Group, Strings};
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
    Focus::new(Dialog::new(column).title(title))
        .can_request_focus(false)
        .bind(KeyPress::ESC, cx.send(Msg::Close))
        .bind('?', cx.send(Msg::Close))
        .bind(KeyPress::TAB, cx.send(Msg::Toggle))
}

/// The widths of one column: its keys, then its labels, each plus the gap.
fn widths(group: &Group) -> (usize, usize) {
    let width = |text: &str| text.chars().count();
    let keys = group.rows.iter().map(|row| width(row.0)).max().unwrap_or(0) + 2;
    let labels = group.rows.iter().map(|row| width(row.1)).max().unwrap_or(0) + 4;
    (keys, labels.max(width(group.heading) + 4 - keys.min(width(group.heading) + 4)))
}

fn pad(text: &str, width: usize) -> String {
    let fill = width.saturating_sub(text.chars().count());
    format!("{text}{}", " ".repeat(fill))
}

/// The headings and the rows of the three columns, the keys in bold.
fn list_rows(s: &Strings, tokens: &Tokens) -> Vec<AnyWidget> {
    let groups = &s.shortcut_groups;
    let sizes: Vec<(usize, usize)> = groups.iter().map(widths).collect();
    let last = groups.len() - 1;
    let mut out: Vec<AnyWidget> = Vec::new();
    let mut heading = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let width = sizes[index].0 + sizes[index].1;
        heading.push(Span::styled(
            if index == last {
                group.heading.to_string()
            } else {
                pad(group.heading, width)
            },
            tokens.bold(),
        ));
    }
    out.push(Text::rich_line(heading).into());
    let height = groups.iter().map(|group| group.rows.len()).max().unwrap_or(0);
    for row in 0..height {
        let mut spans = Vec::new();
        for (index, group) in groups.iter().enumerate() {
            let (keys, labels) = sizes[index];
            match group.rows.get(row) {
                Some((key, label)) => {
                    spans.push(Span::styled(pad(key, keys), tokens.accent()));
                    spans.push(Span::new(if index == last {
                        (*label).to_string()
                    } else {
                        pad(label, labels)
                    }));
                }
                None if index != last => spans.push(Span::new(pad("", keys + labels))),
                None => {}
            }
        }
        out.push(Text::rich_line(spans).into());
    }
    out.push(Text::new(String::new()).into());
    out
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
