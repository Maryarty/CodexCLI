//! Shard `chat_status_3` (screens S10, owner ui-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// A bottom popup of the live chat at 100 columns. The oracle renders the popup
/// alone, so its rows are the last rows of our screen (the bottom pane).
fn bottom_popup(command: &str, snap: &str) -> Screen {
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    s.type_text(command);
    s.keys("enter");
    let (_, rows) = kit::snap_size(snap);
    s.assert_snap_region(snap, 24usize.saturating_sub(rows), 0);
    s
}

// ---- oracle: chatwidget/tests/status_surface_previews.rs (16 screen-level tests here; 6 state tests left to ui-pane)
// ORACLE chatwidget/tests/status_surface_previews.rs:16 thread_color_preview_matches_footer kind=snap owner=ui-pane
#[test]
fn thread_color_preview_matches_footer() {
    kit::na(
        Owner::UiPane,
        "asserts that a status-line preview Line equals the footer's, and a Debug print of it: helper values, not rows; a state test",
    );
}

// ORACLE chatwidget/tests/status_surface_previews.rs:138 status_surface_preview_lines_live_only_snapshot kind=snap owner=ui-pane snap=status_surface_previews_live_only
#[test]
fn status_surface_preview_lines_live_only_snapshot() {
    kit::na(
        Owner::UiPane,
        "asserts the combined status-line and terminal-title preview text (a helper's return), not screen rows; a state test",
    );
}

// ORACLE chatwidget/tests/status_surface_previews.rs:164 status_line_setup_popup_live_only_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_live_only
#[test]
fn status_line_setup_popup_live_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /statusline popup in the ported chat; the live seed (status-line items, git branch, thread name, project root) has no entry or script",
    );
    bottom_popup("/statusline", "status_line_setup_popup_live_only");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:183 status_surface_preview_lines_hardcoded_only_snapshot kind=snap owner=ui-pane snap=status_surface_previews_hardcoded_only
#[test]
fn status_surface_preview_lines_hardcoded_only_snapshot() {
    kit::na(
        Owner::UiPane,
        "asserts the combined status-line and terminal-title preview text (a helper's return), not screen rows; a state test",
    );
}

// ORACLE chatwidget/tests/status_surface_previews.rs:222 status_line_setup_popup_hardcoded_only_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_hardcoded_only
#[test]
fn status_line_setup_popup_hardcoded_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /statusline popup in the ported chat; the status-line items (profile, provider) have no seed",
    );
    bottom_popup("/statusline", "status_line_setup_popup_hardcoded_only");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:239 status_line_setup_popup_workspace_headline_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_workspace_headline
#[test]
fn status_line_setup_popup_workspace_headline_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /statusline popup in the ported chat; the workspace headline has no seed",
    );
    bottom_popup("/statusline", "status_line_setup_popup_workspace_headline");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:251 status_surface_preview_lines_mixed_snapshot kind=snap owner=ui-pane snap=status_surface_previews_mixed
#[test]
fn status_surface_preview_lines_mixed_snapshot() {
    kit::na(
        Owner::UiPane,
        "asserts the combined status-line and terminal-title preview text (a helper's return), not screen rows; a state test",
    );
}

// ORACLE chatwidget/tests/status_surface_previews.rs:274 status_surface_preview_lines_rate_limits_snapshot kind=snap owner=ui-pane snap=status_surface_previews_rate_limits
#[test]
fn status_surface_preview_lines_rate_limits_snapshot() {
    kit::na(
        Owner::UiPane,
        "asserts the combined status-line and terminal-title preview text (a helper's return), not screen rows; a state test",
    );
}

// ORACLE chatwidget/tests/status_surface_previews.rs:291 status_surface_preview_lines_thread_usage_snapshot kind=snap owner=ui-pane snap=status_surface_previews_thread_usage
#[test]
fn status_surface_preview_lines_thread_usage_snapshot() {
    kit::na(
        Owner::UiPane,
        "asserts the combined status-line and terminal-title preview text (a helper's return), not screen rows; a state test",
    );
}

