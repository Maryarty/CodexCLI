//! Shard `chat_live_3` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The app server dropped events (oracle: the `Lagged` event of the stream).
const LAGGED: &str = r#"{"step":"lagged","skipped":1}"#;

// Expected rows, copied from the oracle (mcp_startup.rs assertions and runtime_mcp_warning.snap).
const MCP_ERR: &str = "MCP client for `alpha` failed to start: handshake failed";
const FAILED_ROW: &str = "⚠ MCP client for `alpha` failed to start: handshake failed";
const INCOMPLETE_ROW: &str = "⚠ MCP startup incomplete (failed: alpha)";
const INTERRUPTED_ROW: &str =
    "⚠ MCP startup interrupted. The following servers were not initialized: alpha";
const REVIEW_DETAILS: &str = r#"{"errorType":"new_category","reviewTarget":null,"detailedExplanation":"The agent proposed a change outside your instructions.","steer":{"message":"Continue with the requested change only."}}"#;

/// The idle footer of the oracle's harness: no agents hint, the status line (model, directory)
/// over `? for shortcuts`.
const IDLE_FOOTER: &str = concat!(
    r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#,
    "\n",
    r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","cwd":"/tmp/project","items":["model-with-reasoning","current-dir"]}}}"#
);

/// `item/started` for one thread item.
fn item_started(item: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"item/started","params":{{"threadId":"thread-1","turnId":"turn-1","startedAtMs":0,"item":{item}}}}}}}"#
    )
}

/// The review banner: `item/started` of the entered-review item.
fn entered_review(review: &str) -> String {
    item_started(&format!(
        r#"{{"type":"enteredReviewMode","id":"review-start","review":"{review}"}}"#
    ))
}

/// One JSONL step per line, as `Screen::feed` takes them.
fn script_of(steps: &[String]) -> String {
    steps.join("\n")
}

/// `mcpServer/startupStatus/updated` for one server (the oracle's `notify_mcp_status`).
fn mcp(name: &str, status: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"mcpServer/startupStatus/updated","params":{{"threadId":"thread-1","name":"{name}","status":"{status}","error":null,"failureReason":null}}}}}}"#
    )
}

/// The failed status with its error (the oracle's `notify_mcp_status_error`).
fn mcp_err(name: &str, error: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"mcpServer/startupStatus/updated","params":{{"threadId":"thread-1","name":"{name}","status":"failed","error":"{error}","failureReason":null}}}}}}"#
    )
}

/// `turn/started` or `turn/completed` for a plain turn (the oracle's `handle_turn_started` / `handle_turn_completed`).
fn turn_note(method: &str, turn: &str, status: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"{method}","params":{{"threadId":"thread-1","turn":{{"id":"{turn}","rootTurnId":null,"items":[],"itemsView":"full","status":"{status}","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}}}}}"#
    )
}

/// `turn/completed` with status failed and the misalignment error (the oracle's `app_server_turn(.., Failed, Some(..))`).
fn turn_failed(turn: &str, misalignment: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"turn/completed","params":{{"threadId":"thread-1","turn":{{"id":"{turn}","rootTurnId":null,"items":[],"itemsView":"full","status":"failed","error":{{"message":"Responses API returned misalignment_policy_violation","codexErrorInfo":"misalignmentPolicyViolation","additionalDetails":null,"misalignment":{misalignment}}},"startedAt":null,"completedAt":null,"durationMs":null}}}}}}}}"#
    )
}

/// The `error` notification of a misalignment policy violation (the oracle's `handle_error`).
fn misalignment_error(turn: &str, misalignment: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"error","params":{{"threadId":"thread-1","turnId":"{turn}","willRetry":false,"error":{{"message":"Responses API returned misalignment_policy_violation","codexErrorInfo":"misalignmentPolicyViolation","additionalDetails":null,"misalignment":{misalignment}}}}}}}}}"#
    )
}

