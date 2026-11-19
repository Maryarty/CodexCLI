//! The page's actions and key tables.

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
/// `rename_key` with the default keymap), bound with rtwk's `KeyBindings`:
/// the keys a focused widget takes (the arrows, the page keys, Home and End
/// of the list; the text of the editing line) never reach them. The list
/// keymap's `j`/`k` and ctrl+n/p/j/k step the highlight.
pub(super) const HELP_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("?", Act::Help),
    ("ctrl+c", Act::Interrupt),
];
pub(super) const SEARCH_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("f3", Act::Search),
    ("ctrl+c", Act::Interrupt),
    ("down", Act::Move(Move::Step(true))),
    ("up", Act::Move(Move::Step(false))),
    ("pagedown", Act::Move(Move::Page(true))),
    ("ctrl+f", Act::Move(Move::Page(true))),
    ("pageup", Act::Move(Move::Page(false))),
    ("ctrl+b", Act::Move(Move::Page(false))),
    ("home", Act::Move(Move::Edge(false))),
    ("end", Act::Move(Move::Edge(true))),
];
pub(super) const RENAME_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("f3", Act::Search),
    ("ctrl+c", Act::Interrupt),
];
pub(super) const BROWSE_KEYS: &[(&str, Act)] = &[
    ("escape", Act::Escape),
    ("tab", Act::Filter { back: false }),
    ("shift+tab", Act::Filter { back: true }),
    ("f3", Act::Search),
    ("ctrl+c", Act::Interrupt),
    ("down", Act::Move(Move::Step(true))),
    ("ctrl+j", Act::Move(Move::Step(true))),
    ("ctrl+n", Act::Move(Move::Step(true))),
    ("j", Act::Move(Move::Step(true))),
    ("up", Act::Move(Move::Step(false))),
    ("ctrl+k", Act::Move(Move::Step(false))),
    ("ctrl+p", Act::Move(Move::Step(false))),
    ("k", Act::Move(Move::Step(false))),
    ("pagedown", Act::Move(Move::Page(true))),
    ("ctrl+f", Act::Move(Move::Page(true))),
    ("pageup", Act::Move(Move::Page(false))),
    ("ctrl+b", Act::Move(Move::Page(false))),
    ("home", Act::Move(Move::Edge(false))),
    ("end", Act::Move(Move::Edge(true))),
    ("enter", Act::Accept),
    ("backspace", Act::Delete),
    ("right", Act::Right),
    ("?", Act::Help),
    ("g", Act::Grouping),
    ("/", Act::Search),
    ("n", Act::New),
    ("r", Act::Rename),
    ("a", Act::Archive),
    ("h", Act::Hide),
    ("p", Act::Pin),
    ("x", Act::Stop),
    ("o", Act::Resume),
    ("f", Act::Fork),
    ("w", Act::NewWorktree),
];

/// One mode's keys as `KeyBindings` takes them: each sends its act.
pub(super) fn table(cx: &mut Cx<'_, ScreenState>, keys: &[(&'static str, Act)]) -> view::Table {
    keys.iter()
        .map(|(name, act)| (*name, cx.send(Msg::Act(*act))))
        .collect()
}