// ORACLE chatwidget/tests/status_surface_previews.rs:438 status_line_setup_popup_rate_limits_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_rate_limits
#[test]
fn status_line_setup_popup_rate_limits_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /statusline popup in the ported chat; the rate-limit snapshot (35% and 50% used) has no seed",
    );
    bottom_popup("/statusline", "status_line_setup_popup_rate_limits");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:453 status_line_setup_popup_mixed_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_mixed
#[test]
fn status_line_setup_popup_mixed_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /statusline popup in the ported chat; the account, the daybreak feature and the status-line items have no seed",
    );
    bottom_popup("/statusline", "status_line_setup_popup_mixed");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:474 terminal_title_setup_popup_live_only_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_live_only
#[test]
fn terminal_title_setup_popup_live_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /title popup in the ported chat; the terminal-title items and the live branch and plan progress have no seed",
    );
    bottom_popup("/title", "terminal_title_setup_popup_live_only");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:494 terminal_title_setup_popup_hardcoded_only_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_hardcoded_only
#[test]
fn terminal_title_setup_popup_hardcoded_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /title popup in the ported chat; the terminal-title items have no seed",
    );
    bottom_popup("/title", "terminal_title_setup_popup_hardcoded_only");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:509 terminal_title_setup_popup_mixed_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_mixed
#[test]
fn terminal_title_setup_popup_mixed_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /title popup in the ported chat; the account, the daybreak feature and the terminal-title items have no seed",
    );
    bottom_popup("/title", "terminal_title_setup_popup_mixed");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:529 terminal_title_setup_popup_rate_limits_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_rate_limits
#[test]
fn terminal_title_setup_popup_rate_limits_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /title popup in the ported chat; the rate-limit snapshot has no seed",
    );
    bottom_popup("/title", "terminal_title_setup_popup_rate_limits");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:544 terminal_title_setup_popup_thread_usage_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_thread_usage
#[test]
fn terminal_title_setup_popup_thread_usage_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /title popup in the ported chat; the thread-credits and estimated-cost items have no seed",
    );
    bottom_popup("/title", "terminal_title_setup_popup_thread_usage");
}

// ---- oracle: chatwidget/tests/subagent_activity_tests.rs (2 screen-level tests here; 1 state tests left to ui-pane)
// ORACLE chatwidget/tests/subagent_activity_tests.rs:81 subagent_completion_preserves_stream_and_authoritative_answer kind=snap owner=ui-pane
#[test]
fn subagent_completion_preserves_stream_and_authoritative_answer() {
    kit::pending(
        Owner::UiPane,
        "no subagent activity in the live chat: the turn deltas, the subagent completion and the authoritative answer have no script yet",
    );
    // The oracle loops over three orders of deltas and activity (answer before activity;
    // activity between the deltas; a dropped final delta). Each ends with the same finalized
    // transcript rows, its inline snapshot; the events and the stream and interrupt fields are state.
    let s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.assert_contains("• The top is PR #42.");
    s.assert_contains("  Branch: feature/rollout.");
    s.assert_contains("• Completed `/root/stack_tip`");
}

// ORACLE chatwidget/tests/subagent_activity_tests.rs:136 interrupted_answer_flushes_queued_subagent_activity kind=ui owner=ui-pane
#[test]
fn interrupted_answer_flushes_queued_subagent_activity() {
    kit::pending(
        Owner::UiPane,
        "no subagent activity and no turn interrupt in the live chat: the seed has no script yet",
    );
    // The visible part: the interruption notice after the answer and the subagent activity.
    // The event order and the stream and interrupt fields are state, not rows.
    let s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.assert_contains("■ Conversation interrupted");
}

