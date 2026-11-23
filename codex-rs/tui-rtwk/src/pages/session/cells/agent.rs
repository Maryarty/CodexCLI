//! The assistant's message cell (oracle history_cell/messages.rs
//! `AgentMessageCell`): the dim bullet and the Markdown of the message, one
//! column free at the right. The Markdown is `citation::view` (directives,
//! stream preview) in the transcript's theme.
//!
//! Flutter: `Row[Text("• "), Expanded(MarkdownBody)]` with a right padding.

use rtwk::AnyWidget;
use rtwk::layout::{CrossAxisAlignment, EdgeInsets, Expanded, Padding, Row};
use rtwk::style::Style;
use rtwk::widgets::Text;
use rtwk_rich::markdown::MarkdownTheme;

use super::citation;
use super::strings::Strings;

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
        .child(Text::new(look.bullet.to_owned()).style(look.bullet_style.clone()))
        // one column of the text stays free (oracle `right_reserve`)
        .child(Expanded::new(
            1,
            Padding::new(EdgeInsets::only(0, 0, 1, 0), markdown),
        ))
        .into()
}
