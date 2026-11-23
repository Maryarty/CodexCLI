//! The keys Codex hardwires, the ones its `tui.keymap` has no action for (Up and Down walk the
//! prompt history, Enter and Tab of a popup, Esc, the arrows over a placeholder, the kill keys the
//! field's own editing answers to), as actions with fixed bindings and no config key. In Flutter
//! these are the `Shortcuts` entries an app writes where no user setting exists: the key is
//! still named by an `Intent` (an [`ActionId`] here), never spelled in the widget. A page asks
//! `pressed(keymap, id, key)` for them as for any action (`src/keymap`), and builds the press an
//! action stands for with `press_of`. Not rows of the catalogue: the user cannot rebind them, the
//! resolver does not see them, and a test keeps their names apart from the catalogue's.

use super::catalogue::{ActionId, Context};
use super::key::{Key, Mods};

/// Which modifiers a fixed binding takes (the oracle matches some keys by code alone, some by the
/// exact chord, `Alt+V` and `Ctrl+V` as "holds Ctrl or Alt").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModsRule {
    /// Whatever is held: the page reads the modifiers itself where they matter.
    Any,
    /// Exactly these.
    Exact(Mods),
    /// At least these.
    Has(Mods),
}

impl ModsRule {
    /// Whether the modifiers held satisfy the rule.
    pub fn accepts(self, held: Mods) -> bool {
        match self {
            ModsRule::Any => true,
            ModsRule::Exact(mods) => held == mods,
            ModsRule::Has(mods) => held.contains(mods),
        }
    }

    /// The modifiers of the press this rule stands for.
    pub const fn pressed(self) -> Mods {
        match self {
            ModsRule::Any => Mods::NONE,
            ModsRule::Exact(mods) | ModsRule::Has(mods) => mods,
        }
    }
}

/// One key of a fixed action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedBinding {
    pub key: Key,
    pub mods: ModsRule,
}

const fn any(key: Key) -> FixedBinding {
    FixedBinding {
        key,
        mods: ModsRule::Any,
    }
}

const fn exact(key: Key, mods: Mods) -> FixedBinding {
    FixedBinding {
        key,
        mods: ModsRule::Exact(mods),
    }
}

const fn has(key: Key, mods: Mods) -> FixedBinding {
    FixedBinding {
        key,
        mods: ModsRule::Has(mods),
    }
}

const fn composer(name: &'static str) -> ActionId {
    ActionId::new(Context::Composer, name)
}

const fn editor(name: &'static str) -> ActionId {
    ActionId::new(Context::Editor, name)
}

/// The field's own Enter and Tab (submit and queue are the keymap's, these are what is left).
pub const ENTER: ActionId = composer("enter");
pub const TAB: ActionId = composer("tab");
/// Esc: closes a popup, leaves shell mode or a search.
pub const DISMISS: ActionId = composer("dismiss");
/// Up and Down walk the prompt history at the draft's ends (and the remote image rows).
pub const HISTORY_PREVIOUS: ActionId = composer("history_previous");
pub const HISTORY_NEXT: ActionId = composer("history_next");
/// A popup's highlight (`@` mentions, slash commands).
pub const POPUP_PREVIOUS: ActionId = composer("popup_previous");
pub const POPUP_NEXT: ActionId = composer("popup_next");
/// Enter picks the highlighted row, Tab completes it as text.
pub const POPUP_ACCEPT: ActionId = composer("popup_accept");
pub const POPUP_COMPLETE: ActionId = composer("popup_complete");
/// Left and Right switch the `@` popup's tab while the caret is in the token.
pub const POPUP_TAB_PREVIOUS: ActionId = composer("popup_tab_previous");
pub const POPUP_TAB_NEXT: ActionId = composer("popup_tab_next");
/// Ctrl+V or Alt+V: the clipboard's image.
pub const PASTE_IMAGE: ActionId = composer("paste_image");
/// Ctrl+C on a draft with text clears it before it interrupts or quits.
pub const CLEAR_FOR_CTRL_C: ActionId = composer("clear_for_ctrl_c");
/// The keys of a history search (Ctrl+R) and of Vim's `/` query.
pub const SEARCH_ACCEPT: ActionId = composer("search_accept");
pub const SEARCH_CANCEL: ActionId = composer("search_cancel");
pub const SEARCH_ERASE: ActionId = composer("search_erase");
pub const SEARCH_CLEAR: ActionId = composer("search_clear");
/// The port's own hidden debug key: no field of the oracle's swallows it.
pub const DEBUG_KEY: ActionId = ActionId::new(Context::Global, "debug_key");