// ---- oracle: chatwidget/tests/terminal_program_status.rs (2 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE chatwidget/tests/terminal_program_status.rs:11 session_status_follows_existing_lifecycle_state kind=ui owner=ui-pane
#[test]
fn session_status_follows_existing_lifecycle_state() {
    kit::pending(
        Owner::UiPane,
        "no turn, exec-approval and background-terminal seed in the live chat; the program-status sequence is not on screen",
    );
    // The oracle's sequence of program states (Idle, Working, Blocked, Working, Done, Idle)
    // is state; the visible part is the background-terminal line in the bottom popup.
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.keys("y");
    s.assert_contains("1 background terminal running");
}

// ORACLE chatwidget/tests/terminal_program_status.rs:63 session_status_detail_uses_only_foreground_activity kind=snap owner=ui-pane
#[test]
fn session_status_detail_uses_only_foreground_activity() {
    kit::na(
        Owner::UiPane,
        "asserts the iTerm session detail of each program status (a helper's return, inline), not screen rows; a state test",
    );
}

// ---- oracle: chatwidget/tests/terminal_title.rs (4 screen-level tests here; 3 state tests left to ui-pane)
// ORACLE chatwidget/tests/terminal_title.rs:8 daybreak_status_surfaces_follow_the_thread_preference kind=snap owner=ui-pane
#[test]
fn daybreak_status_surfaces_follow_the_thread_preference() {
    kit::na(
        Owner::UiPane,
        "asserts the Daybreak item value and the last terminal title (helper values, inline), not screen rows; a state test",
    );
}

// ORACLE chatwidget/tests/terminal_title.rs:38 goal_clock_refresh_redraws_only_when_elapsed_label_changes kind=snap owner=ui-pane
#[test]
fn goal_clock_refresh_redraws_only_when_elapsed_label_changes() {
    kit::na(
        Owner::UiPane,
        "asserts redraw requests and refresh deadlines, and the goal-label helper's spans (inline), not screen rows; a state test",
    );
}

// ORACLE chatwidget/tests/terminal_title.rs:96 terminal_title_shows_action_required_while_exec_approval_is_pending kind=ui owner=ui-pane
#[test]
fn terminal_title_shows_action_required_while_exec_approval_is_pending() {
    kit::na(
        Owner::UiPane,
        "asserts the terminal window title (OSC, not a screen row), its spinner and blink deadlines and the draw requests; a state test",
    );
}

// ORACLE chatwidget/tests/terminal_title.rs:286 thread_identity_renders_names_and_fallbacks_on_both_surfaces kind=snap owner=ui-pane
#[test]
fn thread_identity_renders_names_and_fallbacks_on_both_surfaces() {
    kit::na(
        Owner::UiPane,
        "asserts status-line and terminal-title item values (helper returns, inline), not screen rows; a state test",
    );
}

// ---- oracle: chatwidget/tests/tool_activity_tests.rs (1 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE chatwidget/tests/tool_activity_tests.rs:4 image_result_live_and_replay_render_in_the_owning_call kind=snap owner=ui-pane
#[test]
fn image_result_live_and_replay_render_in_the_owning_call() {
    kit::pending(
        Owner::UiPane,
        "the chat has no MCP tool call with an image result, live or replayed: no seed script for either path",
    );
    // The oracle renders the one call's cell live and by replay; both equal the same snapshot.
    // The cell's row on the transcript is set by ui-pane with the seed.
    let live = Screen::build(Owner::UiPane).size(80, 24).live().open();
    live.assert_snap_region(
        "image_result_live_and_replay_render_in_the_owning_call",
        0,
        0,
    );
    let replay = Screen::build(Owner::UiPane).size(80, 24).live().open();
    replay.assert_snap_region(
        "image_result_live_and_replay_render_in_the_owning_call",
        0,
        0,
    );
}

