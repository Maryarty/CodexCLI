//! Shard `chat_live_5` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.

use crate::kit::{self, Owner, Screen};

// Steps as `Screen::feed` takes them (shapes of src/screens/chat/fixtures/streamed_answer.jsonl).
// The oracle's helpers (chatwidget/tests/helpers.rs) are named in each builder.

/// The screen row that holds `text` (a locator for a cell's snapshot).
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or(usize::MAX)
}

/// A `notify` step: one server notification (method and params JSON).
fn notify(method: &str, params: &str) -> String {
    format!(r#"{{"step":"notify","message":{{"method":"{method}","params":{params}}}}}"#)
}

/// A turn object (the oracle's `app_server_turn`).
fn turn(id: &str, status: &str) -> String {
    format!(
        r#"{{"id":"{id}","rootTurnId":null,"items":[],"itemsView":"full","status":"{status}","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}"#
    )
}

/// `turn/started` (the oracle's `handle_turn_started`).
fn turn_started(id: &str) -> String {
    notify(
        "turn/started",
        &format!(
            r#"{{"threadId":"thread-1","turn":{}}}"#,
            turn(id, "inProgress")
        ),
    )
}

/// `turn/completed` with a status: "interrupted" is the oracle's `handle_turn_interrupted`.
fn turn_ended(id: &str, status: &str) -> String {
    notify(
        "turn/completed",
        &format!(r#"{{"threadId":"thread-1","turn":{}}}"#, turn(id, status)),
    )
}

/// `item/started` for one thread item.
fn item_started(item: &str) -> String {
    notify(
        "item/started",
        &format!(r#"{{"threadId":"thread-1","turnId":"turn-1","startedAtMs":0,"item":{item}}}"#),
    )
}

/// `item/completed` for one thread item.
fn item_completed(item: &str) -> String {
    notify(
        "item/completed",
        &format!(r#"{{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"item":{item}}}"#),
    )
}

/// The `enteredReviewMode` item (the oracle's `handle_entered_review_mode` payload).
fn entered_review_item(review: &str) -> String {
    format!(r#"{{"type":"enteredReviewMode","id":"review-start","review":"{review}"}}"#)
}

/// The review banner: `item/started` of the entered-review item.
fn entered_review(review: &str) -> String {
    item_started(&entered_review_item(review))
}

/// A user message item (the oracle's `complete_user_message`).
fn user_message(id: &str, text: &str) -> String {
    format!(
        r#"{{"type":"userMessage","id":"{id}","clientId":null,"content":[{{"type":"text","text":"{text}","text_elements":[]}}],"startedAtMs":null,"completedAtMs":null}}"#
    )
}

/// An agent message item (the oracle's `complete_assistant_message`).
fn agent_message(id: &str, text: &str) -> String {
    format!(
        r#"{{"type":"agentMessage","id":"{id}","text":"{text}","phase":null,"moderation":null}}"#
    )
}

/// A streamed answer delta (the oracle's `on_agent_message_delta`).
fn agent_delta(delta: &str) -> String {
    agent_delta_in("turn-1", delta)
}

fn agent_delta_in(turn: &str, delta: &str) -> String {
    notify(
        "item/agentMessage/delta",
        &format!(
            r#"{{"threadId":"thread-1","turnId":"{turn}","itemId":"msg-1","delta":"{delta}"}}"#
        ),
    )
}

/// A plan stream delta (the oracle's `on_plan_delta`).
fn plan_delta(delta: &str) -> String {
    notify(
        "item/plan/delta",
        &format!(
            r#"{{"threadId":"thread-1","turnId":"turn-1","itemId":"plan-1","delta":"{delta}"}}"#
        ),
    )
}

/// A running command (the oracle's `begin_exec`).
fn exec_begin(call: &str, command: &str) -> String {
    item_started(&format!(
        r#"{{"type":"commandExecution","id":"{call}","command":"{command}","cwd":"/tmp","processId":null,"source":"agent","status":"inProgress","commandActions":[],"aggregatedOutput":null,"exitCode":null,"durationMs":null}}"#
    ))
}

/// The thread goal reaching its budget (the oracle's `ThreadGoalUpdated` with `BudgetLimited`).
fn goal_budget_limited(turn_id: &str) -> String {
    notify(
        "thread/goal/updated",
        &format!(
            r#"{{"threadId":"thread-1","turnId":"{turn_id}","goal":{{"threadId":"thread-1","objective":"Run until the token budget is limited","status":"budgetLimited","tokenBudget":10000,"tokensUsed":10500,"timeUsedSeconds":0,"createdAt":0,"updatedAt":1}}}}"#
        ),
    )
}

// ---- oracle: chatwidget/tests/review_mode.rs (33 screen-level tests here; 8 state tests left to chat-live)
// ORACLE chatwidget/tests/review_mode.rs:106 entered_review_mode_uses_request_hint kind=ui owner=chat-live
#[test]
fn entered_review_mode_uses_request_hint() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&entered_review("feature branch"));
    s.assert_has_row(">> Code review started: feature branch <<");
}

// ORACLE chatwidget/tests/review_mode.rs:119 entered_review_mode_defaults_to_current_changes_banner kind=ui owner=chat-live
#[test]
fn entered_review_mode_defaults_to_current_changes_banner() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&entered_review("current changes"));
    s.assert_has_row(">> Code review started: current changes <<");
}

// ORACLE chatwidget/tests/review_mode.rs:131 live_review_prompt_item_is_not_rendered kind=ui owner=chat-live
#[test]
fn live_review_prompt_item_is_not_rendered() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&entered_review("changes against 'main'"));
    s.assert_contains("Code review started");
    let before = s.shot();
    // oracle: the review prompt user message adds no history cell.
    s.feed(&item_completed(&user_message(
        "review-prompt",
        "Review the code changes against the base branch 'main'.",
    )));
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/review_mode.rs:149 live_app_server_review_prompt_item_is_not_rendered kind=ui owner=chat-live
#[test]
fn live_app_server_review_prompt_item_is_not_rendered() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&entered_review("changes against 'main'"));
    s.assert_contains("Code review started");
    let before = s.shot();
    // oracle: the completed entered-review item inserts nothing more.
    s.feed(&item_completed(&entered_review_item(
        "changes against 'main'",
    )));
    s.assert_same_rows(&before);
    // oracle: the completed review prompt user message inserts nothing.
    s.feed(&item_completed(&user_message(
        "review-prompt",
        "Review the code changes against the base branch 'main'.",
    )));
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/review_mode.rs:293 esc_with_review_queued_steers_shows_warning_and_does_not_interrupt kind=snap owner=chat-live snap=review_submission_warning_snapshot
#[test]
fn esc_with_review_queued_steers_shows_warning_and_does_not_interrupt() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&entered_review("feature branch"));
    // oracle: a pending steer ("review follow-up") is queued while the review runs.
    s.type_text("review follow-up");
    s.keys("enter");
    s.keys("esc");
    // oracle: the warning is a transcript-only cell (`new_warning_event`); the detailed transcript shows it.
    s.keys("ctrl+t");
    s.assert_snap_region(
        "review_submission_warning_snapshot",
        row_of(&s, "Steer messages"),
        0,
    );
}

