//! The keys of the settings sheets: the user's list keymap (`tui.keymap.list.*`, ADR-006) over
//! the sheets' lists, as the oracle's `ListKeymap` is over its pickers. In Flutter it is the
//! `Shortcuts` map above the sheet; here the key callback of the sheet's `FocusNode` runs
//! between builds, so the keymap it reads is shared with the state and replaced when the
//! app provides another (`did_change_dependencies`).

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::Cx;
use rtwk::focus::{KeyCombo, KeyPress};

use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::{
    LIST_ACCEPT, LIST_CANCEL, LIST_JUMP_BOTTOM, LIST_JUMP_TOP, LIST_MOVE_DOWN, LIST_MOVE_LEFT,
    LIST_MOVE_RIGHT, LIST_MOVE_UP, LIST_PAGE_DOWN, LIST_PAGE_UP,
};

use crate::keymap::{keymap, pressed, types_text, unmodified};
use crate::widgets::session_list::Move;

/// A footer hint of the sheets with the list keymap's own keys: `enter` is the first binding of
/// the accept action and `esc` of the cancel (`picker_hint_line_for_keymap`); an action with no
/// binding keeps the word, as the sheets' hints are written for the default keymap.
pub(super) fn keyed(hint: &str, keys: &Keymap) -> String {
    let mut hint = hint.to_owned();
    for (word, action) in [("enter", LIST_ACCEPT), ("esc", LIST_CANCEL)] {
        if let Some(label) = crate::keymap::label(keys, action) {
            hint = hint.replace(word, &label);
        }
    }
    hint
}

/// The keymap the sheet's key callback reads.
#[derive(Clone)]
pub(super) struct Keys(Rc<RefCell<Rc<Keymap>>>);

impl Keys {
    pub(super) fn new() -> Self {
        Self(Rc::new(RefCell::new(Rc::new(Keymap::defaults()))))
    }

    /// Takes the keymap the app provides now.
    pub(super) fn refresh<S: 'static>(&self, cx: &mut Cx<'_, S>) {
        *self.0.borrow_mut() = keymap(cx);
    }

    pub(super) fn get(&self) -> Rc<Keymap> {
        self.0.borrow().clone()
    }
}

/// What a key means to a list of a sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ListKey {
    Move(Move),
    /// `list.move_left` / `list.move_right`: reorder (setups), or none.
    Left,
    Right,
    Accept,
    Cancel,
}

const MOVES: [(codex_tui_core::keymap::ActionId, Move); 6] = [
    (LIST_MOVE_UP, Move::Step(false)),
    (LIST_MOVE_DOWN, Move::Step(true)),
    (LIST_PAGE_UP, Move::Page(false)),
    (LIST_PAGE_DOWN, Move::Page(true)),
    (LIST_JUMP_TOP, Move::Edge(false)),
    (LIST_JUMP_BOTTOM, Move::Edge(true)),
];

/// The single character a key types, if it types one.
pub(super) fn letter_of(key: &KeyPress) -> Option<char> {
    key.text().and_then(|text| {
        let mut chars = text.chars();
        let first = chars.next()?;
        chars.next().is_none().then_some(first)
    })
}

/// The list key a press is under the user's keymap (`MultiSelectPicker::handle_key_event`).
/// With a search line (`searching`) printable characters are the query's: they move nothing,
/// but a key the keymap binds to accept or cancel is still that.
pub(super) fn list_key(keys: &Keymap, key: &KeyPress, searching: bool) -> Option<ListKey> {
    if !(searching && letter_of(key).is_some() && types_text(key)) {
        if let Some((_, mv)) = MOVES.iter().find(|(id, _)| pressed(keys, *id, key)) {
            return Some(ListKey::Move(*mv));
        }
        if pressed(keys, LIST_MOVE_LEFT, key) {
            return Some(ListKey::Left);
        }
        if pressed(keys, LIST_MOVE_RIGHT, key) {
            return Some(ListKey::Right);
        }
    }
    if pressed(keys, LIST_ACCEPT, key) {
        return Some(ListKey::Accept);
    }
    if pressed(keys, LIST_CANCEL, key) {
        return Some(ListKey::Cancel);
    }
    None
}

/// Space, with no modifier: flips the row.
pub(super) fn is_space(key: &KeyPress) -> bool {
    letter_of(key) == Some(' ') && unmodified(key)
}

/// The plain letter `letter` (a key the view binds itself, as the hooks browser's `t`).
pub(super) fn is_letter(key: &KeyPress, letter: char) -> bool {
    letter_of(key) == Some(letter) && unmodified(key)
}

/// Ctrl+C: the view's own cancel too (`on_ctrl_c`).
pub(super) fn is_interrupt(key: &KeyPress) -> bool {
    KeyCombo::parse("ctrl+c").is_ok_and(|combo| combo.accepts_press(key))
}
