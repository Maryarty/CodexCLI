//! Shard `chat_live_4` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const PLAN_ITEM: &str = include_str!("fixtures/chat_live_4/plan_item.jsonl");
const FOOTER: &str = include_str!("fixtures/chat_live_4/footer.jsonl");
const LIVE_DONE: &str = include_str!("fixtures/chat_live_4/live_done.jsonl");
const LIVE_PLAN: &str = include_str!("fixtures/chat_live_4/live_plan.jsonl");
const PLAN_REVISED: &str = include_str!("fixtures/chat_live_4/plan_revised.jsonl");
const PLAN_UPDATE: &str = include_str!("fixtures/chat_live_4/plan_update.jsonl");
const REVISE_ECHO: &str = include_str!("fixtures/chat_live_4/revise_echo.jsonl");
const STEER_ECHO: &str = include_str!("fixtures/chat_live_4/steer_echo.jsonl");
const USAGE_90K: &str = include_str!("fixtures/chat_live_4/usage_90k.jsonl");
const REPLAYED_PLAN: &str = include_str!("fixtures/chat_live_4/replayed_plan.jsonl");
const REPLAY_MESSAGES: &str = include_str!("fixtures/chat_live_4/replay_messages.jsonl");
const RUNNING: &str = include_str!("fixtures/chat_live_4/running.jsonl");
const TURN_DONE: &str = include_str!("fixtures/chat_live_4/turn_done.jsonl");

/// The oracle's `PLAN_IMPLEMENTATION_TITLE` (chatwidget/plan_implementation.rs:9).
const PLAN_TITLE: &str = "Implement this plan?";

/// The oracle's hidden-paste payload: a shell line of a thousand characters.
fn hidden_shell_payload() -> String {
    format!("!echo {}", "x".repeat(1000))
}

/// The row a text is drawn on; the oracle renders a bottom popup on its own,
/// the port draws it inside the full screen, so the region starts there.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains(text))
        .unwrap_or(0)
}

// ---- oracle: chatwidget/tests/plan_mode.rs (40 screen-level tests here; 21 state tests left to chat-live)
// ORACLE chatwidget/tests/plan_mode.rs:68 plan_draft_footer_snapshot kind=snap owner=chat-live snap=plan_draft_footer
#[test]
fn plan_draft_footer_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(FOOTER)
        .open();
    s.type_text("make a plan");
    s.snap("plan_draft_footer")
        .at(row_of(&s, "› make a plan"), 0)
        .assert();
}

// ORACLE chatwidget/tests/plan_mode.rs:84 plan_implementation_popup_snapshot kind=snap owner=chat-live snap=plan_implementation_popup
#[test]
fn plan_implementation_popup_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(PLAN_ITEM)
        .open();
    s.feed(TURN_DONE);
    s.snap("plan_implementation_popup")
        .at(row_of(&s, PLAN_TITLE), 0)
        .assert();
}

// ORACLE chatwidget/tests/plan_mode.rs:94 plan_implementation_popup_context_usage_snapshot kind=snap owner=chat-live snap=plan_implementation_popup_context_usage
#[test]
fn plan_implementation_popup_context_usage_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(PLAN_ITEM)
        .open();
    s.feed(USAGE_90K);
    s.feed(TURN_DONE);
    s.snap("plan_implementation_popup_context_usage")
        .at(row_of(&s, PLAN_TITLE), 0)
        .assert();
}

// ORACLE chatwidget/tests/plan_mode.rs:107 plan_implementation_popup_yes_emits_submit_message_event kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_yes_emits_submit_message_event() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvent FollowTranscript and SubmitUserMessageWithMode (text and collaboration mode) sent on Enter; the events are not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:130 plan_implementation_popup_clear_context_emits_clear_submit_event kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_clear_context_emits_clear_submit_event() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvent ImplementPlanInFreshSession with the plan markdown sent on Enter; the event is not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:233 reasoning_selection_in_plan_mode_opens_scope_prompt_event kind=ui owner=chat-live
#[test]
fn reasoning_selection_in_plan_mode_opens_scope_prompt_event() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvent OpenPlanReasoningScopePrompt sent on Enter; the event is not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:260 reasoning_selection_in_plan_mode_without_effort_change_does_not_open_scope_prompt_event kind=ui owner=chat-live
#[test]
fn reasoning_selection_in_plan_mode_without_effort_change_does_not_open_scope_prompt_event() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvent ApplyModelSelection (model, effort) sent on Enter; the event is not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:288 reasoning_selection_in_plan_mode_matching_plan_effort_but_different_global_opens_scope_prompt kind=ui owner=chat-live
#[test]
fn reasoning_selection_in_plan_mode_matching_plan_effort_but_different_global_opens_scope_prompt() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvent OpenPlanReasoningScopePrompt with the Medium effort sent on Enter; the event is not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:319 reasoning_shortcut_in_plan_mode_updates_plan_override_without_prompt_or_persist kind=ui owner=chat-live
#[test]
fn reasoning_shortcut_in_plan_mode_updates_plan_override_without_prompt_or_persist() {
    kit::na(
        Owner::ChatLive,
        "asserts which AppEvents an Alt-period shortcut sends (plan override update, no prompt, no persistence); the events are not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:366 advanced_reasoning_selection_in_plan_mode_uses_expected_scope kind=ui owner=chat-live
#[test]
fn advanced_reasoning_selection_in_plan_mode_uses_expected_scope() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvents ApplyAdvancedReasoning or OpenPlanReasoningScopePrompt sent for Ultra and Max; the events are not drawn",
    );
}