// ORACLE chatwidget/tests/review_mode.rs:316 live_agent_message_renders_during_review_mode kind=ui owner=chat-live
#[test]
fn live_agent_message_renders_during_review_mode() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&entered_review("current changes"));
    s.feed(&item_completed(&agent_message(
        "review-message",
        "Review progress update",
    )));
    s.assert_contains("Review progress update");
}

// ORACLE chatwidget/tests/review_mode.rs:489 steer_enter_queues_while_plan_stream_is_active kind=ui owner=chat-live
#[test]
fn steer_enter_queues_while_plan_stream_is_active() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&plan_delta("- Step 1"));
    s.type_text("queued submission");
    s.keys("enter");
    // oracle: the text is queued (queued_user_messages), no steer, no submit, no history cell.
    s.assert_contains("queued submission");
}

// ORACLE chatwidget/tests/review_mode.rs:516 steer_enter_uses_pending_steers_while_turn_is_running_without_streaming kind=ui owner=chat-live
#[test]
fn steer_enter_uses_pending_steers_while_turn_is_running_without_streaming() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.type_text("queued while running");
    s.keys("enter");
    // oracle: the steer is submitted as Op::UserTurn, and no history cell is inserted yet.
    s.feed(&item_completed(&user_message(
        "user-1",
        "queued while running",
    )));
    // oracle: the completed steer is inserted as one history cell with its text.
    s.assert_contains("queued while running");
}

