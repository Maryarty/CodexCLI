//! The page's actions and key tables.

use std::collections::HashSet;

use codex_tui_core::keymap::actions::*;
use codex_tui_core::keymap::{ActionId, Binding, Context, Key, Keymap, Mods};

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
}

/// The keys of each mode (oracle: `handle_key_event`, `command_center_key`,
/// `rename_key`), the app's keymap over the page's actions, bound with rtwk's
/// `KeyBindings`: the keys a focused widget takes (the arrows, the page keys,
/// Home and End of the list; the text of the editing line) never reach them.
/// What the user set in `tui.keymap.list` and `.agents` replaces the default
/// keys; the page's own keys (`?`, Tab, Shift+Tab) yield to any key the
/// center's actions take (`center_shortcut_keys`).
pub(super) type Pairs = Vec<(String, Act)>;

/// Every action the center binds, with what its key does.
const ACTIONS: &[(ActionId, Act)] = &[
    (LIST_CANCEL, Act::Escape),
    (AGENTS_SEARCH, Act::Search),
    (LIST_MOVE_DOWN, Act::Move(Move::Step(true))),
    (LIST_MOVE_UP, Act::Move(Move::Step(false))),
    (LIST_PAGE_DOWN, Act::Move(Move::Page(true))),
    (LIST_PAGE_UP, Act::Move(Move::Page(false))),
    (LIST_JUMP_TOP, Act::Move(Move::Edge(false))),
    (LIST_JUMP_BOTTOM, Act::Move(Move::Edge(true))),
    (LIST_ACCEPT, Act::Accept),
    (AGENTS_DELETE, Act::Delete),
    (LIST_MOVE_RIGHT, Act::Right),
    (AGENTS_TOGGLE_GROUPING, Act::Grouping),
    (AGENTS_NEW_TASK, Act::New),
    (AGENTS_RENAME, Act::Rename),
    (AGENTS_ARCHIVE, Act::Archive),
    (AGENTS_HIDE, Act::Hide),
    (AGENTS_TOGGLE_PIN, Act::Pin),
    (AGENTS_STOP, Act::Stop),
    (AGENTS_RESUME, Act::Resume),
    (AGENTS_FORK, Act::Fork),
    (AGENTS_NEW_WORKTREE, Act::NewWorktree),
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
    out
}

/// Closing the help: the cancel key, or `?` again.
pub(super) fn help(keymap: &Keymap) -> Pairs {
    let mut out = only(keymap, &[LIST_CANCEL]);
    out.push(("?".to_owned(), Act::Help));
    out
}

/// The search line: the cancel key, the search key which closes it, and the
/// list's movement keys that are not text (the oracle types a plain
/// character into the line before it asks the keymap, `handle_key_event`).
pub(super) fn search(keymap: &Keymap) -> Pairs {
    let mut out = only(keymap, &[LIST_CANCEL, AGENTS_SEARCH]);
    for (id, act) in ACTIONS
        .iter()
        .filter(|(_, act)| matches!(act, Act::Move(_)))
    {
        for binding in keymap.bindings(*id) {
            let is_text = matches!(binding, Binding::Single(stroke)
                if matches!(stroke.key, Key::Char(_))
                    && !stroke.mods.contains(Mods::CTRL)
                    && !stroke.mods.contains(Mods::ALT)
                    && !stroke.mods.contains(Mods::SUPER));
            if !is_text && let Some(name) = crate::keymap::name_of_binding(&binding) {
                out.push((name, *act));
            }
        }
    }
    out
}

/// The rename line.
pub(super) fn rename(keymap: &Keymap) -> Pairs {
    only(keymap, &[LIST_CANCEL])
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
    out
}

/// One mode's keys as `KeyBindings` takes them: each sends its act.
pub(super) fn table(cx: &mut Cx<'_, ScreenState>, keys: Pairs) -> view::Table {
    keys.into_iter()
        .map(|(name, act)| (name, cx.send(Msg::Act(act))))
        .collect()
}
