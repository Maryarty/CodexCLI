//! The view half of `/recap` (ADR-013): the words of its notes. The flow is
//! `codex_tui_core::features::recap`. Not built yet: the cell of the recap (`↳ Recap:` with its
//! next action) and the "Recapping..." row while `State::generating`, and the automatic recap
//! after the user has been away (the window's focus and a timer are the surface's).

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::Never;
use codex_tui_core::features::recap::{Note, Recap};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Recap {
    /// A verb, no sheet a command opens.
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
