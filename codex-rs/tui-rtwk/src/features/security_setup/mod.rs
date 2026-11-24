//! The view half of the security-setup reminder (ADR-013): the words of its notes. The feature is
//! `codex_tui_core::features::security_setup` (the notice the host fetched, its occurrence, the
//! action's verb). UI LEFT: the inline banner over the composer (title, description, one action
//! row; Esc dismisses it for its occurrence, a dismissal survives the usage banner's replacement
//! and the thread switches) is built here by the UI half.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::Never;
use codex_tui_core::features::security_setup::{Note, SecuritySetup};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for SecuritySetup {
    /// A banner, not a sheet: no command opens one.
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
