//! What the debug menu lists for the providers: the sheet over a made-up session. The list
//! itself is what the server answers (`providers/list`), so a test or a run against a server
//! gives the rows; the fixture scripts seed what is not the server's (the client's launch
//! selection of the default).

use rtwk::AnyWidget;

use codex_tui_core::screens::Kind;

use super::sheet;
use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};

/// The slot of the sheet: the screen map does not name it, the directory goes last.
pub const ID: &str = "providers";
pub const KIND: Kind = Kind::Sheet;

pub fn entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "providers_list",
            description: "the providers the server knows, a chat open",
            script: include_str!("fixtures/list.jsonl"),
            open: EntryOpen::Pane(|_| sheet::sheet()),
        },
        DebugEntry {
            name: "providers_client_override",
            description: "the launch settings select the default for new chats",
            script: include_str!("fixtures/override.jsonl"),
            open: EntryOpen::Pane(|_| sheet::sheet()),
        },
    ]
}

/// The sheet with no seeded model (the directory's row): nothing to show.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
