//! The view half of `/feedback` (ADR-013): the words of its notes. The flow itself is
//! `codex_tui_core::features::feedback` (the report's checks, its delivery to the host and the
//! line that says how it went). The category and consent steps are still
//! `sheets::dialogs::feedback`, opened by `Target::Dialogs`; the note step and the move of those
//! two into this directory are the UI half's.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::Never;
use codex_tui_core::features::feedback::{Feedback, Note};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Feedback {
    /// A verb for now: the steps open through the dialogs' sheet.
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
