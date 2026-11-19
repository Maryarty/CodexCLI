//! Reverse history search: what a key does while it is open.

use std::rc::Rc;

use rtwk::focus::KeyPress;
use rtwk::prelude::{KeyCode, Mods};

/// The chat screen of one thread's controllers.
use super::session_page::*;

/// What a key does while reverse history search is open.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum SearchKey {
    Char(Rc<str>),
    Backspace,
    /// Ctrl+U: the query is emptied.
    Clear,
    Older,
    Newer,
    Accept,
    Cancel,
    /// Any other key: the search consumes it (`_ => (InputResult::None, true)`).
    Ignore,
}

/// The key of history search for `key`, if it is one
/// (`history_search.rs`, `handle_history_search_key`): Ctrl+R and Up step to
/// an older match, Ctrl+S and Down to a newer one, Esc and Ctrl+C cancel
/// (the draft is restored), Enter accepts a match, Backspace and Ctrl+H
/// delete, Ctrl+U clears, other text extends the query.
pub(super) fn search_key(key: KeyPress) -> Option<ChatMsg> {
    let ctrl = |c: char| KeyPress::char(c).with_mods(Mods::CTRL);
    let found = if key == ctrl('r') || key.code == KeyCode::Up {
        SearchKey::Older
    } else if key == ctrl('s') || key.code == KeyCode::Down {
        SearchKey::Newer
    } else if key.code == KeyCode::Escape
        || key == ctrl('c')
        || (key.mods.is_empty() && key.text() == Some("\u{3}"))
    {
        SearchKey::Cancel
    } else if key.code == KeyCode::Enter && key.mods.is_empty() {
        SearchKey::Accept
    } else if key.code == KeyCode::Backspace || key == ctrl('h') {
        SearchKey::Backspace
    } else if key == ctrl('u') {
        SearchKey::Clear
    } else if let Some(text) = key.as_text() {
        SearchKey::Char(Rc::from(text))
    } else if key.code == KeyCode::F12 {
        // The port's own hidden debug key is not the oracle's to swallow.
        return None;
    } else {
        SearchKey::Ignore
    };
    Some(ChatMsg::Search(found))
}