// ---- oracle: status_indicator_widget.rs (6 screen-level tests here; 4 state tests left to ui-pane)
// ORACLE status_indicator_widget.rs:367 renders_with_working_header kind=snap owner=ui-pane snap=renders_with_working_header
#[test]
fn renders_with_working_header() {
    kit::pending(
        Owner::UiPane,
        "the status-indicator row (working header, timer, interrupt hint) is not on the ported chat screen, and no 80x2 screen shows it alone",
    );
    Screen::build(Owner::UiPane)
        .size(80, 2)
        .live()
        .open()
        .assert_snap("renders_with_working_header");
}

// ORACLE status_indicator_widget.rs:387 renders_truncated kind=snap owner=ui-pane snap=renders_truncated
#[test]
fn renders_truncated() {
    kit::pending(
        Owner::UiPane,
        "the status-indicator row is not on the ported chat screen, and no 20x2 screen shows it alone",
    );
    Screen::build(Owner::UiPane)
        .size(20, 2)
        .live()
        .open()
        .assert_snap("renders_truncated");
}

// ORACLE status_indicator_widget.rs:407 renders_wrapped_details_panama_two_lines kind=snap owner=ui-pane snap=renders_wrapped_details_panama_two_lines
#[test]
fn renders_wrapped_details_panama_two_lines() {
    kit::pending(
        Owner::UiPane,
        "the status-indicator row is not on the ported chat screen; no seed sets the details (the panama phrase, capitalized first), hides the interrupt hint, pauses the timer",
    );
    Screen::build(Owner::UiPane)
        .size(30, 3)
        .live()
        .open()
        .assert_snap("renders_wrapped_details_panama_two_lines");
}

// ORACLE status_indicator_widget.rs:437 renders_without_spinner_when_animations_disabled kind=ui owner=ui-pane
#[test]
fn renders_without_spinner_when_animations_disabled() {
    kit::missing(
        Owner::UiPane,
        "KIT GAP: the oracle asserts that row 0 starts with `Working (0s • esc to interrupt)` at 80x1 with animations off; the kit has no row-prefix assert (assert_row needs the whole row, assert_contains is not a prefix). Also the status row is not on the ported chat screen.",
    );
}

// ORACLE status_indicator_widget.rs:462 renders_remapped_interrupt_hint kind=snap owner=ui-pane snap=renders_remapped_interrupt_hint
#[test]
fn renders_remapped_interrupt_hint() {
    kit::pending(
        Owner::UiPane,
        "the status-indicator row is not on the ported chat screen; no seed remaps the interrupt key to F12",
    );
    Screen::build(Owner::UiPane)
        .size(80, 1)
        .live()
        .open()
        .assert_snap("renders_remapped_interrupt_hint");
}

// ORACLE status_indicator_widget.rs:483 hook_status_reflows_without_displacing_controls_or_details kind=snap owner=ui-pane snap=hook_status_reflows_with_background_activity snap=hook_status_reflows_without_background_activity
#[test]
fn hook_status_reflows_without_displacing_controls_or_details() {
    kit::missing(
        Owner::UiPane,
        "KIT GAP: each snapshot holds four `N columns:` frames (the fit width at 2 rows, one column less at 3 rows, 24 columns at 3 rows, the fit width again) and the kit compares one screen per name; no frame compare exists. Also the hook message, details and background line have no seed, and the status row is not on the ported chat screen.",
    );
}

// ---- oracle: status_indicator_widget/effects_tests.rs (1 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE status_indicator_widget/effects_tests.rs:9 shimmer_and_progress_are_independent_and_obey_master_switch kind=snap owner=ui-pane snap=codex_tui__status_indicator_widget__effects_tests__shimmer_and_progress_are_independent_and_obey_master_switch
#[test]
fn shimmer_and_progress_are_independent_and_obey_master_switch() {
    kit::missing(
        Owner::UiPane,
        "KIT GAP: the snapshot holds the Debug of the header's spans (their styles: shimmer colours, progress bullet), and the kit compares rows only; the four animation, shimmer and progress settings have no seed either",
    );
}
