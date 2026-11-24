//! Shard `small_dialogs_1` (screens S23, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: bottom_pane/app_link_view.rs (14 screen-level tests here; 9 state tests left to dialogs-pane)
// ORACLE bottom_pane/app_link_view.rs:1149 horizontal_list_keys_move_action_selection kind=ui owner=dialogs-pane
#[test]
fn horizontal_list_keys_move_action_selection() {
    // The oracle moves the action selection right and back (ctrl+l, ctrl+h); the
    // selection is drawn, so the screen is the first one again when it is back.
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("select", "app_link_installed")
        .open();
    let first = s.shot();
    s.keys("ctrl+l");
    s.assert_rows_differ(&first);
    s.keys("ctrl+h");
    s.assert_same_rows(&first);
}

// ORACLE bottom_pane/app_link_view.rs:1176 remapped_horizontal_list_keys_control_action_selection kind=ui owner=dialogs-pane
#[test]
fn remapped_horizontal_list_keys_control_action_selection() {
    let mut s = Screen::build(Owner::DialogsPane)
        .keymap(&[("list.move_left", "x"), ("list.move_right", "z")])
        .entry("select", "app_link_installed")
        .open();
    let first = s.shot();
    s.keys("l right");
    s.assert_same_rows(&first);
    s.keys("z");
    s.assert_rows_differ(&first);
    let second = s.shot();
    s.keys("h left");
    s.assert_same_rows(&second);
    s.keys("x");
    s.assert_same_rows(&first);
}

// ORACLE bottom_pane/app_link_view.rs:1217 toggle_action_sends_set_app_enabled_and_updates_label kind=ui owner=dialogs-pane
#[test]
fn toggle_action_sends_set_app_enabled_and_updates_label() {
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("select", "app_link_installed")
        .open();
    s.keys("2");
    s.assert_in_order(&["Manage on ChatGPT", "Enable app", "Back"]);
}

// ORACLE bottom_pane/app_link_view.rs:1254 generic_url_elicitation_resolves_without_connector_refresh kind=ui owner=dialogs-pane
#[test]
fn generic_url_elicitation_resolves_without_connector_refresh() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built for a generic URL elicitation: entry `app_link_url` missing (payments URL, codex_apps not used)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(72, 13)
        .entry("dialogs", "app_link_url")
        .open();
    s.keys("enter");
    s.assert_snap("app_link_view_generic_url_elicitation_confirmation");
    s.keys("enter");
    s.assert_absent("payments.example");
}

// ORACLE bottom_pane/app_link_view.rs:1306 install_confirmation_does_not_split_long_url_like_token_without_scheme kind=ui owner=dialogs-pane
#[test]
fn install_confirmation_does_not_split_long_url_like_token_without_scheme() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 16)
        .entry("select", "app_link_long_url")
        .open();
    s.keys("enter");
    // The pane is narrower than the token, so no row holds it whole; it is not split at a
    // hyphen or a slash, but runs on to the width and goes on in the next row.
    let url = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";
    s.assert_token_wraps_whole(url);
}

// ORACLE bottom_pane/app_link_view.rs:1350 install_confirmation_render_keeps_url_tail_visible_when_narrow kind=ui owner=dialogs-pane
#[test]
fn install_confirmation_render_keeps_url_tail_visible_when_narrow() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(36, 24)
        .entry("select", "app_link_long_url_tail")
        .open();
    s.keys("enter");
    s.assert_contains("tail42");
}

// ORACLE bottom_pane/app_link_view.rs:1403 install_tool_suggestion_resolves_elicitation_after_confirmation kind=ui owner=dialogs-pane
#[test]
fn install_tool_suggestion_resolves_elicitation_after_confirmation() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built for a Google Calendar install suggestion: entry `app_link_install` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "app_link_install")
        .open();
    s.keys("enter");
    s.keys("enter");
    s.assert_absent("Google Calendar");
}

// ORACLE bottom_pane/app_link_view.rs:1461 declined_tool_suggestion_resolves_elicitation_decline kind=ui owner=dialogs-pane
#[test]
fn declined_tool_suggestion_resolves_elicitation_decline() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built for a Google Calendar install suggestion: entry `app_link_install` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "app_link_install")
        .open();
    s.keys("2");
    s.assert_absent("Google Calendar");
}

