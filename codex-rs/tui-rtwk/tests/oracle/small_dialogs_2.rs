//! Shard `small_dialogs_2` (screens S23, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: bottom_pane/hooks_browser_view.rs (23 screen-level tests here; 5 state tests left to dialogs-pane)
// ORACLE bottom_pane/hooks_browser_view.rs:902 renders_event_browser kind=snap owner=dialogs-pane snap=hooks_browser_events snap=hooks_browser_model_requests
#[test]
fn renders_event_browser() {
    Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events")
        .open()
        .assert_snap("hooks_browser_events");
    // The last event selected: Up wraps to it.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events")
        .open();
    s.keys("up");
    s.assert_snap("hooks_browser_model_requests");
    // The oracle clears keymap.accept after the last event is selected; the
    // footers of the port are fixed strings (the default keymap) until the
    // keymap config reaches the sheets.
    kit::pending(
        Owner::DialogsPane,
        "no keymap config in the port: the sheets' footers are fixed strings, so an accept key cleared from the list keymap (the oracle's `keymap.accept` emptied) cannot hide `enter details` (the same gap as the skills footer test)",
    );
    Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events_no_accept")
        .open()
        .assert_absent("enter details");
}

// ORACLE bottom_pane/hooks_browser_view.rs:915 compact_hooks_keep_the_last_event_and_selection_visible kind=snap owner=dialogs-pane snap=compact_hooks_keep_the_last_event_and_selection_visible
#[test]
fn compact_hooks_keep_the_last_event_and_selection_visible() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 16)
        .entry("settings", "hooks_events")
        .open();
    s.keys("up");
    s.assert_snap("compact_hooks_keep_the_last_event_and_selection_visible");
    s.assert_contains("Interrupt");
    s.assert_contains("Active/Installed");
    // The selected row (row 12, the `›` one) fills its whole width with the selection background.
    s.assert_style(0, 12, "bg=Reset");
    s.assert_style(39, 12, "bg=Reset");
}

// ORACLE bottom_pane/hooks_browser_view.rs:945 selected_event_rows_use_the_shared_selection_style kind=ui owner=dialogs-pane
#[test]
fn selected_event_rows_use_the_shared_selection_style() {
    let s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events")
        .open();
    // The selected event row: the shared selection foreground and BOLD (without a probed background: Reset).
    kit::pending(
        Owner::DialogsPane,
        "the selected row is the theme's accent and bold in every dialog of the port, not the oracle's unprobed-palette selection foreground (the shared selection style, not this dialog)",
    );
    s.assert_text_style("PreToolUse", "fg=Reset bold");
}

// ORACLE bottom_pane/hooks_browser_view.rs:965 renders_event_browser_with_review_column_when_needed kind=snap owner=dialogs-pane snap=hooks_browser_events_with_review_column
#[test]
fn renders_event_browser_with_review_column_when_needed() {
    let s = Screen::build(Owner::DialogsPane)
        .size(112, 19)
        .entry("settings", "hooks_events_review_column")
        .open();
    s.assert_snap("hooks_browser_events_with_review_column");
    // The review count of the first event row: the selection foreground and BOLD.
    kit::pending(
        Owner::DialogsPane,
        "the selected row is the theme's accent and bold in every dialog of the port, not the oracle's unprobed-palette selection foreground (the shared selection style, not this dialog)",
    );
    s.assert_style(48, 8, "fg=Reset bold");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1002 renders_event_browser_with_issues kind=snap owner=dialogs-pane snap=hooks_browser_events_with_issues
#[test]
fn renders_event_browser_with_issues() {
    let s = Screen::build(Owner::DialogsPane)
        .size(112, 21)
        .entry("settings", "hooks_events_issues")
        .open();
    s.assert_snap("hooks_browser_events_with_issues");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1021 renders_handler_browser_with_details kind=snap owner=dialogs-pane snap=hooks_browser_handlers
#[test]
fn renders_handler_browser_with_details() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events")
        .open();
    s.keys("enter");
    s.assert_snap("hooks_browser_handlers");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1028 renders_handler_status_messages_with_blank_fallback kind=snap owner=dialogs-pane snap=hooks_browser_status_messages
#[test]
fn renders_handler_status_messages_with_blank_fallback() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 18)
        .entry("settings", "hooks_status_messages")
        .open();
    s.keys("enter");
    s.assert_snap("hooks_browser_status_messages");
    // The selected row and the one below: the number is dim, the title is not; row 4 holds the wide glyphs.
    for y in [4, 5] {
        s.assert_style(7, y, "dim");
        s.assert_style(10, y, "!dim");
    }
    s.assert_text_at(10, 4, "检");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1068 renders_mcp_tool_handler_with_server_and_tool_details kind=snap owner=dialogs-pane snap=hooks_browser_mcp_tool_handler
#[test]
fn renders_mcp_tool_handler_with_server_and_tool_details() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 16)
        .entry("settings", "hooks_mcp_tool")
        .open();
    s.keys("enter");
    s.assert_snap("hooks_browser_mcp_tool_handler");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1100 renders_handler_additional_context_limit kind=snap owner=dialogs-pane snap=hooks_browser_additional_context_limit
