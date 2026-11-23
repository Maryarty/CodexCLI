//! The page's actions and key tables.

use std::collections::HashSet;

use codex_tui_core::keymap::actions::*;
use codex_tui_core::keymap::{ActionId, Binding, Context, Keymap};

use super::*;

/// What a bound key means: a request to the model, or a change of the
/// screen's own state. The task a request is about is the list's cursor,
/// read when the key arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// `ctrl+c`: it cancels a line, otherwise it quits (the app's).
    Interrupt,
    Escape,
    Move(Move),
    Accept,
    Filter {
        back: bool,
    },
    Help,
    Search,
    Grouping,
    New,
    NewWorktree,
    Rename,
    Archive,
    Delete,
    Hide,
    Pin,
    Stop,
    /// `o`: the resume picker (oracle: `AppEvent::OpenResumePicker`).
    Resume,
    Fork,
    Right,
    /// A key the page takes and ignores: `Esc` when the user's cancel key is
    /// another. The center has no back but `list.cancel` (oracle), and the
    /// app's `Navigator` would pop the route on it (rtwk's `back_key`).
    Swallow,
}

/// The keys of each mode (oracle: `handle_key_event`, `command_center_key`,
/// `rename_key`), the app's keymap over the page's actions, bound with rtwk's
/// `KeyBindings`: the keys a focused widget takes (the arrows, the page keys,
/// Home and End of the list; the text of the editing line) never reach them.
/// What the user set in `tui.keymap.list` and `.agents` replaces the default
/// keys; the page's own keys (`?`, Tab, Shift+Tab) yield to any key the
/// center's actions take (`center_shortcut_keys`).
pub(super) type Pairs = Vec<(String, Act)>;

/// Every action the center binds, with what its key does, in the order the
/// oracle asks them (`handle_key_event`): the center's own actions, then the
/// list's. A key two actions are bound to does what the first of them does
/// (`table`).
const ACTIONS: &[(ActionId, Act)] = &[
    (AGENTS_SEARCH, Act::Search),
    (AGENTS_RESUME, Act::Resume),
    (AGENTS_TOGGLE_GROUPING, Act::Grouping),
    (AGENTS_TOGGLE_PIN, Act::Pin),
    (AGENTS_NEW_TASK, Act::New),
    (AGENTS_NEW_WORKTREE, Act::NewWorktree),
    (AGENTS_FORK, Act::Fork),
    (AGENTS_RENAME, Act::Rename),
    (AGENTS_ARCHIVE, Act::Archive),
    (AGENTS_DELETE, Act::Delete),
    (AGENTS_HIDE, Act::Hide),
    (AGENTS_STOP, Act::Stop),
    (LIST_MOVE_DOWN, Act::Move(Move::Step(true))),
    (LIST_MOVE_UP, Act::Move(Move::Step(false))),
    (LIST_JUMP_TOP, Act::Move(Move::Edge(false))),
    (LIST_JUMP_BOTTOM, Act::Move(Move::Edge(true))),
    (LIST_ACCEPT, Act::Accept),
    (LIST_CANCEL, Act::Escape),
    (LIST_PAGE_UP, Act::Move(Move::Page(false))),
    (LIST_PAGE_DOWN, Act::Move(Move::Page(true))),
    (LIST_MOVE_RIGHT, Act::Right),
];

/// The names of the keys the center's actions start with, single keys and the
/// first key of a chord (`center_shortcut_keys`).
pub(super) fn taken(keymap: &Keymap) -> HashSet<String> {
    keymap
        .actions()
        .filter(|(id, _)| matches!(id.context, Context::List | Context::Agents))
        .flat_map(|(_, bindings)| bindings)
        .filter_map(|binding| crate::keymap::name_of_binding(&Binding::Single(binding.first())))
        .collect()
}

fn bind(out: &mut Pairs, keymap: &Keymap, id: ActionId, act: Act) {
    for name in crate::keymap::names(keymap, id) {
        out.push((name, act));
    }
}

/// A key of the page's own, unless a center action has taken it.
fn fixed(out: &mut Pairs, taken: &HashSet<String>, name: &str, act: Act) {
    if !taken.contains(name) {
        out.push((name.to_owned(), act));
    }
}

fn only(keymap: &Keymap, ids: &[ActionId]) -> Pairs {
    let mut out = Pairs::new();
    for (id, act) in ACTIONS.iter().filter(|(id, _)| ids.contains(id)) {
        bind(&mut out, keymap, *id, *act);
    }
    out.push(("ctrl+c".to_owned(), Act::Interrupt));
    out.push(("escape".to_owned(), Act::Swallow));
    out
}

/// Closing the help: the cancel key, or `?` again.
pub(super) fn help(keymap: &Keymap) -> Pairs {
    let mut out = only(keymap, &[LIST_CANCEL]);
    out.push(("?".to_owned(), Act::Help));
    out
}

/// An editing line's keys besides its text: the cancel key, the search key
/// (it closes the search line), and every task action bound to a key the line
/// does not type (the oracle types a plain character into the line before it
/// asks the keymap, `handle_key_event`; an action on a plain character is no
/// shortcut while the line is open). The search line also moves the list and
/// accepts a task by the list's keys that are not text.
fn editing(keymap: &Keymap, list: bool) -> Pairs {
    let mut out = only(keymap, &[LIST_CANCEL, AGENTS_SEARCH]);
    for (id, act) in ACTIONS {
        let wanted = match act {
            Act::Move(_) | Act::Accept => list,
            Act::Resume
            | Act::Grouping
            | Act::Pin
            | Act::New
            | Act::NewWorktree
            | Act::Fork
            | Act::Rename
            | Act::Archive
            | Act::Delete
            | Act::Hide
            | Act::Stop => true,
            _ => false,
        };
        if !wanted {
            continue;
        }
        for binding in keymap.bindings(*id) {
            let is_text =
                matches!(binding, Binding::Single(stroke) if crate::keymap::is_text(&stroke));
            if !is_text && let Some(name) = crate::keymap::name_of_binding(&binding) {
                out.push((name, *act));
            }
        }
    }
    out
}

/// The search line.
pub(super) fn search(keymap: &Keymap) -> Pairs {
    editing(keymap, true)
}

/// The rename line.
pub(super) fn rename(keymap: &Keymap) -> Pairs {
    editing(keymap, false)
}

/// The list.
pub(super) fn browse(keymap: &Keymap) -> Pairs {
    let taken = taken(keymap);
    let mut out = Pairs::new();
    fixed(&mut out, &taken, "tab", Act::Filter { back: false });
    fixed(&mut out, &taken, "shift+tab", Act::Filter { back: true });
    fixed(&mut out, &taken, "?", Act::Help);
    out.push(("ctrl+c".to_owned(), Act::Interrupt));
    for (id, act) in ACTIONS {
        bind(&mut out, keymap, *id, *act);
    }
    out.push(("escape".to_owned(), Act::Swallow));
    out
}

/// One mode's keys as `KeyBindings` takes them: each sends its act.
pub(super) fn table(cx: &mut Cx<'_, ScreenState>, keys: Pairs) -> view::Table {
    crate::keymap::first_of_each(keys)
        .into_iter()
        .map(|(name, act)| (name, cx.send(Msg::Act(act))))
        .collect()
}