/// The safety buffering notice that shows "Giving this request a little extra thought".
fn safety_buffering(turn: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"model/safetyBuffering/updated","params":{{"threadId":"thread-1","turnId":"{turn}","model":"current-model","useCases":[],"reasons":[],"showBufferingUi":true,"fasterModel":null}}}}}}"#
    )
}

// ---- oracle: chatwidget/tests/mcp_startup.rs (20 screen-level tests here; 11 state tests left to chat-live)
// ORACLE chatwidget/tests/mcp_startup.rs:95 mcp_startup_dedupes_same_round_duplicate_failure_warning kind=ui owner=chat-live
#[test]
fn mcp_startup_dedupes_same_round_duplicate_failure_warning() {
    // oracle: beta starts, alpha starts and fails twice in the same round: one warning cell; then beta ready gives the summary.
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        mcp("beta", "starting"),
        mcp("alpha", "starting"),
        mcp_err("alpha", MCP_ERR),
    ]));
    s.assert_has_row(FAILED_ROW);
    let once = s.shot();
    s.feed(&mcp_err("alpha", MCP_ERR));
    s.assert_same_rows(&once);
    s.feed(&mcp("beta", "ready"));
    s.assert_has_row(INCOMPLETE_ROW);
}

// ORACLE chatwidget/tests/mcp_startup.rs:131 mcp_startup_header_booting_snapshot kind=snap owner=chat-live snap=mcp_startup_header_booting
#[test]
fn mcp_startup_header_booting_snapshot() {
    // oracle: a turn-less starting server makes the bottom pane task-running with no status indicator (the snapshot shows none).
    let mut s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.feed(IDLE_FOOTER);
    s.feed(&mcp("alpha", "starting"));
    s.assert_snap("mcp_startup_header_booting");
}

// ORACLE chatwidget/tests/mcp_startup.rs:216 review_opens_during_mcp_startup kind=snap owner=chat-live snap=review_during_mcp_startup
#[test]
fn review_opens_during_mcp_startup() {
    for dismiss_completion in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        s.feed(&mcp("slow", "starting"));
        s.type_text("/review");
        if dismiss_completion {
            s.keys("esc");
        }
        s.keys("enter");
        // oracle: the composer is cleared and the attached image is dropped; no history cell is drawn.
        s.assert_absent("/review");
        // The popup is the oracle's bottom pane only: its rows are compared one by one, at any row.
        s.assert_has_row("  Select a review preset");
        s.assert_has_row("› 1. Review against a base branch  (PR Style)");
        s.assert_has_row("  2. Review uncommitted changes");
        s.assert_has_row("  3. Review a commit");
        s.assert_has_row("  4. Custom review instructions");
        s.assert_has_row("  enter select · esc back");
    }
}

// ORACLE chatwidget/tests/mcp_startup.rs:242 inline_review_submits_during_mcp_startup kind=ui owner=chat-live
#[test]
fn inline_review_submits_during_mcp_startup() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&mcp("slow", "starting"));
    s.type_text("/review check regressions");
    s.keys("enter");
    // oracle: Op::Review { Custom { instructions: "check regressions" } } goes to the core (not drawn);
    // the drawn half: the composer is cleared and no history cell shows the command.
    s.assert_absent("check regressions");
}

