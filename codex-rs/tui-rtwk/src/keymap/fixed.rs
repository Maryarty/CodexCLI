//! The keys Codex hardwires, the ones its `tui.keymap` has no action for (Up and Down walk the
//! prompt history, Enter and Tab of a popup, Esc, the arrows of the placeholders), named as a
//! page reads them. In Flutter these are the `LogicalKeyboardKey` constants one writes in
//! `Shortcuts` where no user setting exists; here `KeyCode` stays in `src/keymap/` (ADR-006) and a
//! page asks a press what it is ([`FixedKey::fixed`]) or builds one ([`Fixed::press`]). A key the
//! user can rebind is an action of the keymap (`pressed`, `names`), never a [`Fixed`].

use rtwk::focus::KeyPress;
use rtwk::prelude::KeyCode;

macro_rules! fixed_keys {
    ($($name:ident),* $(,)?) => {
        /// A key that is no character: the code of a [`KeyPress`] without its modifiers.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Fixed {
            $($name),*
        }

        impl Fixed {
            /// The press of this key with no modifiers.
            pub fn press(self) -> KeyPress {
                KeyPress::code(match self {
                    $(Fixed::$name => KeyCode::$name),*
                })
            }
        }

        impl FixedKey for KeyPress {
            fn fixed(&self) -> Fixed {
                match self.code {
                    $(KeyCode::$name => Fixed::$name),*
                }
            }
        }
    };
}

fixed_keys!(
    Char, Enter, Tab, Backspace, Escape, Up, Down, Left, Right, Home, End, PageUp, PageDown,
    Insert, Delete, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
);

/// What a key press is, as a [`Fixed`] key.
pub trait FixedKey {
    fn fixed(&self) -> Fixed;
}
