//! Shard `chat_approvals_2` (screens S13, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

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
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live, chat-status): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), `threads.rs` does not append `ThreadPart::take_history` to the transcript (chat-live, a decision item for the cells owner), the Working row does not read `ThreadPart.reviews` through `sheets::approvals::review_status` (chat-status)",
    );
    // oracle: a write_stdin review InProgress then Approved draws no history and no status; InProgress then TimedOut leaves
    // one history cell (the stdin timeout) and the header Working.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started; guardian write_stdin review InProgress, Approved; InProgress, TimedOut (process 42)
    s.assert_contains("Working");
    s.assert_has_row(
        r#"✗ Review timed out before codex could send input to terminal 42: "confirm\n""#,
    );
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
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), and the approved-warning filter and the approved review are built (no cell), the chat only has to carry them",
    );
    // oracle: a guardian warning (approved, risk low) and an Approved assessment draw no history; the bottom pane at width 120.
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    // needs feed: guardian warning "Automatic approval review approved (risk: low, authorization: high): ..."; Approved assessment
    s.assert_snap("guardian_approved_exec_is_hidden_from_history");
}

// ORACLE chatwidget/tests/guardian.rs:384 guardian_approved_request_permissions_clears_status_without_history kind=snap owner=dialogs-pane snap=guardian_approved_request_permissions_clears_status_without_history
#[test]
fn guardian_approved_request_permissions_clears_status_without_history() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live, chat-status): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), the Working row does not read `ThreadPart.reviews` through `sheets::approvals::review_status` (chat-status)",
    );
    // oracle: a RequestPermissions review InProgress shows the header and details; Approved clears it without history.
    let s = Screen::build(Owner::DialogsPane)
        .size(110, 24)
        .live()
        .open();
    // needs feed: guardian assessment InProgress (guardian-request-permissions, reason "Need write access for generated report assets.")
    s.assert_contains("Reviewing approval request");
    s.assert_contains("permission request: Need write access for generated report assets.");
    // needs feed: the Approved assessment (risk low, authorization high)
    s.assert_contains("Working");
    s.assert_snap("guardian_approved_request_permissions_clears_status_without_history");
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
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), `threads.rs` does not append `ThreadPart::take_history` to the transcript (chat-live, a decision item for the cells owner)",
    );
    // oracle: ItemGuardianApprovalReviewStarted then Completed (Denied, risk high) draw the denied request at width 140.
    let s = Screen::build(Owner::DialogsPane)
        .size(140, 24)
        .live()
        .open();
    // needs feed: item/guardianApprovalReview started (guardian-1), completed (Denied, rationale "Would exfiltrate local source code.")
    s.assert_snap("app_server_guardian_review_denied_renders_denied_request");
}

// ORACLE chatwidget/tests/guardian.rs:663 app_server_guardian_review_timed_out_renders_timed_out_request_snapshot kind=snap owner=dialogs-pane snap=app_server_guardian_review_timed_out_renders_timed_out_request
#[test]
fn app_server_guardian_review_timed_out_renders_timed_out_request_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), `threads.rs` does not append `ThreadPart::take_history` to the transcript (chat-live, a decision item for the cells owner)",
    );
    // oracle: ItemGuardianApprovalReviewStarted then Completed (TimedOut) draw the timed-out request at width 140.
    let s = Screen::build(Owner::DialogsPane)
        .size(140, 24)
        .live()
        .open();
    // needs feed: item/guardianApprovalReview started (guardian-1), completed (TimedOut)
    s.assert_snap("app_server_guardian_review_timed_out_renders_timed_out_request");
}

// ORACLE chatwidget/tests/guardian.rs:744 guardian_parallel_reviews_render_aggregate_status_snapshot kind=snap owner=dialogs-pane snap=guardian_parallel_reviews_render_aggregate_status
#[test]
fn guardian_parallel_reviews_render_aggregate_status_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live, chat-status): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), the Working row does not read `ThreadPart.reviews` through `sheets::approvals::review_status` (chat-status)",
    );
    // oracle: two InProgress reviews under a started turn draw one aggregate status at width 72.
    let s = Screen::build(Owner::DialogsPane).size(72, 24).live().open();
    // needs feed: turn started; guardian-1 and guardian-2 InProgress (rm -rf of two targets)
    s.assert_snap("guardian_parallel_reviews_render_aggregate_status");
}