// ORACLE chatwidget/tests/review_mode.rs:551 steer_enter_uses_pending_steers_while_final_answer_stream_is_active kind=ui owner=chat-live
#[test]
fn steer_enter_uses_pending_steers_while_final_answer_stream_is_active() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&agent_delta("Final answer line\\n"));
    s.type_text("queued while streaming");
    s.keys("enter");
    s.feed(&item_completed(&user_message(
        "user-1",
        "queued while streaming",
    )));
    s.assert_contains("queued while streaming");
}

// ORACLE chatwidget/tests/review_mode.rs:592 failed_pending_steer_submit_does_not_add_pending_preview kind=ui owner=chat-live
#[test]
fn failed_pending_steer_submit_does_not_add_pending_preview() {
    // The oracle drops the op channel, so the submit fails at once; the port's counterpart is a
    // steer the server refuses (`turn/steer` answers an error).
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.server_refuses("turn/steer", Some(-32000), "boom");
    s.type_text("queued while streaming");
    s.keys("enter");
    // oracle: no pending steer and no queued message remain after the refused submit.
    s.assert_absent("Messages to be submitted after next tool call");
    s.assert_absent("Messages to be submitted at end of turn");
    s.assert_absent("Queued follow-up inputs");
    s.assert_absent("↳");
    // The message is no history cell: it stands once, in the composer it came back to (the
    // oracle says nothing of the composer, and its channel drop draws no error line where an
    // RPC refusal draws one: the port's own rows, not asserted).
    s.assert_text_count("queued while streaming", 1);
}

// ORACLE chatwidget/tests/review_mode.rs:611 item_completed_only_pops_front_pending_steer kind=ui owner=chat-live
#[test]
fn item_completed_only_pops_front_pending_steer() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&agent_delta("Final answer line\\n"));
    s.type_text("first");
    s.keys("enter");
    s.type_text("second");
    s.keys("enter");
    s.feed(&item_completed(&user_message("user-other", "other")));
    s.assert_contains("other");
    // oracle: the front pending steer ("first") is still pending after another item completes.
    s.assert_contains("first");
    s.assert_contains("second");
    s.feed(&item_completed(&user_message("user-first", "first")));
    // oracle: "first" is inserted into history and "second" stays pending.
    s.assert_contains("first");
    s.assert_contains("second");
}

// ORACLE chatwidget/tests/review_mode.rs:741 steer_enter_during_final_stream_preserves_follow_up_prompts_in_order kind=ui owner=chat-live
#[test]
fn steer_enter_during_final_stream_preserves_follow_up_prompts_in_order() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&agent_delta("Final answer line\\n"));
    s.type_text("first follow-up");
    s.keys("enter");
    s.type_text("second follow-up");
    s.keys("enter");
    s.assert_in_order(&["first follow-up", "second follow-up"]);
    s.feed(&item_completed(&user_message("user-1", "first follow-up")));
    s.assert_contains("first follow-up");
    s.feed(&item_completed(&user_message("user-2", "second follow-up")));
    s.assert_contains("second follow-up");
}

// ORACLE chatwidget/tests/review_mode.rs:826 manual_interrupt_restores_pending_steers_to_composer kind=ui owner=chat-live
#[test]
fn manual_interrupt_restores_pending_steers_to_composer() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&agent_delta("Final answer line\\n"));
    s.type_text("queued while streaming");
    s.keys("enter");
    s.feed(&turn_ended("turn-1", "interrupted"));
    // oracle: the pending steer is back in the composer, and no submit op is sent.
    s.assert_contains("queued while streaming");
}

