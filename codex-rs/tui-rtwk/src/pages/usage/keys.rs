//! The keys of the dashboard (oracle: `analytics.rs` `handle_key`). The list
//! context of the keymap (`tui.keymap.list`) is what moves and opens: its
//! actions are asked of the app's `Keymap` and bound with rtwk's
//! `KeyBindings`; `h` and `l` stand for the left and right arrows when no
//! action has them; the page's own keys (the digits, Tab, the controls, `z`,
//! `?`, `q`) are fixed and yield to a key an action took.

use std::collections::HashSet;

use codex_tui_core::keymap::actions::*;
use codex_tui_core::keymap::{ActionId, Binding, Context, Keymap};

use super::Msg;

/// What the list context asks of a report (oracle: `ListAction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListAct {
    Up,
    Down,
    Left,
    Right,
    Top,
    Bottom,
    Accept,
    Cancel,
    PageUp,
    PageDown,
}

/// The actions the page asks, with what each does.
const ACTIONS: &[(ActionId, ListAct)] = &[
    (LIST_MOVE_UP, ListAct::Up),
    (LIST_MOVE_DOWN, ListAct::Down),
    (LIST_MOVE_LEFT, ListAct::Left),
    (LIST_MOVE_RIGHT, ListAct::Right),
    (LIST_JUMP_TOP, ListAct::Top),
    (LIST_JUMP_BOTTOM, ListAct::Bottom),
    (LIST_ACCEPT, ListAct::Accept),
    (LIST_CANCEL, ListAct::Cancel),
    (LIST_PAGE_UP, ListAct::PageUp),
    (LIST_PAGE_DOWN, ListAct::PageDown),
];

/// The names of every key a list action starts with.
fn taken(keymap: &Keymap) -> HashSet<String> {
    keymap
        .actions()
        .filter(|(id, _)| id.context == Context::List)
        .flat_map(|(_, bindings)| bindings)
        .filter_map(|binding| crate::keymap::name_of_binding(&Binding::Single(binding.first())))
        .collect()
}

/// Whether a key of the page's own is free: no list action is bound to it.
pub(super) fn free(keymap: &Keymap, name: &str) -> bool {
    !taken(keymap).contains(name)
}

/// `?` opens the help only when the keymap leaves both spellings of it alone
/// (oracle: `help_shortcut_available`).
pub(super) fn help_available(keymap: &Keymap) -> bool {
    let taken = taken(keymap);
    !taken.contains("?") && !taken.contains("shift+?")
}

/// The list keys as `KeyBindings` takes them, each sending its message:
/// the actions' bindings, then Space as accept and `h`/`l` for the arrows
/// where nothing is bound to them.
pub(super) fn list(keymap: &Keymap) -> Vec<(String, ListAct)> {
    let mut out: Vec<(String, ListAct)> = Vec::new();
    for (id, act) in ACTIONS {
        for name in crate::keymap::names(keymap, *id) {
            out.push((name, *act));
        }
    }
    let taken = taken(keymap);
    for (name, act) in [
        ("space", ListAct::Accept),
        ("h", ListAct::Left),
        ("l", ListAct::Right),
    ] {
        if !taken.contains(name) {
            out.push((name.to_owned(), act));
        }
    }
    crate::keymap::first_of_each(out)
}

/// The pairs, each with the message `act` becomes.
pub(super) fn messages(keymap: &Keymap) -> Vec<(String, Msg)> {
    list(keymap)
        .into_iter()
        .map(|(name, act)| (name, Msg::List(act)))
        .collect()
}

/// The hint a list action shows (oracle: `hint`): the label of its first
/// key, empty while unbound.
pub(super) fn hint(keymap: &Keymap, id: ActionId) -> String {
    crate::keymap::label(keymap, id).unwrap_or_default()
}

pub(super) const UP: ActionId = LIST_MOVE_UP;
pub(super) const DOWN: ActionId = LIST_MOVE_DOWN;
pub(super) const LEFT: ActionId = LIST_MOVE_LEFT;
pub(super) const RIGHT: ActionId = LIST_MOVE_RIGHT;
pub(super) const ACCEPT: ActionId = LIST_ACCEPT;
pub(super) const CANCEL: ActionId = LIST_CANCEL;
pub(super) const PAGE_UP: ActionId = LIST_PAGE_UP;
pub(super) const PAGE_DOWN: ActionId = LIST_PAGE_DOWN;

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::keymap::KeymapSpec;

    fn with(action: &str, specs: &[&str]) -> Keymap {
        Keymap::resolve_unchecked(&KeymapSpec::default().with("list", action, specs))
    }

    fn act_of(keymap: &Keymap, name: &str) -> Option<ListAct> {
        list(keymap)
            .into_iter()
            .find(|(key, _)| key == name)
            .map(|(_, act)| act)
    }

    #[test]
    fn the_defaults_move_open_and_page_and_h_l_stand_for_the_arrows() {
        let keymap = Keymap::defaults();
        assert_eq!(act_of(&keymap, "enter"), Some(ListAct::Accept));
        assert_eq!(act_of(&keymap, "pagedown"), Some(ListAct::PageDown));
        assert_eq!(act_of(&keymap, "h"), Some(ListAct::Left));
        assert_eq!(act_of(&keymap, "l"), Some(ListAct::Right));
        assert_eq!(act_of(&keymap, "space"), Some(ListAct::Accept));
        assert!(help_available(&keymap));
        assert!(free(&keymap, "q"));
    }

    #[test]
    fn a_remapped_action_takes_its_key_from_the_pages_own() {
        // `q` as cancel and `?` as page down: the page's own `q` and `?`
        // yield, `h` stays the left arrow, the help is not offered.
        let keymap = with("cancel", &["q"]);
        assert_eq!(act_of(&keymap, "q"), Some(ListAct::Cancel));
        assert!(!free(&keymap, "q"));
        let keymap = with("page_down", &["?"]);
        assert_eq!(act_of(&keymap, "?"), Some(ListAct::PageDown));
        assert!(!help_available(&keymap));
        assert_eq!(act_of(&keymap, "pagedown"), None);
    }

    #[test]
    fn a_key_an_action_has_keeps_h_and_l_for_it() {
        let keymap = with("move_left", &["h"]);
        assert_eq!(act_of(&keymap, "h"), Some(ListAct::Left));
        let keymap = with("move_right", &["a"]);
        assert_eq!(act_of(&keymap, "l"), Some(ListAct::Right));
        assert_eq!(act_of(&keymap, "a"), Some(ListAct::Right));
    }
}