// ORACLE chatwidget/tests/guardian.rs:802 guardian_cleanup_drops_stale_reviews_and_restores_mcp_status kind=snap owner=dialogs-pane snap=guardian_goal_continuation_drops_stale_reviews
#[test]
fn guardian_cleanup_drops_stale_reviews_and_restores_mcp_status() {
    kit::pending(
        Owner::DialogsPane,
        "needs(chat-live, chat-status): the model (core approvals::guardian), the cell words (DecisionCell) and the list rows are built and tested in core/tests/approvals.rs and tests/sheets/approvals.rs; what is left is wiring of other owners: the router does not carry guardian notifications to the slot (`screens::project_thread`, `protocol/targets.rs` notification_thread: chat-live), the turn's end must apply `ThreadEv::TurnEnded` to the slot (threads.rs `end_turn`), the Working row does not read `ThreadPart.reviews` through `sheets::approvals::review_status` (chat-status); no MCP startup status either",
    );
    // oracle: alpha MCP starting, a stale review under turn-1, turn-1 interrupted: the header is Booting MCP server: alpha;
    // turn-2 with a current review draws the aggregate (width 72); turn-2 completes: the header is again Booting MCP server: alpha.
    let s = Screen::build(Owner::DialogsPane).size(72, 24).live().open();
    // needs feed: turn-1 started; mcp server alpha Starting; guardian stale-review InProgress; turn-1 interrupted
    s.assert_contains("Booting MCP server: alpha");
    // needs feed: turn-2 started; guardian current-review InProgress
    s.assert_snap("guardian_goal_continuation_drops_stale_reviews");
    // needs feed: turn-2 completed
    s.assert_contains("Booting MCP server: alpha");
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
// ORACLE chatwidget/tests/question_turn_end_tests.rs:19 question_turn_end_escapes_command_drafts_before_submission kind=snap owner=dialogs-pane
#[test]
fn question_turn_end_escapes_command_drafts_before_submission() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor in the composer and no turn-end draft recovery on the chat screen: the question block and its answer are not drawn",
    );
    // oracle: a draft (escaped when it starts a command or slash) with one question answered `answer` is recovered at turn end
    // as `escaped\n\n> Which way?\n\nanswer`; the non-queued submit sends it as one user turn, the queued one queues it (model).
    let drafts = [
        ("!echo partial", "\\!echo partial"),
        ("  !echo partial", "  !echo partial"),
        ("/", "\\/"),
        ("/diff", "\\/diff"),
        ("\t/diff", "\t\\/diff"),
        (" /diff", " /diff"),
        ("/tmp/file", "/tmp/file"),
        ("\\/diff", "\\/diff"),
    ];
    for (draft, escaped) in drafts {
        let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // needs feed: turn started; question "Which way?" added; alt+up; paste "answer"; composer set to the draft; turn completed
        s.assert_in_order(&[escaped, "> Which way?", "answer"]);
        if draft == "!echo partial" {
            s.assert_snap("question_turn_end_escaped_shell_command");
        }
    }
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:81 question_turn_end_appends_open_drafts_in_order_once kind=snap owner=dialogs-pane snap=large
#[test]
fn question_turn_end_appends_open_drafts_in_order_once() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor and no turn-end append of open drafts on the chat screen; the `$tool` element draft is not built",
    );
    // oracle: history "Earlier answer", composer "$tool existing draft" (an element), two open questions answered in order;
    // turn end appends the answers once; a second turn end appends nothing; a large (200-repeat) answer becomes a pending paste.
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started; history "Earlier answer"; composer "$tool existing draft"; questions "Which style?", "What budget?"
    s.keys("alt+up");
    s.paste("  Over-ear  ");
    s.keys("alt+up");
    s.paste("Under 250");
    s.keys("ctrl+r e");
    // needs feed: turn "ended-turn" completed
    s.assert_in_order(&[
        "$tool existing draft",
        "> Which style?",
        "Over-ear",
        "> What budget?",
        "Under 250",
    ]);
    s.assert_snap("question_turn_end_recovered_composer");
    // needs feed: turn "ended-turn" completed again (nothing appended twice)
    // needs feed: question "Which way?" (large) added
    s.keys("alt+up");
    s.paste(&"long answer ".repeat(200));
    // needs feed: turn "ended-turn" completed
    s.assert_snap("question_turn_end_large_draft");
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:143 question_turn_end_recovers_collapsed_drafts_on_completion_and_failure kind=snap owner=dialogs-pane
#[test]
fn question_turn_end_recovers_collapsed_drafts_on_completion_and_failure() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor to collapse and no turn-end recovery of a collapsed draft into the composer on the chat screen",
    );
    for status in ["completed", "failed"] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // oracle: question "Which way?" open; "answer" typed and collapsed (alt+down); "main" typed; history "earlier prompt"
        // recalled (ctrl+r e); the turn ends (failed also queues a prompt and raises the error): the composer shows "earlier prompt".
        let _ = status;
        // needs feed: turn started; question "Which way?"; the turn ends as `status`
        s.keys("alt+up");
        s.type_text("answer");
        s.keys("alt+down");
        s.type_text("main");
        s.keys("ctrl+r e");
        s.assert_contains("earlier prompt");
        // the recovered draft after Esc
        s.keys("esc");
        s.assert_in_order(&["main", "> Which way?", "answer"]);
    }
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:209 question_turn_end_recovers_after_interruption_restores_queued_input kind=snap owner=dialogs-pane
#[test]
fn question_turn_end_recovers_after_interruption_restores_queued_input() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor, no composer history search (ctrl+r) and no interruption recovery of queued input and pending steers on the chat screen",
    );
    // oracle: three main inputs (history accept, history cancel, buffered typing "+") over a question "Which way?" answered
    // "answer"; a queued prompt and a pending steer are held; the turn is interrupted: the composer is "earlier prompt"
    // (history) or the main draft with its typing; the search miss keeps the draft; the recovered draft holds the steer, the prompt and the answer.
    for main_input in ["history_accept", "history_cancel", "buffered_typing"] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // needs feed: composer "main draft"; turn started; question "Which way?" added
        s.keys("alt+up");
        s.paste("answer");
        s.keys("alt+down");
        if main_input == "buffered_typing" {
            s.type_text("+");
        } else {
            s.keys("ctrl+r e");
            s.assert_contains("earlier prompt");
        }
        // needs feed: queued prompt "queued prompt", pending steer "pending steer"; turn interrupted
        if main_input != "buffered_typing" {
            s.assert_contains("earlier prompt");
            s.keys("z enter");
            s.assert_contains("main draft");
            s.keys("backspace");
            if main_input == "history_accept" {
                s.assert_snap("question_turn_end_keeps_history_search");
            }
            s.keys(if main_input == "history_accept" {
                "enter"
            } else {
                "esc"
            });
        }
    }
}