// ORACLE chatwidget/tests/review_mode.rs:871 esc_interrupt_sends_all_pending_steers_immediately_and_keeps_existing_draft kind=ui owner=chat-live
#[test]
fn esc_interrupt_sends_all_pending_steers_immediately_and_keeps_existing_draft() {
    // the scripted turn is `turn-0`: the fake backend numbers the turns it starts from `turn-1`
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-0"));
    s.feed(&agent_delta_in("turn-0", "Final answer line\\n"));
    s.type_text("first pending steer");
    s.keys("enter");
    s.type_text("second pending steer");
    s.keys("enter");
    // oracle: a queued draft ("queued draft") waits behind the steers; Tab queues the composer text while a turn runs.
    s.type_text("queued draft");
    s.keys("tab");
    s.type_text("still editing");
    s.keys("esc");
    s.feed(&turn_ended("turn-0", "interrupted"));
    // oracle: the composer draft stays, the queued draft stays queued, both steers are inserted into history.
    s.assert_contains("still editing");
    s.assert_contains("queued draft");
    s.assert_contains("first pending steer");
    s.assert_contains("second pending steer");
}

// ORACLE chatwidget/tests/review_mode.rs:951 esc_with_pending_steers_overrides_agent_command_interrupt_behavior kind=ui owner=chat-live
#[test]
fn esc_with_pending_steers_overrides_agent_command_interrupt_behavior() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.type_text("pending steer");
    s.keys("enter");
    s.type_text("/subagents ");
    s.keys("esc");
    // oracle: the interrupt is sent, and the composer still reads "/subagents " (the grid has no
    // trailing blank, so the row is compared up to the command)
    s.assert_interrupts(1);
    s.assert_has_row("› /subagents");
}

// ORACLE chatwidget/tests/review_mode.rs:973 manual_interrupt_restores_pending_steer_mention_bindings_to_composer kind=ui owner=chat-live
#[test]
fn manual_interrupt_restores_pending_steer_mention_bindings_to_composer() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&agent_delta("Final answer line\\n"));
    s.type_text("please use $figma");
    s.keys("enter");
    s.feed(&turn_ended("turn-1", "interrupted"));
    // oracle: the composer reads "please use $figma" again. The mention bindings themselves are not drawn.
    s.assert_contains("please use $figma");
}

// ORACLE chatwidget/tests/review_mode.rs:1017 manual_interrupt_restores_pending_steers_before_queued_messages kind=ui owner=chat-live
#[test]
fn manual_interrupt_restores_pending_steers_before_queued_messages() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&agent_delta("Final answer line\\n"));
    s.type_text("pending steer");
    s.keys("enter");
    s.type_text("queued draft");
    s.keys("tab");
    s.feed(&turn_ended("turn-1", "interrupted"));
    // oracle: the composer reads "pending steer" then "queued draft", one per row.
    s.assert_in_order(&["pending steer", "queued draft"]);
}

// ORACLE chatwidget/tests/review_mode.rs:1060 ctrl_c_shutdown_works_with_caps_lock kind=ui owner=chat-live
#[test]
fn ctrl_c_shutdown_works_with_caps_lock() {
    kit::na(
        Owner::ChatLive,
        "asserts AppEvent::Exit(ShutdownFirst) is sent for Ctrl+C with Caps Lock: an app event, nothing is drawn",
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1069 ctrl_c_interrupts_without_arming_quit_when_double_press_disabled kind=ui owner=chat-live
#[test]
fn ctrl_c_interrupts_without_arming_quit_when_double_press_disabled() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.keys("ctrl+c");
    // oracle: the interrupt is sent, and the quit reminder is not armed (no hint shown).
    s.assert_absent("again to quit");
    s.keys("ctrl+c");
    s.assert_absent("again to quit");
}

// ORACLE chatwidget/tests/review_mode.rs:1087 ctrl_c_cleared_prompt_is_recoverable_via_history kind=ui owner=chat-live
#[test]
fn ctrl_c_cleared_prompt_is_recoverable_via_history() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("draft message ");
    // oracle: attach_image(/tmp/preview.png) appends "[Image #1]" to the draft
    let path = s.attach_image("preview.png");
    s.keys("ctrl+c");
    s.assert_absent("draft message");
    s.keys("up");
    // oracle: the recalled draft starts with "draft message " and ends with "[Image #1]".
    s.assert_in_order(&["draft message", "[Image #1]"]);
    // oracle: and its image is back (take_recent_submission_images == [preview.png])
    s.keys("enter");
    s.wait_for("draft message");
    s.assert_sole_turn(kit::Turn::any().image_paths(&[path.as_path()]));
}

