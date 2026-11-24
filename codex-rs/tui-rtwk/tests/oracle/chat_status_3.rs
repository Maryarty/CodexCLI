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
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; no scripted state for a named thread with a thread color",
    );
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    s.type_text("/statusline");
    s.keys("enter");
    // The oracle's preview Line, with colors on: the name in the thread color, the dot grey.
    s.assert_contains("Named task · Named task");
    s.assert_text_style("Named task", "fg=Rgb(242, 205, 205)");
    s.assert_text_style(" · ", "fg=Rgb(140, 140, 140) !bold !dim");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:138 status_surface_preview_lines_live_only_snapshot kind=snap owner=ui-pane snap=status_surface_previews_live_only
#[test]
fn status_surface_preview_lines_live_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; no scripted state for project root, branch, thread name and plan progress",
    );
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    // The oracle's two preview lines are the preview rows of the /statusline and /title setup popups.
    s.type_text("/statusline");
    s.keys("enter");
    s.assert_contains("preview-live-root · feature/live-preview-branch · Live preview thread");
    s.keys("esc");
    s.type_text("/title");
    s.keys("enter");
    s.assert_contains(
        "preview-live-root | Live preview thread | feature/live-preview-branch | Tasks 2/5",
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
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the hardcoded preview items have no scripted state",
    );
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    // The oracle's two preview lines are the preview rows of the /statusline and /title setup popups.
    s.type_text("/statusline");
    s.keys("enter");
    s.assert_contains(
        "my-project · feat/awesome-feature · thread title · Read Only · Ask for approval",
    );
    s.keys("esc");
    s.type_text("/title");
    s.keys("enter");
    s.assert_contains("thread title | feat/awesome-feature | Tasks 0/0");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:222 status_line_setup_popup_hardcoded_only_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_hardcoded_only
#[test]
fn status_line_setup_popup_hardcoded_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the status-line items (profile, provider) have no scripted state",
    );
    bottom_popup("/statusline", "status_line_setup_popup_hardcoded_only");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:239 status_line_setup_popup_workspace_headline_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_workspace_headline
#[test]
fn status_line_setup_popup_workspace_headline_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the workspace headline has no scripted state",
    );
    bottom_popup("/statusline", "status_line_setup_popup_workspace_headline");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:251 status_surface_preview_lines_mixed_snapshot kind=snap owner=ui-pane snap=status_surface_previews_mixed
#[test]
fn status_surface_preview_lines_mixed_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; no scripted state for the mixed live and hardcoded items",
    );
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    // The oracle's two preview lines are the preview rows of the /statusline and /title setup popups.
    s.type_text("/statusline");
    s.keys("enter");
    s.assert_contains("my-project · feature/mixed-preview · Mixed preview thread");
    s.keys("esc");
    s.type_text("/title");
    s.keys("enter");
    s.assert_contains("project | Mixed preview thread | Tasks 0/0");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:274 status_surface_preview_lines_rate_limits_snapshot kind=snap owner=ui-pane snap=status_surface_previews_rate_limits
#[test]
fn status_surface_preview_lines_rate_limits_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /statusline or /title setup popup in the ported chat; no rate-limit seed",
    );
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    // The oracle's two preview lines are the preview rows of the /statusline and /title setup popups.
    s.type_text("/statusline");
    s.keys("enter");
    s.assert_contains("monthly 65% left · weekly 50% left");
    s.keys("esc");
    s.type_text("/title");
    s.keys("enter");
    s.assert_contains("monthly 65% left | weekly 50% left");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:291 status_surface_preview_lines_thread_usage_snapshot kind=snap owner=ui-pane snap=status_surface_previews_thread_usage
#[test]
fn status_surface_preview_lines_thread_usage_snapshot() {
    kit::pending(
        Owner::UiPane,
        "no /statusline or /title setup popup in the ported chat; no thread-usage seed",
    );
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    // The oracle's two preview lines are the preview rows of the /statusline and /title setup popups.
    s.type_text("/statusline");
    s.keys("enter");
    s.assert_contains("5.2 credits · ~$0.21");
    s.keys("esc");
    s.type_text("/title");
    s.keys("enter");
    s.assert_contains("5.2 credits | ~$0.21");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:438 status_line_setup_popup_rate_limits_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_rate_limits
#[test]
fn status_line_setup_popup_rate_limits_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the rate-limit snapshot (35% and 50% used) has no scripted state",
    );
    bottom_popup("/statusline", "status_line_setup_popup_rate_limits");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:453 status_line_setup_popup_mixed_snapshot kind=snap owner=ui-pane snap=status_line_setup_popup_mixed