// ORACLE chatwidget/tests/question_turn_end_tests.rs:297 question_turn_end_keeps_multiline_question_and_answer_as_text kind=ui owner=dialogs-pane
#[test]
fn question_turn_end_keeps_multiline_question_and_answer_as_text() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor: a question title with CR/LF and escape codes, and its multi-line answer, are not drawn as a recovered composer draft",
    );
    // oracle: the title "First line\r\n\r\n\x1b[31mLast\r line?\x1b[0m\x07" is recovered as "> First line", ">", "> Last line?"
    // (sanitized); the answer "  First answer line\n\nLast answer line  " follows; the composer and the unanswered count are as recovered.
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started; question with that title added
    s.keys("alt+up");
    s.paste("  First answer line\n\nLast answer line  ");
    s.keys("alt+down");
    // needs feed: turn completed
    s.assert_in_order(&[
        "> First line",
        "> Last line?",
        "First answer line",
        "Last answer line",
    ]);
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
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor in the composer: the freeform question and the queued reply are not drawn on the chat screen",
    );
    // oracle: composer "main draft"; two questions open; alt+up; an empty enter sends nothing. Not queued: the freeform
    // question draws `freeform_question`; "!literal answer" is delivered on enter and the composer keeps "main draft".
    // Queued (the queue autosend suppressed): the answered reply is queued and draws `queued_async_question_reply`.
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: composer "main draft"; questions "Which way?" and "Any details?" added
    s.keys("alt+up enter");
    s.assert_snap("freeform_question");
    s.paste("!literal answer");
    s.keys("enter");
    s.assert_contains("main draft");
    let mut q = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: as above, with the queue autosend suppressed
    q.keys("alt+up enter");
    q.paste("!literal answer");
    q.keys("enter");
    q.assert_snap("queued_async_question_reply");
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
    kit::pending(
        Owner::DialogsPane,
        "no numbered question choices drawn in the composer's question block (the long label and the `Answer too long` refusal)",
    );
    // oracle: a question with one option of 300 chars (named_question) is answered by its number and sent; an option of
    // MAX_USER_INPUT_TEXT_CHARS (1 << 20) chars (oversized_question) is refused on enter, and on a second enter after a paste of
    // MAX/2 quotes (JSON escaping counts toward the limit) with the line `  Answer too long; shorten it before sending`.
    for (length, snapshot) in [(300, "named_question"), (1 << 20, "oversized_question")] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // needs feed: question "What next?" with the option "x" * length + " but do not deploy"; thread started
        s.keys("alt+up");
        s.assert_snap(snapshot);
        if length > 512 {
            s.keys("enter");
            s.paste(&"\"".repeat((1 << 20) / 2));
            s.keys("enter");
            s.assert_has_row("  Answer too long; shorten it before sending");
        }
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:469 question_inputs_support_vim_editing_and_search kind=snap owner=dialogs-pane
#[test]
fn question_inputs_support_vim_editing_and_search() {
    kit::pending(
        Owner::DialogsPane,
        "no vim mode in the question editor on the chat screen (NORMAL and SEARCH frames, the Vim footer, the `d w`, `c i w` and `/` motions)",
    );
    // oracle: with vim on, "alpha beta" / "alpha gamma" typed into the answer; the NORMAL frame and the SEARCH frame (`/gamma`) are
    // snapshots (question_vim_<single|other>); `dw` removes "alpha beta" until `u` restores it; `ctrl+r` redoes; `ciw` + "delta" answers.
    for name in ["single", "other"] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // needs feed: vim mode on; question "Which way?" added (options: none for single; "Named" for other)
        s.keys("alt+up");
        if name == "other" {
            s.keys("2");
        }
        s.keys("i");
        s.paste("alpha beta\nalpha gamma");
        s.keys("esc 0 k");
        s.assert_snap_section(&format!("question_vim_{name}"), "NORMAL");
        s.keys("d w");
        s.assert_absent("alpha beta");
        s.keys("u");
        s.assert_contains("alpha beta");
        s.keys("ctrl+r");
        s.assert_absent("alpha beta");
        s.keys("u");
        s.type_text("/gamma");
        s.assert_snap_section(&format!("question_vim_{name}"), "SEARCH");
        s.keys("enter");
        s.keys("c i w");
        s.paste("delta");
        s.keys("esc enter");
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:529 question_history_search_uses_updated_bindings kind=ui owner=dialogs-pane
#[test]
fn question_history_search_uses_updated_bindings() {
    kit::pending(
        Owner::DialogsPane,
        "no composer history search over the question editor on the chat screen, and no keymap config seed (the rebound history_search_previous key)",
    );
    // oracle: history "[Image #2]earlier answer" then "newest answer"; up walks the history (newest, earlier, newest) for the
    // binding in the config (f12, or ctrl-x r); a typed query finds "persistent answer" from the host response for ctrl-x r.
    for (binding, word) in [("f12", "earlier"), ("ctrl+x r", "persistent")] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // needs feed: history entries (id 7); question "Which way?" added; alt+up; history "newest answer" recorded
        s.keys("up");
        s.assert_contains("newest answer");
        s.keys("up");
        s.assert_contains("earlier answer");
        s.keys("down");
        s.assert_contains("newest answer");
        s.keys("ctrl+c");
        s.keys(binding);
        s.type_text(word);
        // needs (ctrl-x r only): the host's history lookup response "persistent answer"
        s.keys("enter");
        s.keys("enter");
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:595 question_editor_keeps_working_status_and_queued_messages_visible kind=snap owner=dialogs-pane
#[test]
fn question_editor_keeps_working_status_and_queued_messages_visible() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor over the working status and the queued-message preview: the open and only-questions frames are not drawn",
    );
    // oracle: task running, composer "main draft", queued "queued follow-up", questions open: the OPEN frame (before `1 of 2`);
    // the queue cleared, the status line off, Esc: the ONLY QUESTIONS frame.
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn running; composer "main draft"; queued "queued follow-up"; questions "Which way?", "Any details?"
    s.keys("alt+up");
    s.assert_snap_section("questions_with_status_and_queue", "OPEN");
    // needs feed: turn finished; queue cleared; status line off
    s.keys("esc");
    s.assert_snap_section("questions_with_status_and_queue", "ONLY QUESTIONS");
}