// ORACLE chatwidget/tests/review_mode.rs:1120 review_popup_custom_prompt_action_sends_event kind=ui owner=chat-live
#[test]
fn review_popup_custom_prompt_action_sends_event() {
    kit::na(
        Owner::ChatLive,
        "asserts AppEvent::OpenReviewCustomPrompt is sent when the custom preset is chosen: an app event, nothing drawn (the view it opens is checked by review_custom_prompt_escape_navigates_back_then_dismisses)",
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1146 review_commit_picker_shows_subjects_without_timestamps kind=ui owner=chat-live
#[test]
fn review_commit_picker_shows_subjects_without_timestamps() {
    let mut s = Screen::build(Owner::ChatLive).size(72, 24).live().open();
    s.type_text("/review");
    s.keys("enter");
    // oracle: `Review a commit` is the third preset; the host answers the log it asks for
    s.keys("down down enter");
    let asks = s.asked_git_logs();
    s.answer_git_log(
        &asks[0],
        &[("abc1234", "Add new feature X"), ("def5678", "Fix bug Y")],
    );
    // oracle: the commit subjects are drawn, and no relative time words are (the oracle lowercases the screen).
    s.assert_contains("Add new feature X");
    s.assert_contains("Fix bug Y");
    for word in [" ago", " second", " minute", " hour", " day"] {
        s.assert_absent(word);
        s.assert_absent(&word.to_uppercase());
    }
}

// ORACLE chatwidget/tests/review_mode.rs:1208 custom_prompt_submit_sends_review_op kind=ui owner=chat-live
#[test]
fn custom_prompt_submit_sends_review_op() {
    kit::na(
        Owner::ChatLive,
        "asserts AppEvent::CodexOp(Op::Review { Custom { instructions } }) is sent on Enter: an app event, nothing drawn",
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1233 custom_prompt_enter_empty_does_not_send kind=ui owner=chat-live
#[test]
fn custom_prompt_enter_empty_does_not_send() {
    kit::na(
        Owner::ChatLive,
        "asserts no AppEvent is sent on Enter in an empty custom prompt: an absent event, nothing drawn",
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1247 interrupt_exec_marks_failed_snapshot kind=snap owner=chat-live snap=interrupt_exec_marks_failed
#[test]
fn interrupt_exec_marks_failed_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .inline(&[])
        .live()
        .open();
    s.feed(&turn_started("turn-1"));
    s.feed(&exec_begin("call-int", "sleep 1"));
    s.feed(&turn_ended("turn-1", "interrupted"));
    // The oracle snapshots the finalized exec cell it inserted into the history.
    s.assert_snap_region("interrupt_exec_marks_failed", row_of(&s, "sleep 1"), 0);
}

// ORACLE chatwidget/tests/review_mode.rs:1270 interrupted_turn_error_message_snapshot kind=snap owner=chat-live snap=interrupted_turn_error_message
#[test]
fn interrupted_turn_error_message_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&turn_ended("turn-1", "interrupted"));
    s.assert_snap_region(
        "interrupted_turn_error_message",
        row_of(&s, "Conversation interrupted"),
        0,
    );
    // oracle: every span of the notice has the secondary text style.
    s.assert_text_style(
        "Conversation interrupted - use /feedback if something went wrong",
        "fg=Reset !bold !dim",
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1298 interrupted_turn_after_goal_budget_limited_uses_budget_message_snapshot kind=snap owner=chat-live snap=interrupted_turn_goal_budget_limited_message
#[test]
fn interrupted_turn_after_goal_budget_limited_uses_budget_message_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&goal_budget_limited("turn-1"));
    s.feed(&turn_ended("turn-1", "interrupted"));
    s.assert_snap_region(
        "interrupted_turn_goal_budget_limited_message",
        row_of(&s, "Goal budget reached"),
        0,
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1366 direct_budget_limited_turn_uses_budget_message_snapshot kind=snap owner=chat-live snap=direct_budget_limited_turn_message
#[test]
fn direct_budget_limited_turn_uses_budget_message_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    // oracle: mark_budget_limited(turn-1) then the interrupt; the goal update stands in for the mark (no wire step of its own).
    s.feed(&goal_budget_limited("turn-1"));
    s.feed(&turn_ended("turn-1", "interrupted"));
    s.assert_snap_region(
        "direct_budget_limited_turn_message",
        row_of(&s, "Goal budget reached"),
        0,
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1398 interrupted_turn_pending_steers_message_snapshot kind=snap owner=chat-live snap=interrupted_turn_pending_steers_message
#[test]
fn interrupted_turn_pending_steers_message_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    // oracle: a pending steer ("steer 1") with submit_pending_steers_after_interrupt set.
    s.type_text("steer 1");
    s.keys("enter");
    s.keys("esc");
    s.feed(&turn_ended("turn-1", "interrupted"));
    s.assert_snap_region(
        "interrupted_turn_pending_steers_message",
        row_of(&s, "Model interrupted"),
        0,
    );
}

// ORACLE chatwidget/tests/review_mode.rs:1422 review_custom_prompt_escape_navigates_back_then_dismisses kind=ui owner=chat-live
#[test]
fn review_custom_prompt_escape_navigates_back_then_dismisses() {
    let mut s = Screen::build(Owner::ChatLive).size(60, 24).live().open();
    s.type_text("/review");
    s.keys("enter");
    // oracle: the custom prompt is the fourth preset.
    s.keys("down down down enter");
    // (the preset list has a row of that name too: the prompt is told by its placeholder)
    s.assert_contains("Custom review instructions");
    s.assert_contains("Type instructions and press Enter");
    s.keys("esc");
    // oracle: Esc once returns to the parent popup.
    s.assert_contains("Select a review preset");
    s.assert_absent("Type instructions and press Enter");
    s.keys("esc");
    // oracle: Esc again dismisses all panels (normal composer mode).
    s.assert_absent("Select a review preset");
}

// ORACLE chatwidget/tests/review_mode.rs:1457 review_branch_picker_escape_navigates_back_then_dismisses kind=ui owner=chat-live
#[test]
fn review_branch_picker_escape_navigates_back_then_dismisses() {
    let mut s = Screen::build(Owner::ChatLive).size(60, 24).live().open();
    s.type_text("/review");
    s.keys("enter");
    // oracle: "Review against a base branch" is the first preset; the oracle reads the branches
    // from a temp dir that is no repository: the host answers with none
    s.keys("enter");
    let asks = s.asked_git_branches();
    s.answer_git_branches(&asks[0], None, &[]);
    s.assert_contains("Select a base branch");
    s.keys("esc");
    s.assert_contains("Select a review preset");
    s.keys("esc");
    s.assert_absent("Select a review preset");
}

// ORACLE chatwidget/tests/review_mode.rs:1491 enter_submits_steer_while_review_is_running kind=ui owner=chat-live
#[test]
fn enter_submits_steer_while_review_is_running() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(&turn_started("turn-1"));
    s.feed(&entered_review("current changes"));
    s.type_text("Steer submitted while /review was running.");
    s.keys("enter");
    // oracle: the steer is a pending steer with its text, and no history cell is inserted.
    s.assert_contains("Steer submitted while /review was running.");
}

// ORACLE chatwidget/tests/review_mode.rs:1531 review_queues_user_messages_snapshot kind=snap owner=chat-live snap=review_queues_user_messages_snapshot
#[test]
fn review_queues_user_messages_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 18).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#);
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","cwd":"/tmp/project","items":["model-with-reasoning","current-dir"]}}}"#);
    s.feed(&turn_started("turn-1"));
    s.feed(&entered_review("current changes"));
    s.paste("Steer submitted while /review was running.");
    s.keys("enter");
    // the server refuses the steer of a review turn: the message waits for the end of the turn
    let asks = s.asked_steers();
    s.answer_error(&asks[0], "cannot steer a review turn");
    // The oracle drew the chat widget alone: its history was drained and its composer has no
    // gap row (the app's reader control and copy feedback). The rows of its snapshot, blanks
    // aside, are the app's bottom rows from the working line on (the spinner glyph follows the
    // clock).
    let glyph: &[char] = &['•', '◦'];
    let want: Vec<String> = kit::snap_rows("review_queues_user_messages_snapshot", "")
        .iter()
        .map(|row| row.trim_end().to_string())
        .filter(|row| !row.is_empty())
        .collect();
    let ours: Vec<String> = s
        .rows()
        .iter()
        .map(|row| row.trim_end().to_string())
        .filter(|row| !row.is_empty())
        .collect();
    let from = ours
        .iter()
        .position(|row| row.trim_start_matches(glyph) == want[0].trim_start_matches(glyph))
        .unwrap_or_else(|| panic!("no working line:\n{}", s.text()));
    let got = &ours[from..(from + want.len()).min(ours.len())];
    let got: Vec<String> = got
        .iter()
        .map(|row| row.trim_start_matches(glyph).to_string())
        .collect();
    let want: Vec<&str> = want
        .iter()
        .map(|row| row.trim_start_matches(glyph))
        .collect();
    kit::assert_rows_are(&got, &want);
    s.assert_absent("cannot steer a review turn");
}

// ---- oracle: chatwidget/tests/side.rs (10 screen-level tests here; 3 state tests left to chat-live)
// ORACLE chatwidget/tests/side.rs:7 suppressed_interrupted_turn_notice_skips_history_warning kind=ui owner=chat-live
#[test]
fn suppressed_interrupted_turn_notice_skips_history_warning() {
    // the side conversation's own thread (the kit's `/side` seed): the side feature makes it quiet
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .side()
        .open();
    let of_side = |step: String| step.replace("\"thread-1\"", &format!("\"{}\"", kit::SIDE_THREAD));
    s.feed(&of_side(turn_started("turn-1")));
    s.feed(&of_side(agent_delta("partial output")));
    // the steps reach the side conversation, so the absence below is not a miss
    s.assert_contains("partial output");
    s.feed(&of_side(turn_ended("turn-1", "interrupted")));
    // the streamed text stays: a `turn/completed` that lists no item does not take the turn's items
    s.assert_contains("partial output");
    // oracle: no 'Conversation interrupted' and no steer-interrupt notice in the history of a side conversation.
    s.assert_absent("Conversation interrupted");
    s.assert_absent("Model interrupted to submit steer instructions.");
}

// ORACLE chatwidget/tests/side.rs:71 slash_commands_without_side_flag_are_rejected_for_side_threads kind=ui owner=chat-live
#[test]
fn slash_commands_without_side_flag_are_rejected_for_side_threads() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"flags":{"side_conversation_active":true}}}"#);
    s.type_text("/review");
    s.keys("enter");
    s.assert_reads(
        "'/review' is unavailable in side conversations. Press Ctrl+C to return to the main thread first.",
    );
    // oracle: a composer draft "/fork" survives Esc and Enter (the remote image URL of the oracle is not a kit step).
    s.type_text("/fork");
    s.keys("esc enter");
    s.assert_contains("/fork");
}

