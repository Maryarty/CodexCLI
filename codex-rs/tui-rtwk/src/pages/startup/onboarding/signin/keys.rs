//! The keys the sign-in wizards take for themselves (the oracle's fixed ones: the arrows and
//! `j`/`k` of a list, Tab between fields): a press is compared with the `KeyPress` constants of
//! rtwk by its code, character and modifiers, as the handlers of a `FocusNode` read it; the
//! configurable keys are the keymap's (`crate::keymap`).

use rtwk::focus::KeyPress;
use rtwk::prelude::Mods;

/// The press is `want`: the same code, character and modifiers.
pub(crate) fn pressed(key: &KeyPress, want: &KeyPress) -> bool {
    is_key(key, want) && key.mods == want.mods
}

/// The press is of `want`'s key, whatever modifier is held.
pub(crate) fn is_key(key: &KeyPress, want: &KeyPress) -> bool {
    key.code == want.code && (want.ch.is_none() || key.ch == want.ch)
}

/// `c` with Ctrl held.
pub(crate) fn ctrl(c: char) -> KeyPress {
    KeyPress::char(c).with_mods(Mods::CTRL)
}