// ORACLE chatwidget/tests/questions_tests.rs:624 async_questions_reject_blank_answers_but_allow_explicit_skip kind=ui owner=dialogs-pane
#[test]
fn async_questions_reject_blank_answers_but_allow_explicit_skip() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor: a blank answer is refused on the chat screen (the drawn screen does not change), and ctrl+5 skips",
    );
    // oracle: a blank and a whitespace-only answer leave the bottom popup unchanged and send nothing; ctrl+5 skips the question.
    for numbered in [false, true] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // needs feed: thread started; question "Which way?" (with the option "Named" when numbered)
        s.keys("alt+up");
        if numbered {
            s.keys("2");
        }
        for text in ["", " \t\n"] {
            s.paste(text);
            let before = s.shot();
            s.keys("enter");
            s.assert_same_rows(&before);
        }
        s.keys("ctrl+5");
        // the skip leaves no question: the unanswered count 0 and no op (model values, not drawn)
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:673 expanded_question_yields_mouse_to_terminal_for_links kind=ui owner=dialogs-pane
#[test]
fn expanded_question_yields_mouse_to_terminal_for_links() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor and no warnings popup over the chat: the link URL of an expanded question is not drawn, and the mouse yield is terminal state",
    );
    // oracle: a question titled with a long URL: mouse capture is inherited, then disabled when expanded (owned screen); the
    // warnings popup shows the URL; collapsing restores the inherited capture (terminal state, not drawn).
    let url = "https://example.com/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/r2/overview";
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    // needs feed: thread started; question with the URL as its title added
    s.keys("alt+up");
    // needs: the warnings popup opened over the expanded question (open_warnings(&[]); no key in the oracle test)
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
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor and no queued-message pop into the composer on the chat screen: the question with its queue is not drawn",
    );
    // oracle: queued "older" and "newer" and an open question; alt+up draws `last_question_with_queue`; the next alt+up pops
    // "newer" into the composer, " edited" is typed, the next alt+up expands "newer edited", the next pops "older"; enter sends it.
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: queued "older" and "newer"; question "Which way?" added
    s.keys("alt+up");
    s.assert_snap("last_question_with_queue");
    s.keys("alt+up");
    s.assert_contains("newer");
    s.paste(" edited");
    s.keys("alt+up");
    s.assert_contains("newer edited");
    s.keys("alt+up");
    s.assert_contains("older");
    s.keys("enter");
}