#[test]
fn renders_handler_additional_context_limit() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_context_limit")
        .open();
    s.keys("enter");
    s.assert_snap("hooks_browser_additional_context_limit");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1129 renders_untrusted_enabled_handler_as_inactive kind=snap owner=dialogs-pane snap=hooks_browser_untrusted_enabled_handler
#[test]
fn renders_untrusted_enabled_handler_as_inactive() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 16)
        .entry("settings", "hooks_untrusted_enabled")
        .open();
    s.keys("enter");
    s.assert_snap("hooks_browser_untrusted_enabled_handler");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1161 review_needed_handler_rows_use_warning_color kind=ui owner=dialogs-pane
#[test]
fn review_needed_handler_rows_use_warning_color() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_trusted_and_review")
        .open();
    s.keys("enter");
    // The oracle's subject is the second row (`handler_row_lines(..)[1]`), the
    // untrusted handler's: its title is yellow. (The command, `untrusted.sh`,
    // is on the screen only in the details of the selected row.)
    s.assert_text_style("Unnamed hook · new", "fg=Yellow");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1217 renders_managed_handler_without_toggle_hint kind=snap owner=dialogs-pane snap=hooks_browser_managed_handler
#[test]
fn renders_managed_handler_without_toggle_hint() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 16)
        .entry("settings", "hooks_events")
        .open();
    s.keys("down enter");
    s.assert_snap("hooks_browser_managed_handler");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1228 renders_selected_managed_handler kind=snap owner=dialogs-pane snap=hooks_browser_selected_managed_handler
#[test]
fn renders_selected_managed_handler() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_two_managed")
        .open();
    s.keys("enter down");
    s.assert_snap("hooks_browser_selected_managed_handler");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1266 renders_scrolled_handler_window kind=snap owner=dialogs-pane snap=hooks_browser_scrolled_handlers
#[test]
fn renders_scrolled_handler_window() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 23)
        .entry("settings", "hooks_many")
        .open();
    s.keys("enter");
    // MAX_POPUP_ROWS + 1 = 9 presses of down
    s.keys("down down down down down down down down down");
    s.assert_snap("hooks_browser_scrolled_handlers");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1304 renders_command_details_with_three_line_cap kind=snap owner=dialogs-pane snap=hooks_browser_capped_command_details
#[test]
fn renders_command_details_with_three_line_cap() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(44, 19)
        .entry("settings", "hooks_long_command")
        .open();
    s.keys("enter");
    s.assert_contains("twelve thirteen fourteen");
    kit::pending(
        Owner::DialogsPane,
        "pending(G74): rtwk's ellipsis of the last of max_lines rows cuts the rest of the text at the width (`fourteen fift…`), the oracle wraps first and appends the ellipsis to the third row (`fourteen…`)",
    );
    s.assert_snap("hooks_browser_capped_command_details");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1333 renders_empty_handler_browser_message kind=snap owner=dialogs-pane snap=hooks_browser_empty_handlers
#[test]
fn renders_empty_handler_browser_message() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 8)
        .entry("settings", "hooks_empty")
        .open();
    s.keys("down enter");
    s.assert_snap("hooks_browser_empty_handlers");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1438 renders_review_needed_handler kind=snap owner=dialogs-pane snap=hooks_browser_review_needed_handler
#[test]
fn renders_review_needed_handler() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 16)
        .entry("settings", "hooks_review_needed")
        .open();
    s.keys("enter");
    s.assert_snap("hooks_browser_review_needed_handler");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1502 space_does_not_toggle_managed_handler kind=ui owner=dialogs-pane
#[test]
fn space_does_not_toggle_managed_handler() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_managed_handler")
        .open();
    s.keys("enter");
    let before = s.shot();
    // Space on a managed handler changes nothing (no toggle event): the rows stay, `[x]` stays.
    s.keys("space");
    s.assert_same_rows(&before);
    s.assert_contains("[x]");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1526 trust_key_trusts_review_needed_handler_without_changing_enablement kind=ui owner=dialogs-pane