/// The keys of the field's own editing, each the one rtwk's editing and Codex's placeholder-aware
/// editing answer to for the Editor action of that name (`Edit::canonical`).
pub const FIELD_LEFT: ActionId = editor("field_left");
pub const FIELD_RIGHT: ActionId = editor("field_right");
pub const FIELD_UP: ActionId = editor("field_up");
pub const FIELD_DOWN: ActionId = editor("field_down");
pub const FIELD_WORD_LEFT: ActionId = editor("field_word_left");
pub const FIELD_WORD_RIGHT: ActionId = editor("field_word_right");
pub const FIELD_LINE_START: ActionId = editor("field_line_start");
pub const FIELD_LINE_END: ActionId = editor("field_line_end");
pub const FIELD_DELETE_BACKWARD: ActionId = editor("field_delete_backward");
pub const FIELD_DELETE_FORWARD: ActionId = editor("field_delete_forward");
pub const FIELD_DELETE_BACKWARD_WORD: ActionId = editor("field_delete_backward_word");
pub const FIELD_DELETE_FORWARD_WORD: ActionId = editor("field_delete_forward_word");
pub const FIELD_KILL_LINE_START: ActionId = editor("field_kill_line_start");
pub const FIELD_KILL_LINE_END: ActionId = editor("field_kill_line_end");
pub const FIELD_YANK: ActionId = editor("field_yank");

/// A fixed action and its keys; the first is the one `press_of` builds.
#[derive(Clone, Copy, Debug)]
pub struct Fixed {
    pub id: ActionId,
    pub bindings: &'static [FixedBinding],
}

const CTRL: Mods = Mods::CTRL;
const ALT: Mods = Mods::ALT;

