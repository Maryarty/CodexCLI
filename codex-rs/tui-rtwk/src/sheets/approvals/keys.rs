//! The keys of the two question dialogs (`user_input`, `form`): what a key press asks of a dialog
//! that shows one question or field at a time. The oracle reads the list keymap (`tui.keymap.list`:
//! the moves and the accept), the chat's interrupt key and a few keys of its own (`Ctrl+C`,
//! `Ctrl+P`/`Ctrl+N`, PageUp/PageDown, Space, Tab, Backspace, digits); here the configurable ones
//! are the keymap's actions (ADR-006: [`pressed`] over the app's `Keymap`) and the fixed ones are
//! rtwk key names, so no widget of the dialogs names a key code.
//! Flutter: a `Shortcuts` map from `SingleActivator`s to `Intent`s; the dialog's `Actions` turn
//! the intent into its state change.

use rtwk::focus::{KeyCombo, KeyPress};

use codex_tui_core::keymap::Keymap;
use codex_tui_core::keymap::actions::{
    CHAT_INTERRUPT_TURN, LIST_ACCEPT, LIST_MOVE_DOWN, LIST_MOVE_LEFT, LIST_MOVE_RIGHT, LIST_MOVE_UP,
};

use crate::keymap::{pressed, unmodified};

/// What a key press asks of a question dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// The interrupt key (Esc by default): closes the notes, goes back, or dismisses.
    Cancel,
    /// Ctrl+C: clears typed text, else dismisses.
    Interrupt,
    /// The previous (`false`) or the next question or field (ctrl+p/n, PageUp/PageDown).
    Question(bool),
    /// The previous or the next one from a list (the list keymap's left and right, h/l).
    Side(bool),
    /// The option cursor a row on (-1 up, 1 down).
    Cursor(i64),
    /// Backspace or Delete.
    Clear,
    /// Space.
    Pick,
    /// Tab.
    Tab,
    /// The list keymap's accept.
    Accept,
    /// A digit 1 to 9.
    Digit(usize),
}

/// Whether `key` is the key rtwk names `spec` (`ctrl+c`, `pagedown`).
fn is(spec: &str, key: &KeyPress) -> bool {
    KeyCombo::parse(spec).is_ok_and(|combo| combo.accepts_press(key))
}

/// The single character a key types, if it types one.
fn letter_of(key: &KeyPress) -> Option<char> {
    let mut chars = key.text()?.chars();
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
}

/// Whether the key types text (a letter, a digit, Space; with Ctrl too): what a text area would
/// take as its own.
pub fn types_text(key: &KeyPress) -> bool {
    key.text().is_some()
}

/// Whether the key is Backspace (not Delete).
pub fn is_backspace(key: &KeyPress) -> bool {
    is("backspace", key)
}

/// What `key` asks of the dialog, over the app's `keymap`; none for a key the dialog does not
/// read.
pub fn press_of(keymap: &Keymap, key: &KeyPress) -> Option<Press> {
    // A terminal sends ctrl+h as Backspace (the C0 character): it is the keymap's `ctrl-h`, as
    // the oracle reads control characters.
    let c0;
    let key = if is("ctrl+backspace", key) {
        c0 = KeyCombo::parse("ctrl+h").map_or_else(|_| key.clone(), KeyCombo::press);
        &c0
    } else {
        key
    };
    let plain = unmodified(key);
    let letter = letter_of(key);
    if is("ctrl+c", key) {
        return Some(Press::Interrupt);
    }
    if is("escape", key) || pressed(keymap, CHAT_INTERRUPT_TURN, key) {
        return Some(Press::Cancel);
    }
    if is("ctrl+p", key) || is("pageup", key) {
        return Some(Press::Question(false));
    }
    if is("ctrl+n", key) || is("pagedown", key) {
        return Some(Press::Question(true));
    }
    if pressed(keymap, LIST_MOVE_LEFT, key) || (plain && letter == Some('h')) {
        return Some(Press::Side(false));
    }
    if pressed(keymap, LIST_MOVE_RIGHT, key) || (plain && letter == Some('l')) {
        return Some(Press::Side(true));
    }
    if pressed(keymap, LIST_MOVE_UP, key) || (plain && letter == Some('k')) {
        return Some(Press::Cursor(-1));
    }
    if pressed(keymap, LIST_MOVE_DOWN, key) || (plain && letter == Some('j')) {
        return Some(Press::Cursor(1));
    }
    if plain && (is("backspace", key) || is("delete", key)) {
        return Some(Press::Clear);
    }
    if plain && letter == Some(' ') {
        return Some(Press::Pick);
    }
    if plain && is("tab", key) {
        return Some(Press::Tab);
    }
    if pressed(keymap, LIST_ACCEPT, key) {
        return Some(Press::Accept);
    }
    if plain {
        return letter
            .and_then(|digit| digit.to_digit(10))
            .filter(|digit| *digit > 0)
            .and_then(|digit| usize::try_from(digit).ok())
            .map(Press::Digit);
    }
    None
}