// ORACLE bottom_pane/app_link_view.rs:1503 enable_tool_suggestion_resolves_elicitation_after_enable kind=ui owner=dialogs-pane
#[test]
fn enable_tool_suggestion_resolves_elicitation_after_enable() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built for a Google Calendar enable suggestion: entry `app_link_enable` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "app_link_enable")
        .open();
    s.keys("2");
    s.assert_absent("Google Calendar");
}

// ORACLE bottom_pane/app_link_view.rs:1611 install_suggestion_with_reason_snapshot kind=snap owner=dialogs-pane snap=app_link_view_install_suggestion_with_reason
#[test]
fn install_suggestion_with_reason_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built: entry `app_link_install` missing (Google Calendar install suggestion with its reason, 72 columns)",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 14)
        .entry("dialogs", "app_link_install")
        .open();
    s.assert_snap("app_link_view_install_suggestion_with_reason");
}

// ORACLE bottom_pane/app_link_view.rs:1640 enable_suggestion_with_reason_snapshot kind=snap owner=dialogs-pane snap=app_link_view_enable_suggestion_with_reason
#[test]
fn enable_suggestion_with_reason_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built: entry `app_link_enable` missing (Google Calendar enable suggestion with its reason, 72 columns)",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 16)
        .entry("dialogs", "app_link_enable")
        .open();
    s.assert_snap("app_link_view_enable_suggestion_with_reason");
}

// ORACLE bottom_pane/app_link_view.rs:1669 auth_suggestion_with_reason_snapshot kind=snap owner=dialogs-pane snap=app_link_view_auth_suggestion_with_reason
#[test]
fn auth_suggestion_with_reason_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built: entry `app_link_auth` missing (Google Calendar sign-in suggestion with its reason, 72 columns)",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 14)
        .entry("dialogs", "app_link_auth")
        .open();
    s.assert_snap("app_link_view_auth_suggestion_with_reason");
}

// ORACLE bottom_pane/app_link_view.rs:1699 generic_url_elicitation_snapshot kind=snap owner=dialogs-pane snap=app_link_view_generic_url_elicitation
#[test]
fn generic_url_elicitation_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built for a generic URL elicitation: entry `app_link_url` missing (72 columns)",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 15)
        .entry("dialogs", "app_link_url")
        .open();
    s.assert_snap("app_link_view_generic_url_elicitation");
}

// ORACLE bottom_pane/app_link_view.rs:1728 generic_url_elicitation_confirmation_snapshot kind=snap owner=dialogs-pane snap=app_link_view_generic_url_elicitation_confirmation
#[test]
fn generic_url_elicitation_confirmation_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "feature app-link dialog (w2-approvals, dialogs screen): the dialog is not built for a generic URL elicitation: entry `app_link_url` missing; the confirmation is reached by Enter (72 columns)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(72, 13)
        .entry("dialogs", "app_link_url")
        .open();
    s.keys("enter");
    s.assert_snap("app_link_view_generic_url_elicitation_confirmation");
}

// ---- oracle: bottom_pane/custom_prompt_view_tests.rs (13 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE bottom_pane/custom_prompt_view_tests.rs:9 picker_prompt_keeps_multiline_cursor_and_escape_hint_visible_after_resize kind=snap owner=dialogs-pane
#[test]
fn picker_prompt_keeps_multiline_cursor_and_escape_hint_visible_after_resize() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built (CustomPromptView, title Edit goal) with vim insert mode on: entry `custom_prompt_vim` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 16)
        .entry("dialogs", "custom_prompt_vim")
        .open();
    s.paste(&"Check the parser and preserve 日本語.\n".repeat(12));
    s.assert_snap("picker_prompt_multiline_80x16");
    s.resize(40, 8);
    s.assert_snap("picker_prompt_multiline_40x8");
    s.resize(80, 16);
    s.assert_snap("picker_prompt_multiline_80x16");
    s.resize(12, 3);
    s.resize(80, 16);
    s.keys("esc");
    s.assert_contains("esc normal mode");
    s.keys("esc");
    s.assert_absent("Check the parser");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:47 paste_burst_newline_does_not_submit_short_first_line kind=ui owner=dialogs-pane