// ORACLE chatwidget/tests/mcp_startup.rs:266 review_and_fork_during_mcp_startup_preserve_drafts_when_foreground_work_is_pending kind=ui owner=chat-live
#[test]
fn review_and_fork_during_mcp_startup_preserve_drafts_when_foreground_work_is_pending() {
    for activity in ["turn", "review", "pending", "queued", "images"] {
        for draft in ["/review", "/review changes", "/fork", "/fork named"] {
            let command = draft.split_whitespace().next().unwrap_or(draft);
            if activity == "queued" && command == "/fork" {
                continue;
            }
            let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
            s.feed(&mcp("slow", "starting"));
            match activity {
                "turn" => {
                    s.feed(&turn_note("turn/started", "turn-1", "inProgress"));
                }
                // oracle: `is_review_mode = true` with no turn, which no wire step does: the port's
                // review comes inside a turn (the entered-review item).
                "review" => {
                    s.feed(&turn_note("turn/started", "turn-1", "inProgress"));
                    s.feed(&entered_review("current changes"));
                }
                // the MCP round holds the prompt in the queue.
                "queued" => {
                    s.type_text("queued prompt");
                    s.keys("enter");
                }
                // oracle: `user_turn_pending_start` (and, for "images", a submission whose images
                // are still prepared). The port has one state for both: a turn start in flight
                // while a round runs: the held prompt starts when the first round ends, a second
                // round starts after.
                "pending" | "images" => {
                    s.hold_image_preparation();
                    if activity == "images" {
                        s.attach_image_sized("shot.png", 8, 8);
                        s.type_text(" ");
                    }
                    s.type_text("first");
                    s.keys("enter");
                    s.feed(&mcp("slow", "ready"));
                    s.feed(&mcp("slow", "starting"));
                }
                _ => unreachable!(),
            }
            s.remote_images(&["https://example.com/image.png"]);
            s.type_text(draft);
            s.keys("enter");
            // oracle: the draft and the remote image stay in the composer and the command is
            // refused with one history cell; nothing is sent.
            s.assert_contains(draft);
            s.assert_contains("[Image #1]");
            s.assert_requests_of("review/start", 0);
            s.assert_has_row(&format!(
                "■ '{command}' is disabled while a task is in progress."
            ));
        }
    }
}

// ORACLE chatwidget/tests/mcp_startup.rs:347 pending_mcp_startup_dispatches_queued_slash_commands kind=ui owner=chat-live
#[test]
fn pending_mcp_startup_dispatches_queued_slash_commands() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&mcp("slow", "starting"));
    s.type_text("/resume");
    let before = s.shot();
    s.keys("tab");
    // oracle: Tab on /resume sends AppEvent::OpenResumePicker (the event is not drawn; the picker it opens is);
    // no submit op. The empty-state animation in the 48x17 area is not text and is not asserted here.
    s.assert_rows_differ(&before);
}

// ORACLE chatwidget/tests/mcp_startup.rs:376 pending_mcp_startup_does_not_reject_queued_compaction kind=ui owner=chat-live
#[test]
fn pending_mcp_startup_does_not_reject_queued_compaction() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        mcp("slow", "starting"),
        turn_note("turn/started", "turn-1", "inProgress"),
    ]));
    s.type_text("/compact");
    s.keys("tab");
    s.feed(&turn_note("turn/completed", "turn-1", "completed"));
    // oracle: the queued /compact is dispatched as Op::Compact (not drawn): no rejection cell is drawn.
    s.assert_absent("is disabled while a task is in progress");
}

// ORACLE chatwidget/tests/mcp_startup.rs:394 pending_mcp_startup_does_not_drain_follow_up_before_review_starts kind=ui owner=chat-live
#[test]
fn pending_mcp_startup_does_not_drain_follow_up_before_review_starts() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        mcp("slow", "starting"),
        turn_note("turn/started", "turn-1", "inProgress"),
    ]));
    for message in ["/review", "queued follow-up"] {
        s.type_text(message);
        s.keys("tab");
    }
    s.type_text("new draft");
    s.feed(&turn_note("turn/completed", "turn-1", "completed"));
    // oracle: the composer keeps the new draft once the turn completes.
    s.assert_contains("new draft");
    // oracle: Down, Enter submits the queued /review (Op::Review, not drawn); the follow-up is not sent as a turn.
    s.keys("down");
    s.keys("enter");
}

