//! The view half of `/daemon` (ADR-013): `/daemon` is `Target::Feature(Daemon)`, whose `open` says
//! the verb `Verb::Open` (the probe, then the menu as a wish). The menu, its two confirmations
//! and the rows' words are the sheet's (UI LEFT: P3 of the full-port map); it reads
//! `daemon::State::page`, says `Verb`s, and the shell runs `State::update_wanted` after exit.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::daemon::{Daemon, Note, Sheet, Verb};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::{FeatureSheet, FeatureView};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Daemon {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

impl FeatureSheet for Daemon {
    type Args = Args;

    fn args(_target: Sheet, _cwd: Cwd, _thread: Option<ThreadId>) -> Args {
        Args
    }

    fn open<S: 'static>(cx: &mut Cx<'_, S>, _args: Args) {
        super::verb(cx, Verb::Open);
        // The menu sheet is not built (UI LEFT): the popup says so until it lands, and the
        // `Wish::Menu` the verb asks for waits for its presenter.
        let action = crate::sheets::select::strings::of(cx).daemon_action;
        drop(crate::widgets::page_keys::not_yet(cx, action));
    }
}

/// What opens `/daemon`: nothing, the verb needs no ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args;