#[test]
fn paste_burst_newline_does_not_submit_short_first_line() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built (CustomPromptView, title Edit goal): entry `custom_prompt` missing",
    );
    for (first, second) in [("x", "rest"), ("id", "body"), ("foo", "bar")] {
        let mut s = Screen::build(Owner::DialogsPane)
            .entry("dialogs", "custom_prompt")
            .open();
        s.type_text(first);
        s.keys("enter");
        s.type_text(second);
        s.assert_contains("Edit goal");
        s.assert_contains(second);
        s.keys("enter");
        s.assert_absent("Edit goal");
    }
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:79 paste_burst_newline_after_tab_does_not_submit kind=ui owner=dialogs-pane
#[test]
fn paste_burst_newline_after_tab_does_not_submit() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built (CustomPromptView, title Edit goal): entry `custom_prompt` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt")
        .open();
    s.type_text("x");
    s.keys("tab enter");
    s.type_text("rest");
    s.assert_contains("Edit goal");
    s.assert_contains("rest");
    s.keys("enter");
    s.assert_absent("Edit goal");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:105 delayed_enter_after_typing_submits kind=ui owner=dialogs-pane
#[test]
fn delayed_enter_after_typing_submits() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built (CustomPromptView, title Edit goal): entry `custom_prompt` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt")
        .open();
    s.type_text("foo");
    s.keys("enter");
    s.assert_absent("Edit goal");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:119 vim_insert_escape_clears_paste_burst_before_enter kind=ui owner=dialogs-pane
#[test]
fn vim_insert_escape_clears_paste_burst_before_enter() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built with vim insert mode on (title Edit goal): entry `custom_prompt_vim` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt_vim")
        .open();
    s.type_text("abc");
    s.keys("esc");
    s.keys("enter");
    s.assert_absent("Edit goal");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:135 vim_prompt_hint_tracks_escape_behavior kind=snap owner=dialogs-pane snap=vim_prompt_hint_tracks_escape_behavior
#[test]
fn vim_prompt_hint_tracks_escape_behavior() {
    kit::pending(
        Owner::DialogsPane,
        "no custom prompt entries `custom_prompt` and `custom_prompt_vim` (title Edit goal, plain and vim insert mode): the hint is the prompt's last row",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 5)
        .entry("dialogs", "custom_prompt")
        .open();
    s.assert_row(4, "  enter submit · esc back");
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 5)
        .entry("dialogs", "custom_prompt_vim")
        .open();
    s.assert_row(
        4,
        "  enter submit · esc normal mode                                 Vim: Insert",
    );
    s.assert_style(67, 4, "fg=Green");
    s.resize(60, 5);
    s.assert_row(
        4,
        "  enter submit · esc normal mode             Vim: Insert",
    );
    s.resize(14, 5);
    s.assert_row(4, "  enter subm");
    s.resize(80, 5);
    s.keys("esc");
    s.assert_row(
        4,
        "  enter submit · esc back                                        Vim: Normal",
    );
    s.assert_style(67, 4, "fg=Magenta");
    s.keys("R");
    s.assert_row(
        4,
        "  enter submit · esc normal mode                                Vim: Replace",
    );
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:170 vim_insert_cursor_tracks_mode_and_normal_mode_commands kind=ui owner=dialogs-pane
#[test]
fn vim_insert_cursor_tracks_mode_and_normal_mode_commands() {
    kit::pending(
        Owner::DialogsPane,
        "no custom prompt with the text rename (entries `custom_prompt_rename` and `custom_prompt_rename_vim`) and the editor keys remapped (editor.move_left f2, vim_normal.delete_char f3)",
    );
    // KIT GAP: the oracle asserts the cursor shape (SetCursorStyle) and a cursor
    // column with no literal; no kit call reads the cursor shape. The text steps
    // are the visible half.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 10)
        .keymap(&[("editor.move_left", "f2"), ("vim_normal.delete_char", "f3")])
        .entry("dialogs", "custom_prompt_rename")
        .open();
    s.keys("f2");
    s.assert_contains("rename");
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 10)
        .keymap(&[("editor.move_left", "f2"), ("vim_normal.delete_char", "f3")])
        .entry("dialogs", "custom_prompt_rename_vim")
        .open();
    s.keys("esc");
    s.keys("d enter");
    s.assert_contains("rename");
    s.keys("d esc");
    s.assert_contains("rename");
    s.keys("d i esc h f3");
    s.assert_contains("renae");
    s.keys("i m");
    s.assert_contains("rename");
    s.keys("esc R x");
    s.assert_contains("renaxe");
    s.keys("esc");
    s.keys("esc");
    s.assert_absent("Edit goal");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:253 escape_without_vim_cancels_prompt kind=ui owner=dialogs-pane