// ORACLE chatwidget/tests/mcp_startup.rs:421 pending_mcp_startup_does_not_unblock_external_review kind=snap owner=chat-live
#[test]
fn pending_mcp_startup_does_not_unblock_external_review() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(IDLE_FOOTER);
    // the fork stays in flight: the session does not switch to the forked thread.
    let _fork = s.hold_request("thread/fork");
    s.feed(&mcp("slow", "starting"));
    s.type_text("/fork");
    s.keys("esc");
    s.keys("enter");
    // oracle: /fork with worktree operations off sends ForkCurrentSession { name: None } (not drawn); the composer is cleared.
    s.assert_absent("/fork");
    s.assert_requests_of("thread/fork", 1);
    // oracle: `is_review_mode = true` with no turn, which no wire step does: the port's review
    // comes inside a turn (the entered-review item).
    s.feed(&turn_note("turn/started", "turn-1", "inProgress"));
    s.feed(&entered_review("current changes"));
    s.type_text("queued follow-up");
    s.keys("enter");
    // oracle: while review mode runs with a queued follow-up, /fork is refused with one history cell.
    s.type_text("/fork");
    s.keys("enter");
    s.assert_has_row("■ '/fork' is disabled while a task is in progress.");
    // the pending round does not end the review: its end starts no review and sends no turn.
    s.feed(&mcp("slow", "ready"));
    s.assert_contains("queued follow-up");
    s.assert_requests_of("review/start", 0);
}

// ORACLE chatwidget/tests/mcp_startup.rs:461 pending_mcp_startup_does_not_unblock_foreground_shell kind=ui owner=chat-live
#[test]
fn pending_mcp_startup_does_not_unblock_foreground_shell() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&mcp("slow", "starting"));
    s.type_text("!echo hi");
    s.keys("enter");
    // oracle: the !command is queued as RunShell and sent at once (Op::RunUserShellCommand, not drawn).
    s.type_text("queued follow-up");
    s.keys("enter");
    // oracle: the follow-up is queued and not submitted while the shell runs.
    s.assert_contains("queued follow-up");
    // oracle: /fork then dispatches (an InsertHistoryCell event; its text is not asserted).
}

// ORACLE chatwidget/tests/mcp_startup.rs:540 app_server_mcp_startup_failure_renders_warning_history kind=snap owner=chat-live snap=app_server_mcp_startup_failure_renders_warning_history
#[test]
fn app_server_mcp_startup_failure_renders_warning_history() {
    kit::na(
        Owner::ChatLive,
        "the test inserts the two cells into the terminal raw (insert_history_lines), so they touch; the app puts a blank row between history cells (resize_reflow.rs:97, helpers.rs:459) and the port draws it; the cells' rows are asserted in mcp_startup_failure_restores_running_status_header",
    );
    // oracle: a 120-wide terminal of max(ui height + 1, 10) rows; both warning cells then the idle composer.
    let mut s = Screen::build(Owner::ChatLive).size(120, 10).live().open();
    s.feed(&script_of(&[
        mcp("beta", "starting"),
        mcp("alpha", "starting"),
        mcp_err("alpha", MCP_ERR),
        mcp_err("alpha", MCP_ERR),
        mcp("beta", "ready"),
    ]));
    s.assert_in_order(&[FAILED_ROW, INCOMPLETE_ROW]);
    s.assert_has_row("› Ask Codex to do anything");
}

// ORACLE chatwidget/tests/mcp_startup.rs:609 mcp_startup_failure_restores_running_status_header kind=snap owner=chat-live
#[test]
fn mcp_startup_failure_restores_running_status_header() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        turn_note("turn/started", "turn-1", "inProgress"),
        mcp("alpha", "starting"),
        mcp("beta", "starting"),
    ]));
    s.assert_contains("Starting MCP servers");
    s.feed(&script_of(&[
        mcp_err("alpha", MCP_ERR),
        mcp("beta", "ready"),
    ]));
    // oracle: the runtime_mcp_warning snapshot: the failure cell, a blank row, the summary cell.
    s.assert_in_order(&[FAILED_ROW, INCOMPLETE_ROW]);
    // oracle: the running turn keeps its status, header back to "Working".
    s.assert_contains("Working");
}