// ORACLE chatwidget/tests/side.rs:106 slash_side_is_rejected_for_side_threads kind=ui owner=chat-live
#[test]
fn slash_side_is_rejected_for_side_threads() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"flags":{"side_conversation_active":true}}}"#);
    s.type_text("/side");
    s.keys("enter");
    s.assert_reads(
        "'/side' is unavailable in side conversations. Press Ctrl+C to return to the main thread first.",
    );
}

// ORACLE chatwidget/tests/side.rs:135 side_aliases_are_rejected_during_review_mode kind=ui owner=chat-live
#[test]
fn side_aliases_are_rejected_during_review_mode() {
    for (command, name) in [("/side", "side"), ("/btw", "btw")] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
        s.feed(&entered_review("current changes"));
        s.type_text(command);
        s.keys("enter");
        s.assert_contains(&format!(
            "'/{name}' is unavailable while code review is running."
        ));
    }
}

// ORACLE chatwidget/tests/side.rs:166 slash_btw_is_rejected_before_the_session_starts kind=ui owner=chat-live
#[test]
fn slash_btw_is_rejected_before_the_session_starts() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .held_start()
        .open();
    s.type_text("/btw");
    s.keys("enter");
    s.assert_contains("'/btw' is unavailable before the session starts.");
}

// ORACLE chatwidget/tests/side.rs:213 side_aliases_without_args_start_empty_side_conversations kind=ui owner=chat-live
#[test]
fn side_aliases_without_args_start_empty_side_conversations() {
    kit::na(
        Owner::ChatLive,
        "asserts AppEvent::FollowTranscript and AppEvent::StartSide { user_message: None, Reuse } are sent and no op is submitted: app events and an absent op, nothing drawn",
    );
}