#[test]
fn escape_without_vim_cancels_prompt() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built (CustomPromptView, title Edit goal): entry `custom_prompt` missing",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt")
        .open();
    s.keys("esc");
    s.assert_absent("Edit goal");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:263 background_prefill_requires_a_matching_unedited_prompt kind=ui owner=dialogs-pane
#[test]
fn background_prefill_requires_a_matching_unedited_prompt() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built with a background text suggestion (loading Loading, context Ready) applied: entry `custom_prompt_prefill` missing",
    );
    // The stale request is ignored (nothing drawn); the matching one fills the
    // text and sets the context label, and Enter submits it.
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt_prefill")
        .open();
    s.assert_contains("Suggested text");
    s.assert_contains("Ready");
    s.keys("enter");
    s.assert_absent("Edit goal");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:293 background_prefill_preserves_typing_even_after_the_text_is_deleted kind=ui owner=dialogs-pane
#[test]
fn background_prefill_preserves_typing_even_after_the_text_is_deleted() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built where a character was typed and deleted before the suggestion (Generated) is applied: entry `custom_prompt_prefill_typed` missing",
    );
    let s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt_prefill_typed")
        .open();
    s.assert_contains("Edit goal");
    s.assert_absent("Generated");
    s.assert_absent("Ready");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:309 background_prefill_preserves_pasted_text kind=ui owner=dialogs-pane
#[test]
fn background_prefill_preserves_pasted_text() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built where text was pasted before the suggestion (Generated) is applied: entry `custom_prompt_prefill_pasted` missing",
    );
    let s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt_prefill_pasted")
        .open();
    s.assert_contains("Manual title");
    s.assert_absent("Generated");
    s.assert_absent("Ready");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:323 background_prefill_survives_cursor_movement kind=ui owner=dialogs-pane
#[test]
fn background_prefill_survives_cursor_movement() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built with the text Existing, cursor moved left, then the suggestion Generated applied: entry `custom_prompt_prefill_cursor` missing",
    );
    let s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt_prefill_cursor")
        .open();
    s.assert_contains("Generated");
    s.assert_contains("Ready");
    s.assert_absent("Existing");
}

// ORACLE bottom_pane/custom_prompt_view_tests.rs:340 background_prefill_preserves_replace_backspace_recovery kind=ui owner=dialogs-pane
#[test]
fn background_prefill_preserves_replace_backspace_recovery() {
    kit::pending(
        Owner::DialogsPane,
        "feature custom prompt view (w2-approvals, dialogs screen): the view is not built in vim replace mode with the suggestion abc applied (then Left, X, Backspace): entry `custom_prompt_prefill_replace` missing",
    );
    let s = Screen::build(Owner::DialogsPane)
        .entry("dialogs", "custom_prompt_prefill_replace")
        .open();
    s.assert_contains("abc");
    s.assert_contains("Replace");
}

// ---- oracle: bottom_pane/experimental_features_view_tests.rs (3 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE bottom_pane/experimental_features_view_tests.rs:5 experimental_features_keep_following_choice_at_wrap_boundary kind=snap owner=dialogs-pane snap=experimental_features_keep_following_choice_at_wrap_boundary
#[test]
fn experimental_features_keep_following_choice_at_wrap_boundary() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(72, 11)
        .entry("settings", "experimental_features_two")
        .open();
    // The oracle renders into `desired_height(width)` rows: the screen is the
    // sheet alone (ours sits at the bottom edge over the debug menu).
    for (width, height) in [(68, 13), (69, 13), (70, 12), (71, 11), (72, 11)] {
        s.resize(width, height);
        s.assert_snap_section(
            "experimental_features_keep_following_choice_at_wrap_boundary",
            &format!("width={width}"),
        );
    }
}

// ORACLE bottom_pane/experimental_features_view_tests.rs:90 experimental_features_discovery_preserves_server_order_and_accepts_new_keys kind=snap owner=dialogs-pane snap=experimental_features_save_failed snap=experimental_features_saving snap=experimental_features_server_discovery
#[test]
fn experimental_features_discovery_preserves_server_order_and_accepts_new_keys() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 12)
        .entry("settings", "experimental_features_discovery")
        .open();
    s.assert_snap("experimental_features_server_discovery");
    // Space toggles the selected row: the second off, the first off, the third off.
    // The oracle flips the third row directly and leaves its cursor on the
    // first; the keys go there and come back.
    s.keys("down space up space down down space up up");
    s.resize(80, 13);
    s.keys("enter");
    s.assert_snap("experimental_features_saving");
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 13)
        .entry("settings", "experimental_features_save_failed")
        .open();
    s.assert_snap("experimental_features_save_failed");
}

