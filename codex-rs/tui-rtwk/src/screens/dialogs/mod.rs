//! `dialogs`: the view half of S23 (the small dialogs): the shortcut overlay,
//! the warnings viewer and the feedback flow. They share one host widget that
//! reads which dialog the model holds (`core::screens::dialogs::Open`) through
//! a provider projection and returns what the user does as intents.
//! Oracle: `bottom_pane/shortcut_overlay.rs`, `warnings_view.rs`,
//! `feedback_view.rs`.

mod dialog;
mod feedback;
mod shortcuts;
mod warnings;

use rtwk::AnyWidget;
use rtwk::Cx;
use rtwk::overlay::{DialogOptions, DialogResult, show_dialog};

use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;

pub mod strings;

/// What the debug menu lists for this directory: one entry per dialog worth
/// opening, on the model a fixture script made.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries = [
        (
            "shortcut_overlay",
            "the shortcut overlay, list page",
            include_str!("fixtures/shortcut_overlay.jsonl"),
        ),
        (
            "warnings",
            "the warnings viewer, two warnings",
            include_str!("fixtures/warnings.jsonl"),
        ),
        (
            "warnings_long",
            "the warnings viewer, a long text",
            include_str!("fixtures/warnings_long.jsonl"),
        ),
        (
            "feedback_categories",
            "/feedback: how was this?",
            include_str!("fixtures/feedback_categories.jsonl"),
        ),
        (
            "feedback_consent",
            "/feedback: include logs?",
            include_str!("fixtures/feedback_consent.jsonl"),
        ),
        (
            "feedback_consent_windows",
            "/feedback: include logs, Windows",
            include_str!("fixtures/feedback_consent_windows.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Dialog(|_| dialog::SmallDialog.into()),
        })
        .collect()
}

/// Opens the dialog from the widget that asks (`show_dialog`); it reads the
/// dialog the model holds, and what the user does returns as intents.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| dialog::SmallDialog.into(),
        DialogOptions::default(),
    )
}

/// Opens the dialogs with no seeded model (debug menu): nothing to show.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
