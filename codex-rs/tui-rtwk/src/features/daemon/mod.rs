//! The view half of `/daemon` (ADR-013): `/daemon` is `Target::Feature(Daemon)`, whose `open` says
//! the verb `Verb::Open` (the probe, then the menu as a wish). The menu, its two confirmations
//! and the rows' words are the sheet's (UI LEFT: P3 of the full-port map); it reads
//! `daemon::State::page`, says `Verb`s; `Update and exit` asks the host to keep the update and the app ends (the host runs it
//! once the terminal is restored).

pub mod sheet;
pub mod strings;

use std::rc::Rc;

use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::Scope;
use codex_tui_core::features::daemon::{Daemon, Note, Sheet, Verb, Wish};
use codex_tui_core::ids::{Cwd, ThreadId};

use self::sheet::DaemonSheet;
use super::{FeatureSheet, FeatureView, Shown};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Daemon {
    /// A page opens once for the wish, over the chat (no sheet is kept open for it: each page
    /// closes before the next is wished for).
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let Wish::Menu(menu) = *wish;
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget { DaemonSheet { menu }.into() }),
            options: sheet::options(menu),
        })
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

impl FeatureSheet for Daemon {
    type Args = Args;

    fn args(_target: Sheet, _cwd: Cwd, _thread: Option<ThreadId>) -> Args {
        Args
    }

    /// The verb probes the connection; the menu opens as the feature's wish.
    fn open<S: 'static>(cx: &mut Cx<'_, S>, _args: Args) {
        super::verb(cx, Verb::Open);
    }
}

/// What opens `/daemon`: nothing, the verb needs no ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args;