// ORACLE bottom_pane/experimental_features_view_tests.rs:206 experimental_features_empty_error_and_cancel_remain_usable kind=snap owner=dialogs-pane snap=experimental_features_changed_default snap=experimental_features_empty snap=experimental_features_unavailable
#[test]
fn experimental_features_empty_error_and_cancel_remain_usable() {
    for (entry, rows) in [
        ("experimental_features_empty", 11),
        ("experimental_features_unavailable", 12),
        ("experimental_features_changed_default", 10),
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, rows)
            .entry("settings", entry)
            .open();
        s.assert_snap(entry);
        s.keys("ctrl+c");
        s.assert_absent("Experimental features");
    }
}

// ---- oracle: bottom_pane/feedback_note_view_tests.rs (12 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/feedback_note_view_tests.rs:52 feedback_category_titles kind=ui owner=dialogs-pane
#[test]
fn feedback_category_titles() {
    // The digits follow the oracle's feedback_selection_params order: Bug 1, Bad result 2,
    // Good result 3, Safety check 4, Other 5.
    for (digit, title) in [
        ("1", "▌ Tell us more (bug)"),
        ("2", "▌ Tell us more (bad result)"),
        ("3", "▌ Tell us more (good result)"),
        ("5", "▌ Tell us more (other)"),
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(60, 24)
            .entry("feedback", "feedback_categories")
            .open();
        // The category, then `Yes` to the logs: the note step follows.
        s.keys(digit);
        s.keys("1");
        s.assert_has_row(title);
    }
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:65 feedback_view_bug kind=snap owner=dialogs-pane snap=feedback_view_bug
#[test]
fn feedback_view_bug() {
    let s = Screen::build(Owner::DialogsPane)
        .size(60, 10)
        .entry("feedback", "feedback_note_bug")
        .open();
    s.assert_snap("feedback_view_bug");
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:72 feedback_view_safety_check kind=snap owner=dialogs-pane snap=feedback_view_safety_check
#[test]
fn feedback_view_safety_check() {
    let s = Screen::build(Owner::DialogsPane)
        .size(60, 10)
        .entry("feedback", "feedback_note_safety_check")
        .open();
    s.assert_snap("feedback_view_safety_check");
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:79 feedback_disclosure_matches_audience_at_normal_and_narrow_widths kind=snap owner=dialogs-pane snap=feedback_disclosures
#[test]
fn feedback_disclosure_matches_audience_at_normal_and_narrow_widths() {
    // KIT GAP: the snapshot joins its states under `external (80 columns)`-style
    // headers, which oracle_snap::header does not take as frames.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 16)
        .entry("feedback", "feedback_note_bug_external")
        .open();
    s.assert_link("Learnmore", "https://help.openai.com/en/articles/5722486-how-your-data-is-used-to-improve-model-performance");
    kit::pending(
        Owner::DialogsPane,
        "oracle_snap::header takes no `external (80 columns)`-style frame headers (KIT GAP)",
    );
    s.assert_snap_section("feedback_disclosures", "external (80 columns)");
    s.resize(40, 16);
    s.assert_link("Learnmore", "https://help.openai.com/en/articles/5722486-how-your-data-is-used-to-improve-model-performance");
    s.assert_snap_section("feedback_disclosures", "external (40 columns)");
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 16)
        .entry("feedback", "feedback_note_bug_employee")
        .open();
    s.assert_link("here", "https://go/codex-employee-feedback-faq");
    s.assert_snap_section("feedback_disclosures", "employee (80 columns)");
    s.resize(40, 16);
    s.assert_link("here", "https://go/codex-employee-feedback-faq");
    s.assert_snap_section("feedback_disclosures", "employee (40 columns)");
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 16)
        .entry("feedback", "feedback_note_bug_remote")
        .open();
    s.assert_snap_section("feedback_disclosures", "remote server (80 columns)");
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:130 feedback_disclosure_scrolls_without_hiding_input_in_short_panes kind=snap owner=dialogs-pane snap=feedback_disclosure_short_panes
#[test]
fn feedback_disclosure_scrolls_without_hiding_input_in_short_panes() {
    // The snapshot's states are headed `40x12 initial`-style, which
    // oracle_snap::header does not take as frames (KIT GAP): the rest is checked by hand.
    for (entry, width, height) in [
        ("feedback_note_employee_40x12", 40, 12),
        ("feedback_note_employee_80x8", 80, 8),
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(width, height)
            .entry("feedback", entry)
            .open();
        s.paste("A visible note.");
        s.assert_contains("A visible note.");
        s.assert_contains("enter send");
        s.keys("pagedown");
        s.assert_contains("sensitive personal");
        s.keys("pageup");
        s.assert_contains("A visible note.");
    }
    kit::pending(
        Owner::DialogsPane,
        "oracle_snap::header takes no `40x12 initial`-style frame headers (KIT GAP)",
    );
    for (entry, width, height) in [
        ("feedback_note_employee_40x12", 40, 12),
        ("feedback_note_employee_80x8", 80, 8),
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(width, height)
            .entry("feedback", entry)
            .open();
        s.paste("A visible note.");
        s.assert_snap_section(
            "feedback_disclosure_short_panes",
            &format!("{width}x{height} initial"),
        );
        s.keys("pagedown");
        s.assert_snap_section(
            "feedback_disclosure_short_panes",
            &format!("{width}x{height} scrolled"),
        );
        s.keys("pageup");
        s.assert_snap_section(
            "feedback_disclosure_short_panes",
            &format!("{width}x{height} restored"),
        );
    }
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:165 feedback_disclosure_keeps_editor_visible_in_four_rows kind=snap owner=dialogs-pane snap=feedback_disclosure_four_rows
#[test]
fn feedback_disclosure_keeps_editor_visible_in_four_rows() {
    // The snapshot joins the initial and the scrolled state by a blank row with no header: the
    // four rows of each are compared by hand.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 4)
        .entry("feedback", "feedback_note_employee_80x4")
        .open();
    s.paste("A visible note.");
    s.assert_contains("Feedback sent to OpenAI may be used to improve our models and products");
    s.assert_snap_state_rows("feedback_disclosure_four_rows", 0, 4);
    // a page down per line of the disclosure (the oracle's `intro_lines`), the sensitive-information
    // notice passing by on the way
    s.assert_passes_by("pagedown", 40, "personal information.");
    s.assert_contains("which is not sent to OpenAI");
    s.assert_snap_state_rows("feedback_disclosure_four_rows", 5, 4);
    for entry in [
        "feedback_note_safety_check_employee_24x4",
        "feedback_note_safety_check_external_24x4",
    ] {
        let s = Screen::build(Owner::DialogsPane)
            .size(24, 4)
            .entry("feedback", entry)
            .open();
        s.assert_contains("Feedback sent to");
        s.assert_contains("save · enter send");
    }
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:214 feedback_paste_burst_preserves_multiline_note_until_submit kind=snap owner=dialogs-pane snap=feedback_multiline_paste
#[test]
fn feedback_paste_burst_preserves_multiline_note_until_submit() {
    kit::pending(
        Owner::DialogsPane,
        "TextArea has no paste-burst: a fast Enter after typed text must insert a newline (NEED: widget)",
    );
    for entry in ["feedback_note_bug_no_logs", "feedback_note_bug"] {
        for note in ["x\nrest", "id\n\nbody", "foo\nbar", "x\t\nrest"] {
            let mut s = Screen::build(Owner::DialogsPane)
                .size(60, 11)
                .entry("feedback", entry)
                .open();
            for ch in note.chars() {
                match ch {
                    '\n' => s.keys("enter"),
                    '\t' => s.keys("tab"),
                    other => s.type_text(&other.to_string()),
                };
            }
            if entry == "feedback_note_bug_no_logs" && note == "foo\nbar" {
                s.assert_snap("feedback_multiline_paste");
            }
            s.assert_contains(note.rsplit('\n').next().unwrap_or(""));
            s.keys("enter");
            s.assert_absent("Tell us more");
        }
    }
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:277 feedback_bracketed_paste_does_not_reach_chat_composer kind=ui owner=dialogs-pane
#[test]
fn feedback_bracketed_paste_does_not_reach_chat_composer() {
    // The flow runs over the live chat composer: `/feedback`, the category (1 is the bug), the
    // consent (1 with the logs, 2 without), then the note.
    for logs in ["1", "2"] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        s.type_text("/feedback");
        s.keys("enter");
        s.keys("1");
        s.keys(logs);
        s.type_text("x");
        s.paste("\nrest");
        s.assert_contains("rest");
        s.keys("enter");
        s.assert_absent("rest");
    }
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:316 remote_save_shortcut_keeps_the_note_for_explicit_upload kind=ui owner=dialogs-pane
#[test]
fn remote_save_shortcut_keeps_the_note_for_explicit_upload() {
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("feedback", "feedback_note_remote_save")
        .open();
    s.keys("ctrl+s");
    s.assert_contains("something broke");
    s.keys("enter");
    s.assert_absent("something broke");
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:349 submit_feedback_omits_empty_note kind=ui owner=dialogs-pane
#[test]
fn submit_feedback_omits_empty_note() {
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("feedback", "feedback_note_good_result")
        .open();
    s.keys("enter");
    s.assert_absent("Tell us more");
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:377 save_shortcut_keeps_note_and_consent_with_or_without_uploads kind=snap owner=dialogs-pane snap=feedback_note_uploads_disabled
#[test]
fn save_shortcut_keeps_note_and_consent_with_or_without_uploads() {
    for entry in [
        "feedback_note_bug_turn_no_uploads",
        "feedback_note_bug_turn_external",
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(60, 9)
            .entry("feedback", entry)
            .open();
        s.paste("Review before sharing");
        if entry == "feedback_note_bug_turn_no_uploads" {
            s.assert_snap("feedback_note_uploads_disabled");
            s.keys("enter");
            s.assert_contains("Review before sharing");
        }
        s.keys("ctrl+s");
        s.assert_absent("Review before sharing");
    }
}

// ORACLE bottom_pane/feedback_note_view_tests.rs:411 feedback_note_cancel_does_not_submit kind=ui owner=dialogs-pane
#[test]
fn feedback_note_cancel_does_not_submit() {
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("feedback", "feedback_note_unfinished")
        .open();
    s.assert_contains("unfinished feedback");
    s.keys("esc");
    s.assert_absent("unfinished feedback");
}

// ---- oracle: bottom_pane/feedback_view.rs (6 screen-level tests here; 2 state tests left to dialogs-pane)
// ORACLE bottom_pane/feedback_view.rs:390 feedback_upload_consent_lists_doctor_report kind=snap owner=dialogs-pane snap=feedback_upload_consent_lists_doctor_report
#[test]
fn feedback_upload_consent_lists_doctor_report() {
    let s = Screen::build(Owner::DialogsPane)
        .size(60, 24)
        .entry("feedback", "feedback_consent")
        .open();
    let (row, col) = locate(&s, "Include logs and session history?");
    s.assert_snap_region("feedback_upload_consent_lists_doctor_report", row, col);
}

// ORACLE bottom_pane/feedback_view.rs:409 feedback_upload_consent_lists_windows_sandbox_log_when_included kind=snap owner=dialogs-pane snap=feedback_upload_consent_lists_windows_sandbox_log_when_included
#[test]
fn feedback_upload_consent_lists_windows_sandbox_log_when_included() {
    let s = Screen::build(Owner::DialogsPane)
        .size(60, 24)
        .entry("feedback", "feedback_consent_windows")
        .open();
    let (row, col) = locate(&s, "Include logs and session history?");
    s.assert_snap_region(
        "feedback_upload_consent_lists_windows_sandbox_log_when_included",
        row,
        col,
    );
}

// ORACLE bottom_pane/feedback_view.rs:501 feedback_success_cell_matches_external_bug_copy kind=ui owner=dialogs-pane
#[test]
fn feedback_success_cell_matches_external_bug_copy() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 12)
        .entry("feedback", "feedback_success_external_bug")
        .open();
    let (row, _) = locate(
        &s,
        "• Feedback uploaded. Please open an issue using the following URL:",
    );
    s.assert_row(
        row,
        "• Feedback uploaded. Please open an issue using the following URL:",
    );
    s.assert_row(row + 1, "");
    s.assert_row(row + 2, "  https://github.com/openai/codex/issues/new?template=3-cli.yml&steps=Uploaded%20thread:%20thread-1");
    s.assert_row(row + 3, "");
    s.assert_row(
        row + 4,
        "  Or mention your thread ID thread-1 in an existing issue.",
    );
}

// ORACLE bottom_pane/feedback_view.rs:518 feedback_success_cell_matches_employee_bug_copy kind=snap owner=dialogs-pane snap=feedback_success_employee
#[test]
fn feedback_success_cell_matches_employee_bug_copy() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 12)
        .entry("feedback", "feedback_success_employee_bug")
        .open();
    let (row, col) = locate(
        &s,
        "• Feedback uploaded. You can share this in #codex-feedback:",
    );
    s.assert_snap_region("feedback_success_employee", row, col);
}

// ORACLE bottom_pane/feedback_view.rs:532 feedback_success_cell_matches_good_result_copy kind=ui owner=dialogs-pane
#[test]
fn feedback_success_cell_matches_good_result_copy() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 12)
        .entry("feedback", "feedback_success_good_result")
        .open();
    let (row, _) = locate(
        &s,
        "• Feedback recorded (no logs). Thanks for the feedback!",
    );
    s.assert_row(
        row,
        "• Feedback recorded (no logs). Thanks for the feedback!",
    );
    s.assert_row(row + 1, "");
    s.assert_row(row + 2, "  Thread ID: thread-3");
}

// ORACLE bottom_pane/feedback_view.rs:549 feedback_success_cell_uses_issue_links_for_remaining_categories kind=ui owner=dialogs-pane
#[test]
fn feedback_success_cell_uses_issue_links_for_remaining_categories() {
    for entry in [
        "feedback_success_bad_result",
        "feedback_success_safety_check",
        "feedback_success_other",
    ] {
        let s = Screen::build(Owner::DialogsPane)
            .size(120, 12)
            .entry("feedback", entry)
            .open();
        s.assert_contains("Please open an issue using the following URL:");
        s.assert_contains("thread-4");
    }
}

// ---- oracle: bottom_pane/hooks_browser_render_tests.rs (2 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/hooks_browser_render_tests.rs:24 wide_titles_preserve_reverse_video_selection_and_dim_index kind=ui owner=dialogs-pane
#[test]
fn wide_titles_preserve_reverse_video_selection_and_dim_index() {
    // The oracle draws one row of the list; the port shows the row in the
    // hooks page of the view (the entry with the wide title, 40 columns).
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 22)
        .entry("settings", "hooks_status_messages")
        .open();
    s.keys("enter");
    let (row, _) = locate(&s, "› [x]");
    s.assert_style(7, row, "dim");
    s.assert_style(10, row, "!dim");
    s.assert_text_at(10, row, "检");
    // The oracle's selection with no palette is reverse video over the whole
    // row; the port's selection is the theme's accent and BOLD on every
    // dialog (the same as the pickers' rows), a decision of the port.
    kit::pending(
        Owner::DialogsPane,
        "the selected row is the theme's accent and bold in every dialog of the port, not the oracle's reverse video of an unprobed palette (the shared selection style, not this dialog)",
    );
    for x in 0..40 {
        s.assert_style(x, row, "fg=Reset bg=Reset reversed");
    }
}