// ORACLE chatwidget/tests/side.rs:242 side_aliases_request_forked_questions_while_task_running kind=snap owner=chat-live snap=slash_side_requests_forked_side_question_while_task_running
#[test]
fn side_aliases_request_forked_questions_while_task_running() {
    for input in ["/side explore the codebase", "/btw explore the codebase"] {
        let mut s = Screen::build(Owner::ChatLive).size(80, 8).live().open();
        s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#);
        s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","items":["model-with-reasoning"]}}}"#);
        s.feed(&turn_started("turn-1"));
        s.type_text(input);
        s.keys("enter");
        // oracle: StartSide { New, user_message "explore the codebase" } and no op on the parent thread (events, not drawn).
        if input.starts_with("/side") {
            // The status row's spinner blinks between `•` and `◦` with the clock (the snapshot
            // holds its first frame): every row but that glyph is the oracle's.
            s.assert_snap_rows_from(
                "slash_side_requests_forked_side_question_while_task_running",
                0,
                8,
                true,
            );
        }
    }
}

// ORACLE chatwidget/tests/side.rs:295 side_context_label_preserves_status_line_snapshot kind=snap owner=chat-live snap=side_context_label_preserves_status_line
#[test]
fn side_context_label_preserves_status_line_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"flags":{"side_conversation_active":true}}}"#);
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#);
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"facts":{"side_label":"Side from main thread \u00b7 ctrl+/ to switch \u00b7 ctrl+c to close"}}}"#,
    );
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","items":["model-name"]}}}"#);
    s.assert_snap("side_context_label_preserves_status_line");
}

