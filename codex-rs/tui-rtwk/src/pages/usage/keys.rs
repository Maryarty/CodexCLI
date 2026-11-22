//! The keys of the dashboard that are the list keymap's (oracle: `analytics.rs`
//! `handle_key` with the default `ListKeymap`: `h`/`l` stand for the arrows when
//! they are not bound), bound with rtwk's `KeyBindings`.

use super::{Day, Msg};

/// The keys of a report's days and details: each sends its message.
pub(super) const LIST_KEYS: &[(&str, Msg)] = &[
    ("left", Msg::Day(Day::By(-1))),
    ("h", Msg::Day(Day::By(-1))),
    ("ctrl+h", Msg::Day(Day::By(-1))),
    ("right", Msg::Day(Day::By(1))),
    ("l", Msg::Day(Day::By(1))),
    ("ctrl+l", Msg::Day(Day::By(1))),
    ("up", Msg::Day(Day::By(-1))),
    ("k", Msg::Day(Day::By(-1))),
    ("ctrl+p", Msg::Day(Day::By(-1))),
    ("ctrl+k", Msg::Day(Day::By(-1))),
    ("down", Msg::Day(Day::By(1))),
    ("j", Msg::Day(Day::By(1))),
    ("ctrl+n", Msg::Day(Day::By(1))),
    ("ctrl+j", Msg::Day(Day::By(1))),
    ("home", Msg::Day(Day::Edge(false))),
    ("end", Msg::Day(Day::Edge(true))),
    ("enter", Msg::Details),
    ("space", Msg::Details),
    ("escape", Msg::Escape),
];
