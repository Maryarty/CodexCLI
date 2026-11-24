//! The view half of `/mcp` (ADR-013): the words of its note. The feature is
//! `codex_tui_core::features::mcp_inventory` (the ask, the pages, the loading row in
//! `State::loading`); `/mcp` is routed by `Feature::OPENS`, its `open` draws nothing and says the verb.

pub mod cell;
pub mod strings;

use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::mcp_inventory::{Intent, McpInventory, Note, Sheet};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::{FeatureSheet, FeatureView, verb};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for McpInventory {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    /// The inventory as the oracle draws it: coloured states, a link in the empty list. A failure
    /// is a row of text.
    fn cell(note: &Note) -> Option<AnyWidget> {
        match note {
            Note::Failed(_) => None,
            Note::Inventory(_) | Note::Empty => {
                Some(cell::InventoryCell { note: note.clone() }.into())
            }
        }
    }
}

impl FeatureSheet for McpInventory {
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
}

/// What opens the command: the thread in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub thread: Option<ThreadId>,
}
