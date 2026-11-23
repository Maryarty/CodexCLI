//! `select`: the view half of S14 (the generic selection dialog), with two
//! parameterisations: the model picker and the permissions menu. The rows are
//! the model's (`core::screens::select::Menu`), read through a provider
//! projection; the dialog returns a pick as an intent (ids, not data).
//! Oracle: `bottom_pane/list_selection_view.rs`.

mod app_link;
mod apps;
mod confirm;
mod dialog;
mod generic;
mod lists;
mod permissions;
mod models;
pub(crate) mod prompt;
mod quick;
mod review;
mod review_sheet;
mod worktree;

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::Cx;
use rtwk::overlay::{DialogOptions, DialogResult, show_dialog};

use codex_tui_core::screens::select::Opening;
use codex_tui_core::screens::select::apps::Link;

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::widgets::picker::list::List;

pub mod strings;

pub use generic::{Purpose, pick};

/// A debug entry of a typed menu: its script makes the model the menu reads, the dialog shows it.
macro_rules! entry {
    ($name:literal, $description:literal, $file:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!($file, ".jsonl")),
            open: EntryOpen::Dialog(|_| dialog::MenuDialog.into()),
        }
    };
}

/// A debug entry of a `/permissions` page: the script's model has asked for the menu, the sheet
/// of the show is built from it (`permissions::seeded`).
macro_rules! permissions_entry {
    ($name:literal, $description:literal, $file:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!($file, ".jsonl")),
            open: EntryOpen::Dialog(permissions::seeded),
        }
    };
}

/// A debug entry of the `/model` sheet: its script asks for the show (`Ev::Showing`), the sheet is
/// the one that show opens.
macro_rules! model_entry {
    ($name:literal, $description:literal, $file:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!($file, ".jsonl")),
            open: EntryOpen::Dialog(models::seeded),
        }
    };
}

/// A debug entry of a generic list: the script seeds the session, the list is the sheet's own
/// (`generic::ListSheet`, read from `<file>.list.json`), so nothing of it is in the model.
macro_rules! list_entry {
    ($name:literal, $description:literal, $file:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!($file, ".jsonl")),
            open: EntryOpen::Dialog(|_| fixture_sheet(include_str!(concat!($file, ".list.json")))),
        }
    };
}

/// A debug entry of the link dialog of an app: the script seeds the session, the link is the
/// sheet's own (read from `<file>.link.json`), so nothing of it is in the model.
macro_rules! link_entry {
    ($name:literal, $description:literal, $file:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!($file, ".jsonl")),
            open: EntryOpen::Dialog(|_| fixture_link(include_str!(concat!($file, ".link.json")))),
        }
    };
}

/// The sheet of a fixture's list.
fn fixture_sheet(json: &str) -> AnyWidget {
    // The fixtures are ours and read by a test that opens every entry.
    let list: List = serde_json::from_str(json).unwrap_or_default();
    generic::ListSheet {
        list: Rc::new(list),
        purpose: Purpose::Fixture,
    }
    .into()
}

/// The link dialog of a fixture's app.
fn fixture_link(json: &str) -> AnyWidget {
    // The fixtures are ours and read by a test that opens every entry.
    serde_json::from_str::<Link>(json).map_or_else(|_| AnyWidget::default(), apps::link_sheet)
}

/// What the debug menu lists for this directory: one entry per parameterisation, on the model a
/// fixture script made; the families that are sheets of their own list theirs after these.
pub fn debug_entries() -> Vec<DebugEntry> {
    let mut entries = fixture_entries();
    entries.extend(quick::debug_entries());
    entries
}

