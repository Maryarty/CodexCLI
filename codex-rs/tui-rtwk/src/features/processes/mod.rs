//! The view half of `/ps` (ADR-013): the words of its note. The feature is
//! `codex_tui_core::features::processes`; its cell is a snapshot of the thread's background
//! terminals. `/ps` is routed by `Feature::OPENS`; its `open` draws nothing and says the verb.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::processes::{Intent, Note, Processes, Sheet};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::{FeatureSheet, FeatureView, verb};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Processes {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

impl FeatureSheet for Processes {
    /// The thread in use: `/ps` draws no sheet, its `open` says the verb.
    type Args = Args;

    fn args(_target: Sheet, _cwd: Cwd, thread: Option<ThreadId>) -> Args {
        Args { thread }
    }

    fn open<S: 'static>(cx: &mut Cx<'_, S>, Args { thread: source }: Args) {
        verb(cx, Intent::Show { source });
    }
}

/// What opens the command: the thread in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub thread: Option<ThreadId>,
}