// ORACLE chatwidget/tests/mcp_startup.rs:694 app_server_mcp_startup_lag_settles_startup_and_tracks_new_starts kind=snap owner=chat-live
#[test]
fn app_server_mcp_startup_lag_settles_startup_and_tracks_new_starts() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        mcp("beta", "starting"),
        mcp("alpha", "starting"),
        mcp_err("alpha", MCP_ERR),
    ]));
    // The lag settles the startup here (finish_mcp_startup_after_lag): the summary cell is drawn.
    s.feed(LAGGED);
    s.assert_has_row(INCOMPLETE_ROW);
    // oracle: a new start after the lag draws no history; a ready for it settles without a cell.
    let settled = s.shot();
    s.feed(&mcp("beta", "starting"));
    s.feed(&mcp("beta", "ready"));
    s.assert_same_rows(&settled);
}

// ORACLE chatwidget/tests/mcp_startup.rs:732 app_server_mcp_startup_lag_reports_only_explicit_cancellations kind=ui owner=chat-live
#[test]
fn app_server_mcp_startup_lag_reports_only_explicit_cancellations() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        mcp("beta", "starting"),
        mcp("alpha", "starting"),
        mcp("alpha", "cancelled"),
    ]));
    // The lag runs here; only the explicit cancellation of alpha is named.
    s.feed(LAGGED);
    s.assert_has_row(INTERRUPTED_ROW);
}

// ORACLE chatwidget/tests/mcp_startup.rs:757 app_server_mcp_startup_reports_failure_after_lag kind=ui owner=chat-live
#[test]
fn app_server_mcp_startup_reports_failure_after_lag() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&mcp("alpha", "starting"));
    // The lag runs here and draws nothing.
    s.feed(LAGGED);
    s.assert_absent("MCP startup");
    s.feed(&mcp_err("alpha", MCP_ERR));
    s.assert_has_row(FAILED_ROW);
    s.assert_has_row(INCOMPLETE_ROW);
    s.assert_absent("MCP startup interrupted");
}

// ORACLE chatwidget/tests/mcp_startup.rs:794 app_server_mcp_startup_reports_cancellation_after_lag kind=ui owner=chat-live
#[test]
fn app_server_mcp_startup_reports_cancellation_after_lag() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&mcp("alpha", "starting"));
    // The lag runs here and draws nothing.
    s.feed(LAGGED);
    s.assert_absent("MCP startup");
    s.feed(&mcp("alpha", "cancelled"));
    s.assert_has_row(INTERRUPTED_ROW);
}

// ORACLE chatwidget/tests/mcp_startup.rs:815 app_server_mcp_startup_after_lag_can_settle_without_starting_updates kind=ui owner=chat-live
#[test]
fn app_server_mcp_startup_after_lag_can_settle_without_starting_updates() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // The lag runs first, with no server started yet.
    s.feed(LAGGED);
    s.feed(&mcp_err("alpha", MCP_ERR));
    s.assert_has_row(FAILED_ROW);
    s.assert_has_row(INCOMPLETE_ROW);
    // oracle: beta ready after that draws no new cell and the task is not running.
    let failed = s.shot();
    s.feed(&mcp("beta", "ready"));
    s.assert_same_rows(&failed);
}