#[test]
fn status_line_setup_popup_mixed_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the account, the daybreak feature and the status-line items have no scripted state",
    );
    bottom_popup("/statusline", "status_line_setup_popup_mixed");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:474 terminal_title_setup_popup_live_only_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_live_only
#[test]
fn terminal_title_setup_popup_live_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the terminal-title items and the live branch and plan progress have no scripted state",
    );
    bottom_popup("/title", "terminal_title_setup_popup_live_only");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:494 terminal_title_setup_popup_hardcoded_only_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_hardcoded_only
#[test]
fn terminal_title_setup_popup_hardcoded_only_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the terminal-title items have no scripted state",
    );
    bottom_popup("/title", "terminal_title_setup_popup_hardcoded_only");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:509 terminal_title_setup_popup_mixed_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_mixed
#[test]
fn terminal_title_setup_popup_mixed_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the account, the daybreak feature and the terminal-title items have no scripted state",
    );
    bottom_popup("/title", "terminal_title_setup_popup_mixed");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:529 terminal_title_setup_popup_rate_limits_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_rate_limits
#[test]
fn terminal_title_setup_popup_rate_limits_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the rate-limit snapshot has no scripted state",
    );
    bottom_popup("/title", "terminal_title_setup_popup_rate_limits");
}

// ORACLE chatwidget/tests/status_surface_previews.rs:544 terminal_title_setup_popup_thread_usage_snapshot kind=snap owner=ui-pane snap=terminal_title_setup_popup_thread_usage
#[test]
fn terminal_title_setup_popup_thread_usage_snapshot() {
    kit::pending(
        Owner::UiPane,
        "feature /statusline and /title setup popups (w2-status): the chat has none in the ported chat; the thread-credits and estimated-cost items have no scripted state",
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
    // The oracle's sequence of program states (Idle, Working, Blocked, Working, Done, Idle)
    // is state; the visible part is the background-terminal line in the bottom popup.
    let s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_status_3/session_status.jsonl"))
        .open();
    s.assert_contains("1 background terminal running");
}

// ORACLE chatwidget/tests/terminal_program_status.rs:63 session_status_detail_uses_only_foreground_activity kind=snap owner=ui-pane
#[test]
fn session_status_detail_uses_only_foreground_activity() {
    kit::na(
        Owner::UiPane,
        "asserts the iTerm session detail string of each program status, an escape sent to the terminal (a helper's return), not a screen row or the window title",
    );
}

// ---- oracle: chatwidget/tests/terminal_title.rs (4 screen-level tests here; 3 state tests left to ui-pane)
// ORACLE chatwidget/tests/terminal_title.rs:8 daybreak_status_surfaces_follow_the_thread_preference kind=snap owner=ui-pane
#[test]
fn daybreak_status_surfaces_follow_the_thread_preference() {
    kit::pending(
        Owner::UiPane,
        "`Daybreak off` is the default and is covered by a page test; the toggle (the thread's `daybreakEnabled` from thread/started metadata) and the side conversation (no side fact in StatusFacts, chunk-live's) have no seed",
    );
    let s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    // terminal_title = ["daybreak"]: the window title follows the Daybreak item.
    s.assert_title("Daybreak off");
    // (Daybreak enabled) the oracle then expects "Daybreak on"; a side conversation, "Daybreak off" again.
    s.assert_title("Daybreak on");
    s.assert_title("Daybreak off");
}

// ORACLE chatwidget/tests/terminal_title.rs:38 goal_clock_refresh_redraws_only_when_elapsed_label_changes kind=snap owner=ui-pane
#[test]
fn goal_clock_refresh_redraws_only_when_elapsed_label_changes() {
    kit::pending(
        Owner::UiPane,
        "no goal fact (ThreadPart.goal from `thread/goal/updated`, chunk-live's): the indicator's clock (`Pursuing goal (1m)`) is drawn by the footer once it exists",
    );
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    // Visible half: the goal indicator label in the bottom pane advances by the minute.
    s.assert_contains("Pursuing goal (1m)");
    s.wait(60_000);
    s.assert_contains("Pursuing goal (2m)");
    // (the draw requests and the refresh deadline are state, not drawn)
}

// ORACLE chatwidget/tests/terminal_title.rs:96 terminal_title_shows_action_required_while_exec_approval_is_pending kind=ui owner=ui-pane
#[test]
fn terminal_title_shows_action_required_while_exec_approval_is_pending() {
    // oracle: a task runs, an exec approval arrives: the title holds the line that says so and the
    // project; the spinner is not shown. Answering restores the title with the project.
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_3/action_required.jsonl"))
        .open();
    s.assert_title_ends_with("project");
    s.feed(
        r#"{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":1,"params":{"kind":"command","itemId":"call-action-required","approvalId":"call-action-required","environmentId":null,"command":"bash -lc \"echo hello\"","cwd":"/tmp","reason":"need confirmation","threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#,
    );
    s.assert_title("[ ! ] Action Required | project");
    s.keys("y");
    s.assert_title_has("project");
    s.assert_title_lacks("Action Required");
}

// ORACLE chatwidget/tests/terminal_title.rs:286 thread_identity_renders_names_and_fallbacks_on_both_surfaces kind=snap owner=ui-pane
#[test]
fn thread_identity_renders_names_and_fallbacks_on_both_surfaces() {
    // oracle: a thread id and no name, animations off. Surfaces: the status line (thread-name,
    // thread-title, session-id) and the title (the same ids; the id is cut to 32 graphemes).
    // The live harness names its thread `thread-1` (the seed's `thread` is ignored: a kit need),
    // so the oracle's 36-character id is not on this screen; its cut to `...44665...` is
    // `a_title_part_is_cut_to_its_graphemes_with_dots` in status_surfaces.rs.
    let id = "thread-1";
    let cut = "thread-1";
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_3/thread_identity.jsonl"))
        .open();
    // unnamed: the name is omitted, the thread title falls back to the id
    s.assert_title(&format!("{id} | {cut}"));
    s.assert_has_row_trimmed(&format!("{id} · {id}"));
    // the default title: the project alone (no activity: animations are off)
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"project_root":"project","animations":false,"items":[]}}}"#);
    s.assert_title("project");
    s.feed(r#"{"step":"notify","message":{"method":"thread/name/updated","params":{"threadId":"thread-1","threadName":"Fix login timeout"}}}"#);
    s.assert_title("Fix login timeout | project");
    // named: the three ids on both surfaces
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"project_root":"project","animations":false,"items":["thread-name","thread-title","session-id"],"terminal_title":["thread-name","thread","session-id"]}}}"#);
    s.assert_title(&format!("Fix login timeout | Fix login timeout | {cut}"));
    s.assert_has_row_trimmed(&format!("Fix login timeout · Fix login timeout · {id}"));
    // a name of blanks is no name
    s.feed(r#"{"step":"notify","message":{"method":"thread/name/updated","params":{"threadId":"thread-1","threadName":"  "}}}"#);
    s.assert_title(&format!("{id} | {cut}"));
}

