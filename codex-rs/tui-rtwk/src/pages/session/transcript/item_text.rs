//! The text of an item of the transcript as it is mounted, written from the
//! item: what find, the select mode and copy read of an item that is scrolled
//! out of the list (what the list's `ListView::item_text` is given).
//!
//! The cells keep their own texts (`cells::text`); a message's Markdown is
//! the `Gap` they declare, and [`markdown_text`] fills it for the assistant's
//! message. What is still a gap (a tool call, a patch, a plan, the reasoning,
//! the collab rows) answers `None`: such an item has no text here, and a
//! reader of this function treats it as it does an item that is not
//! selectable.

use codex_tui_core::threads::Item;
use rtwk_rich::markdown::parse_markdown;

use super::markdown_text;
use crate::pages::session::cells::text::{Env, Gap, Texts, texts};

/// The text of `item` as a `SelectionArea` over its mounted cell selects it:
/// the marks the cells draw as gutters (the assistant's bullet too) are in an
/// `ExcludeSelection` and not in it.
pub fn item_text(item: &Item, env: &Env<'_>) -> Option<String> {
    match texts(item, env) {
        exact @ Texts::Exact(_) => exact.as_mounted(),
        Texts::Gap(Gap::Markdown) => match item {
            Item::Agent { text, .. } => markdown_text::text(&parse_markdown(text)),
            _ => None,
        },
        Texts::Gap(Gap::Rows) => None,
    }
}
