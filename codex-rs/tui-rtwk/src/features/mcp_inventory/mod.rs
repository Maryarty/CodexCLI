//! The view half of `/mcp` (ADR-013): the words of its note. The feature is
//! `codex_tui_core::features::mcp_inventory` (the ask, the pages, the loading row in
//! `State::loading`); `/mcp` is routed by `Feature::OPENS`, its `open` draws nothing and says the verb.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::mcp_inventory::{Intent, McpInventory, Note, Sheet};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::{FeatureView, verb};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for McpInventory {
    /// The thread in use: `/mcp` draws no sheet, its `open` says the verb (the summary).
    type Args = Args;

    fn args(_target: Sheet, _cwd: Cwd, thread: Option<ThreadId>) -> Args {
        Args { thread }
    }

    fn open<S: 'static>(cx: &mut Cx<'_, S>, Args { thread: source }: Args) {
        verb(
            cx,
            Intent::Show {
                source,
                full: false,
            },
        );
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

/// What opens the command: the thread in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub thread: Option<ThreadId>,
}
