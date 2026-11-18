//! `select`: the view half of S14 (the generic selection dialog), with two
//! parameterisations: the model picker and the permissions menu. The rows are
//! the model's (`core::screens::select::Menu`), read through a provider
//! projection; the dialog returns a pick as an intent (ids, not data).
//! Oracle: `bottom_pane/list_selection_view.rs`.

mod dialog;

use rtwk::AnyWidget;
use rtwk::Cx;
use rtwk::overlay::{DialogOptions, DialogResult, show_dialog};

use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;

pub mod strings;

/// What the debug menu lists for this directory: one entry per
/// parameterisation, on the model a fixture script made.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "model_picker",
            description: "three models, the first in use",
            script: include_str!("fixtures/model_picker.jsonl"),
            open: EntryOpen::Dialog(|_| dialog::MenuDialog.into()),
        },
        DebugEntry {
            name: "permissions_menu",
            description: "the /permissions menu",
            script: include_str!("fixtures/permissions_menu.jsonl"),
            open: EntryOpen::Dialog(|_| dialog::MenuDialog.into()),
        },
    ]
}

/// Opens the dialog from the widget that asks (`show_dialog`); it reads the
/// menu the model holds, and a pick returns to the model as an intent.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| dialog::MenuDialog.into(),
        DialogOptions::default(),
    )
}

/// Opens the first parameterisation (debug menu, no seeded model).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
