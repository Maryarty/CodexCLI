//! Shard `import` (screens S06, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: external_agent_config_migration/flow_tests.rs (1 screen-level tests here; 5 state tests left to routes-lists)
// ORACLE external_agent_config_migration/flow_tests.rs:178 external_agent_config_migration_messages_snapshot kind=snap owner=routes-lists snap=external_agent_config_migration_messages
#[test]
fn external_agent_config_migration_messages_snapshot() {
    kit::na(
        Owner::RoutesLists,
        "asserts the history lines of the started and finished notices, inserted in the chat transcript by the chat slice; the lines are built by the view's notices from core's typed result (tests/pages/import.rs::external_agent_config_migration_messages_snapshot)",
    );
}

// ---- oracle: external_agent_config_migration/mod.rs (13 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE external_agent_config_migration/mod.rs:861 prompt_snapshot kind=snap owner=routes-lists snap=external_agent_config_migration_prompt
#[test]
fn prompt_snapshot() {
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 24)
        .entry("import", "review_with_memory")
        .open();
    s.assert_snap("external_agent_config_migration_prompt");
}

// ORACLE external_agent_config_migration/mod.rs:875 customize_snapshot kind=snap owner=routes-lists snap=external_agent_config_migration_customize snap=external_agent_config_migration_customize_windows
#[test]
fn customize_snapshot() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 34)
        .entry("import", "review_with_memory")
        .open();
    s.keys("c");
    // the `_windows` snapshot is the same screen on `C:\` paths (cfg(windows) in the oracle)
    s.assert_snap("external_agent_config_migration_customize");
}

// ORACLE external_agent_config_migration/mod.rs:896 secondary_source_customize_snapshot kind=snap owner=routes-lists snap=external_agent_config_migration_secondary_source_customize
#[test]
fn secondary_source_customize_snapshot() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 18)
        .entry("import", "review_cursor")
        .open();
    s.keys("c");
    s.assert_snap("external_agent_config_migration_secondary_source_customize");
}

// ORACLE external_agent_config_migration/mod.rs:921 customize_action_snapshot kind=snap owner=routes-lists snap=external_agent_config_migration_customize_action snap=external_agent_config_migration_customize_action_windows
#[test]
fn customize_action_snapshot() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_sample")
        .open();
    s.keys("c up");
    s.assert_snap("external_agent_config_migration_customize_action");
}

// ORACLE external_agent_config_migration/mod.rs:943 action_rows_keep_single_line_allocation_and_focus_style kind=ui owner=routes-lists
#[test]
fn action_rows_keep_single_line_allocation_and_focus_style() {
    // the action row is row 23 of the customize view of the sample items (the
    // customize_action snapshot); one row, and dim while the items have the focus
    for width in [30, 80] {
        let mut items = Screen::build(Owner::RoutesLists)
            .size(width, 30)
            .entry("import", "review_sample")
            .open();
        items.keys("c");
        items.assert_text_at(2, 23, "  1. Review selection");
        // The buffer's attribute (G46: the terminal drops DIM on a blank).
        items.assert_painted(2, 23, "dim");
        let mut actions = Screen::build(Owner::RoutesLists)
            .size(width, 30)
            .entry("import", "review_sample")
            .open();
        actions.keys("c up");
        actions.assert_text_at(2, 23, "› 1. Review selection");
        actions.assert_painted(2, 23, "!dim");
    }
}

// ORACLE external_agent_config_migration/mod.rs:998 proceed_returns_selected_items kind=ui owner=routes-lists
#[test]
fn proceed_returns_selected_items() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 24)
        .entry("import", "review_sample")
        .open();
    s.assert_contains("Import setup");
    s.keys("enter");
    // the prompt is done; the items it returns are asserted by
    // core/tests/import.rs::proceed_returns_selected_items
    s.assert_absent("Import setup");
}

