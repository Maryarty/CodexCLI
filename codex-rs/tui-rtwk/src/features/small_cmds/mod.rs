//! The view half of the small commands (ADR-013): the words of their notes. The flow is
//! `codex_tui_core::features::small_cmds` (`/logout`, `/ide`, `/daybreak`, `/debug-config`).
//! Not built yet: the IDE-context indicator of the bottom pane (`State::ide.enabled`), the
//! status line's Daybreak read through `State::saved_daybreak`, the shutdown feedback of a
//! logout (`Wish::Shutdown`) and the styled rows of the debug-config cell.

pub mod strings;

use codex_tui_core::features::small_cmds::{Note, SmallCmds};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for SmallCmds {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