#[test]
fn trust_key_trusts_review_needed_handler_without_changing_enablement() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_review_needed")
        .open();
    s.keys("enter");
    s.keys("t");
    // Trusted now; the handler is still off (the TrustHook event is not drawn).
    s.assert_contains("Trust     Trusted");
    s.assert_contains("[ ]");
    s.assert_absent("review required");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1562 trust_key_preserves_disabled_modified_handler kind=ui owner=dialogs-pane
#[test]
fn trust_key_preserves_disabled_modified_handler() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_modified_handler")
        .open();
    s.keys("enter");
    s.keys("t");
    // The oracle: still disabled, now trusted.
    s.assert_contains("Trust     Trusted");
    s.assert_contains("[ ]");
    s.assert_absent("Modified since last trusted");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1601 trust_key_on_event_page_trusts_all_review_needed_hooks kind=ui owner=dialogs-pane
#[test]
fn trust_key_on_event_page_trusts_all_review_needed_hooks() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events_review_mixed")
        .open();
    s.keys("t");
    // All three are trusted: no hook needs review any more (the TrustHooks event is not drawn).
    s.assert_absent("needs review");
    s.assert_absent("trust all");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1679 escape_returns_to_the_selected_event kind=ui owner=dialogs-pane
#[test]
fn escape_returns_to_the_selected_event() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events")
        .open();
    s.keys("down enter esc");
    // Back on the events page, the second event (PermissionRequest) is selected.
    s.assert_contains("Lifecycle hooks from config and enabled plugins.");
    s.assert_contains("› PermissionRequest");
}

// ORACLE bottom_pane/hooks_browser_view.rs:1693 esc_routes_through_the_view kind=ui owner=dialogs-pane
#[test]
fn esc_routes_through_the_view() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 17)
        .entry("settings", "hooks_events")
        .open();
    s.keys("enter");
    // esc is handled by the view (back to the events page) and does not close the dialog.
    s.keys("esc");
    s.assert_contains("Lifecycle hooks from config and enabled plugins.");
}

// ---- oracle: bottom_pane/memories_settings_view_tests.rs (1 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/memories_settings_view_tests.rs:6 reset_confirmation_keeps_both_choices_at_wrap_boundary kind=snap owner=dialogs-pane snap=reset_confirmation_keeps_both_choices_at_wrap_boundary
#[test]
fn reset_confirmation_keeps_both_choices_at_wrap_boundary() {
    // Each width is one screen; the heights are the view's desired_height(width):
    // the sheet sits at the bottom edge of the screen over the debug menu.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(68, 13)
        .entry("settings", "memories_reset_confirmation")
        .open();
    s.assert_snap_section(
        "reset_confirmation_keeps_both_choices_at_wrap_boundary",
        "width=68",
    );
    s.assert_contains("Go back");
    s.resize(69, 12);
    s.assert_snap_section(
        "reset_confirmation_keeps_both_choices_at_wrap_boundary",
        "width=69",
    );
    s.resize(70, 12);
    s.assert_snap_section(
        "reset_confirmation_keeps_both_choices_at_wrap_boundary",
        "width=70",
    );
    s.resize(71, 12);
    s.assert_snap_section(
        "reset_confirmation_keeps_both_choices_at_wrap_boundary",
        "width=71",
    );
    s.resize(72, 12);
    s.assert_snap_section(
        "reset_confirmation_keeps_both_choices_at_wrap_boundary",
        "width=72",
    );
}

// ---- oracle: bottom_pane/shortcut_overlay_tests.rs (2 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/shortcut_overlay_tests.rs:71 shortcut_overlay_uses_runtime_bindings_and_wsl_paste kind=snap owner=dialogs-pane snap=shortcut_overlay_customized_wsl
#[test]
fn shortcut_overlay_uses_runtime_bindings_and_wsl_paste() {
    kit::pending(
        Owner::DialogsPane,
        "the overlay reads the keymap (`.keymap(..)` seeds the ctrl+x chords and the unbound rows), but its Paste image key is the fixed `⌃v`: the oracle's `is_wsl` makes it `⌃⌥v` and no fact of the sheet says WSL (services/clipboard/machine.rs probes the process); `esc_backtrack_hint` (`esc` for Edit last message) is no seed either; no entry dialogs/shortcut_overlay_custom_wsl",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 21)
        .entry("dialogs", "shortcut_overlay_custom_wsl")
        .open();
    s.assert_snap("shortcut_overlay_customized_wsl");
}