// ---- oracle: chatwidget/tests/tool_activity_tests.rs (1 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE chatwidget/tests/tool_activity_tests.rs:4 image_result_live_and_replay_render_in_the_owning_call kind=snap owner=ui-pane
#[test]
fn image_result_live_and_replay_render_in_the_owning_call() {
    kit::pending(
        Owner::UiPane,
        "feature image result of an MCP tool call (w2-cells): the chat has none, live or replayed: no scripted state script for either path",
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
    Screen::build(Owner::UiPane)
        .size(80, 2)
        .entry("chat", "working_row")
        .first_frame()
        .open()
        .assert_snap("renders_with_working_header");
}

// ORACLE status_indicator_widget.rs:387 renders_truncated kind=snap owner=ui-pane snap=renders_truncated
#[test]
fn renders_truncated() {
    Screen::build(Owner::UiPane)
        .size(20, 2)
        .entry("chat", "working_row")
        .first_frame()
        .open()
        .assert_snap("renders_truncated");
}

// ORACLE status_indicator_widget.rs:407 renders_wrapped_details_panama_two_lines kind=snap owner=ui-pane snap=renders_wrapped_details_panama_two_lines
#[test]
fn renders_wrapped_details_panama_two_lines() {
    Screen::build(Owner::UiPane)
        .size(30, 3)
        .entry("chat", "working_row_wrapped_details")
        .first_frame()
        .open()
        .assert_snap("renders_wrapped_details_panama_two_lines");
}

// ORACLE status_indicator_widget.rs:437 renders_without_spinner_when_animations_disabled kind=ui owner=ui-pane
#[test]
fn renders_without_spinner_when_animations_disabled() {
    let s = Screen::build(Owner::UiPane)
        .size(80, 1)
        .entry("chat", "working_row_no_animations")
        .first_frame()
        .open();
    s.assert_row_starts_with(0, "Working (0s • esc to interrupt)");
}

// ORACLE status_indicator_widget.rs:462 renders_remapped_interrupt_hint kind=snap owner=ui-pane snap=renders_remapped_interrupt_hint
#[test]
fn renders_remapped_interrupt_hint() {
    Screen::build(Owner::UiPane)
        .size(80, 1)
        .keymap(&[("chat.interrupt_turn", "f12")])
        .entry("chat", "working_row_remapped_interrupt")
        .first_frame()
        .open()
        .assert_snap("renders_remapped_interrupt_hint");
}

// ORACLE status_indicator_widget.rs:483 hook_status_reflows_without_displacing_controls_or_details kind=snap owner=ui-pane snap=hook_status_reflows_with_background_activity snap=hook_status_reflows_without_background_activity
#[test]
fn hook_status_reflows_without_displacing_controls_or_details() {
    kit::pending(
        Owner::UiPane,
        "G44: rtwk's text engine clusters a halfwidth katakana and its sound mark (`ｶﾞ`) into one cell, the oracle counts the mark as its own cell: the hook text `checking 日本語 ｶﾞﾊﾟ policy` is one cell narrower per mark, so the fit width (61 / 124) and the rows at 24 columns differ; the rest of the row (the reflow under the label, the details, the ellipsis) is the oracle's",
    );
    // The oracle draws the row alone at four widths: the fit width (2 rows), one column less
    // (3 rows), 24 columns (3 rows) and the fit width again.
    for (entry, snap, widths) in [
        (
            "working_row_hook",
            "hook_status_reflows_without_background_activity",
            [61u16, 60, 24, 61],
        ),
        (
            "working_row_hook_terminal",
            "hook_status_reflows_with_background_activity",
            [124u16, 123, 24, 124],
        ),
    ] {
        let mut s = Screen::build(Owner::UiPane)
            // wider than every frame: a resize to the opening size would blank the screen
            .size(130, 24)
            .entry("chat", entry)
            .first_frame()
            .open();
        for (i, width) in widths.into_iter().enumerate() {
            s.resize(width, 24);
            let frame = if i == 3 {
                format!("{width} columns#2")
            } else {
                format!("{width} columns")
            };
            s.snap(snap).frame(&frame).at(0, 0).assert();
        }
    }
}

// ---- oracle: status_indicator_widget/effects_tests.rs (1 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE status_indicator_widget/effects_tests.rs:9 shimmer_and_progress_are_independent_and_obey_master_switch kind=snap owner=ui-pane snap=codex_tui__status_indicator_widget__effects_tests__shimmer_and_progress_are_independent_and_obey_master_switch
#[test]
fn shimmer_and_progress_are_independent_and_obey_master_switch() {
    fn open(entry: &'static str) -> Screen {
        Screen::build(Owner::UiPane)
            .size(80, 2)
            // the oracle's `with_test_default_colors`
            .terminal_colors(0xf0_f0_f0, 0x10_10_10)
            .entry("chat", entry)
            .first_frame()
            .open()
    }
    // (true, false, false): the plain label, "esc" bold, the rest dim.
    let s = open("working_row_effects_plain");
    s.assert_has_row("Working (0s • esc to interrupt)");
    s.assert_text_style("esc", "fg=Rgb(240, 240, 240) bold !dim");
    // (without the blanks at the edges: G46, the presenter drops DIM on a blank cell)
    s.assert_text_style("(0s •", "dim");
    s.assert_text_style("to interrupt)", "dim");
    // (true, false, true): the same label behind the progress bullet.
    open("working_row_effects_progress").assert_has_row("• Working (0s • esc to interrupt)");
    // (true, true, false): each label letter is one shimmer step, the first grey Rgb(128, 128, 128).
    let s = open("working_row_effects_shimmer");
    s.assert_has_row("Working (0s • esc to interrupt)");
    s.assert_text_style("W", "fg=Rgb(128, 128, 128)");
    // (false, true, true): the master switch is off: the plain row, no bullet, no shimmer.
    let s = open("working_row_effects_master_off");
    s.assert_has_row("Working (0s • esc to interrupt)");
    s.assert_absent("• Working");
}
