//! The view half of the small commands (ADR-013): the words of their notes. The flow is
//! `codex_tui_core::features::small_cmds` (`/logout`, `/ide`, `/daybreak`, `/debug-config`).
//! Not built yet: the IDE-context indicator of the bottom pane (`State::ide.enabled`), the
//! status line's Daybreak read through `State::saved_daybreak`, the shutdown feedback of a
//! logout (`Wish::Shutdown`) and the styled rows of the debug-config cell.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::Never;
use codex_tui_core::features::small_cmds::{Note, SmallCmds};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for SmallCmds {
    /// Verbs and a wish, no sheet a command opens.
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