pub const TABLE: &[Fixed] = &[
    Fixed {
        id: ENTER,
        bindings: &[any(Key::Enter)],
    },
    Fixed {
        id: TAB,
        bindings: &[any(Key::Tab)],
    },
    Fixed {
        id: DISMISS,
        bindings: &[any(Key::Esc)],
    },
    Fixed {
        id: HISTORY_PREVIOUS,
        bindings: &[any(Key::Up)],
    },
    Fixed {
        id: HISTORY_NEXT,
        bindings: &[any(Key::Down)],
    },
    Fixed {
        id: POPUP_PREVIOUS,
        bindings: &[any(Key::Up), exact(Key::Char('p'), CTRL)],
    },
    Fixed {
        id: POPUP_NEXT,
        bindings: &[any(Key::Down), exact(Key::Char('n'), CTRL)],
    },
    Fixed {
        id: POPUP_ACCEPT,
        bindings: &[any(Key::Enter)],
    },
    Fixed {
        id: POPUP_COMPLETE,
        bindings: &[any(Key::Tab)],
    },
    Fixed {
        id: POPUP_TAB_PREVIOUS,
        bindings: &[any(Key::Left)],
    },
    Fixed {
        id: POPUP_TAB_NEXT,
        bindings: &[any(Key::Right)],
    },
    Fixed {
        id: PASTE_IMAGE,
        bindings: &[
            has(Key::Char('v'), CTRL),
            has(Key::Char('v'), ALT),
            has(Key::Char('V'), CTRL),
            has(Key::Char('V'), ALT),
        ],
    },
    Fixed {
        id: CLEAR_FOR_CTRL_C,
        bindings: &[exact(Key::Char('c'), CTRL)],
    },
    Fixed {
        id: SEARCH_ACCEPT,
        bindings: &[any(Key::Enter)],
    },
    Fixed {
        id: SEARCH_CANCEL,
        bindings: &[any(Key::Esc), exact(Key::Char('c'), CTRL)],
    },
    Fixed {
        id: SEARCH_ERASE,
        bindings: &[any(Key::Backspace), exact(Key::Char('h'), CTRL)],
    },
    Fixed {
        id: SEARCH_CLEAR,
        bindings: &[exact(Key::Char('u'), CTRL)],
    },
    Fixed {
        id: DEBUG_KEY,
        bindings: &[any(Key::F(12))],
    },
    Fixed {
        id: FIELD_LEFT,
        bindings: &[any(Key::Left)],
    },
    Fixed {
        id: FIELD_RIGHT,
        bindings: &[any(Key::Right)],
    },
    Fixed {
        id: FIELD_UP,
        bindings: &[any(Key::Up)],
    },
    Fixed {
        id: FIELD_DOWN,
        bindings: &[any(Key::Down)],
    },
    Fixed {
        id: FIELD_WORD_LEFT,
        bindings: &[exact(Key::Left, ALT)],
    },
    Fixed {
        id: FIELD_WORD_RIGHT,
        bindings: &[exact(Key::Right, ALT)],
    },
    Fixed {
        id: FIELD_LINE_START,
        bindings: &[any(Key::Home)],
    },
    Fixed {
        id: FIELD_LINE_END,
        bindings: &[any(Key::End)],
    },
    Fixed {
        id: FIELD_DELETE_BACKWARD,
        bindings: &[any(Key::Backspace)],
    },
    Fixed {
        id: FIELD_DELETE_FORWARD,
        bindings: &[any(Key::Delete)],
    },
    Fixed {
        id: FIELD_DELETE_BACKWARD_WORD,
        bindings: &[
            exact(Key::Char('w'), CTRL),
            exact(Key::Backspace, ALT),
            exact(Key::Backspace, CTRL),
        ],
    },
    Fixed {
        id: FIELD_DELETE_FORWARD_WORD,
        bindings: &[
            exact(Key::Char('d'), ALT),
            exact(Key::Delete, ALT),
            exact(Key::Delete, CTRL),
        ],
    },
    Fixed {
        id: FIELD_KILL_LINE_START,
        bindings: &[exact(Key::Char('u'), CTRL)],
    },
    Fixed {
        id: FIELD_KILL_LINE_END,
        bindings: &[exact(Key::Char('k'), CTRL)],
    },
    Fixed {
        id: FIELD_YANK,
        bindings: &[exact(Key::Char('y'), CTRL)],
    },
];

/// The keys of a fixed action; none for an action of the catalogue.
pub fn bindings_of(id: ActionId) -> Option<&'static [FixedBinding]> {
    TABLE
        .iter()
        .find(|fixed| fixed.id == id)
        .map(|fixed| fixed.bindings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::catalogue::ROWS;

    #[test]
    fn a_fixed_action_is_no_row_of_the_catalogue_and_named_once() {
        for (at, fixed) in TABLE.iter().enumerate() {
            assert!(
                ROWS.iter().all(|row| row.id != fixed.id),
                "{} is a catalogue action",
                fixed.id.config_path()
            );
            assert!(
                TABLE[..at].iter().all(|other| other.id != fixed.id),
                "{} twice",
                fixed.id.config_path()
            );
            assert!(!fixed.bindings.is_empty());
        }
    }

    #[test]
    fn the_rules_read_the_modifiers() {
        assert!(ModsRule::Any.accepts(Mods::CTRL));
        assert!(ModsRule::Exact(Mods::CTRL).accepts(Mods::CTRL));
        assert!(!ModsRule::Exact(Mods::CTRL).accepts(Mods::CTRL.with(Mods::SHIFT)));
        assert!(ModsRule::Has(Mods::ALT).accepts(Mods::ALT.with(Mods::SHIFT)));
        assert!(!ModsRule::Has(Mods::ALT).accepts(Mods::NONE));
    }
}
