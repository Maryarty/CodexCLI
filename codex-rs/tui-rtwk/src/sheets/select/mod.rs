//! `select`: the view half of S14 (the generic selection dialog), with two
//! parameterisations: the model picker and the permissions menu. The rows are
//! the model's (`core::screens::select::Menu`), read through a provider
//! projection; the dialog returns a pick as an intent (ids, not data).
//! Oracle: `bottom_pane/list_selection_view.rs`.

mod app_link;
mod dialog;
mod lists;
pub(crate) mod prompt;
mod review;

use rtwk::AnyWidget;
use rtwk::Cx;
use rtwk::layout::Alignment;
use rtwk::overlay::{DialogOptions, DialogResult, show_dialog};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;

pub mod strings;

/// What the debug menu lists for this directory: one entry per
/// parameterisation, on the model a fixture script made.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries: [(&'static str, &'static str, &'static str); 83] = [
        (
            "model_picker",
            "three models, the first in use",
            include_str!("fixtures/model_picker.jsonl"),
        ),
        (
            "models_quick",
            "the quick /model menu: an auto mode and All models",
            include_str!("fixtures/models_quick.jsonl"),
        ),
        (
            "model_reasoning",
            "the reasoning menu of one model, More reasoning… last",
            include_str!("fixtures/model_reasoning.jsonl"),
        ),
        (
            "model_advanced",
            "the advanced reasoning menu: Max and Ultra",
            include_str!("fixtures/model_advanced.jsonl"),
        ),
        (
            "model_plan_scope",
            "where a reasoning change in Plan mode applies",
            include_str!("fixtures/model_plan_scope.jsonl"),
        ),
        (
            "model_loading",
            "the /model menu while the catalog loads",
            include_str!("fixtures/model_loading.jsonl"),
        ),
        (
            "permissions_menu",
            "the /permissions menu",
            include_str!("fixtures/permissions_menu.jsonl"),
        ),
        (
            "profile_permissions",
            "the /permissions menu on a catalog: the built-in modes, :workspace in use",
            include_str!("fixtures/profile_permissions.jsonl"),
        ),
        (
            "profile_permissions_disallowed_full_access",
            "the /permissions menu with Full Access disabled by requirements",
            include_str!("fixtures/profile_permissions_disallowed_full_access.jsonl"),
        ),
        (
            "profile_permissions_custom",
            "the /permissions menu with the server's custom profiles",
            include_str!("fixtures/profile_permissions_custom.jsonl"),
        ),
        (
            "permission_discovery_loading",
            "the /permissions menu while the catalog is read",
            include_str!("fixtures/permission_discovery_loading.jsonl"),
        ),
        (
            "permission_discovery_remote",
            "the /permissions menu on a remote server's catalog",
            include_str!("fixtures/permission_discovery_remote.jsonl"),
        ),
        (
            "permission_discovery_empty",
            "the /permissions menu of an empty catalog",
            include_str!("fixtures/permission_discovery_empty.jsonl"),
        ),
        (
            "permission_discovery_failure",
            "the /permissions menu after a failed read, with Retry",
            include_str!("fixtures/permission_discovery_failure.jsonl"),
        ),
        (
            "permissions_unnamed_server_profile",
            "the /permissions menu with Approve for me in use",
            include_str!("fixtures/permissions_unnamed_server_profile.jsonl"),
        ),
        (
            "permissions_constrained",
            "the /permissions preset menu with Full Access forbidden by the config",
            include_str!("fixtures/permissions_constrained.jsonl"),
        ),
        (
            "permissions_cyber_full_access",
            "the full-access confirmation of a cyber model",
            include_str!("fixtures/permissions_cyber_full_access.jsonl"),
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
            "approval modes under a title and a subtitle",
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
            "two tabs opening on Beta, with a search line",
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
            "multi_select",
            "three checkbox rows, Enter confirms",
            include_str!("fixtures/multi_select.jsonl"),
        ),
        (
            "single_line_long",
            "one single-line row longer than the width",
            include_str!("fixtures/single_line_long.jsonl"),
        ),
        (
            "auto_columns",
            "two single-line rows, label column as wide as needed",
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
            "three rows with a ten-x description",
            include_str!("fixtures/three_long_descriptions.jsonl"),
        ),
        (
            "halfwidth_marks",
            "two rows with halfwidth sound marks",
            include_str!("fixtures/halfwidth_marks.jsonl"),
        ),
        (
            "stacked_side_content",
            "preview too wide to sit beside: the stacked one",
            include_str!("fixtures/stacked_side_content.jsonl"),
        ),
        (
            "side_content",
            "one item with a preview beside the rows",
            include_str!("fixtures/side_content.jsonl"),
        ),
        (
            "side_content_blue",
            "one item with a preview on a blue background beside the rows",
            include_str!("fixtures/side_content_blue.jsonl"),
        ),
        (
            "skills_menu",
            "the skills menu: list skills or enable and disable them",
            include_str!("fixtures/skills_menu.jsonl"),
        ),
        (
            "copy_picker",
            "the copy picker: the whole status or one of its fields",
            include_str!("fixtures/copy_picker.jsonl"),
        ),
        (
            "apps_loading",
            "the apps list while it loads: one disabled row",
            include_str!("fixtures/apps_loading.jsonl"),
        ),
        (
            "app_link_installed",
            "the link dialog of an installed app: its page, the switch and Back",
            include_str!("fixtures/app_link_installed.jsonl"),
        ),
        (
            "app_link_install",
            "the link dialog of an app that is not installed: install, then the confirmation",
            include_str!("fixtures/app_link_install.jsonl"),
        ),
        (
            "worktrees_conversation",
            "where the conversation goes",
            include_str!("fixtures/worktrees_conversation.jsonl"),
        ),
        (
            "worktrees_fork",
            "where the forked conversation runs",
            include_str!("fixtures/worktrees_fork.jsonl"),
        ),
        (
            "worktrees_new",
            "where the new conversation runs",
            include_str!("fixtures/worktrees_new.jsonl"),
        ),
        (
            "col_width_auto_visible",
            "nine rows, label column from the rows in view",
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
        (
            "filled_tabs_seven",
            "seven tabs, the first active",
            include_str!("fixtures/filled_tabs_seven.jsonl"),
        ),
        (
            "filled_tabs_unicode",
            "three tabs, the second with wide letters",
            include_str!("fixtures/filled_tabs_unicode.jsonl"),
        ),
        (
            "filled_tab_all",
            "one tab",
            include_str!("fixtures/filled_tab_all.jsonl"),
        ),
        (
            "multi_orderable",
            "a multi-select with a fixed row, a rule and two orderable rows",
            include_str!("fixtures/multi_orderable.jsonl"),
        ),
        (
            "multi_orderable_two",
            "two orderable multi-select rows",
            include_str!("fixtures/multi_orderable_two.jsonl"),
        ),
        (
            "section_break",
            "a multi-select whose first row has a rule under it",
            include_str!("fixtures/section_break.jsonl"),
        ),
        (
            "multi_alpha_jupiter",
            "a searchable multi-select of alpha and jupiter",
            include_str!("fixtures/multi_alpha_jupiter.jsonl"),
        ),
        (
            "multi_appearance",
            "twelve fields with a rule, ordering and a preview line",
            include_str!("fixtures/multi_appearance.jsonl"),
        ),
        (
            "multi_fields_twelve",
            "twelve orderable fields",
            include_str!("fixtures/multi_fields_twelve.jsonl"),
        ),
        (
            "browser_keymap",
            "the keymap browser: five tabs of thirty actions, search",
            include_str!("fixtures/browser_keymap.jsonl"),
        ),
        (
            "browser_single_line",
            "the browser, single-line rows",
            include_str!("fixtures/browser_single_line.jsonl"),
        ),
        (
            "browser_wrapped",
            "the browser, wrapped rows",
            include_str!("fixtures/browser_wrapped.jsonl"),
        ),
        (
            "description_hide_when_narrow",
            "two rows whose description column hides when narrow",
            include_str!("fixtures/description_hide_when_narrow.jsonl"),
        ),
        (
            "description_hide_numbered",
            "the same two rows, the blocked one numbered by hand as the oracle's raw rows are",
            include_str!("fixtures/description_hide_numbered.jsonl"),
        ),
        (
            "shared_menu",
            "the shared menu: a title, a subtitle, three rows",
            include_str!("fixtures/shared_menu.jsonl"),
        ),
        (
            "multi_twelve",
            "twelve multi-select rows",
            include_str!("fixtures/multi_twelve.jsonl"),
        ),
        (
            "accept_alternative",
            "a searchable row, accept bound to a printable key",
            include_str!("fixtures/accept_alternative.jsonl"),
        ),
        (
            "toggle_then_next",
            "a toggle row and the row after it, searchable",
            include_str!("fixtures/toggle_then_next.jsonl"),
        ),
        (
            "animations_checkbox",
            "one checkbox row",
            include_str!("fixtures/animations_checkbox.jsonl"),
        ),
        (
            "first_second_searchable",
            "two rows under a search line",
            include_str!("fixtures/first_second_searchable.jsonl"),
        ),
        (
            "first_second_searchable_second",
            "two rows under a search line, the second highlighted",
            include_str!("fixtures/first_second_searchable_second.jsonl"),
        ),
        (
            "twelve_items",
            "twelve rows, no title",
            include_str!("fixtures/twelve_items.jsonl"),
        ),
        (
            "hint_default",
            "one row, the standard footer",
            include_str!("fixtures/hint_default.jsonl"),
        ),
        (
            "hint_picker",
            "one row, the standard footer, shared picker preset",
            include_str!("fixtures/hint_picker.jsonl"),
        ),
        (
            "semantic_spans",
            "a row with underlined, success and dim runs",
            include_str!("fixtures/semantic_spans.jsonl"),
        ),
        (
            "selected_row",
            "one highlighted row in the accent style",
            include_str!("fixtures/selected_row.jsonl"),
        ),
        (
            "hard_breaks",
            "a two-line name, a two-line description",
            include_str!("fixtures/hard_breaks.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Dialog(|_| dialog::MenuDialog.into()),
        })
        .collect()
}

/// Opens the dialog from the widget that asks (`show_dialog`); it reads the
/// menu the model holds, and a pick returns to the model as an intent.
pub fn open<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    // On the nearest navigator: the dialog is built under the providers of the
    // screen that asks (a seeded debug entry provides its own model to its
    // route only; the root navigator is above it).
    show_dialog(
        cx,
        move |_| dialog::MenuDialog.into(),
        DialogOptions {
            use_root_navigator: false,
            ..crate::sheets::sheet_options()
        },
    )
}

/// Opens the same dialog as a confirmation floats: centered on the screen, not
/// at its bottom (the oracle's `ViewPresentation::Centered`, the shared dialog
/// geometry of `SelectionViewParams::confirmation()`). The menu it shows is a
/// `Menu::Confirm`; the dialog lays itself out to the width the oracle gives it.
pub fn open_centered<S>(cx: &mut Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        move |_| dialog::MenuDialog.into(),
        DialogOptions {
            use_root_navigator: false,
            alignment: Alignment::CENTER,
            ..crate::sheets::sheet_options()
        },
    )
}

/// Opens the first parameterisation (debug menu, no seeded model).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