/// Default-mode settings and the live settings the backend echoes after changing mode or effort.
fn reasoning_settings(thread: &str, mode: &str, effort: &str) -> String {
    serde_json::json!({"step":"notify","message":{"method":"thread/settings/updated","params":{
        "threadId":thread,
        "threadSettings":{"cwd":"/tmp/project","approvalPolicy":"on-request","approvalsReviewer":"user",
            "sandboxPolicy":{"type":"dangerFullAccess"},"activePermissionProfile":null,
            "model":"gpt-5.5","modelProvider":"openai","serviceTier":null,"effort":effort,"summary":null,
            "collaborationMode":{"mode":mode,"settings":{"model":"gpt-5.5","reasoning_effort":effort,"developer_instructions":null}},
            "personality":null}}}})
    .to_string()
}

fn plan_reasoning_session(configured_effort: Option<&str>) -> Screen {
    let catalog: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/chat_live_4/plan_reasoning_catalog.json"
    ))
    .unwrap_or_else(|error| panic!("plan reasoning catalog fixture: {error}"));
    let mut s = Screen::build(Owner::ChatLive)
        .size(120, 24)
        .live()
        .model_catalog(catalog["models"].clone())
        .collaboration_catalog(catalog["collaboration"].clone())
        .open();
    s.server_answers("thread/settings/update", serde_json::json!({}));
    // The host's configured global High and the live Plan mode's effort can differ.
    s.feed(
        &serde_json::json!({"step":"screen","dir":"chat","ev":{"status":{
            "model":"gpt-5.5","effort":"high","plan_effort":configured_effort,
            "items":["model-with-reasoning"]
        }}})
        .to_string(),
    );
    s.feed(&reasoning_settings("thread-1", "default", "high"));
    s.assert_contains("GPT-5.5 high");
    s.type_text("/plan").keys("enter");
    let effective = configured_effort.unwrap_or("medium");
    s.assert_last_request_has(
        "thread/settings/update",
        &[r#""reasoning_effort":""#, effective, "\""].concat(),
    );
    s.feed(&reasoning_settings("thread-1", "plan", effective));
    s.assert_contains("Plan mode");
    s.assert_contains(&["GPT-5.5 ", effective].concat());
    s
}

fn open_reasoning_picker(s: &mut Screen) {
    s.type_text("/model").keys("enter");
    s.assert_contains("Select Model and Effort");
    s.keys("enter");
    s.assert_contains("Select Reasoning Level for GPT-5.5");
}