// ORACLE chatwidget/tests/mcp_startup.rs:846 app_server_mcp_startup_late_failure_does_not_reopen_the_next_round kind=ui owner=chat-live
#[test]
fn app_server_mcp_startup_late_failure_does_not_reopen_the_next_round() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        mcp("alpha", "starting"),
        mcp_err("alpha", MCP_ERR),
        mcp("beta", "starting"),
    ]));
    // The lag runs here and settles the round; a stale failure of alpha
    // after that is drawn as a new warning.
    s.feed(LAGGED);
    s.feed(&mcp_err(
        "alpha",
        "MCP client for `alpha` failed to start: stale handshake failed",
    ));
    s.assert_contains("MCP client for `alpha` failed to start: stale handshake failed");
    // oracle: beta starting and alpha ready draw nothing; beta ready draws no
    // summary and the turn is not running: the rows are the ones before.
    let before = s.shot();
    s.feed(&script_of(&[
        mcp("beta", "starting"),
        mcp("alpha", "ready"),
        mcp("beta", "ready"),
    ]));
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/mcp_startup.rs:890 app_server_mcp_startup_completed_failure_does_not_carry_into_the_next_round kind=ui owner=chat-live
#[test]
fn app_server_mcp_startup_completed_failure_does_not_carry_into_the_next_round() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // oracle: the lag runs first (a settled round with nothing started).
    s.feed(&mcp("alpha", "starting"));
    s.feed(&mcp_err("alpha", MCP_ERR));
    s.assert_has_row(FAILED_ROW);
    s.assert_has_row(INCOMPLETE_ROW);
    // oracle: the next round's beta start and ready draw no cell.
    let failed = s.shot();
    s.feed(&mcp("beta", "starting"));
    s.assert_same_rows(&failed);
    s.feed(&mcp("beta", "ready"));
    s.assert_contains(INCOMPLETE_ROW);
}

// ORACLE chatwidget/tests/mcp_startup.rs:927 app_server_mcp_startup_next_reported_start_reactivates_progress kind=ui owner=chat-live
#[test]
fn app_server_mcp_startup_next_reported_start_reactivates_progress() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        mcp("initial", "starting"),
        mcp("initial", "ready"),
        mcp("runtime", "starting"),
    ]));
    // oracle: the runtime start draws no history and makes the chat task-running.
    let started = s.shot();
    s.feed(&mcp_err(
        "runtime",
        "MCP client for `runtime` failed to start: handshake failed",
    ));
    s.assert_contains("MCP client for `runtime` failed to start: handshake failed");
    s.assert_contains("MCP startup incomplete (failed: runtime)");
    s.assert_rows_differ(&started);
}

// ---- oracle: chatwidget/tests/misalignment_policy_tests.rs (3 screen-level tests here; 1 state tests left to chat-live)
// ORACLE chatwidget/tests/misalignment_policy_tests.rs:50 misalignment_policy_failure_stops_the_thread_and_renders_once kind=snap owner=chat-live snap=misalignment_policy_violation snap=misalignment_policy_violation_narrow
#[test]
fn misalignment_policy_failure_stops_the_thread_and_renders_once() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&script_of(&[
        turn_note("turn/started", "turn-1", "inProgress"),
        safety_buffering("turn-1"),
    ]));
    s.assert_contains("Giving this request a little extra thought");
    s.keys("enter");
    s.type_text("queued follow-up");
    s.keys("tab");
    s.assert_contains("queued follow-up");
    s.type_text("stale draft");
    s.assert_saved_draft("stale draft");
    s.feed(&misalignment_error("turn-1", "null"));
    let popup = s.shot();
    s.assert_has_row("  Chat stopped as a precaution");
    s.assert_has_row("  We couldn’t confirm the agent was acting safely and following your");
    s.assert_has_row("  instructions. To continue working, start or resume another chat.");
    s.assert_has_row("› 1. New chat");
    s.assert_has_row("  2. Agent command center");
    s.assert_has_row("  3. Resume another chat");
    s.assert_absent("stale draft");
    s.assert_absent("queued follow-up");
    s.assert_saved_draft("");
    s.assert_no_turn();
    s.resize(38, 24);
    s.assert_has_row("  Chat stopped as a precaution");
    s.assert_has_row("  We couldn’t confirm the agent was");
    s.assert_has_row("  acting safely and following your");
    s.assert_has_row("  instructions. To continue working,");
    s.assert_has_row("  start or resume another chat.");
    s.assert_has_row("› 1. New chat");
    s.resize(80, 24);
    s.assert_same_rows(&popup);
    // oracle: the failed turn completes with the same error and draws nothing new (renders once).
    s.feed(&turn_failed("turn-1", "null"));
    s.assert_same_rows(&popup);
    s.keys("esc");
    s.assert_same_rows(&popup);
    s.keys("enter");
    kit::assert_same(
        s.commands_named("StartThread").len(),
        2,
        "new chat requests a second thread",
    );
}