// ORACLE bottom_pane/hooks_browser_render_tests.rs:57 wrapped_hook_command_preserves_the_complete_url_destination kind=snap owner=dialogs-pane snap=hooks_browser_wrapped_url
#[test]
fn wrapped_hook_command_preserves_the_complete_url_destination() {
    kit::pending(
        Owner::DialogsPane,
        "G49 + G72: rtwk Text does not break after a slash (the oracle breaks `https://github.com/` / `openai/codex/pull/12345?` / `diff=split&review=complete-`) and the URL is plain text, so no cell carries its OSC 8 link",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(44, 19)
        .entry("settings", "hooks_wrapped_url")
        .open();
    s.keys("enter");
    s.assert_link(
        "https://github.com/openai/codex/pull/12345?diff=split&review=complete-",
        "https://github.com/openai/codex/pull/12345?diff=split&review=complete-destination",
    );
    s.assert_contains("…");
    s.assert_snap("hooks_browser_wrapped_url");
}

/// The row and the column (in chars) where `text` starts on the screen: a
/// locator for a region of the screen; the text itself is the oracle's.
fn locate(s: &Screen, text: &str) -> (usize, usize) {
    for (y, row) in s.rows().iter().enumerate() {
        if let Some(byte) = row.find(text) {
            return (y, row[..byte].chars().count());
        }
    }
    panic!("no `{text}` on the screen:\n{}", s.text())
}
