//! `status`: the view half of S19 (the `/status` card): the session's
//! configuration and usage as labelled rows over the card the model holds.
//! Oracle: `codex-rs/tui/src/status/`.

mod card;
mod format;

use rtwk::AnyWidget;
use rtwk::Cx;
use rtwk::overlay::{DialogOptions, DialogResult, show_dialog};

use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;

pub mod strings;

/// What the debug menu lists for this directory: one entry per state worth
/// opening, on the model a fixture script made.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![DebugEntry {
        name: "reasoning_details",
        description: "the card with reasoning details, two limits",
        script: include_str!("fixtures/reasoning_details.jsonl"),
        open: EntryOpen::Dialog(|_| card::CardDialog.into()),
    }]
}

/// Opens the card from the widget that asks (`show_dialog`); it reads the
/// card the model holds. A modal sheet (centred) over the page until chat-live
/// makes `/status` a cell of the transcript, as the oracle has it.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(cx, move |_| card::CardDialog.into(), DialogOptions::default())
}

/// Opens the card with no seeded model (debug menu): nothing to show.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