fn plan_reasoning_picker(low_override: bool) -> Screen {
    let mut s = plan_reasoning_session(None);
    if low_override {
        // The user chooses Low through the Plan shortcut and the server echoes it.
        let updates = s.requests_of("thread/settings/update");
        s.keys("alt+,");
        s.assert_requests_of("thread/settings/update", updates + 1);
        s.assert_last_request_has("thread/settings/update", r#""low""#);
        s.assert_last_request_has("thread/settings/update", r#""plan""#);
        s.feed(&reasoning_settings("thread-1", "plan", "low"));
        s.assert_contains("GPT-5.5 low");
    }
    open_reasoning_picker(&mut s);
    s
}

fn choose_medium_reasoning(s: &mut Screen) {
    let writes = s.requests_of("config/batchWrite");
    let updates = s.requests_of("thread/settings/update");
    s.click_text("Medium (default)");
    s.assert_requests_of("config/batchWrite", writes);
    s.assert_requests_of("thread/settings/update", updates);
    s.assert_contains("Choose where to apply medium reasoning.");
}

// ORACLE chatwidget/tests/plan_mode.rs:412 plan_mode_reasoning_override_is_marked_current_in_reasoning_popup kind=ui owner=chat-live
#[test]
fn plan_mode_reasoning_override_is_marked_current_in_reasoning_popup() {
    let s = plan_reasoning_picker(true);
    s.assert_contains("Low (current)");
    s.assert_absent("High (current)");
}

// ORACLE chatwidget/tests/plan_mode.rs:435 reasoning_selection_in_plan_mode_model_switch_does_not_open_scope_prompt_event kind=ui owner=chat-live
#[test]
fn reasoning_selection_in_plan_mode_model_switch_does_not_open_scope_prompt_event() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvent ApplyModelSelection (gpt-5.6-terra, Some effort) sent on Enter; the event is not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:461 plan_reasoning_scope_popup_all_modes_persists_global_and_plan_override kind=ui owner=chat-live
#[test]
fn plan_reasoning_scope_popup_all_modes_persists_global_and_plan_override() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvents UpdatePlanModeReasoningEffort, PersistPlanModeReasoningEffort and PersistModelSelection sent on the all-modes choice; the events are not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:618 plan_reasoning_scope_popup_mentions_selected_reasoning kind=ui owner=chat-live
#[test]
fn plan_reasoning_scope_popup_mentions_selected_reasoning() {
    let mut s = plan_reasoning_picker(true);
    choose_medium_reasoning(&mut s);
    s.assert_contains("Always use medium reasoning in Plan mode");
    s.assert_contains("Apply to Plan mode override");
    s.assert_contains("Apply to global default and Plan mode override");
    s.assert_contains_unwrapped("user-chosen Plan override (low)");
}

// ORACLE chatwidget/tests/plan_mode.rs:640 plan_reasoning_scope_popup_mentions_built_in_plan_default_when_no_override kind=ui owner=chat-live
#[test]
fn plan_reasoning_scope_popup_mentions_built_in_plan_default_when_no_override() {
    let mut s = plan_reasoning_picker(false);
    choose_medium_reasoning(&mut s);
    s.assert_contains_unwrapped("built-in Plan default (medium)");
    let updates = s.requests_of("thread/settings/update");
    s.keys("esc");
    s.assert_absent("Choose where to apply medium reasoning.");
    s.assert_no_request("config/batchWrite");
    s.assert_requests_of("thread/settings/update", updates);
    s.assert_contains("GPT-5.5 medium");
}

#[test]
fn plan_reasoning_picker_reads_the_configured_override_on_entering_plan() {
    let mut s = plan_reasoning_session(Some("low"));
    open_reasoning_picker(&mut s);
    s.assert_contains("Low (current)");
    choose_medium_reasoning(&mut s);
    s.assert_contains_unwrapped("user-chosen Plan override (low)");
}

#[test]
fn switching_conversations_drops_the_scope_draft_and_the_shortcut_is_local() {
    let mut s = plan_reasoning_picker(true);
    choose_medium_reasoning(&mut s);
    s.server_resumes("thread-2").visit_thread("thread-2");
    s.assert_active_thread("thread-2");
    s.assert_absent("Choose where to apply medium reasoning.");
    s.assert_no_request("config/batchWrite");
    s.feed(&reasoning_settings("thread-2", "default", "high"));
    s.type_text("/plan").keys("enter");
    s.assert_last_request_has("thread/settings/update", r#""threadId":"thread-2""#);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"medium""#);
    s.feed(&reasoning_settings("thread-2", "plan", "medium"));
    open_reasoning_picker(&mut s);
    s.assert_contains("Medium (default) (current)");
    choose_medium_reasoning(&mut s);
    s.assert_contains_unwrapped("built-in Plan default (medium)");
}

#[test]
fn refused_plan_scope_save_keeps_the_original_effort_and_reports_to_its_thread() {
    let mut s = plan_reasoning_picker(true);
    choose_medium_reasoning(&mut s);
    let updates = s.requests_of("thread/settings/update");
    let write = s.hold_request("config/batchWrite");
    s.server_refuses(
        "config/batchWrite",
        Some(-32000),
        "Plan configuration is read-only",
    );
    s.click_text("Apply to Plan mode override");
    s.assert_requests_of("config/batchWrite", 1);
    s.assert_last_request_has(
        "config/batchWrite",
        r#""keyPath":"plan_mode_reasoning_effort""#,
    );
    s.assert_last_request_mentions(
        "config/batchWrite",
        r#""keyPath":"model_reasoning_effort""#,
        false,
    );
    s.assert_requests_of("thread/settings/update", updates);
    s.server_resumes("thread-2").visit_thread("thread-2");
    s.release(&write);
    s.assert_active_thread("thread-2");
    s.assert_absent("Plan configuration is read-only");
    s.visit_thread("thread-1");
    s.assert_active_thread("thread-1");
    s.assert_contains("Plan configuration is read-only");
    s.assert_contains("GPT-5.5 low");
    s.assert_requests_of("thread/settings/update", updates);
}

#[test]
fn all_modes_reasoning_waits_for_the_config_write_before_updating_plan() {
    let mut s = plan_reasoning_picker(true);
    choose_medium_reasoning(&mut s);
    let updates = s.requests_of("thread/settings/update");
    let write = s.hold_request("config/batchWrite");
    s.server_answers(
        "config/batchWrite",
        serde_json::json!({
            "status":"ok","version":"v","filePath":"/c","overriddenMetadata":null
        }),
    );
    s.click_text("Apply to global default and Plan mode override");
    s.assert_requests_of("config/batchWrite", 1);
    s.assert_last_request_has("config/batchWrite", r#""keyPath":"model_reasoning_effort""#);
    s.assert_last_request_has(
        "config/batchWrite",
        r#""keyPath":"plan_mode_reasoning_effort""#,
    );
    s.assert_requests_of("thread/settings/update", updates);
    s.release(&write);
    s.assert_requests_of("thread/settings/update", updates + 1);
    s.assert_last_request_has("thread/settings/update", r#""threadId":"thread-1""#);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"medium""#);
    s.feed(&reasoning_settings("thread-1", "plan", "medium"));
    s.assert_contains("GPT-5.5 medium");
    s.keys("shift+tab");
    s.assert_last_request_has("thread/settings/update", r#""mode":"default""#);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"medium""#);
}

#[test]
fn accepting_plan_only_uses_the_new_effort_for_plan_and_keeps_default() {
    let mut s = plan_reasoning_picker(true);
    choose_medium_reasoning(&mut s);
    let updates = s.requests_of("thread/settings/update");
    s.server_answers(
        "config/batchWrite",
        serde_json::json!({
            "status":"ok","version":"v","filePath":"/c","overriddenMetadata":null
        }),
    );
    s.click_text("Apply to Plan mode override");
    s.assert_requests_of("config/batchWrite", 1);
    s.assert_last_request_has(
        "config/batchWrite",
        r#""keyPath":"plan_mode_reasoning_effort""#,
    );
    s.assert_last_request_mentions(
        "config/batchWrite",
        r#""keyPath":"model_reasoning_effort""#,
        false,
    );
    s.assert_requests_of("thread/settings/update", updates + 1);
    s.assert_last_request_has("thread/settings/update", r#""mode":"plan""#);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"medium""#);
    s.feed(&reasoning_settings("thread-1", "plan", "medium"));
    open_reasoning_picker(&mut s);
    s.assert_contains("Medium (default) (current)");
    s.keys("esc").keys("esc");
    s.keys("shift+tab");
    s.assert_last_request_has("thread/settings/update", r#""mode":"default""#);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"high""#);
    s.keys("shift+tab");
    s.assert_last_request_has("thread/settings/update", r#""mode":"plan""#);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"medium""#);
    s.type_text("plan with this reasoning").keys("enter");
    s.assert_turn_texts(&["plan with this reasoning"]);
}

#[test]
fn an_older_plan_save_cannot_replace_a_newer_acknowledged_choice() {
    let mut s = plan_reasoning_picker(true);
    choose_medium_reasoning(&mut s);
    s.server_answers(
        "config/batchWrite",
        serde_json::json!({
            "status":"ok","version":"v","filePath":"/c","overriddenMetadata":null
        }),
    );
    let older = s.hold_request("config/batchWrite");
    s.click_text("Apply to Plan mode override");
    s.assert_requests_of("config/batchWrite", 1);
    open_reasoning_picker(&mut s);
    s.click_text("High");
    s.assert_contains("Choose where to apply high reasoning.");
    let newer = s.hold_request("config/batchWrite");
    s.click_text("Apply to Plan mode override");
    s.assert_requests_of("config/batchWrite", 2);

    let updates = s.requests_of("thread/settings/update");
    s.release(&newer);
    s.assert_requests_of("thread/settings/update", updates + 1);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"high""#);
    s.feed(&reasoning_settings("thread-1", "plan", "high"));
    s.release(&older);
    s.assert_requests_of("thread/settings/update", updates + 1);
    s.assert_contains("GPT-5.5 high");

    s.server_resumes("thread-2").visit_thread("thread-2");
    s.feed(&reasoning_settings("thread-2", "default", "high"));
    s.type_text("/plan").keys("enter");
    s.assert_last_request_has("thread/settings/update", r#""threadId":"thread-2""#);
    s.assert_last_request_has("thread/settings/update", r#""reasoning_effort":"high""#);
}

// ORACLE chatwidget/tests/plan_mode.rs:657 plan_reasoning_scope_popup_plan_only_does_not_update_all_modes_reasoning kind=ui owner=chat-live
#[test]
fn plan_reasoning_scope_popup_plan_only_does_not_update_all_modes_reasoning() {
    kit::na(
        Owner::ChatLive,
        "asserts the AppEvent UpdatePlanModeReasoningEffort sent and no UpdateReasoningEffort; the events are not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:680 submit_user_message_with_mode_errors_when_mode_changes_during_running_turn kind=ui owner=chat-live
#[test]
fn submit_user_message_with_mode_errors_when_mode_changes_during_running_turn() {
    kit::na(
        Owner::ChatLive,
        "asserts the widget's submit_user_message_with_mode refusing a mode change under a running turn; the port's only caller is the prompt's Yes, which exists after a turn ends: core/tests/plan_mode.rs yes_while_a_turn_runs_is_refused",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:708 submit_user_message_blocks_when_thread_model_is_unavailable kind=ui owner=chat-live
#[test]
fn submit_user_message_blocks_when_thread_model_is_unavailable() {
    let mut s = Screen::build(Owner::ChatLive).live().open();
    // the thread's model is gone (`set_model("")`)
    s.set_thread_model("");
    s.type_text("hello").keys("enter");
    s.assert_contains("Thread model is unavailable.");
    s.assert_no_turn();
    // the line stays in the composer
    s.assert_composer("hello");
}

// ORACLE chatwidget/tests/plan_mode.rs:788 plan_implementation_popup_skips_replayed_turn_complete kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_skips_replayed_turn_complete() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(REPLAYED_PLAN)
        .open();
    s.assert_absent(PLAN_TITLE);
}

// ORACLE chatwidget/tests/plan_mode.rs:828 plan_implementation_popup_shows_once_when_replay_precedes_live_turn_complete kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_shows_once_when_replay_precedes_live_turn_complete() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(REPLAYED_PLAN)
        .open();
    s.assert_absent(PLAN_TITLE);
    s.feed(LIVE_PLAN);
    s.feed(LIVE_DONE);
    s.assert_contains(PLAN_TITLE);
    s.keys("esc");
    s.assert_absent(PLAN_TITLE);
    s.feed(LIVE_DONE);
    s.assert_absent(PLAN_TITLE);
}

// ORACLE chatwidget/tests/plan_mode.rs:905 plan_implementation_popup_skips_when_messages_queued kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_skips_when_messages_queued() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(RUNNING)
        .open();
    s.type_text("Queued message").keys("tab");
    s.feed(PLAN_ITEM);
    s.feed(TURN_DONE);
    s.assert_absent(PLAN_TITLE);
}

// ORACLE chatwidget/tests/plan_mode.rs:928 plan_implementation_popup_skips_without_proposed_plan kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_skips_without_proposed_plan() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(PLAN_UPDATE)
        .open();
    s.feed(TURN_DONE);
    s.assert_absent(PLAN_TITLE);
}

// ORACLE chatwidget/tests/plan_mode.rs:955 plan_implementation_popup_shows_after_proposed_plan_output kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_shows_after_proposed_plan_output() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(PLAN_ITEM)
        .open();
    s.feed(TURN_DONE);
    s.assert_contains(PLAN_TITLE);
}

// ORACLE chatwidget/tests/plan_mode.rs:977 plan_implementation_popup_skips_when_steer_follows_proposed_plan kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_skips_when_steer_follows_proposed_plan() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(PLAN_ITEM)
        .open();
    s.type_text("Please continue.").keys("enter");
    s.feed(STEER_ECHO);
    s.feed(TURN_DONE);
    s.assert_absent(PLAN_TITLE);
}

// ORACLE chatwidget/tests/plan_mode.rs:1020 plan_implementation_popup_shows_after_new_plan_follows_steer kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_shows_after_new_plan_follows_steer() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(PLAN_ITEM)
        .open();
    s.type_text("Please revise.").keys("enter");
    s.feed(REVISE_ECHO);
    s.feed(PLAN_REVISED);
    s.feed(TURN_DONE);
    s.assert_contains(PLAN_TITLE);
}

/// The same 92% completion with different catalog responses. The earlier startup list is
/// distinct from the list the rate-limit offer asks after this turn ends.
fn plan_pending_rate_offer(models: Result<Vec<serde_json::Value>, &str>) -> Screen {
    let mut s = plan_rate_offer_screen(models);
    s.feed(
        r#"{"step":"respond","method":"account/rateLimits/read","result":{"rateLimits":{}}}
{"step":"screen","dir":"chat","ev":{"status":{"account_usage":true}}}"#,
    );
    s.feed(include_str!("fixtures/kit/rate_limit_92.jsonl"));
    s.assert_absent("Approaching rate limits");
    s
}

fn plan_rate_offer_screen(models: Result<Vec<serde_json::Value>, &str>) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 30)
        .fake_turn_ids()
        .script(PLAN_ITEM)
        .open();
    let mut reply = serde_json::json!({"step":"respond","method":"model/list"});
    match models {
        Ok(models) => reply["result"] = serde_json::json!({"data":models,"nextCursor":null}),
        Err(reason) => reply["error"] = serde_json::json!({"code":-32000,"message":reason}),
    }
    s.feed(&reply.to_string());
    s.feed(r#"{"step":"screen","dir":"status","ev":{"kind":"account","account":{"chat_gpt":{"email":"user@example.com","plan":"plus"}}}}"#);
    s
}

fn lower_cost_model() -> serde_json::Value {
    serde_json::json!({
        "additionalSpeedTiers": [], "availabilityNux": null, "defaultReasoningEffort": "medium",
        "defaultServiceTier": null, "description": "Lower-cost model",
        "displayName": "GPT-6 Luna", "hidden": false, "id": "gpt-6-luna",
        "inputModalities": ["text"], "isDefault": false, "model": "gpt-6-luna",
        "modelSpecialty": null, "serviceTiers": [],
        "supportedReasoningEfforts": [{"description": "Balanced", "reasoningEffort": "medium"}],
        "supportsPersonality": false, "upgrade": null, "upgradeInfo": null
    })
}

fn complete_with_rate_offer_pending(s: &mut Screen) {
    let before = s.requests_of("model/list");
    s.feed(TURN_DONE);
    s.assert_requests_of("model/list", before + 1);
    s.assert_absent("Approaching rate limits");
    s.assert_absent(PLAN_TITLE);
}

fn complete_second_plan(s: &mut Screen) {
    let next = PLAN_ITEM
        .replace("turn-1", "turn-2")
        .replace("plan-1", "plan-2");
    s.feed(&next);
    s.feed(&TURN_DONE.replace("turn-1", "turn-2"));
}

fn other_conversation_reply(s: &mut Screen, method: &str) {
    s.feed(
        &serde_json::json!({
            "step": "respond", "method": method, "result": {
                "thread": {
                    "id": "thread-other", "sessionId": "thread-other",
                    "name": "Another conversation", "preview": "Another conversation",
                    "ephemeral": false, "modelProvider": "openai", "createdAt": 1700000000,
                    "updatedAt": 1700000100, "status": {"type": "idle"}, "cwd": "/other",
                    "cliVersion": "0.0.0", "source": "cli", "turns": []
                },
                "model": "gpt-5", "modelProvider": "openai", "cwd": "/other",
                "instructionSources": [], "approvalPolicy": "on-request",
                "approvalsReviewer": "user", "sandbox": {"type": "dangerFullAccess"},
                "reasoningEffort": null
            }
        })
        .to_string(),
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:1067 plan_implementation_popup_skips_when_rate_limit_prompt_pending kind=ui owner=chat-live
#[test]
fn plan_implementation_popup_skips_when_rate_limit_prompt_pending() {
    for (held, action) in [(true, "esc"), (false, "enter")] {
        let mut s = plan_pending_rate_offer(Ok(vec![lower_cost_model()]));
        let before = s.requests_of("model/list");
        if held {
            let model_lookup = s.hold_request("model/list");
            complete_with_rate_offer_pending(&mut s);
            s.feed(TURN_DONE);
            s.assert_requests_of("model/list", before + 1);
            s.release(&model_lookup);
        } else {
            s.feed(TURN_DONE);
        }
        s.assert_text_count("Approaching rate limits", 1);
        s.assert_absent(PLAN_TITLE);
        s.keys(action);
        s.assert_absent("Approaching rate limits");
        s.assert_absent(PLAN_TITLE);
        s.feed(TURN_DONE);
        s.assert_absent("Approaching rate limits");
        s.assert_absent(PLAN_TITLE);
        s.assert_requests_of("model/list", before + 1);
        s.assert_no_turn();
        s.assert_model_updates(if held { &[] } else { &["gpt-6-luna"] });
    }

    for models in [Ok(Vec::new()), Err("model catalog unavailable")] {
        let mut s = plan_pending_rate_offer(models);
        let model_lookup = s.hold_request("model/list");
        complete_with_rate_offer_pending(&mut s);
        s.release(&model_lookup);
        // The oracle skips the Plan before it discovers the rate catalog has no offer.
        s.assert_absent("Approaching rate limits");
        s.assert_absent(PLAN_TITLE);
        // A new turn without a reservation can still ask for implementation.
        complete_second_plan(&mut s);
        s.assert_text_count(PLAN_TITLE, 1);
        s.keys("esc");
        s.assert_no_turn();
    }

    let mut s = plan_pending_rate_offer(Ok(vec![lower_cost_model()]));
    let model_lookup = s.hold_request("model/list");
    complete_with_rate_offer_pending(&mut s);
    s.feed(r#"{"step":"notify","message":{"method":"account/rateLimits/updated","params":{"rateLimits":{"primary":{"usedPercent":92,"windowDurationMins":60},"credits":{"hasCredits":true,"unlimited":false,"balance":"10"}}}}}"#);
    s.release(&model_lookup);
    s.assert_absent("Approaching rate limits");
    s.assert_absent(PLAN_TITLE);
    s.assert_no_turn();

    for reply_before_resume in [true, false] {
        let mut s = plan_pending_rate_offer(Ok(vec![lower_cost_model()]));
        let model_lookup = s.hold_request("model/list");
        complete_with_rate_offer_pending(&mut s);
        other_conversation_reply(&mut s, "thread/resume");
        let resume = s.hold_request("thread/resume");
        s.visit_thread("thread-other");
        s.assert_request_mentions("thread/resume", "thread-other");
        if reply_before_resume {
            s.assert_contains("Proposed Plan");
            s.release(&model_lookup);
            s.assert_absent("Approaching rate limits");
            s.release(&resume);
        } else {
            s.release(&resume);
            s.assert_absent("Proposed Plan");
            s.release(&model_lookup);
        }
        s.assert_absent("Proposed Plan");
        s.assert_absent("Approaching rate limits");
        s.assert_absent(PLAN_TITLE);
        s.visit_thread("thread-1");
        s.assert_contains("Proposed Plan");
        s.assert_absent("Approaching rate limits");
        s.assert_absent(PLAN_TITLE);
        s.assert_no_turn();
    }

    for failed in [false, true] {
        let mut s = plan_pending_rate_offer(Ok(vec![lower_cost_model()]));
        let model_lookup = s.hold_request("model/list");
        complete_with_rate_offer_pending(&mut s);
        s.type_text("a separate turn").keys("enter");
        s.assert_turn_texts(&["a separate turn"]);
        if failed {
            s.feed(r#"{"step":"notify","message":{"method":"error","params":{"threadId":"thread-1","turnId":"fake-turn-1","error":{"message":"Second turn failed","codexErrorInfo":null,"additionalDetails":null},"willRetry":false}}}"#);
            s.assert_contains("Second turn failed");
        }
        s.release(&model_lookup);
        s.assert_absent("Approaching rate limits");
        s.assert_absent(PLAN_TITLE);
    }

    rate_offer_waits_behind_a_banner();
    rate_offer_lifetime_ends_with_the_connection_or_account();
    rate_offer_lifetime_ends_when_a_new_turn_starts();
    rate_offer_survives_repeated_completion_of_an_earlier_turn();
    rate_offer_cannot_interrupt_a_new_conversation();
}

fn rate_offer_survives_repeated_completion_of_an_earlier_turn() {
    let mut s = plan_rate_offer_screen(Ok(vec![lower_cost_model()]));
    s.feed(TURN_DONE);
    s.assert_text_count(PLAN_TITLE, 1);
    s.keys("esc");
    s.feed(include_str!("fixtures/kit/rate_limit_92.jsonl"));
    let before = s.requests_of("model/list");
    complete_second_plan(&mut s);
    s.assert_requests_of("model/list", before + 1);
    s.assert_text_count("Approaching rate limits", 1);
    s.assert_absent(PLAN_TITLE);

    s.feed(TURN_DONE);
    s.assert_requests_of("model/list", before + 1);
    s.assert_text_count("Approaching rate limits", 1);
    s.assert_absent(PLAN_TITLE);
    s.keys("enter");
    s.assert_model_updates(&["gpt-6-luna"]);
    s.assert_absent("Approaching rate limits");
    s.assert_absent(PLAN_TITLE);
    s.assert_no_turn();
}

fn rate_offer_lifetime_ends_when_a_new_turn_starts() {
    let mut s = plan_pending_rate_offer(Ok(vec![lower_cost_model()]));
    s.feed(TURN_DONE);
    s.assert_text_count("Approaching rate limits", 1);
    s.feed(RUNNING);
    s.assert_text_count("Approaching rate limits", 1);
    s.feed(&RUNNING.replace("turn-1", "turn-2"));
    s.assert_contains("Working");
    s.assert_absent("Approaching rate limits");
    s.assert_absent(PLAN_TITLE);
    s.keys("enter");
    s.assert_model_updates(&[]);
    s.assert_no_turn();
}

fn rate_offer_cannot_interrupt_a_new_conversation() {
    let mut s = plan_pending_rate_offer(Ok(vec![lower_cost_model()]));
    let model_lookup = s.hold_request("model/list");
    complete_with_rate_offer_pending(&mut s);
    other_conversation_reply(&mut s, "thread/start");
    let start = s.hold_request("thread/start");
    s.type_text("/new").keys("enter");
    s.assert_requests_of("thread/start", 1);
    s.assert_contains("Proposed Plan");
    s.release(&model_lookup);
    s.assert_absent("Approaching rate limits");
    s.assert_absent(PLAN_TITLE);
    s.release(&start);
    s.assert_absent("Proposed Plan");
    s.assert_absent("Approaching rate limits");
    s.assert_no_turn();
}

fn rate_offer_waits_behind_a_banner() {
    let mut s = plan_rate_offer_screen(Ok(vec![lower_cost_model()]));
    s.feed(r#"{"step":"respond","method":"account/rateLimits/read","result":{"accountId":"workspace-a","rateLimits":{},"rateLimitUpsell":{"banner_type":"selected_model_limit","model_slug":"gpt-5","presentation":"dismissible","title":"Selected model usage exhausted","description":"Switch to another model.","ctas":[]}}}
{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5","account_usage":true}}}"#);
    s.feed(include_str!("fixtures/kit/rate_limit_92.jsonl"));
    let before = s.requests_of("model/list");
    s.feed(TURN_DONE);
    s.assert_contains("Selected model usage exhausted");
    s.assert_requests_of("model/list", before);
    s.assert_absent("Approaching rate limits");
    s.assert_absent(PLAN_TITLE);
    s.keys("esc");
    s.assert_absent("Selected model usage exhausted");
    s.assert_requests_of("model/list", before);
    complete_second_plan(&mut s);
    s.assert_requests_of("model/list", before + 1);
    s.assert_text_count("Approaching rate limits", 1);
    s.assert_absent(PLAN_TITLE);
}

fn rate_offer_lifetime_ends_with_the_connection_or_account() {
    for transport_replaced in [false, true] {
        let mut s = plan_pending_rate_offer(Ok(vec![lower_cost_model()]));
        s.feed(TURN_DONE);
        s.assert_text_count("Approaching rate limits", 1);
        s.other_backend_account_updated();
        s.assert_text_count("Approaching rate limits", 1);
        if transport_replaced {
            s.replace_transport();
        } else {
            s.feed(r#"{"step":"notify","message":{"method":"account/updated","params":{"authMode":"chatgpt","planType":"plus"}}}"#);
        }
        s.assert_absent("Approaching rate limits");
        s.assert_absent(PLAN_TITLE);
        s.assert_contains("Proposed Plan");
        s.keys("enter");
        s.assert_model_updates(&[]);
        s.assert_no_turn();
    }
}

// ORACLE chatwidget/tests/plan_mode.rs:1298 enter_submits_when_plan_stream_is_not_active kind=ui owner=chat-live
#[test]
fn enter_submits_when_plan_stream_is_not_active() {
    kit::na(
        Owner::ChatLive,
        "asserts the composer's queued-message list is empty and an Op::UserTurn is sent on Enter; neither is drawn by these assertions",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:1319 collab_mode_shift_tab_cycles_only_when_idle kind=ui owner=chat-live
#[test]
fn collab_mode_shift_tab_cycles_only_when_idle() {
    kit::na(
        Owner::ChatLive,
        "asserts the active and current collaboration-mode accessors through BackTab; the assertions read state, not the footer",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:1338 mode_switch_surfaces_model_change_notification_when_effective_model_changes kind=ui owner=chat-live
#[test]
fn mode_switch_surfaces_model_change_notification_when_effective_model_changes() {
    kit::pending(
        Owner::ChatLive,
        "G-mask: the mode switch works (/plan, Shift+Tab) but the port has no collaboration-mask catalogue, so no per-mode model/effort (gpt-5.6-luna medium for Plan) and no model-change notice; the masks come from collaborationMode/list, which no screen reads yet",
    );
    let mut s = Screen::build(Owner::ChatLive).live().open();
    s.keys("shift+tab");
    s.assert_contains("Model changed to gpt-5.6-luna medium for Plan mode.");
    s.keys("shift+tab");
    s.assert_contains("Model changed to gpt-5 default for Default mode.");
}

// ORACLE chatwidget/tests/plan_mode.rs:1377 mode_switch_surfaces_reasoning_change_notification_when_model_stays_same kind=ui owner=chat-live
#[test]
fn mode_switch_surfaces_reasoning_change_notification_when_model_stays_same() {
    kit::pending(
        Owner::ChatLive,
        "G-mask: the mode switch works but the port has no collaboration-mask catalogue (per-mode reasoning effort, gpt-5.6-terra High), so no reasoning-change notice",
    );
    let mut s = Screen::build(Owner::ChatLive).live().open();
    s.keys("shift+tab");
    s.assert_contains("Model changed to gpt-5.6-terra medium for Plan mode.");
}

// ORACLE chatwidget/tests/plan_mode.rs:1416 plan_slash_command_with_args_submits_prompt_in_plan_mode kind=ui owner=chat-live
#[test]
fn plan_slash_command_with_args_submits_prompt_in_plan_mode() {
    kit::na(
        Owner::ChatLive,
        "asserts the Op::UserTurn items (one text item) and the active collaboration-mode accessor after /plan with arguments; neither is drawn by these assertions",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:1442 plan_slash_command_with_hidden_shell_paste_submits_literal_prompt kind=ui owner=chat-live
#[test]
fn plan_slash_command_with_hidden_shell_paste_submits_literal_prompt() {
    let payload = hidden_shell_payload();
    let mut s = Screen::build(Owner::ChatLive).live().open();
    s.type_text("/plan ").paste(&payload);
    s.assert_contains(&format!("/plan [Pasted Content {} chars]", payload.len()));
    s.keys("enter");
    // The oracle then checks the UserTurn carries the literal payload in Plan mode: the
    // user's own cell shows it as typed, not as a shell command.
    s.assert_has_row("› !echo");
    s.assert_absent("/plan");
}

// ORACLE chatwidget/tests/plan_mode.rs:1453 plan_slash_command_with_hidden_shell_paste_rejected_image_remains_literal kind=ui owner=chat-live
#[test]
fn plan_slash_command_with_hidden_shell_paste_rejected_image_remains_literal() {
    let payload = hidden_shell_payload();
    let mut s = Screen::build(Owner::ChatLive).live().open();
    s.type_text("/plan ").paste(&payload);
    s.assert_contains(&format!("/plan [Pasted Content {} chars]", payload.len()));
    // The oracle sets a remote image URL, sends Enter (refused), clears it and sends Enter again.
    s.keys("enter");
}

// ORACLE chatwidget/tests/plan_mode.rs:1476 plan_slash_command_with_hidden_shell_paste_unavailable_model_remains_literal kind=ui owner=chat-live
#[test]
fn plan_slash_command_with_hidden_shell_paste_unavailable_model_remains_literal() {
    let payload = hidden_shell_payload();
    let mut s = Screen::build(Owner::ChatLive).live().open();
    s.type_text("/plan ").paste(&payload);
    s.assert_contains(&format!("/plan [Pasted Content {} chars]", payload.len()));
    s.keys("enter");
}

// ORACLE chatwidget/tests/plan_mode.rs:1491 plan_slash_command_with_hidden_shell_paste_queued_during_turn_submits_literal_prompt kind=ui owner=chat-live
#[test]
fn plan_slash_command_with_hidden_shell_paste_queued_during_turn_submits_literal_prompt() {
    let payload = hidden_shell_payload();
    let mut s = Screen::build(Owner::ChatLive).script(RUNNING).open();
    s.type_text("/plan ").paste(&payload);
    s.assert_contains(&format!("/plan [Pasted Content {} chars]", payload.len()));
    s.keys("tab");
    s.feed(TURN_DONE);
}

// ORACLE chatwidget/tests/plan_mode.rs:1512 plan_slash_command_with_hidden_shell_paste_queued_before_session_submits_literal_prompt kind=ui owner=chat-live
#[test]
fn plan_slash_command_with_hidden_shell_paste_queued_before_session_submits_literal_prompt() {
    let payload = hidden_shell_payload();
    let mut s = Screen::build(Owner::ChatLive).live().held_start().open();
    s.type_text("/plan ").paste(&payload);
    s.keys("enter");
    s.type_text("queued follow-up").keys("enter");
    s.assert_in_order(&["/plan !echo", "queued follow-up"]);
    s.start_thread();
    s.assert_turn_texts(&[&payload]);
    s.assert_contains("Plan mode");
}

// ORACLE chatwidget/tests/plan_mode.rs:1579 vim_mode_default_enabled_starts_composer_in_insert_mode kind=ui owner=chat-live
#[test]
fn vim_mode_default_enabled_starts_composer_in_insert_mode() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .vim_mode()
        .open();
    s.keys("x right esc .");
    s.assert_has_row("› xx");
}

// ORACLE chatwidget/tests/plan_mode.rs:1686 collab_mode_is_sent_after_enabling kind=ui owner=chat-live
#[test]
fn collab_mode_is_sent_after_enabling() {
    kit::na(
        Owner::ChatLive,
        "asserts the Op::UserTurn collaboration_mode field (Default) after enabling the feature and Enter; the field is not drawn",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:1710 collab_mode_applies_default_preset kind=ui owner=chat-live
#[test]
fn collab_mode_applies_default_preset() {
    kit::na(
        Owner::ChatLive,
        "asserts the Op::UserTurn collaboration_mode field and the active and current mode accessors after Enter; the assertions read state, not the screen",
    );
}

// ORACLE chatwidget/tests/plan_mode.rs:1736 plan_update_renders_history_cell kind=ui owner=chat-live
#[test]
fn plan_update_renders_history_cell() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(PLAN_UPDATE)
        .open();
    s.assert_contains("Updated Plan");
    s.assert_contains("Explore codebase");
    s.assert_contains("Implement feature");
    s.assert_contains("Write tests");
}

// ---- oracle: chatwidget/tests/replay_render_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/replay_render_tests.rs:7 completed_replay_preserves_messages_and_draft_across_reconstruction kind=snap owner=chat-live
#[test]
fn completed_replay_preserves_messages_and_draft_across_reconstruction() {
    let mut first = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    first.paste("Unsent draft");
    first.feed(REPLAY_MESSAGES);
    first.assert_in_order(&["› Question", "• First answer", "• Second answer"]);
    first.assert_contains("Unsent draft");
    // The oracle builds the screen a second time and expects the same output.
    let mut second = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    second.paste("Unsent draft");
    second.feed(REPLAY_MESSAGES);
    second.assert_same_rows(&first.shot());
}