// ORACLE bottom_pane/shortcut_overlay_tests.rs:96 shortcut_overlay_clipping_preserves_customization_in_small_areas kind=snap owner=dialogs-pane snap=shortcut_overlay_customize_only
#[test]
fn shortcut_overlay_clipping_preserves_customization_in_small_areas() {
    kit::pending(
        Owner::DialogsPane,
        "the footer props of this test (task running, agents key Left, customized keys, 28x1) are not a seed: no entry dialogs/shortcut_overlay_footer_small",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(28, 1)
        .entry("dialogs", "shortcut_overlay_footer_small")
        .open();
    s.assert_snap("shortcut_overlay_customize_only");
}

// ---- oracle: bottom_pane/skills_toggle_view.rs (5 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE bottom_pane/skills_toggle_view.rs:422 renders_basic_popup kind=snap owner=dialogs-pane snap=skills_toggle_basic
#[test]
fn renders_basic_popup() {
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 12)
        .entry("settings", "skills_basic")
        .open();
    s.assert_snap("skills_toggle_basic");
}

// ORACLE bottom_pane/skills_toggle_view.rs:473 filtering_long_skill_names_preserves_the_matching_row kind=snap owner=dialogs-pane snap=skills_toggle_pasted_search
#[test]
fn filtering_long_skill_names_preserves_the_matching_row() {
    // The oracle's snapshot is the view at the height its one filtered row
    // needs (`desired_height`): the screen starts at the two rows' and shrinks.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(96, 12)
        .entry("settings", "skills_long_names")
        .open();
    s.keys("c");
    s.paste("  ompletion\r\n\t ");
    s.resize(96, 11);
    s.assert_snap("skills_toggle_pasted_search");
    s.paste(" \r\n\t ");
    s.assert_snap("skills_toggle_pasted_search");
}

// ORACLE bottom_pane/skills_toggle_view.rs:503 renders_long_names_using_available_width kind=snap owner=dialogs-pane snap=skills_toggle_long_names_use_available_width
#[test]
fn renders_long_names_using_available_width() {
    let s = Screen::build(Owner::DialogsPane)
        .size(96, 12)
        .entry("settings", "skills_long_names")
        .open();
    s.assert_snap("skills_toggle_long_names_use_available_width");
}

// ORACLE bottom_pane/skills_toggle_view.rs:519 renders_long_names_at_narrow_width kind=snap owner=dialogs-pane snap=skills_toggle_long_names_at_narrow_width
#[test]
fn renders_long_names_at_narrow_width() {
    let s = Screen::build(Owner::DialogsPane)
        .size(48, 13)
        .entry("settings", "skills_long_names")
        .open();
    s.assert_snap("skills_toggle_long_names_at_narrow_width");
}

// ORACLE bottom_pane/skills_toggle_view.rs:535 footer_hint_uses_list_keymap_accept_and_cancel kind=ui owner=dialogs-pane
#[test]
fn footer_hint_uses_list_keymap_accept_and_cancel() {
    // The list keymap's accept and cancel keys, ctrl+t and ctrl+x (the oracle's `ListKeymap`).
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 12)
        .keymap(&[("list.accept", "ctrl+t"), ("list.cancel", "ctrl+x")])
        .entry("settings", "skills_empty")
        .open();
    s.assert_contains("⌃t");
    s.assert_contains("⌃x");
    s.assert_absent("enter");
    s.assert_absent("esc");
}

// ---- oracle: bottom_pane/status_line_setup.rs (4 screen-level tests here; 12 state tests left to dialogs-pane)
// ORACLE bottom_pane/status_line_setup.rs:701 picker_previews_the_pr_link_and_preserves_the_color_preference kind=ui owner=dialogs-pane
#[test]
fn picker_previews_the_pr_link_and_preserves_the_color_preference() {
    let url = "https://example.com/pr/42";
    for colors in [false, true] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(100, 24)
            .live()
            .open();
        s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"cwd":"/w"}}}"#);
        s.feed(&serde_json::json!({
            "step":"screen","dir":"chat","ev":{"status":{"items":["pull-request-number"],"colors":colors}}
        }).to_string());
        s.type_text("/statusline").keys("enter");
        s.feed(
            &serde_json::json!({
                "step":"screen","dir":"chat","ev":{"facts":{"git_summary":{
                    "request":1,"summary":{"pull_request":{"number":42,"url":url}}
                }}}
            })
            .to_string(),
        );
        s.assert_in_order(&["Configure Status Line", "PR #42", "enter save"]);
        s.assert_text_count("PR #42", 1);
        s.assert_link("PR #42", url);
        s.keys("enter");
        s.assert_config_write(&[
            (
                "tui.status_line",
                serde_json::json!(["pull-request-number"]),
            ),
            ("tui.status_line_use_colors", serde_json::json!(colors)),
        ]);
    }
}