// ORACLE external_agent_config_migration/mod.rs:1017 toggle_item_then_proceed_keeps_remaining_selection kind=ui owner=routes-lists
#[test]
fn toggle_item_then_proceed_keeps_remaining_selection() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_sample")
        .open();
    s.keys("c space");
    s.assert_contains("Selected 3 of 4 items.");
    s.assert_contains("[ ] Settings");
    s.keys("b");
    s.assert_contains("Importing: none");
    s.keys("1");
    // the remaining three items are what core/tests/import.rs asserts
    s.assert_absent("Import setup");
}

// ORACLE external_agent_config_migration/mod.rs:1043 escape_skips_prompt kind=ui owner=routes-lists
#[test]
fn escape_skips_prompt() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 24)
        .entry("import", "review_sample")
        .open();
    s.keys("esc");
    s.assert_absent("Import setup");
}

// ORACLE external_agent_config_migration/mod.rs:1059 numeric_shortcuts_follow_visible_actions_when_proceed_is_disabled kind=ui owner=routes-lists
#[test]
fn numeric_shortcuts_follow_visible_actions_when_proceed_is_disabled() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_sample")
        .open();
    s.keys("c n b 1");
    // nothing is selected: `1` is Customize, not Import selected
    s.assert_contains("Choose what to import");
}

// ORACLE external_agent_config_migration/mod.rs:1077 empty_selection_enter_opens_customize_instead_of_proceeding kind=ui owner=routes-lists
#[test]
fn empty_selection_enter_opens_customize_instead_of_proceeding() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_none_selected")
        .open();
    s.keys("enter");
    s.assert_contains("Choose what to import");
}

// ORACLE external_agent_config_migration/mod.rs:1093 control_exit_shortcuts_cancel_prompt kind=ui owner=routes-lists
#[test]
fn control_exit_shortcuts_cancel_prompt() {
    for key in ["ctrl+c", "ctrl+d"] {
        let mut s = Screen::build(Owner::RoutesLists)
            .size(80, 24)
            .entry("import", "review_sample")
            .open();
        s.keys(key);
        s.assert_absent("Import setup");
    }
}

// ORACLE external_agent_config_migration/mod.rs:1111 numeric_shortcuts_choose_actions kind=ui owner=routes-lists
#[test]
fn numeric_shortcuts_choose_actions() {
    let mut proceed = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_sample")
        .open();
    proceed.keys("1");
    proceed.assert_absent("Import setup");

    let mut customize = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_sample")
        .open();
    customize.keys("2");
    customize.assert_contains("Choose what to import");
    customize.keys("1");
    customize.assert_contains("Import setup");

    let mut skip = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_sample")
        .open();
    skip.keys("3");
    skip.assert_absent("Import setup");
}

// ORACLE external_agent_config_migration/mod.rs:1151 summary_does_not_toggle_selection kind=ui owner=routes-lists
#[test]
fn summary_does_not_toggle_selection() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 30)
        .entry("import", "review_sample")
        .open();
    let before = s.shot();
    s.keys("space");
    s.assert_same_rows(&before);
}

// ---- oracle: external_agent_config_migration/source_tests.rs (3 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE external_agent_config_migration/source_tests.rs:17 external_agent_config_source_prompt_snapshot kind=snap owner=routes-lists snap=external_agent_config_source_prompt
#[test]
fn external_agent_config_source_prompt_snapshot() {
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("import", "source_prompt")
        .open();
    s.assert_snap("external_agent_config_source_prompt");
}

// ORACLE external_agent_config_migration/source_tests.rs:28 external_agent_config_source_prompt_selects_highlighted_source kind=ui owner=routes-lists
#[test]
fn external_agent_config_source_prompt_selects_highlighted_source() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("import", "source_prompt")
        .open();
    s.keys("down enter");
    // the source chosen is Cursor: its items are the next screen (the sample
    // item of the second source), and the prompt is gone
    s.assert_absent("Choose an import source");
    s.assert_contains("Import setup");
}

// ORACLE external_agent_config_migration/source_tests.rs:39 external_agent_config_source_prompt_can_cancel kind=ui owner=routes-lists
#[test]
fn external_agent_config_source_prompt_can_cancel() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("import", "source_prompt")
        .open();
    s.keys("esc");
    s.assert_absent("Choose an import source");
    s.assert_absent("Import setup");
}