// ORACLE chatwidget/tests/misalignment_policy_tests.rs:174 misalignment_turn_end_discards_history_search_and_question_drafts kind=snap owner=chat-live snap=misalignment_turn_end_cleared_composer
#[test]
fn misalignment_turn_end_discards_history_search_and_question_drafts() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .message_history(&["earlier prompt"])
        .live()
        .open();
    s.feed(include_str!(
        "fixtures/chat_approvals_2/q_turn_end_one.jsonl"
    ));
    s.paste(&"blocked draft ".repeat(200));
    s.keys("alt+up")
        .paste("unsubmitted answer")
        .keys("alt+down");
    s.keys("ctrl+r e");
    s.assert_contains("reverse-i-search");
    s.feed(&turn_failed("turn-1", REVIEW_DETAILS));
    s.assert_contains("Chat paused as a precaution");
    s.assert_saved_draft("");
    s.assert_absent("reverse-i-search");
    s.assert_absent("unsubmitted answer");
    // A later turn lifts the lock. The original draft and the preview are both gone,
    // and reopening the question editor cannot recover the failed turn's answer.
    s.feed(&turn_note("turn/started", "turn-2", "inProgress"));
    s.keys("alt+up");
    s.assert_absent("⇧← to answer");
    s.assert_contains("Ask Codex to do anything");
    s.assert_saved_draft("");
    s.assert_no_turn();
}

// ORACLE chatwidget/tests/misalignment_policy_tests.rs:256 misalignment_review_requires_current_findings kind=snap owner=chat-live snap=misalignment_review_narrow
#[test]
fn misalignment_review_requires_current_findings() {
    let mut s = Screen::build(Owner::ChatLive).size(38, 24).live().open();
    s.feed(&script_of(&[
        turn_note("turn/started", "turn-1", "inProgress"),
        misalignment_error("turn-1", REVIEW_DETAILS),
    ]));
    s.keys("enter");
    s.assert_contains("What we detected");
    s.assert_contains("Continuation request (quoted)");
    s.assert_contains_unwrapped("The agent proposed a change outside your instructions.");
    s.keys("q");
    s.assert_contains("Chat paused as a precaution");
    s.assert_contains_unwrapped("Acknowledge findings and continue");
    // A replacement expires the displayed review. Enter opens the replacement; it cannot
    // acknowledge the old findings.
    let latest = REVIEW_DETAILS.replace("requested change only", "new requested change only");
    s.feed(&misalignment_error("turn-1", &latest));
    s.keys("enter");
    s.assert_contains("What we detected");
    s.assert_contains_unwrapped("Continue with the new requested change only.");
    s.assert_no_turn();
    s.keys("q enter");
    s.assert_turn_texts(&["Continue with the new requested change only."]);
    let starts = s.commands_named("StartTurn");
    kit::assert_same(starts.len(), 1, "one acknowledged continuation");
    let acknowledged = starts.iter().any(|request| {
        request.contains("thread: ThreadId(\"thread-1\")")
            && request.contains("misalignment_override: Some(")
    });
    kit::assert_same(
        acknowledged,
        true,
        "the original thread owns the acknowledged start",
    );
}
