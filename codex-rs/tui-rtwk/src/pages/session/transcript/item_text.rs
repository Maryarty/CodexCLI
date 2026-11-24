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
use crate::pages::session::cells::text::{Env, Gap, Part, Texts, texts};

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

/// The text of `item` as rtwk's `SelectableList` reads its mounted row: every `Text` below the
/// row, the gutters in their `ExcludeSelection` too (`selectable_nodes` of `selection_text.rs`
/// walks the render tree without leaving an `ExcludeSelection` out as the area does), the parts
/// joined by line breaks. What the list's `ItemTexts` model has to say for a range of it to land
/// on the cells it names; `item_text` is the text that is the item's own (copy). The assistant's
/// bullet (`bullet`) is the first part of its message.
pub fn mounted_text(item: &Item, env: &Env<'_>, bullet: &str) -> Option<String> {
    mounted_parts(item, env, bullet).map(|parts| {
        parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// The parts of [`mounted_text`], each with whether it is a gutter: what the select mode reads
/// (a caret never stops on a gutter, a copy leaves it out).
pub fn mounted_parts(item: &Item, env: &Env<'_>, bullet: &str) -> Option<Vec<Part>> {
    match texts(item, env) {
        Texts::Exact(parts) => Some(parts),
        Texts::Gap(Gap::Markdown) => match item {
            Item::Agent { text, .. } => {
                let body = markdown_text::text(&parse_markdown(text))?;
                Some(vec![Part::gutter(bullet), Part::text(body)])
            }
            _ => None,
        },
        Texts::Gap(Gap::Rows) => None,
    }
}
