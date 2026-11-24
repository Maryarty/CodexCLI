//! The view half of `/side` and `/btw` (ADR-013): the words of its notes. The feature itself is
//! `codex_tui_core::features::side` (the fork, the links between a parent and its side
//! conversation, the footer label); it has no sheet of its own: `Ctrl+/` and `Ctrl+C` are verbs
//! the session page says.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::Never;
use codex_tui_core::features::side::{Note, Side};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Side {
    /// A verb, not a sheet: the command says it.
    type Args = Never<Self>;

    fn args(target: Never<Self>, _cwd: Cwd, _thread: Option<ThreadId>) -> Never<Self> {
        target.absurd()
    }

    fn open<S: 'static>(_cx: &mut Cx<'_, S>, args: Never<Self>) {
        args.absurd()
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