// ORACLE bottom_pane/status_line_setup.rs:728 setup_view_snapshot_uses_runtime_preview_values kind=snap owner=dialogs-pane snap=setup_view_snapshot_uses_runtime_preview_values
#[test]
fn setup_view_snapshot_uses_runtime_preview_values() {
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 17)
        .entry("settings", "status_line_preview")
        .open();
    s.assert_snap("setup_view_snapshot_uses_runtime_preview_values");
}

// ORACLE bottom_pane/status_line_setup.rs:763 setup_view_snapshot_includes_thread_usage_items kind=snap owner=dialogs-pane snap=setup_view_snapshot_includes_thread_usage_items
#[test]
fn setup_view_snapshot_includes_thread_usage_items() {
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 17)
        .entry("settings", "status_line_thread_usage")
        .open();
    s.assert_snap("setup_view_snapshot_includes_thread_usage_items");
}

// ORACLE bottom_pane/status_line_setup.rs:789 setup_view_snapshot_includes_hostname kind=snap owner=dialogs-pane snap=setup_view_snapshot_includes_hostname
#[test]
fn setup_view_snapshot_includes_hostname() {
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 17)
        .entry("settings", "status_line_hostname")
        .open();
    s.assert_snap("setup_view_snapshot_includes_hostname");
}

// ---- oracle: bottom_pane/title_setup.rs (1 screen-level tests here; 11 state tests left to dialogs-pane)
// ORACLE bottom_pane/title_setup.rs:433 renders_title_setup_popup kind=snap owner=dialogs-pane snap=terminal_title_setup_basic
#[test]
fn renders_title_setup_popup() {
    let s = Screen::build(Owner::DialogsPane)
        .size(84, 17)
        .entry("settings", "terminal_title")
        .open();
    s.assert_snap("terminal_title_setup_basic");
}

// ---- oracle: bottom_pane/warnings_view_tests.rs (6 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/warnings_view_tests.rs:53 warning_pages_copy_only_the_current_diagnostic kind=ui owner=dialogs-pane
#[test]
fn warning_pages_copy_only_the_current_diagnostic() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 12)
        .script(include_str!("fixtures/small_dialogs_2/warnings_two.jsonl"))
        .keymap(&[("list.move_right", "x")])
        .open();
    // The oracle pushes the viewer itself; the port opens it from the footer's `f2`.
    s.keys("f2");
    s.keys("x");
    s.assert_contains("2 of 2");
    s.assert_contains("Unknown setting `old_option`");
    // app.copy (ctrl+o) copies page 2: a CopyWarning event, not visible
    s.keys("ctrl+o");
    s.keys("left");
    s.assert_contains("1 of 2");
    s.assert_contains("MCP example could not connect");
    s.keys("ctrl+o");
    // Esc closes the viewer (the oracle asserts it is handled)
    s.keys("esc");
}

// ORACLE bottom_pane/warnings_view_tests.rs:76 warning_pages_wrap_and_scroll_long_diagnostics kind=snap owner=dialogs-pane snap=warnings_narrow
#[test]
fn warning_pages_wrap_and_scroll_long_diagnostics() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 9)
        .script(include_str!(
            "fixtures/small_dialogs_2/warnings_long_scroll.jsonl"
        ))
        .open();
    // The oracle pushes the viewer itself; the port opens it from the footer's `f2`.
    s.keys("f2");
    s.assert_snap("warnings_narrow");
    s.keys("pagedown");
    s.assert_absent("Long diagnostic");
    s.keys("end");
    s.assert_contains("END OF DIAGNOSTIC");
    s.keys("right");
    s.assert_contains("Unknown setting");
    s.keys("left");
    s.assert_contains("Long diagnostic");
}

