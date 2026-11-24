//! Shard `chat_status_2` (screens S10, owner ui-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit;
use crate::kit::{Owner, Screen};

/// The session of `set_chatgpt_auth`: a ChatGPT login on a provider that needs the OpenAI auth
/// (the host's account read and its config), the model of the card, and the models the server lists.
const CHATGPT: &str = r#"{"step":"screen","dir":"status","ev":{"kind":"account","account":{"chat_gpt":{"email":"user@example.com","plan":"plus"}}}}
{"step":"screen","dir":"status","ev":{"kind":"catalog","models":[{"model":"gpt-5.5","display_name":"GPT-5.5","default_effort":"medium"}]}}
{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.5","cwd":"/tmp/project","account_usage":true}}}"#;

/// What the server answers `account/rateLimits/read` with: the `codex` bucket, `used` percent used.
fn limits(used: u8) -> String {
    format!(
        r#"{{"step":"respond","method":"account/rateLimits/read","result":{{"rateLimits":{{"limitId":"codex","primary":{{"usedPercent":{used},"windowDurationMins":300,"resetsAt":2000000000}}}},"rateLimitsByLimitId":{{"codex":{{"limitId":"codex","primary":{{"usedPercent":{used},"windowDurationMins":300,"resetsAt":2000000000}}}}}}}}}}"#
    )
}

/// `set_chatgpt_auth` on a Business plan (the plan with a thread usage): the account, the model
/// of the card, and the chat's facts (the usage page link, `account_usage`).
const BUSINESS: &str = r#"{"step":"screen","dir":"status","ev":{"kind":"account","account":{"chat_gpt":{"email":"user@example.com","plan":"business"}}}}
{"step":"screen","dir":"status","ev":{"kind":"catalog","models":[{"model":"gpt-5.5","display_name":"GPT-5.5","default_effort":"medium"}]}}
{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.5","cwd":"/tmp/project","account_usage":true}}}"#;

/// What the server answers `account/usage/read` of the live thread with: `usage` is the
/// `threadUsage` object without its thread id.
fn thread_usage(usage: &str) -> String {
    let usage = usage.replacen('{', r#"{"threadId":"thread-1","#, 1);
    format!(
        r#"{{"step":"respond","method":"account/usage/read","result":{{"summary":{{"lifetimeTokens":null,"peakDailyTokens":null,"longestRunningTurnSec":null,"currentStreakDays":null,"longestStreakDays":null}},"dailyUsageBuckets":null,"threadUsage":{usage}}}}}"#
    )
}

/// 50 credits, ~$1.82, and the groups of the oracle's breakdown test.
const BREAKDOWN: &str = r#"{"estimatedUsageCreditsMicros":50000000,"estimatedUsageUsdMicros":1820000,"groups":[{"model":"gpt-5.4","reasoningEffort":"high","speed":"fast","estimatedUsageCreditsMicros":40000000,"netNewInputTokens":80,"cachedInputTokens":20,"inputTokens":100,"outputTokens":40,"totalTokens":140},{"model":"gpt-5-mini","reasoningEffort":"medium","speed":"standard","estimatedUsageCreditsMicros":10000000,"netNewInputTokens":15,"cachedInputTokens":5,"inputTokens":20,"outputTokens":10,"totalTokens":30}]}"#;

/// 50 credits and no dollars, one group.
const CREDITS_ONLY: &str = r#"{"estimatedUsageCreditsMicros":50000000,"estimatedUsageUsdMicros":null,"groups":[{"model":"gpt-5.4","reasoningEffort":"high","speed":"fast","estimatedUsageCreditsMicros":50000000,"netNewInputTokens":80,"cachedInputTokens":20,"inputTokens":100,"outputTokens":40,"totalTokens":140}]}"#;

/// The screen row the `Thread usage:` rows start at.
fn usage_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains("Thread usage:"))
        .unwrap_or_else(|| panic!("no `Thread usage:` row:\n{}", s.text()))
}

/// How many rows of the screen say `text`.
// ---- oracle: chatwidget/tests/status_and_layout.rs (47 screen-level tests here; 40 state tests left to ui-pane)
// ORACLE chatwidget/tests/status_and_layout.rs:3171 unsupported_code_mode_warning_renders_as_warning_history_cell kind=snap owner=ui-pane
#[test]
fn unsupported_code_mode_warning_renders_as_warning_history_cell() {
    kit::pending(
        Owner::UiPane,
        "the warning notification has no cell on the thread: `Warning` events are not applied to the transcript yet (chunk-live's ThreadModel apply; cells chunk draws it)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: handle_warning(the Code Mode warning for gpt-5.4): one warning history cell
    s.assert_snap("unsupported_code_mode_warning");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3187 repeated_model_metadata_warning_is_hidden_for_same_slug kind=ui owner=ui-pane
#[test]
fn repeated_model_metadata_warning_is_hidden_for_same_slug() {
    kit::pending(
        Owner::UiPane,
        "the warning notification has no cell on the thread: `Warning` events are not applied to the transcript yet (chunk-live's ThreadModel apply; cells chunk draws it)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: the same model metadata warning for unknown-model twice: one cell
    s.assert_contains("unknown-model");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3215 status_line_invalid_items_warn_once kind=ui owner=ui-pane
#[test]
fn status_line_invalid_items_warn_once() {
    kit::pending(
        Owner::UiPane,
        "an unknown id in `status_line` / `terminal_title` is dropped silently: the once-only warning cell needs the chat `apply` of the status event (core/src/screens/chat.rs, chunk-live's) to push one notice cell per distinct invalid list",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: status_line = model_name, bogus_item, lines_changed, bogus_item; refresh twice
    s.assert_contains("bogus_item");
    // the second refresh adds no warning cell: the transcript keeps one
}

// ORACLE chatwidget/tests/status_and_layout.rs:3243 status_line_hostname_renders_current_machine_hostname kind=ui owner=ui-pane
#[test]
fn status_line_hostname_renders_current_machine_hostname() {
    // oracle: `hostname` is a valid status line item and renders the machine's name, with no
    // warning cell. The host reads the name (`os_host_name`, host/src/facts.rs) into the fact the
    // script seeds; the page draws the fact.
    let s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/hostname.jsonl"))
        .open();
    s.assert_has_row_trimmed("devbox.local");
    s.assert_absent("invalid");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3258 status_line_context_used_renders_labeled_percent kind=ui owner=ui-pane
#[test]
fn status_line_context_used_renders_labeled_percent() {
    let s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/context_used.jsonl"))
        .open();
    s.assert_has_row_trimmed("Context 0% used");
    s.assert_absent("invalid");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3273 status_line_context_remaining_renders_labeled_percent kind=ui owner=ui-pane
#[test]
fn status_line_context_remaining_renders_labeled_percent() {
    let s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!(
            "fixtures/chat_status_2/context_remaining.jsonl"
        ))
        .open();
    s.assert_has_row_trimmed("Context 100% left");
    s.assert_absent("invalid");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3291 status_line_legacy_context_usage_renders_context_used_percent kind=ui owner=ui-pane
#[test]
fn status_line_legacy_context_usage_renders_context_used_percent() {
    let s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/context_usage.jsonl"))
        .open();
    s.assert_has_row_trimmed("Context 0% used");
    s.assert_absent("invalid");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3306 status_line_estimated_thread_cost_fetches_and_renders_backend_estimate kind=ui owner=ui-pane
#[test]
fn status_line_estimated_thread_cost_fetches_and_renders_backend_estimate() {
    // a thread with backend auth on a Business plan, status_line = estimated-thread-cost; the
    // usage answer is ~$1.82 (estimated_usage_usd_micros 1_820_000); an idle refresh polls nothing
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/estimated_cost.jsonl"))
        .open();
    s.wait_for("~$1.82");
    s.assert_has_row_trimmed("~$1.82");
    s.wait(120_000);
    s.assert_requests_of("account/usage/read", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:3343 status_line_thread_credits_fetches_and_renders_fractional_credits kind=ui owner=ui-pane
#[test]
fn status_line_thread_credits_fetches_and_renders_fractional_credits() {
    // status_line = thread-credits; the usage answer is 5.24 credits, shown as 5.2
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/thread_credits.jsonl"))
        .open();
    s.wait_for("5.2 credits");
    s.assert_has_row_trimmed("5.2 credits");
    s.wait(120_000);
    s.assert_requests_of("account/usage/read", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:3453 terminal_title_thread_usage_fetches_and_renders_without_status_line kind=ui owner=ui-pane
#[test]
fn terminal_title_thread_usage_fetches_and_renders_without_status_line() {
    // status_line empty, terminal_title = thread-credits, estimated-thread-cost; usage 5.2 credits, ~$0.21
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/title_usage.jsonl"))
        .open();
    s.wait(200);
    s.assert_title("5.2 credits | ~$0.21");
    s.assert_absent("5.2 credits");
    s.assert_requests_of("account/usage/read", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:3905 status_line_estimated_thread_cost_footer_snapshot kind=snap owner=ui-pane snap=status_line_estimated_thread_cost_footer
#[test]
fn status_line_estimated_thread_cost_footer_snapshot() {
    kit::pending(
        Owner::UiPane,
        "the hint row differs in every footer snapshot of the shard: the oracle's chat widget has agents navigation off (the app calls `enable_agents_navigation`), ours always has the command center and reads `← for agents · ? for shortcuts` (needs a kit option or a StatusFacts flag for it)",
    );
    // status_line = model-with-reasoning, thread-credits, estimated-thread-cost; usage 5.2 credits, ~$0.21
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!("fixtures/chat_status_2/footer_usage.jsonl"))
        .open();
    s.wait_for("5.2 credits");
    s.assert_snap("status_line_estimated_thread_cost_footer");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3950 status_line_workspace_headline_renders_cached_value kind=ui owner=ui-pane
#[test]
fn status_line_workspace_headline_renders_cached_value() {
    let s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/headline.jsonl"))
        .open();
    s.assert_has_row_trimmed("Workspace maintenance starts at 5pm");
    s.assert_absent("invalid");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4210 interrupted_turn_clears_visible_running_hook kind=snap owner=ui-pane snap=interrupted_turn_clears_visible_running_hook
#[test]
fn interrupted_turn_clears_visible_running_hook() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: turn started, hook started (run post-tool-use), turn interrupted
    s.assert_snap("interrupted_turn_clears_visible_running_hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4237 completed_turn_clears_visible_running_hook kind=snap owner=ui-pane snap=completed_turn_clears_visible_running_hook
#[test]
fn completed_turn_clears_visible_running_hook() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: turn started, hook started, turn completed
    s.assert_snap("completed_turn_clears_visible_running_hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4264 status_line_processing_tier_updates_with_selection kind=snap owner=ui-pane
#[test]
fn status_line_processing_tier_updates_with_selection() {
    // The oracle's last step runs the `remote-only-provider` (not in `config.model_providers`),
    // which offers no Flex tier: the provider table is no fact here, only the provider's id is.
    kit::pending(
        Owner::UiPane,
        "the provider table is no fact: `remote-only-provider` (an id outside `config.model_providers`) shows `Standard` in the oracle, `Flex` here; StatusFacts carries the provider's id only (host feed, chunk-live's)",
    );
    let status = |model: &str, tier: Option<&str>, fast_mode: bool, provider: &str| {
        let tier = tier.map_or(String::new(), |tier| format!(r#""service_tier":"{tier}","#));
        format!(
            r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"{model}",{tier}"fast_mode":{fast_mode},"provider":"{provider}","items":["fast-mode"],"terminal_title":["fast-mode"]}}}}}}"#
        )
    };
    let line = |s: &Screen, text: &str| {
        s.assert_has_row_trimmed(text);
    };
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/fast_tier.jsonl"))
        .open();
    line(&s, "Standard");
    s.feed(&status("gpt-5.4", Some("priority"), true, "openai"));
    line(&s, "fast");
    s.assert_title("fast");
    s.feed(&status("gpt-5.4", Some("priority"), false, "openai"));
    line(&s, "Standard");
    s.assert_title("Standard");
    s.feed(&status("gpt-5.2", Some("flex"), true, "openai"));
    line(&s, "Flex");
    s.feed(&status("gpt-5.2", Some("flex"), true, "amazon-bedrock"));
    line(&s, "Standard");
    // then the provider the oracle does not know: `Standard`
    s.feed(&status(
        "gpt-5.2",
        Some("flex"),
        true,
        "remote-only-provider",
    ));
    line(&s, "Standard");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4319 status_line_fast_mode_footer_snapshot kind=snap owner=ui-pane snap=status_line_fast_mode_footer
#[test]
fn status_line_fast_mode_footer_snapshot() {
    let s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!("fixtures/chat_status_2/fast_mode.jsonl"))
        .open();
    s.assert_snap("status_line_fast_mode_footer");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4375 terminal_title_model_updates_on_model_change_without_manual_refresh kind=snap owner=ui-pane snap=terminal_title_model_display_name
#[test]
fn terminal_title_model_updates_on_model_change_without_manual_refresh() {
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/title_model.jsonl"))
        .open();
    s.assert_title("GPT-5.5");
    // the model changes to gpt-5.2 and the title follows without a refresh
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.2","terminal_title":["model"]}}}"#);
    s.assert_title("gpt-5.2");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4391 status_line_and_terminal_title_reasoning_render_only_effort kind=ui owner=ui-pane
#[test]
fn status_line_and_terminal_title_reasoning_render_only_effort() {
    // oracle: the `reasoning` item is the effort alone on both surfaces (the Fast tier is no part of it).
    let s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/reasoning_only.jsonl"))
        .open();
    s.assert_has_row_trimmed("xhigh");
    s.assert_title("xhigh");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4444 status_line_model_with_reasoning_plan_mode_footer_snapshot kind=snap owner=ui-pane snap=status_line_model_with_reasoning_plan_mode_footer
#[test]
fn status_line_model_with_reasoning_plan_mode_footer_snapshot() {
    let s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!("fixtures/chat_status_2/model_plan.jsonl"))
        .open();
    s.assert_snap("status_line_model_with_reasoning_plan_mode_footer");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4471 renamed_thread_footer_title_snapshot kind=snap owner=ui-pane snap=renamed_thread_footer_title
#[test]
fn renamed_thread_footer_title_snapshot() {
    // oracle: the thread is renamed; the status line shows the new title after the model.
    let s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!("fixtures/chat_status_2/renamed_thread.jsonl"))
        .open();
    s.assert_snap("renamed_thread_footer_title");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4509 status_line_model_with_reasoning_speed_footer_snapshots kind=snap owner=ui-pane snap=status_line_model_with_reasoning_fast_footer snap=status_line_model_with_reasoning_ultrafast_footer
#[test]
fn status_line_model_with_reasoning_speed_footer_snapshots() {
    let status = |tier: &str, items: &str| {
        format!(
            r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.4","effort":"xhigh","service_tier":"{tier}","cwd":"/tmp/project","items":{items},"terminal_title":["model-with-reasoning"]}}}}}}"#
        )
    };
    let all = r#"["model-with-reasoning","context-used","current-dir"]"#;
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!("fixtures/chat_status_2/model_speed.jsonl"))
        .open();
    s.assert_title("gpt-5.4 xhigh Fast");
    s.assert_snap("status_line_model_with_reasoning_fast_footer");
    // then the ultrafast tier on the same chat
    s.feed(&status("ultrafast", all));
    s.assert_title("gpt-5.4 xhigh Ultrafast");
    s.assert_snap("status_line_model_with_reasoning_ultrafast_footer");
    s.feed(&status("flex", r#"["model-with-reasoning"]"#));
    s.assert_has_row_trimmed("gpt-5.4 xhigh flex");
    s.feed(&status("low_latency", r#"["model-with-reasoning"]"#));
    s.assert_has_row_trimmed("gpt-5.4 xhigh low latency");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4583 status_line_model_with_reasoning_context_remaining_footer_snapshot kind=snap owner=ui-pane snap=status_line_model_with_reasoning_context_remaining_footer
#[test]
fn status_line_model_with_reasoning_context_remaining_footer_snapshot() {
    let s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!(
            "fixtures/chat_status_2/model_context_remaining.jsonl"
        ))
        .open();
    s.assert_snap("status_line_model_with_reasoning_context_remaining_footer");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4617 status_line_goal_active_token_budget_footer_snapshot kind=snap owner=ui-pane snap=status_line_goal_active_token_budget_footer
#[test]
fn status_line_goal_active_token_budget_footer_snapshot() {
    let s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!("fixtures/chat_status_2/goal_active.jsonl"))
        .open();
    s.assert_snap("status_line_goal_active_token_budget_footer");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4654 status_line_goal_complete_elapsed_footer_snapshot kind=snap owner=ui-pane snap=status_line_goal_complete_elapsed_footer
#[test]
fn status_line_goal_complete_elapsed_footer_snapshot() {
    let s = Screen::build(Owner::UiPane)
        .size(80, 6)
        .script(include_str!("fixtures/chat_status_2/goal_complete.jsonl"))
        .open();
    s.assert_snap("status_line_goal_complete_elapsed_footer");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4847 goal_status_indicator_line_formats_goal_text kind=ui owner=ui-pane
#[test]
fn goal_status_indicator_line_formats_goal_text() {
    // (status, token budget, tokens used, seconds used, the indicator's line); the oracle builds
    // the indicator by hand, here a goal of that status makes it. `Goal achieved` without a usage
    // has no goal behind it: a finished goal always has its time or tokens.
    let cases = [
        (
            "active",
            Some(5_000),
            4_000,
            1_800,
            "Pursuing goal (4K / 5K)",
        ),
        (
            "budgetLimited",
            Some(5_000),
            4_000,
            1_800,
            "Goal unmet (4K / 5K tokens)",
        ),
        (
            "paused",
            Some(5_000),
            4_000,
            1_800,
            "Goal paused (/goal resume)",
        ),
        (
            "blocked",
            Some(5_000),
            4_000,
            1_800,
            "Goal stalled (/goal resume)",
        ),
        (
            "usageLimited",
            Some(5_000),
            4_000,
            1_800,
            "Goal hit usage limits (/goal resume)",
        ),
        ("budgetLimited", None, 4_000, 1_800, "Goal abandoned"),
        (
            "complete",
            None,
            4_000,
            10 * 3600 + 12 * 60,
            "Goal achieved (10h 12m)",
        ),
    ];
    for (status, budget, used, secs, expected) in cases {
        let budget = budget.map_or("null".to_string(), |b: i64| b.to_string());
        let mut s = Screen::build(Owner::UiPane)
            .size(100, 24)
            .script(include_str!("fixtures/chat_status_2/goal_base.jsonl"))
            .open();
        s.feed(&format!(
            r#"{{"step":"notify","message":{{"method":"thread/goal/updated","params":{{"threadId":"thread-1","turnId":null,"goal":{{"threadId":"thread-1","objective":"Keep improving the benchmark","status":"{status}","tokenBudget":{budget},"tokensUsed":{used},"timeUsedSeconds":{secs},"createdAt":0,"updatedAt":0}}}}}}}}"#
        ));
        s.assert_contains(expected);
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:4913 runtime_metrics_websocket_timing_logs_and_final_separator_sums_totals kind=ui owner=ui-pane
#[test]
fn runtime_metrics_websocket_timing_logs_and_final_separator_sums_totals() {
    kit::pending(
        Owner::UiPane,
        "the runtime metrics WebSocket timing lines and the final separator totals are not built in the chat",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: runtime metrics on; two WebSocket timing logs (TTFT 120ms then 80ms, TBT 50ms), then the turn completes
    s.assert_contains("WebSocket timing:");
    s.assert_contains("TTFT: 80ms (iapi)");
    s.assert_contains("TBT: 50ms (service)");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4959 multiple_agent_messages_in_single_turn_emit_multiple_headers kind=ui owner=ui-pane
#[test]
fn multiple_agent_messages_in_single_turn_emit_multiple_headers() {
    kit::pending(
        Owner::UiPane,
        "reasoning and answer streaming headers, prose previews and per-message headers are not built in the chat skeleton",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: one turn with two agent messages, 'First message' then 'Second message'
    s.assert_in_order(&["First message", "Second message"]);
}

// ORACLE chatwidget/tests/status_and_layout.rs:4998 final_reasoning_then_message_without_deltas_are_rendered kind=snap owner=ui-pane snap=final_reasoning_then_message_without_deltas_are_rendered
#[test]
fn final_reasoning_then_message_without_deltas_are_rendered() {
    kit::pending(
        Owner::UiPane,
        "reasoning and answer streaming headers, prose previews and per-message headers are not built in the chat skeleton",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a final reasoning item, then the final message, without deltas
    s.assert_snap("final_reasoning_then_message_without_deltas_are_rendered");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5023 deltas_then_same_final_message_are_rendered_snapshot kind=snap owner=ui-pane snap=deltas_then_same_final_message_are_rendered_snapshot
#[test]
fn deltas_then_same_final_message_are_rendered_snapshot() {
    kit::pending(
        Owner::UiPane,
        "reasoning and answer streaming headers, prose previews and per-message headers are not built in the chat skeleton",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: reasoning deltas and a final; agent message deltas, then the same final message
    s.assert_snap("deltas_then_same_final_message_are_rendered_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5057 unterminated_prose_is_visible_before_completion kind=snap owner=ui-pane snap=unterminated_agent_prose_preview snap=unterminated_plan_prose_preview
#[test]
fn unterminated_prose_is_visible_before_completion() {
    kit::pending(
        Owner::UiPane,
        "reasoning and answer streaming headers, prose previews and per-message headers are not built in the chat skeleton",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: collaboration mode off, then streamed agent prose without a newline, before completion
    s.assert_snap("unterminated_agent_prose_preview");
    // then the same for the plan mode prose (plan mask on, a new chat state)
    s.assert_snap("unterminated_plan_prose_preview");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5233 reasoning_delta_does_not_double_schedule_visible_status_redraw kind=ui owner=ui-pane
#[test]
fn reasoning_delta_does_not_double_schedule_visible_status_redraw() {
    kit::pending(
        Owner::UiPane,
        "reasoning and answer streaming headers, prose previews and per-message headers are not built in the chat skeleton",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a task started; reasoning delta '**Checking files**' sets the status header
    s.assert_contains("Checking files");
    // the draw scheduling (frame requests) is not drawn: that half is not asserted here
}

// ORACLE chatwidget/tests/status_and_layout.rs:5257 reasoning_delta_restores_recreated_status_indicator_header kind=snap owner=ui-pane snap=reasoning_delta_restores_recreated_status_indicator
#[test]
fn reasoning_delta_restores_recreated_status_indicator_header() {
    kit::pending(
        Owner::UiPane,
        "reasoning and answer streaming headers, prose previews and per-message headers are not built in the chat skeleton",
    );
    let s = Screen::build(Owner::UiPane).size(80, 8).live().open();
    // seed: a task started, the status indicator recreated, reasoning header 'Checking files and preparing a response'
    s.assert_snap("reasoning_delta_restores_recreated_status_indicator");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5296 user_prompt_submit_app_server_hook_notifications_render_snapshot kind=snap owner=ui-pane snap=user_prompt_submit_app_server_hook_notifications_render_snapshot
#[test]
fn user_prompt_submit_app_server_hook_notifications_render_snapshot() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: hook/started and hook/completed notifications of UserPromptSubmit
    s.assert_snap("user_prompt_submit_app_server_hook_notifications_render_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5372 hook_events_render_consistently kind=ui owner=ui-pane
#[test]
fn hook_events_render_consistently() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    for status_message in [
        "cleaning up the interrupted turn", // Interrupt
        "warming the shell",                // PreToolUse
        "warming the shell",                // PostToolUse
        "warming the shell",                // SessionStart
    ] {
        // seed: the hook of the case, started with that status message
        let s = Screen::build(Owner::UiPane).live().open();
        s.assert_contains(status_message);
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:5400 quiet_hook_cleanup_starts_when_the_hook_is_revealed kind=ui owner=ui-pane
#[test]
fn quiet_hook_cleanup_starts_when_the_hook_is_revealed() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: PostToolUse hook 'checking output policy' started, revealed (immediately or after a delayed redraw), then completed
    // the linger expires: the hook row is gone and no history is written
    s.assert_absent("checking output policy");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5440 blocked_and_failed_hooks_render_feedback_and_errors kind=snap owner=ui-pane snap=hook_blocked_failed_feedback_history_snapshot
#[test]
fn blocked_and_failed_hooks_render_feedback_and_errors() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: two completed hooks: one blocked with feedback, one failed with exit code 7
    s.assert_in_order(&[
        "Blocked by hook",
        "run tests before touching the fixture",
        "Hook failed",
        "hook exited with code 7",
    ]);
    s.assert_snap("hook_blocked_failed_feedback_history_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5484 completed_hook_with_output_flushes_immediately kind=snap owner=ui-pane snap=completed_hook_with_output_flushes_immediately_snapshot
#[test]
fn completed_hook_with_output_flushes_immediately() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a hook started and completed with output; the output flushes into history at once
    s.assert_snap("completed_hook_with_output_flushes_immediately_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5523 completed_hook_output_precedes_following_assistant_message kind=snap owner=ui-pane snap=completed_hook_output_precedes_following_assistant_message_snapshot
#[test]
fn completed_hook_output_precedes_following_assistant_message() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a completed hook with output, then the assistant message
    s.assert_snap("completed_hook_output_precedes_following_assistant_message_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5577 completed_same_id_hook_output_survives_restart kind=snap owner=ui-pane snap=completed_same_id_hook_output_survives_restart_snapshot
#[test]
fn completed_same_id_hook_output_survives_restart() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: hook completed, same id started again (restart), with stop output 'continue with more context'
    s.assert_in_order(&["Hook stopped", "continue with more context"]);
    s.assert_snap("completed_same_id_hook_output_survives_restart_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5627 running_hooks_fit_around_background_activity_and_finish_without_history_snapshot kind=snap owner=ui-pane snap=long_running_hook_below_background_activity_row snap=mixed_running_hooks_share_background_activity_row snap=running_hooks_share_background_activity_row
#[test]
fn running_hooks_fit_around_background_activity_and_finish_without_history_snapshot() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items); the exec and status layout of the chat is not built (the skeleton has no status row, footer or exec cell)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a task started with 1 background terminal; long running hook 'checking command policy' and a mixed set of hooks
    s.assert_contains("1 background terminal running");
    s.assert_absent("Running hooks");
    s.assert_absent("checking command policy");
    s.assert_snap("long_running_hook_below_background_activity_row");
    // then the mixed running hooks share the background activity row (a second state)
    s.assert_snap("mixed_running_hooks_share_background_activity_row");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5694 session_end_hook_has_standalone_activity_row kind=snap owner=ui-pane snap=session_end_hook_activity snap=session_end_hook_activity_without_message
#[test]
fn session_end_hook_has_standalone_activity_row() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a SessionEnd hook started and completed, with and without a message
    s.assert_snap("session_end_hook_activity");
    // without a message: a second state
    s.assert_snap("session_end_hook_activity_without_message");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5732 overlapping_hook_live_cell_tracks_parallel_quiet_hooks kind=snap owner=ui-pane snap=overlapping_hook_live_cell_snapshot
#[test]
fn overlapping_hook_live_cell_tracks_parallel_quiet_hooks() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a task started; two parallel quiet hooks start and complete; the status header stays 'Thinking'
    s.assert_contains("Thinking");
    s.assert_snap("overlapping_hook_live_cell_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5806 running_hook_does_not_displace_active_exec_cell kind=snap owner=ui-pane snap=hook_runs_while_exec_active_snapshot
#[test]
fn running_hook_does_not_displace_active_exec_cell() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items); the exec and status layout of the chat is not built (the skeleton has no status row, footer or exec cell)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: an active exec cell, then a running hook started and completed
    s.assert_snap("hook_runs_while_exec_active_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5913 stopped_hook_hides_model_context_and_preserves_stop_reason_snapshot kind=snap owner=ui-pane snap=stopped_hook_hides_model_context_and_preserves_stop_reason
#[test]
fn stopped_hook_hides_model_context_and_preserves_stop_reason_snapshot() {
    kit::pending(
        Owner::UiPane,
        "hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: a stopped hook with a stop reason 'continue with more context'; the model context is hidden
    s.assert_snap("stopped_hook_hides_model_context_and_preserves_stop_reason");
}

// ORACLE chatwidget/tests/status_and_layout.rs:6029 chatwidget_exec_and_status_layout_vt100_snapshot kind=snap owner=ui-pane snap=chatwidget_exec_and_status_layout_vt100_snapshot
#[test]
fn chatwidget_exec_and_status_layout_vt100_snapshot() {
    kit::pending(
        Owner::UiPane,
        "the exec and status layout of the chat is not built (the skeleton has no status row, footer or exec cell)",
    );
    let s = Screen::build(Owner::UiPane).live().open();
    // seed: exec cells and the status layout (vt100 rendering of the chat)
    s.assert_snap("chatwidget_exec_and_status_layout_vt100_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:6130 chatwidget_markdown_code_blocks_vt100_snapshot kind=snap owner=ui-pane snap=chatwidget_markdown_code_blocks_vt100_snapshot
#[test]
fn chatwidget_markdown_code_blocks_vt100_snapshot() {
    kit::pending(
        Owner::UiPane,
        "the exec and status layout of the chat is not built (the skeleton has no status row, footer or exec cell)",
    );
    let s = Screen::build(Owner::UiPane).size(80, 50).live().open();
    // seed: markdown with code blocks, rendered as the chat
    s.assert_snap("chatwidget_markdown_code_blocks_vt100_snapshot");
}

// ORACLE chatwidget/tests/status_and_layout.rs:6211 chatwidget_tall kind=snap owner=ui-pane snap=chatwidget_tall
#[test]
fn chatwidget_tall() {
    kit::pending(
        Owner::UiPane,
        "the exec and status layout of the chat is not built (the skeleton has no status row, footer or exec cell)",
    );
    let s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    // seed: the chat at 80 columns, 24 rows, tall transcript
    s.assert_snap("chatwidget_tall");
}

// ---- oracle: chatwidget/tests/status_command_tests.rs (11 screen-level tests here; 5 state tests left to ui-pane)
// ORACLE chatwidget/tests/status_command_tests.rs:9 status_command_renders_immediately_and_refreshes_rate_limits_for_chatgpt_auth kind=ui owner=ui-pane
#[test]
fn status_command_renders_immediately_and_refreshes_rate_limits_for_chatgpt_auth() {
    let mut s = Screen::build(Owner::UiPane).live().open();
    s.feed(CHATGPT).feed(&limits(92));
    // the read the app makes on its own once it knows the account is not `/status`'s
    let startup = s.requests_of("account/rateLimits/read");
    s.type_text("/status");
    s.keys("enter");
    // the card is in the transcript at once, no dialog, and says nothing of a transient refresh
    s.assert_contains("OpenAI Codex");
    s.assert_contains("Model:");
    s.assert_absent("refreshing limits");
    // the account's limits are asked once for it, and the card shows what came back
    s.assert_requests_of("account/rateLimits/read", startup + 1);
    s.wait_for("8% left");
}

// ORACLE chatwidget/tests/status_command_tests.rs:35 status_command_refresh_updates_cached_limits_for_future_status_outputs kind=ui owner=ui-pane
#[test]
fn status_command_refresh_updates_cached_limits_for_future_status_outputs() {
    // tall enough for both cards: the first card's `8% left` stays on the screen
    let mut s = Screen::build(Owner::UiPane).height(60).live().open();
    s.feed(CHATGPT).feed(&limits(92));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("8% left");
    s.assert_rows_with("8% left", 1);
    // the refresh updated the cache: the next card shows the limits as they are now
    s.feed(&limits(50));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("50% left");
    s.assert_contains("8% left");
}

// ORACLE chatwidget/tests/status_command_tests.rs:78 status_command_renders_immediately_without_rate_limit_refresh kind=ui owner=ui-pane
#[test]
fn status_command_renders_immediately_without_rate_limit_refresh() {
    kit::na(
        Owner::UiPane,
        "asserts that no RefreshRateLimits event is sent for /status on a non-ChatGPT session, not visible",
    );
}

// ORACLE chatwidget/tests/status_command_tests.rs:92 status_command_uses_catalog_default_reasoning_when_config_empty kind=ui owner=ui-pane
#[test]
fn status_command_uses_catalog_default_reasoning_when_config_empty() {
    let mut s = Screen::build(Owner::UiPane).live().open();
    // the thread names no effort and the config sets none: the catalog's default applies
    s.feed(CHATGPT);
    s.type_text("/status");
    s.keys("enter");
    s.assert_contains("GPT-5.5 (reasoning medium, summaries auto)");
}

// ORACLE chatwidget/tests/status_command_tests.rs:111 status_command_renders_native_and_foreign_instruction_sources kind=ui owner=ui-pane
#[test]
fn status_command_renders_native_and_foreign_instruction_sources() {
    // the server's `thread/start` names a native AGENTS.md of the project and a foreign path
    let start = serde_json::json!({
        "thread": {
            "id": "thread-1", "environments": null, "extra": null, "sessionId": "thread-1",
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
            "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": "/tmp/project",
            "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
            "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null,
            "turns": []
        },
        "model": "gpt-5.5", "modelProvider": "openai", "serviceTier": null,
        "cwd": "/tmp/project",
        "instructionSources": ["/tmp/project/AGENTS.md", "C:\\remote\\AGENTS.md"],
        "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    });
    let mut s = Screen::build(Owner::UiPane)
        .thread_start(start)
        .live()
        .open();
    s.type_text("/status");
    s.keys("enter");
    s.assert_contains(r"AGENTS.md, C:\remote\AGENTS.md");
    s.assert_absent("Agents.md  <none>");
}

// ORACLE chatwidget/tests/status_command_tests.rs:148 status_command_overlapping_refreshes_update_matching_cells_only kind=ui owner=ui-pane
#[test]
fn status_command_overlapping_refreshes_update_matching_cells_only() {
    // two cards of this height stand one above the other: both are on the screen
    let mut s = Screen::build(Owner::UiPane).height(60).live().open();
    s.feed(CHATGPT).feed(&limits(92));
    let startup = s.requests_of("account/rateLimits/read");
    s.type_text("/status");
    s.keys("enter");
    s.type_text("/status");
    s.keys("enter");
    // each card has its own refresh; no card is left waiting for a reply that went to the other
    s.assert_absent("refreshing limits");
    s.assert_requests_of("account/rateLimits/read", startup + 2);
    s.wait_for("8% left");
    s.assert_rows_with("8% left", 2);
}

// ORACLE chatwidget/tests/status_command_tests.rs:214 status_command_renders_immediately_and_updates_first_card_without_status_line kind=snap owner=ui-pane snap=status_command_estimated_thread_usage
#[test]
fn status_command_renders_immediately_and_updates_first_card_without_status_line() {
    let mut s = Screen::build(Owner::UiPane).size(90, 24).live().open();
    // ChatGPT auth on a Business plan, no status line; /status, then the thread usage answer
    // (50 credits, ~$1.82), then another /status that shows the figure there is
    s.feed(BUSINESS).feed(&thread_usage(BREAKDOWN));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("50 credits · ~$1.82");
    s.assert_contains("GPT-5.4 80%, GPT-5 Mini 20%");
    s.assert_contains("Medium 20%, High 80%");
    s.assert_contains("Fast mode 80%, Standard 20%");
    s.assert_contains("120 input (25 cached) + 50 output");
    s.assert_snap_rows_at("status_command_estimated_thread_usage", usage_row(&s));
}

// ORACLE chatwidget/tests/status_command_tests.rs:306 status_command_omits_thread_usage_for_unsupported_plans kind=ui owner=ui-pane
#[test]
fn status_command_omits_thread_usage_for_unsupported_plans() {
    for plan in [
        Some("enterprise"),
        Some("pro"),
        Some("edu"),
        Some("self_serve_business_usage_based"),
        Some("unknown"),
        None,
    ] {
        let mut s = Screen::build(Owner::UiPane).live().open();
        // ChatGPT auth on a plan with no thread usage; /status
        let plan = plan.map_or(String::new(), |plan| format!(r#","plan":"{plan}""#));
        s.feed(&BUSINESS.replace(r#","plan":"business""#, &plan))
            .feed(&thread_usage(BREAKDOWN));
        s.type_text("/status");
        s.keys("enter");
        s.assert_contains("Account:");
        s.assert_absent("Thread usage");
        s.assert_requests_of("account/usage/read", 0);
    }
}

// ORACLE chatwidget/tests/status_command_tests.rs:338 status_command_stays_visible_and_updates_after_thread_usage_retry kind=ui owner=ui-pane
#[test]
fn status_command_stays_visible_and_updates_after_thread_usage_retry() {
    let mut s = Screen::build(Owner::UiPane).height(90).live().open();
    // ChatGPT auth on a Business plan; /status, the usage read is refused, then /status again
    // with the usage answer: both cards show the figure
    s.feed(BUSINESS).feed(
        r#"{"step":"respond","method":"account/usage/read","error":{"message":"thread usage temporarily unavailable"}}"#,
    );
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("Account:");
    s.assert_absent("Thread usage");
    s.feed(&thread_usage(
        r#"{"estimatedUsageCreditsMicros":50000000,"estimatedUsageUsdMicros":1820000,"groups":[]}"#,
    ));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("50 credits · ~$1.82");
    s.assert_rows_with("50 credits · ~$1.82", 2);
}

// ORACLE chatwidget/tests/status_command_tests.rs:393 status_command_renders_credits_and_breakdowns_without_usd_estimate kind=snap owner=ui-pane snap=status_command_estimated_thread_usage_without_usd
#[test]
fn status_command_renders_credits_and_breakdowns_without_usd_estimate() {
    let mut s = Screen::build(Owner::UiPane).size(90, 24).live().open();
    // ChatGPT auth on a Business plan; /status with a credits-only usage answer (50 credits)
    s.feed(BUSINESS).feed(&thread_usage(CREDITS_ONLY));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("50 credits");
    s.assert_contains("GPT-5.4 100%");
    s.assert_absent("~$");
    s.assert_contains("High 100%");
    s.assert_contains("Fast mode 100%");
    s.assert_contains("100 input (20 cached) + 40 output");
    s.assert_snap_rows_at(
        "status_command_estimated_thread_usage_without_usd",
        usage_row(&s),
    );
}

// ORACLE chatwidget/tests/status_command_tests.rs:462 status_command_drops_stale_usd_when_updated_usage_has_only_credits kind=ui owner=ui-pane
#[test]
fn status_command_drops_stale_usd_when_updated_usage_has_only_credits() {
    let mut s = Screen::build(Owner::UiPane).height(120).live().open();
    // ChatGPT auth on a Business plan; /status with 40 credits · ~$1.82, /status again (the card
    // starts from that figure) and a credits-only answer of 50 credits, then a third /status
    s.feed(BUSINESS).feed(&thread_usage(
        r#"{"estimatedUsageCreditsMicros":40000000,"estimatedUsageUsdMicros":1820000,"groups":[]}"#,
    ));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("40 credits · ~$1.82");
    s.feed(&thread_usage(
        r#"{"estimatedUsageCreditsMicros":50000000,"estimatedUsageUsdMicros":null,"groups":[]}"#,
    ));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("50 credits");
    s.type_text("/status");
    s.keys("enter");
    s.wait(200);
    // the first card keeps its figure; the later two show the credits alone
    s.assert_rows_with("40 credits · ~$1.82", 1);
    s.assert_rows_with("50 credits", 2);
}

// ---- oracle: chatwidget/tests/status_surface_command_tests.rs (1 screen-level tests here; 1 state tests left to ui-pane)
// ORACLE chatwidget/tests/status_surface_command_tests.rs:29 inline_status_surface_commands_are_session_only_and_atomic kind=snap owner=ui-pane
#[test]
fn inline_status_surface_commands_are_session_only_and_atomic() {
    kit::pending(
        Owner::UiPane,
        "the `/status` card is not built in the chat (the status screen exists only as a debug entry, and `/status` is not dispatched); the status line (`local_settings.tui.status_line` items) is not built in the chat, and the kit has no config seed for it",
    );
    let mut s = Screen::build(Owner::UiPane).live().open();
    // seed: /title and /statusline run inline in the chat (session-only, atomic)
    s.type_text("/statusline monthly-limit");
    s.keys("enter");
    // an ambiguous item is refused with a warning line in the transcript
    s.assert_contains(
        "Ambiguous status-line item 'monthly-limit'. Use 'five-hour-limit' or 'weekly-limit'.",
    );
    s.type_text("/statusline --colors --no-colors");
    s.keys("enter");
    s.assert_contains("Specify only one of --colors or --no-colors.");
}