fn fixture_entries() -> Vec<DebugEntry> {
    vec![
        model_entry!(
            "model_picker",
            "three models, the first in use",
            "fixtures/model_picker"
        ),
        model_entry!(
            "models_quick",
            "the quick /model menu: an auto mode and All models",
            "fixtures/models_quick"
        ),
        model_entry!(
            "model_reasoning",
            "the reasoning menu of one model, More reasoning… last",
            "fixtures/model_reasoning"
        ),
        model_entry!(
            "model_advanced",
            "the advanced reasoning menu: Max and Ultra",
            "fixtures/model_advanced"
        ),
        model_entry!(
            "model_plan_scope",
            "where a reasoning change in Plan mode applies",
            "fixtures/model_plan_scope"
        ),
        model_entry!(
            "model_loading",
            "the /model menu while the catalog loads",
            "fixtures/model_loading"
        ),
        permissions_entry!(
            "permissions_menu",
            "the /permissions menu",
            "fixtures/permissions_menu"
        ),
        permissions_entry!(
            "profile_permissions",
            "the /permissions menu on a catalog: the built-in modes, :workspace in use",
            "fixtures/profile_permissions"
        ),
        permissions_entry!(
            "profile_permissions_disallowed_full_access",
            "the /permissions menu with Full Access disabled by requirements",
            "fixtures/profile_permissions_disallowed_full_access"
        ),
        permissions_entry!(
            "profile_permissions_custom",
            "the /permissions menu with the server's custom profiles",
            "fixtures/profile_permissions_custom"
        ),
        permissions_entry!(
            "permission_discovery_loading",
            "the /permissions menu while the catalog is read",
            "fixtures/permission_discovery_loading"
        ),
        permissions_entry!(
            "permission_discovery_remote",
            "the /permissions menu on a remote server's catalog",
            "fixtures/permission_discovery_remote"
        ),
        permissions_entry!(
            "permission_discovery_empty",
            "the /permissions menu of an empty catalog",
            "fixtures/permission_discovery_empty"
        ),
        permissions_entry!(
            "permission_discovery_failure",
            "the /permissions menu after a failed read, with Retry",
            "fixtures/permission_discovery_failure"
        ),
        permissions_entry!(
            "permissions_unnamed_server_profile",
            "the /permissions menu with Approve for me in use",
            "fixtures/permissions_unnamed_server_profile"
        ),
        permissions_entry!(
            "permissions_constrained",
            "the /permissions preset menu with Full Access forbidden by the config",
            "fixtures/permissions_constrained"
        ),
        permissions_entry!(
            "permissions_cyber_full_access",
            "the full-access confirmation of a cyber model",
            "fixtures/permissions_cyber_full_access"
        ),
        list_entry!(
            "secondary_action",
            "one row with a secondary action (key s)",
            "fixtures/secondary_action"
        ),
        list_entry!(
            "secondary_action_disabled",
            "the same row, disabled",
            "fixtures/secondary_action_disabled"
        ),
        list_entry!(
            "secondary_action_searchable",
            "the same row under a search line",
            "fixtures/secondary_action_searchable"
        ),
        list_entry!(
            "approval_modes",
            "the approval-mode list under its title",
            "fixtures/approval_modes"
        ),
        list_entry!(
            "approval_modes_subtitle",
            "approval modes under a title and a subtitle",
            "fixtures/approval_modes_subtitle"
        ),
        list_entry!(
            "footer_note",
            "one approval row with a footer note that wraps",
            "fixtures/footer_note"
        ),
        list_entry!(
            "searchable_approval",
            "a searchable one-row list",
            "fixtures/searchable_approval"
        ),
        list_entry!(
            "empty_searchable",
            "a searchable list with no items",
            "fixtures/empty_searchable"
        ),
        list_entry!(
            "branch_search",
            "two branches under a search line",
            "fixtures/branch_search"
        ),
        list_entry!(
            "single_main_searchable",
            "a searchable one-item list",
            "fixtures/single_main_searchable"
        ),
        list_entry!(
            "two_tabs_beta",
            "two tabs opening on Beta, with a search line",
            "fixtures/two_tabs_beta"
        ),
        list_entry!(
            "two_tabs_current",
            "two tabs whose second item is the current one",
            "fixtures/two_tabs_current"
        ),
        list_entry!(
            "plugin_toggle",
            "a searchable row with a toggle",
            "fixtures/plugin_toggle"
        ),
        list_entry!(
            "multi_select",
            "three checkbox rows, Enter confirms",
            "fixtures/multi_select"
        ),
        list_entry!(
            "single_line_long",
            "one single-line row longer than the width",
            "fixtures/single_line_long"
        ),
        list_entry!(
            "auto_columns",
            "two single-line rows, label column as wide as needed",
            "fixtures/auto_columns"
        ),
        list_entry!(
            "name_column_18",
            "the same rows with a name column of 18 cells",
            "fixtures/name_column_18"
        ),
        list_entry!(
            "read_only_searchable",
            "a searchable one-item list",
            "fixtures/read_only_searchable"
        ),
        list_entry!(
            "disabled_current",
            "disabled rows, one of them the current one",
            "fixtures/disabled_current"
        ),
        list_entry!(
            "twelve_trailing_disabled",
            "twelve items, the last four disabled",
            "fixtures/twelve_trailing_disabled"
        ),
        list_entry!(
            "approval_long_command",
            "two options, the second a long command",
            "fixtures/approval_long_command"
        ),
        list_entry!(
            "three_models_effort",
            "the three models under Select Model and Effort",
            "fixtures/three_models_effort"
        ),
        list_entry!(
            "three_long_descriptions",
            "three rows with a ten-x description",
            "fixtures/three_long_descriptions"
        ),
        list_entry!(
            "halfwidth_marks",
            "two rows with halfwidth sound marks",
            "fixtures/halfwidth_marks"
        ),
        list_entry!(
            "stacked_side_content",
            "preview too wide to sit beside: the stacked one",
            "fixtures/stacked_side_content"
        ),
        list_entry!(
            "side_content",
            "one item with a preview beside the rows",
            "fixtures/side_content"
        ),
        list_entry!(
            "side_content_blue",
            "one item with a preview on a blue background beside the rows",
            "fixtures/side_content_blue"
        ),
        list_entry!(
            "skills_menu",
            "the skills menu: list skills or enable and disable them",
            "fixtures/skills_menu"
        ),
        list_entry!(
            "copy_picker",
            "the copy picker: the whole status or one of its fields",
            "fixtures/copy_picker"
        ),
        list_entry!(
            "apps_loading",
            "the apps list while it loads: one disabled row",
            "fixtures/apps_loading"
        ),
        link_entry!(
            "app_link_installed",
            "the link dialog of an installed app: its page, the switch and Back",
            "fixtures/app_link_installed"
        ),
        link_entry!(
            "app_link_install",
            "the link dialog of an app that is not installed: install, then the confirmation",
            "fixtures/app_link_install"
        ),
        list_entry!(
            "worktrees_conversation",
            "where the conversation goes",
            "fixtures/worktrees_conversation"
        ),
        list_entry!(
            "worktrees_fork",
            "where the forked conversation runs",
            "fixtures/worktrees_fork"
        ),
        list_entry!(
            "worktrees_new",
            "where the new conversation runs",
            "fixtures/worktrees_new"
        ),
        list_entry!(
            "col_width_auto_visible",
            "nine rows, label column from the rows in view",
            "fixtures/col_width_auto_visible"
        ),
        list_entry!(
            "col_width_auto_all_rows",
            "nine rows, the label column from all rows",
            "fixtures/col_width_auto_all_rows"
        ),
        list_entry!(
            "twelve_items",
            "twelve plain items",
            "fixtures/twelve_items"
        ),
        list_entry!(
            "first_second_searchable",
            "two items under a search line",
            "fixtures/first_second_searchable"
        ),
        list_entry!(
            "filled_tabs_seven",
            "seven tabs, the first active",
            "fixtures/filled_tabs_seven"
        ),
        list_entry!(
            "filled_tabs_unicode",
            "three tabs, the second with wide letters",
            "fixtures/filled_tabs_unicode"
        ),
        list_entry!("filled_tab_all", "one tab", "fixtures/filled_tab_all"),
        list_entry!(
            "multi_orderable",
            "a multi-select with a fixed row, a rule and two orderable rows",
            "fixtures/multi_orderable"
        ),
        list_entry!(
            "multi_orderable_two",
            "two orderable multi-select rows",
            "fixtures/multi_orderable_two"
        ),
        list_entry!(
            "section_break",
            "a multi-select whose first row has a rule under it",
            "fixtures/section_break"
        ),
        list_entry!(
            "multi_alpha_jupiter",
            "a searchable multi-select of alpha and jupiter",
            "fixtures/multi_alpha_jupiter"
        ),
        list_entry!(
            "multi_appearance",
            "twelve fields with a rule, ordering and a preview line",
            "fixtures/multi_appearance"
        ),
        list_entry!(
            "multi_fields_twelve",
            "twelve orderable fields",
            "fixtures/multi_fields_twelve"
        ),
        list_entry!(
            "browser_keymap",
            "the keymap browser: five tabs of thirty actions, search",
            "fixtures/browser_keymap"
        ),
        list_entry!(
            "browser_single_line",
            "the browser, single-line rows",
            "fixtures/browser_single_line"
        ),
        list_entry!(
            "browser_wrapped",
            "the browser, wrapped rows",
            "fixtures/browser_wrapped"
        ),
        list_entry!(
            "description_hide_when_narrow",
            "two rows whose description column hides when narrow",
            "fixtures/description_hide_when_narrow"
        ),
        list_entry!(
            "description_hide_numbered",
            "the same two rows, the blocked one numbered by hand as the oracle's raw rows are",
            "fixtures/description_hide_numbered"
        ),
        list_entry!(
            "shared_menu",
            "the shared menu: a title, a subtitle, three rows",
            "fixtures/shared_menu"
        ),
        list_entry!(
            "multi_twelve",
            "twelve multi-select rows",
            "fixtures/multi_twelve"
        ),
        list_entry!(
            "accept_alternative",
            "a searchable row, accept bound to a printable key",
            "fixtures/accept_alternative"
        ),
        list_entry!(
            "toggle_then_next",
            "a toggle row and the row after it, searchable",
            "fixtures/toggle_then_next"
        ),
        list_entry!(
            "animations_checkbox",
            "one checkbox row",
            "fixtures/animations_checkbox"
        ),
        list_entry!(
            "first_second_searchable",
            "two rows under a search line",
            "fixtures/first_second_searchable"
        ),
        list_entry!(
            "first_second_searchable_second",
            "two rows under a search line, the second highlighted",
            "fixtures/first_second_searchable_second"
        ),
        list_entry!(
            "twelve_items",
            "twelve rows, no title",
            "fixtures/twelve_items"
        ),
        list_entry!(
            "hint_default",
            "one row, the standard footer",
            "fixtures/hint_default"
        ),
        list_entry!(
            "hint_picker",
            "one row, the standard footer, shared picker preset",
            "fixtures/hint_picker"
        ),
        list_entry!(
            "semantic_spans",
            "a row with underlined, success and dim runs",
            "fixtures/semantic_spans"
        ),
        list_entry!(
            "selected_row",
            "one highlighted row in the accent style",
            "fixtures/selected_row"
        ),
        list_entry!(
            "hard_breaks",
            "a two-line name, a two-line description",
            "fixtures/hard_breaks"
        ),
    ]
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

/// The sheet a show of the model opens (the presenter's door: `app::presenters`) and the options
/// it is shown with.
pub fn shown(opening: &Opening) -> (AnyWidget, DialogOptions) {
    match opening {
        Opening::Confirm(confirm) => (
            confirm::ConfirmSheet { confirm: *confirm }.into(),
            confirm::options(),
        ),
        Opening::Review => (review_sheet::sheet(), review_sheet::options()),
        Opening::Worktree(menu) => (worktree::sheet(menu.clone()), worktree::options()),
        Opening::Apps => (apps::sheet(), apps::options()),
        Opening::Quick(menu) => (quick::sheet(menu), quick::options()),
        Opening::Permissions(menu) => (permissions::sheet(menu.clone()), permissions::options()),
        Opening::Models(menu) => (models::sheet(menu.clone()), models::options()),
    }
}

/// Opens the first parameterisation (debug menu, no seeded model).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| AnyWidget::default())
}