// ORACLE chatwidget/tests/questions_tests.rs:793 question_drafts_survive_navigation_and_snapshot_replay kind=ui owner=dialogs-pane
#[test]
fn question_drafts_survive_navigation_and_snapshot_replay() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor: per-question drafts, their navigation (alt+up/alt+down), and the restore of a thread's input are not drawn",
    );
    // oracle: composer "main draft"; question "Which way?" with "first draft", then "Any details?" with "second draft";
    // capture, a buffered question replay (not shown until the snapshot), restore: "second draft" is drawn; the pause and
    // ctrl+] + enter add a question; alt+down walks back to "first draft" and then to the composer "main draft".
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: composer "main draft"; thread started; question "Which way?" added
    s.keys("alt+up");
    s.paste("first draft");
    s.keys("alt+up 2");
    s.paste("second draft");
    // needs: capture and restore of the thread input, and the buffered replay of question "Buffered?"
    s.assert_contains("second draft");
    // needs: the unavailable-thread pause (a host state)
    s.keys("ctrl+] enter");
    s.keys("alt+down");
    s.assert_contains("first draft");
    s.keys("alt+down");
    s.assert_contains("main draft");
}

// ORACLE chatwidget/tests/questions_tests.rs:845 single_question_spacing_with_working_status kind=snap owner=dialogs-pane
#[test]
fn single_question_spacing_with_working_status() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor with the working status above it: the question, the queued-follow-up line and the spacing rows are not drawn",
    );
    // oracle: task running; question "Only question?" open; a typed answer: the queued-follow-up line two rows above the question,
    // one blank row, the answer two rows below, one blank row (snapshot single_question_working_spacing).
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn running; question "Only question?" added
    s.keys("alt+up");
    s.paste("A typed answer");
    s.assert_contains("Queued follow-up inputs");
    s.assert_contains("Only question?");
    s.assert_contains("A typed answer");
    s.assert_snap("single_question_working_spacing");
}

