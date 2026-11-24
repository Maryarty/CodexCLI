// cell: Item::Agent { .. }
//! The assistant's message cell (oracle history_cell/messages.rs
//! `AgentMessageCell`): the dim bullet and the Markdown of the message, one
//! column free at the right. The Markdown is `citation::view` (directives,
//! stream preview) in the transcript's theme.
//!
//! Flutter: `Row[SelectionContainer.disabled(Text("• ")), Expanded(MarkdownBody)]` with a right
//! padding: the bullet is a gutter, not part of what a selection copies.

use rtwk::AnyWidget;
use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{CrossAxisAlignment, EdgeInsets, Expanded, Padding, Row};
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk_rich::markdown::MarkdownTheme;

use super::citation;
use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

/// What a message is drawn with.
pub struct Look<'a> {
    /// The mark in front of the first row (`• `) and its style.
    pub bullet: &'a str,
    pub bullet_style: Style,
    pub markdown: &'a MarkdownTheme,
    pub cells: &'static Strings,
}

/// The cell of an assistant message; `live` while it streams, `cwd` the
/// session's directory (file directives read against it).
pub fn message(source: &str, live: bool, cwd: Option<&str>, look: &Look<'_>) -> AnyWidget {
    let markdown = citation::cited(source, live, cwd, look.cells, look.markdown.clone());
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        // narrower than the bullet, the bullet is cut (the oracle draws the prefix alone)
        .overflow_intended(true)
        .child(ExcludeSelection::new(
            Text::new(look.bullet.to_owned()).style(look.bullet_style.clone()),
        ))
        // one column of the text stays free (oracle `right_reserve`)
        .child(Expanded::new(
            1,
            Padding::new(EdgeInsets::only(0, 0, 1, 0), markdown),
        ))
        .into()
}

/// An assistant message: it keeps one column free at the right.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Agent { text, .. } = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        message(
            text,
            place.streaming,
            env.cwd,
            &Look {
                bullet: env.bullet,
                bullet_style: env.bullet_style.clone(),
                markdown: env.markdown,
                cells: env.cells,
            },
        ),
    )
}