// ORACLE chatwidget/tests/side.rs:318 side_context_label_shows_parent_status_snapshot kind=snap owner=chat-live snap=side_context_label_shows_parent_status
#[test]
fn side_context_label_shows_parent_status_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"flags":{"side_conversation_active":true}}}"#);
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#);
    s.feed(
        r#"{"step":"screen","dir":"chat","ev":{"facts":{"side_label":"Side from main thread \u00b7 main needs input \u00b7 ctrl+/ to switch \u00b7 ctrl+c to close"}}}"#,
    );
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","cwd":"/tmp/project","items":["model-with-reasoning","current-dir"]}}}"#);
    s.assert_snap("side_context_label_shows_parent_status");
}

// ORACLE chatwidget/tests/side.rs:339 side_context_label_shows_hidden_side_snapshot kind=snap owner=chat-live snap=side_context_label_shows_hidden_side
#[test]
fn side_context_label_shows_hidden_side_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}"#);
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"side_label":"ctrl+/ for side"}}}"#);
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","items":["model-name"]}}}"#);
    s.assert_snap("side_context_label_shows_hidden_side");
    // oracle: the 'ctrl+/' hint is the key style (bold), and 'for side' is the secondary style (not bold, not dim).
    s.assert_text_style("ctrl+/", "fg=Reset bold !dim");
    s.assert_text_style("for side", "fg=Reset !bold !dim");
}