// ORACLE chatwidget/tests/questions_tests.rs:889 questions_and_queued_messages_share_the_resolved_shortcut kind=snap owner=dialogs-pane
#[test]
fn questions_and_queued_messages_share_the_resolved_shortcut() {
    kit::pending(
        Owner::DialogsPane,
        "no shortcut hints for the question editor and the queued messages on the chat screen, and no keymap config seed (Open::keymap is missing)",
    );
    // oracle: the hints `⇧← edit last queued message` and `⇧← to answer` (labels from the resolved key), then shift+left shows
    // `main prompt` and `next question` (snapshot question_queue_hint_Left); alt+up/alt+down do the same; the remapped config
    // (edit_queued_message = alt-up, shift-left; prompt_stack_back = alt-down, shift-right) draws `question_queue_hint_Up`;
    // the chord config (ctrl-x up / ctrl-x down) draws its chord labels.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    // needs feed: queued "queued"; questions "Which way?", "Any details?" added
    s.keys("shift+left");
    s.assert_snap("question_queue_hint_Left");
    // needs keymap seed (remapped alt-up / shift-left) for the next screen
    let mut r = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .keymap(&[
            ("chat.edit_queued_message", "alt-up"),
            ("chat.prompt_stack_back", "alt-down"),
        ])
        .open();
    r.keys("alt+up");
    r.assert_snap("question_queue_hint_Up");
}

// ORACLE chatwidget/tests/questions_tests.rs:985 desktop_async_answer_dismisses_only_its_question_and_preserves_the_other_draft kind=snap owner=dialogs-pane
#[test]
fn desktop_async_answer_dismisses_only_its_question_and_preserves_the_other_draft() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question editor: a desktop answer (a user message with the question reply) does not dismiss its question on the chat screen, and the other draft is not kept drawn",
    );
    // oracle: two questions "Same title?"; the second is navigated to and holds a draft; the desktop reply answers question 0
    // (live, or as a ThreadSnapshot item): question 0 is dismissed, the draft stays (snapshot
    // async_question_answered_on_another_client). The transcript cell's Debug dump (desktop_async_question_reply) is not a row dump.
    for replay in [false, true] {
        let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
        // needs feed: thread started; questions "Same title?" twice added
        s.keys("alt+up");
        s.keys("alt+down");
        s.paste("My draft for the second question");
        // needs feed: user message with the desktop reply (question 0, "Answer from **desktop**"), live or as a snapshot item
        let _ = replay;
        s.assert_snap("async_question_answered_on_another_client");
    }
}

// ORACLE chatwidget/tests/questions_tests.rs:1068 async_question_markdown_matches_loaded_history kind=snap owner=dialogs-pane
#[test]
fn async_question_markdown_matches_loaded_history() {
    kit::pending(
        Owner::DialogsPane,
        "no async-question titles drawn as markdown in the history on the chat screen; the oracle compares a Debug dump of styled lines (kit reads row dumps only)",
    );
    // oracle: an agent message with the questions "Keep `--verbose` for **failed** runs?" (option "Only for **CI**") and
    // "See the [guide](https://example.test/guide)?" draws the same transcript cell live and as loaded history (async_question_transcript).
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: agent message "ask" with the two questions, live and replayed
    s.assert_snap("async_question_transcript");
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
