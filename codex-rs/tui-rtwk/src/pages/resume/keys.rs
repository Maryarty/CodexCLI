//! The picker's keys, asked before the search field's own through
//! `EditingKeys` (rtwk's door for an app's keys): the search is always the
//! typing target, so typed characters and paste are the field's (it edits the
//! controller and tells `on_changed`) and every other key is the picker's:
//! it becomes a `Msg` and the field leaves it alone. Flutter: `Shortcuts` and
//! `Actions` above a `TextField`. Oracle: `resume_picker.rs` `handle_key`
//! and `handle_transcript_loading_key`.

use std::rc::Rc;

use rtwk::editing::KeyCx;
use rtwk::focus::{KeyCombo, KeyPress};

use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::{
    LIST_ACCEPT, LIST_CANCEL, LIST_JUMP_BOTTOM, LIST_JUMP_TOP, LIST_MOVE_DOWN, LIST_MOVE_LEFT,
    LIST_MOVE_RIGHT, LIST_MOVE_UP, LIST_PAGE_DOWN, LIST_PAGE_UP,
};
use codex_tui_core::screens::resume::Dir;

use super::Msg;
use crate::keymap::{pressed, types_plain_text};
use crate::widgets::session_list::Move;

/// What the handler reads: where to tell a key, whether a transcript is
/// loading (it takes the keys then) and the keymap whose `list` actions move
/// the cursor, open and cancel (oracle: `self.keymap.list`).
#[derive(Clone)]
pub struct KeyData {
    pub tell: Rc<dyn Fn(Msg)>,
    pub loading: bool,
    pub keymap: Rc<Keymap>,
}

/// Whether the picker takes `key` before the search field.
pub fn take(data: &KeyData, key: &KeyPress, kcx: KeyCx) -> bool {
    if kcx.read_only {
        return false;
    }
    // The picker's own chords, which the keymap does not configure.
    let own = |name: &str| KeyCombo::parse(name).is_ok_and(|combo| combo.accepts_press(key));
    let list = |id| pressed(&data.keymap, id, key);
    // Ctrl+C quits, whatever the picker is doing.
    if own("ctrl+c") {
        (data.tell)(Msg::Quit);
        return true;
    }
    if data.loading {
        // While a transcript loads the picker takes every key; cancel stops it.
        if list(LIST_CANCEL) {
            (data.tell)(Msg::CancelTranscript);
        }
        return true;
    }
    // The session picker is always searchable: typed text, shifted or not, is
    // the field's, so a list binding on a plain character (`k`) is not a move
    // (oracle: `allow_plain_char_navigation`).
    let typed = types_plain_text(key);
    let msg = if list(LIST_CANCEL) {
        Msg::Escape
    } else if own("ctrl+t") {
        Msg::Transcript
    } else if own("ctrl+e") {
        Msg::Expand
    } else if own("ctrl+o") {
        Msg::Density
    } else if list(LIST_ACCEPT) {
        Msg::Accept
    } else if !typed && list(LIST_MOVE_UP) {
        Msg::Move(Move::Step(false))
    } else if !typed && list(LIST_MOVE_DOWN) {
        Msg::Move(Move::Step(true))
    } else if !typed && list(LIST_PAGE_UP) {
        Msg::Move(Move::Page(false))
    } else if !typed && list(LIST_JUMP_TOP) {
        Msg::Move(Move::Edge(false))
    } else if !typed && list(LIST_JUMP_BOTTOM) {
        Msg::Move(Move::Edge(true))
    } else if !typed && list(LIST_PAGE_DOWN) {
        Msg::Move(Move::Page(true))
    } else if own("shift+tab") {
        Msg::Focus(false)
    } else if own("tab") {
        Msg::Focus(true)
    } else if !typed && list(LIST_MOVE_LEFT) {
        Msg::Change(Dir::Prev)
    } else if !typed && list(LIST_MOVE_RIGHT) {
        Msg::Change(Dir::Next)
    } else if [
        "backspace",
        "shift+backspace",
        "ctrl+backspace",
        "alt+backspace",
    ]
    .into_iter()
    .any(own)
    {
        Msg::Backspace
    } else if own("ctrl+a") {
        Msg::Archive
    } else if typed {
        return false;
    } else {
        // a key the picker does not know is not the search's either
        return true;
    };
    (data.tell)(msg);
    true
}