// ORACLE bottom_pane/warnings_view_tests.rs:101 scroll_hint_tracks_wheel_and_keys_across_wrapped_rows kind=ui owner=dialogs-pane
#[test]
fn scroll_hint_tracks_wheel_and_keys_across_wrapped_rows() {
    kit::pending(
        Owner::DialogsPane,
        "the warnings viewer has no scroll hint: its footer holds keep, dismiss, copy and pages only (the oracle adds ` · ↑/z scroll` for the directions with rows left, by the configured list.move_up / list.move_down), its body scrolls by the toolkit's own keys and not by the list keymap, and the wheel is not limited to the body rows (needs the viewer's own offset and max offset: a ScrollController read by the footer)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(84, 10)
        .script(include_str!(
            "fixtures/small_dialogs_2/warnings_word_wrap.jsonl"
        ))
        .keymap(&[("list.move_down", "z")])
        .open();
    // rows 1 and height-1 are outside the scroll area: the wheel does nothing there
    s.wheel_down(2, 1, 1);
    s.wheel_down(2, 9, 1);
    s.assert_row_ends_with(9, " · z scroll");
    s.keys("z");
    s.assert_row_ends_with(9, " · ↑/z scroll");
    s.wheel_down(2, 3, 1);
    s.assert_row_ends_with(9, " · ↑ scroll");
    s.wheel_up(2, 3, 1);
    s.assert_row_ends_with(9, " · z scroll");
    s.resize(120, 10);
    s.assert_absent("scroll");
}

// ORACLE bottom_pane/warnings_view_tests.rs:133 warnings_dismiss_only_drawn_pages_after_navigation_skips_a_frame kind=ui owner=dialogs-pane
#[test]
fn warnings_dismiss_only_drawn_pages_after_navigation_skips_a_frame() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 12)
        .script(include_str!(
            "fixtures/small_dialogs_2/warnings_three.jsonl"
        ))
        .open();
    // The oracle pushes the viewer itself; the port opens it from the footer's `f2`.
    s.keys("f2");
    s.assert_contains("1 of 3");
    s.keys("right right");
    s.assert_contains("3 of 3");
    s.keys("left");
    // F2 closes the viewer: the UpdateWarnings event (dismissed set) is not visible, its
    // outcome is: the pages that were drawn left the retained list, every one here (the kit
    // draws between its keys; the oracle's skipped frame keeps the middle page).
    s.keys("f2");
    s.assert_absent("of 3");
    s.keys("f2");
    s.assert_absent("MCP example could not connect");
}

// ORACLE bottom_pane/warnings_view_tests.rs:171 warnings_keep_current_and_next_without_dismissing_unvisited_pages kind=ui owner=dialogs-pane
#[test]
fn warnings_keep_current_and_next_without_dismissing_unvisited_pages() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 12)
        .script(include_str!(
            "fixtures/small_dialogs_2/warnings_three.jsonl"
        ))
        .keymap(&[("list.move_up", "up"), ("list.cancel", "k")])
        .open();
    s.keys("f2");
    // Keep owns k in this viewer even when a list action is remapped to it
    s.keys("k");
    s.assert_contains("2 of 3");
    s.assert_contains("⌃c dismiss & close");
    // a held k (a repeat) changes nothing
    s.key_repeat("k");
    s.assert_contains("2 of 3");
    s.keys("left");
    s.keys("f2");
    // The UpdateWarnings event (dismissed and kept sets) is not visible, its outcome is: the
    // dismissed page left the retained list, the kept one and the page never shown stayed.
    s.keys("f2");
    s.assert_contains("1 of 2");
    s.assert_contains("MCP example could not connect");
    s.keys("right");
    s.assert_contains("Not yet viewed");
}

// ORACLE bottom_pane/warnings_view_tests.rs:207 warning_links_preserve_destinations_when_wrapped_and_scrolled kind=snap owner=dialogs-pane snap=warnings_wrapped_url
#[test]
fn warning_links_preserve_destinations_when_wrapped_and_scrolled() {
    kit::pending(
        Owner::DialogsPane,
        "no seed: the warnings viewer draws no OSC 8 destination on a wrapped URL yet",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(32, 9)
        .script(include_str!(
            "fixtures/small_dialogs_2/warnings_wrapped_url.jsonl"
        ))
        .open();
    s.assert_snap("warnings_wrapped_url");
    // every linked cell carries the URL as its destination
    s.assert_link(
        "https://github.com",
        "https://github.com/openai/codex/pull/12345?diff=split",
    );
    s.keys("down");
    s.resize(32, 5);
    s.keys("down");
    s.assert_link(
        "https://github.com",
        "https://github.com/openai/codex/pull/12345?diff=split",
    );
}
