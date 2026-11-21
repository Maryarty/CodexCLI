//! `select`: the view half of S14 (the generic selection dialog), with two
//! parameterisations: the model picker and the permissions menu. The rows are
//! the model's (`core::screens::select::Menu`), read through a provider
//! projection; the dialog returns a pick as an intent (ids, not data).
//! Oracle: `bottom_pane/list_selection_view.rs`.

mod dialog;
mod lists;
mod rows;

use rtwk::AnyWidget;
use rtwk::Cx;
use rtwk::overlay::{DialogResult, show_dialog};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;

pub mod strings;

/// What the debug menu lists for this directory: one entry per
/// parameterisation, on the model a fixture script made.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries: [(&'static str, &'static str, &'static str); 29] = [
        (
            "model_picker",
            "three models, the first in use",
            include_str!("fixtures/model_picker.jsonl"),
        ),
        (
            "permissions_menu",
            "the /permissions menu",
            include_str!("fixtures/permissions_menu.jsonl"),
        ),
        (
            "secondary_action",
            "one row with a secondary action (key s)",
            include_str!("fixtures/secondary_action.jsonl"),
        ),
        (
            "secondary_action_disabled",
            "the same row, disabled",
            include_str!("fixtures/secondary_action_disabled.jsonl"),
        ),
        (
            "secondary_action_searchable",
            "the same row under a search line",
            include_str!("fixtures/secondary_action_searchable.jsonl"),
        ),
        (
            "approval_modes",
            "the approval-mode list under its title",
            include_str!("fixtures/approval_modes.jsonl"),
        ),
        (
            "approval_modes_subtitle",
            "the approval-mode list under a title and a subtitle",
            include_str!("fixtures/approval_modes_subtitle.jsonl"),
        ),
        (
            "footer_note",
            "one approval row with a footer note that wraps",
            include_str!("fixtures/footer_note.jsonl"),
        ),
        (
            "searchable_approval",
            "a searchable one-row list",
            include_str!("fixtures/searchable_approval.jsonl"),
        ),
        (
            "empty_searchable",
            "a searchable list with no items",
            include_str!("fixtures/empty_searchable.jsonl"),
        ),
        (
            "branch_search",
            "two branches under a search line",
            include_str!("fixtures/branch_search.jsonl"),
        ),
        (
            "single_main_searchable",
            "a searchable one-item list",
            include_str!("fixtures/single_main_searchable.jsonl"),
        ),
        (
            "two_tabs_beta",
            "two tabs of one item, opening on Beta, with a search line",
            include_str!("fixtures/two_tabs_beta.jsonl"),
        ),
        (
            "two_tabs_current",
            "two tabs whose second item is the current one",
            include_str!("fixtures/two_tabs_current.jsonl"),
        ),
        (
            "plugin_toggle",
            "a searchable row with a toggle",
            include_str!("fixtures/plugin_toggle.jsonl"),
        ),
        (
            "single_line_long",
            "one single-line row longer than the width",
            include_str!("fixtures/single_line_long.jsonl"),
        ),
        (
            "auto_columns",
            "two single-line rows, the label column as wide as its widest",
            include_str!("fixtures/auto_columns.jsonl"),
        ),
        (
            "name_column_18",
            "the same rows with a name column of 18 cells",
            include_str!("fixtures/name_column_18.jsonl"),
        ),
        (
            "read_only_searchable",
            "a searchable one-item list",
            include_str!("fixtures/read_only_searchable.jsonl"),
        ),
        (
            "disabled_current",
            "disabled rows, one of them the current one",
            include_str!("fixtures/disabled_current.jsonl"),
        ),
        (
            "twelve_trailing_disabled",
            "twelve items, the last four disabled",
            include_str!("fixtures/twelve_trailing_disabled.jsonl"),
        ),
        (
            "approval_long_command",
            "two options, the second a long command",
            include_str!("fixtures/approval_long_command.jsonl"),
        ),
        (
            "three_models_effort",
            "the three models under Select Model and Effort",
            include_str!("fixtures/three_models_effort.jsonl"),
        ),
        (
            "three_long_descriptions",
            "three rows with a ten-x description under Debug",
            include_str!("fixtures/three_long_descriptions.jsonl"),
        ),
        (
            "halfwidth_marks",
            "two rows with halfwidth sound marks",
            include_str!("fixtures/halfwidth_marks.jsonl"),
        ),
        (
            "col_width_auto_visible",
            "nine rows, the label column from the rows in view",
            include_str!("fixtures/col_width_auto_visible.jsonl"),
        ),
        (
            "col_width_auto_all_rows",
            "nine rows, the label column from all rows",
            include_str!("fixtures/col_width_auto_all_rows.jsonl"),
        ),
        (
            "twelve_items",
            "twelve plain items",
            include_str!("fixtures/twelve_items.jsonl"),
        ),
        (
            "first_second_searchable",
            "two items under a search line",
            include_str!("fixtures/first_second_searchable.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Dialog(|_| dialog::SelectSheet.into()),
        })
        .collect()
}

/// Opens the dialog from the widget that asks (`show_dialog`); it reads the
/// menu the model holds, and a pick returns to the model as an intent.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| dialog::SelectSheet.into(),
        crate::sheets::sheet_options(),
    )
}

/// Opens the first parameterisation (debug menu, no seeded model).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
