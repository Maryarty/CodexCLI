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

/// A background terminal starts (`begin_unified_exec_startup`: `sleep 2`).
const BACKGROUND: &str = r#"{"step":"notify","message":{"method":"item/started","params":{"item":{"type":"commandExecution","id":"call-1","pluginId":null,"scriptPath":null,"command":"bash -lc 'sleep 2'","cwd":"/tmp/project","processId":"proc-1","source":"unifiedExecStartup","status":"inProgress","commandActions":[],"aggregatedOutput":null,"exitCode":null,"durationMs":null,"startedAtMs":0,"completedAtMs":null},"threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;

/// The agent runs `echo done` (`begin_exec`).
const EXEC_BEGIN: &str = r#"{"step":"notify","message":{"method":"item/started","params":{"item":{"type":"commandExecution","id":"call-1","pluginId":null,"scriptPath":null,"command":"bash -lc 'echo done'","cwd":"/tmp/project","processId":null,"source":"agent","status":"inProgress","commandActions":[{"type":"unknown","command":"echo done"}],"aggregatedOutput":null,"exitCode":null,"durationMs":null,"startedAtMs":0,"completedAtMs":null},"threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;

/// It ends with exit 0 and the output `done` (`end_exec`).
const EXEC_END: &str = r#"{"step":"notify","message":{"method":"item/completed","params":{"item":{"type":"commandExecution","id":"call-1","pluginId":null,"scriptPath":null,"command":"bash -lc 'echo done'","cwd":"/tmp/project","processId":null,"source":"agent","status":"completed","commandActions":[{"type":"unknown","command":"echo done"}],"aggregatedOutput":"done","exitCode":0,"durationMs":5,"startedAtMs":null,"completedAtMs":null},"threadId":"thread-1","turnId":"turn-1","contentTokenCount":null,"completedAtMs":0}}}"#;

/// The turn thinks: a reasoning summary part began (`set_status_header("Thinking")`).
const THINKING: &str = r#"{"step":"notify","message":{"method":"item/reasoning/summaryPartAdded","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"reasoning-1","summaryIndex":0}}}"#;

/// The server completed an assistant message (`complete_assistant_message`).
fn assistant_message(id: &str, text: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"item/completed","params":{{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"item":{{"type":"agentMessage","id":"{id}","text":"{text}","phase":null,"moderation":null}}}}}}}}"#
    )
}

/// The screen row that says `text`.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.trim_end() == text)
        .unwrap_or_else(|| panic!("no `{text}` row:\n{}", s.text()))
}

/// The chat of the oracle's `make_chatwidget_manual`: agents navigation off, the status line of
/// its footer (the model and the directory).
const BASE: &str = r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}
{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","cwd":"/tmp/project","items":["model-with-reasoning","current-dir"]}}}
{"step":"seed","seed":"session","thread":"thread-1"}"#;

/// The turn `turn-1` under way (`chat.on_task_started()`).
const TURN: &str = r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#;

/// The hook run of the oracle's `hook_run_summary`: `status` is `running` or how it ended,
/// `message` its status message, `entries` the JSON of its output entries.
fn hook_run(id: &str, event: &str, status: &str, message: Option<&str>, entries: &str) -> String {
    let message = message.map_or("null".to_string(), |text| format!("{text:?}"));
    let (completed_at, duration) = if status == "running" {
        ("null", "null")
    } else {
        ("11", "10")
    };
    format!(
        r#""run":{{"id":"{id}","eventName":"{event}","handlerType":"command","executionMode":"sync","scope":"turn","sourcePath":"/tmp/hooks.json","source":"user","displayOrder":0,"status":"{status}","statusMessage":{message},"startedAt":1,"completedAt":{completed_at},"durationMs":{duration},"entries":{entries}}}"#
    )
}

/// The server's `hook/started` of a hook that says `message` (`handle_hook_started`).
fn hook_started(id: &str, event: &str, message: Option<&str>) -> String {
    let run = hook_run(id, event, "running", message, "[]");
    format!(
        r#"{{"step":"notify","message":{{"method":"hook/started","params":{{"threadId":"thread-1","turnId":null,{run}}}}}}}"#
    )
}

/// The server's `hook/completed` (`handle_hook_completed`): `status` is `completed`, `failed`,
/// `blocked` or `stopped`, `entries` are `(kind, text)` pairs.
fn hook_completed(
    id: &str,
    event: &str,
    status: &str,
    message: Option<&str>,
    entries: &[(&str, &str)],
) -> String {
    let entries: Vec<String> = entries
        .iter()
        .map(|(kind, text)| format!(r#"{{"kind":"{kind}","text":{text:?}}}"#))
        .collect();
    let run = hook_run(
        id,
        event,
        status,
        message,
        &format!("[{}]", entries.join(",")),
    );
    format!(
        r#"{{"step":"notify","message":{{"method":"hook/completed","params":{{"threadId":"thread-1","turnId":null,{run}}}}}}}"#
    )
}

/// A `hook/started` or `hook/completed` of the turn `turn-1` (the notification carries its turn).
fn in_turn(line: String) -> String {
    line.replace(r#""turnId":null"#, r#""turnId":"turn-1""#)
}

/// The end of the turn `turn-1` (`handle_turn_interrupted`, `handle_turn_completed`): `status`
/// is `interrupted` or `completed`.
fn turn_ended(status: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"turn/completed","params":{{"threadId":"thread-1","turn":{{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"{status}","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}}}}}"#
    )
}

/// The chat of the oracle's `make_chatwidget_manual`, no turn yet.
fn chat() -> Screen {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(BASE);
    s
}

/// A row as the oracle's tests normalise it (`normalize_working_snapshot`): the status row's
/// bullet blinks with the clock and its elapsed time grows with it, so both are the first
/// frame's (`• Working (0s`).
fn steady(row: &str) -> String {
    let row = row.trim_end();
    let row = match row.strip_prefix('◦') {
        Some(rest) => format!("•{rest}"),
        None => row.to_string(),
    };
    let Some(at) = row.find(" (") else {
        return row;
    };
    let tail = &row[at + 2..];
    let digits = tail.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && tail[digits..].starts_with("s ") {
        format!("{} (0s{}", &row[..at], &tail[digits + 1..])
    } else {
        row
    }
}

/// The renders of an oracle snapshot its test glued under labels of its own (`before interrupt:`,
/// `activity:`, `history:`; none for a snapshot of one render), compared with the screen's rows.
struct Frames {
    name: &'static str,
    labels: &'static [&'static str],
    /// The test's labels are headers the kit reads (`80 columns:`): each render is asked for by name.
    named: bool,
}

impl Frames {
    /// The rows of render `k` (from 0, in the order the test wrote them): the lines between the
    /// labels, the first label being the snapshot's own first line.
    fn rows(&self, k: usize) -> Vec<String> {
        if self.named {
            let label = self.labels[k].trim_end_matches(':');
            let mut rows: Vec<String> = kit::snap_rows(self.name, label)
                .iter()
                .map(|row| row.trim_end().to_string())
                .collect();
            while rows.last().is_some_and(String::is_empty) {
                rows.pop();
            }
            return rows;
        }
        // the whole snapshot: the kit cuts it at the labels it takes for headers and leaves them out
        let mut all: Vec<String> = Vec::new();
        for frame in kit::snap_frames(self.name) {
            if !frame.is_empty() {
                all.push(frame.clone());
            }
            all.extend(
                kit::snap_rows(self.name, &frame)
                    .iter()
                    .map(|row| row.trim_end().to_string()),
            );
        }
        let rows = if self.labels.is_empty() {
            all
        } else {
            let bare = |row: &str| row.trim_end_matches(':').to_string();
            let mut chunks: Vec<Vec<String>> = Vec::new();
            for row in all {
                if self.labels.iter().any(|label| bare(label) == bare(&row)) {
                    chunks.push(Vec::new());
                } else if let Some(last) = chunks.last_mut() {
                    last.push(row);
                }
            }
            chunks
                .into_iter()
                .nth(k)
                .unwrap_or_else(|| panic!("{}: no render {k}", self.name))
        };
        let mut rows = rows;
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    /// Render `k`, but its first `skip` rows, equals the screen's rows from `top` on (the status
    /// row's bullet and elapsed time aside, `steady`).
    fn region(&self, s: &Screen, k: usize, top: usize, skip: usize) {
        let want = self.rows(k).split_off(skip);
        let ours = s.rows();
        let got: Vec<String> = (0..want.len())
            .map(|at| steady(ours.get(top + at).map_or("", String::as_str)))
            .collect();
        kit::assert_same(
            got,
            want,
            &format!(
                "{} / render {k}: ours (left) differs from the oracle's (right):\n{}",
                self.name,
                s.text()
            ),
        );
    }

    /// Render `k` is the bottom pane: its first row is as many rows above the composer's as the
    /// oracle's render says.
    fn pane(&self, s: &Screen, k: usize) {
        self.pane_from(s, k, 0);
    }

    /// Render `k` is the chat widget alone: the bottom pane's rows, but the first `skip` rows of
    /// the oracle's render, equal ours. Those rows are its active cell, drawn flush above the pane;
    /// the app keeps the cell in its transcript.
    fn pane_from(&self, s: &Screen, k: usize, skip: usize) {
        let above = self
            .rows(k)
            .iter()
            .position(|row| row.starts_with("› "))
            .unwrap_or_else(|| panic!("{}: no composer row in render {k}", self.name));
        let composer = s
            .rows()
            .iter()
            .rposition(|row| row.starts_with("› "))
            .unwrap_or_else(|| panic!("no composer row:\n{}", s.text()));
        self.region(s, k, composer - (above - skip), skip);
    }

    /// Render `k` is history: it starts at the row `first` says.
    fn history(&self, s: &Screen, k: usize, first: &str) {
        let top = s
            .rows()
            .iter()
            .position(|row| row.trim_end() == first)
            .unwrap_or_else(|| panic!("no `{first}` row:\n{}", s.text()));
        self.region(s, k, top, 0);
    }
}

/// The screen row the `Thread usage:` rows start at.
fn usage_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains("Thread usage:"))
        .unwrap_or_else(|| panic!("no `Thread usage:` row:\n{}", s.text()))
}

// ---- oracle: chatwidget/tests/status_and_layout.rs (47 screen-level tests here; 40 state tests left to ui-pane)
// ORACLE chatwidget/tests/status_and_layout.rs:3171 unsupported_code_mode_warning_renders_as_warning_history_cell kind=snap owner=ui-pane
#[test]
fn unsupported_code_mode_warning_renders_as_warning_history_cell() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(&warning(
        "Code Mode is enabled in configuration, but model `gpt-5.4` does not advertise Code Mode support. This may degrade model performance. Disable `features.code_mode` and `features.code_mode_only`, or select a model whose metadata enables Code Mode.",
    ));
    s.keys("f2");
    s.assert_reads("model `gpt-5.4` does not advertise Code Mode support");
    s.assert_text_count("Code Mode is enabled in configuration", 1);
}

fn warning(message: &str) -> String {
    serde_json::json!({
        "step": "notify", "message": {"method": "warning", "params": {
            "threadId": "thread-1", "message": message
        }}
    })
    .to_string()
}

// ORACLE chatwidget/tests/status_and_layout.rs:3187 repeated_model_metadata_warning_is_hidden_for_same_slug kind=ui owner=ui-pane
#[test]
fn repeated_model_metadata_warning_is_hidden_for_same_slug() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    let same = warning(
        "Model metadata for `unknown-model` not found. Defaulting to fallback metadata; this can degrade performance and cause issues.",
    );
    s.feed(&format!("{same}\n{same}"));
    s.keys("f2");
    s.assert_text_count("Model metadata for `unknown-model`", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:3215 status_line_invalid_items_warn_once kind=ui owner=ui-pane
#[test]
fn status_line_invalid_items_warn_once() {
    let mut s = Screen::build(Owner::UiPane).size(110, 24).live().open();
    let invalid = r#"{"step":"screen","dir":"chat","ev":{"status":{"model":"gpt-5.4","items":["model-name","model_name","bogus_item","lines_changed","bogus_item"],"project_root":"workspace","terminal_title":["project","unknown-title","unknown-title"]}}}"#;
    s.feed(invalid);
    s.assert_title("workspace");
    s.feed(invalid);
    s.keys("ctrl+t");
    s.assert_text_count(
        r#"Ignored invalid status line items: "model_name", "bogus_item" and "lines_changed"."#,
        1,
    );
    s.assert_text_count(
        r#"Ignored invalid terminal title item: "unknown-title"."#,
        1,
    );
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
    // a task started, a PreToolUse hook that says what it checks, revealed; the interrupt
    // clears the run though the hook never ended
    let frames = Frames {
        name: "interrupted_turn_clears_visible_running_hook",
        labels: &["before interrupt:", "after interrupt:"],
        named: false,
    };
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(BASE).feed(TURN).feed(&hook_started(
        "pre-tool-use:0:/tmp/hooks.json",
        "preToolUse",
        Some("checking command policy"),
    ));
    s.wait(300);
    frames.pane(&s, 0);
    s.feed(&turn_ended("interrupted"));
    frames.pane(&s, 1);
    s.assert_absent("checking command policy");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4237 completed_turn_clears_visible_running_hook kind=snap owner=ui-pane snap=completed_turn_clears_visible_running_hook
#[test]
fn completed_turn_clears_visible_running_hook() {
    // the same with a hook that says nothing of itself (`Running hook`) and a completed turn
    let frames = Frames {
        name: "completed_turn_clears_visible_running_hook",
        labels: &["before completion:", "after completion:"],
        named: false,
    };
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(BASE).feed(TURN).feed(&hook_started(
        "post-tool-use:0:/tmp/hooks.json",
        "postToolUse",
        None,
    ));
    s.wait(300);
    frames.pane(&s, 0);
    s.feed(&turn_ended("completed"));
    frames.pane(&s, 1);
    s.assert_absent("Running hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:4264 status_line_processing_tier_updates_with_selection kind=snap owner=ui-pane
#[test]
fn status_line_processing_tier_updates_with_selection() {
    let status = |tier: &str, fast_mode: bool| {
        serde_json::json!({
            "step": "screen", "dir": "chat", "ev": {"status": {
                "model": "gpt-5.4", "service_tier": tier, "fast_mode": fast_mode,
                "provider": "openai", "provider_names": [{"id": "openai", "name": "OpenAI", "flex_fallback": true}],
                "items": ["fast-mode"], "terminal_title": ["fast-mode"],
            }}
        })
        .to_string()
    };
    let settings = |model: &str, provider: &str, tier: &str| {
        serde_json::json!({
            "step": "notify", "message": { "method": "thread/settings/updated", "params": {
                "threadId": "thread-1", "threadSettings": {
                    "cwd": "/tmp/project", "approvalPolicy": "on-request", "approvalsReviewer": "user",
                    "sandboxPolicy": {"type": "dangerFullAccess"}, "activePermissionProfile": null,
                    "model": model, "modelProvider": provider, "serviceTier": tier,
                    "effort": null, "summary": null, "personality": null,
                    "collaborationMode": {"mode": "default", "settings": {
                        "model": model, "reasoning_effort": null, "developer_instructions": null
                    }}
                }
            }}
        }).to_string()
    };
    let surfaces = |s: &Screen, text: &str| {
        s.assert_has_row_trimmed(text);
        s.assert_title(text);
    };
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_2/fast_tier.jsonl"))
        .open();
    s.assert_has_row_trimmed("Standard");
    s.feed(&status("priority", true));
    surfaces(&s, "fast");
    s.feed(&status("priority", false));
    surfaces(&s, "Standard");
    s.feed(&status("priority", true));
    // Host config still selects OpenAI, as the attached thread changes its effective provider.
    for (provider, name) in [
        ("openai", "Flex"),
        ("amazon-bedrock", "Standard"),
        ("amazon-bedrock-runtime", "Standard"),
        ("remote-only-provider", "Standard"),
        ("openai", "Flex"),
    ] {
        s.feed(&settings("gpt-5.2", provider, "flex"));
        surfaces(&s, name);
    }
    s.feed(&settings("uncatalogued-model", "openai", "priority"));
    surfaces(&s, "Standard");
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
            r#"{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.4","effort":"xhigh","service_tier":"{tier}","provider":"openai","provider_names":[{{"id":"openai","name":"OpenAI","flex_fallback":true}}],"cwd":"/tmp/project","items":{items},"terminal_title":["model-with-reasoning"]}}}}}}"#
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
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    let first = assistant_message("msg-first", "First message");
    let second = assistant_message("msg-second", "Second message");
    s.feed(&format!(
        "{TURN}\n{first}\n{second}\n{}",
        turn_ended("completed")
    ));
    s.assert_in_order(&["First message", "Second message"]);
    s.assert_text_count("First message", 1);
    s.assert_text_count("Second message", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:4998 final_reasoning_then_message_without_deltas_are_rendered kind=snap owner=ui-pane snap=final_reasoning_then_message_without_deltas_are_rendered
#[test]
fn final_reasoning_then_message_without_deltas_are_rendered() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    let reasoning = completed_reasoning("Checked the input.");
    let answer = assistant_message("msg-result", "Here is the result.");
    s.feed(&format!(
        "{TURN}\n{reasoning}\n{answer}\n{}",
        turn_ended("completed")
    ));
    s.keys("ctrl+t");
    s.assert_in_order(&["Checked the input.", "Here is the result."]);
}

fn completed_reasoning(line: &str) -> String {
    serde_json::json!({"step": "notify", "message": {"method": "item/completed", "params": {
        "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0,
        "item": {"type": "reasoning", "id": "reasoning-1", "summary": [line], "content": []}
    }}})
    .to_string()
}

fn prose_delta(method: &str, id: &str, text: &str) -> String {
    serde_json::json!({"step": "notify", "message": {"method": method, "params": {
        "threadId": "thread-1", "turnId": "turn-1", "itemId": id, "delta": text
    }}})
    .to_string()
}

// ORACLE chatwidget/tests/status_and_layout.rs:5023 deltas_then_same_final_message_are_rendered_snapshot kind=snap owner=ui-pane snap=deltas_then_same_final_message_are_rendered_snapshot
#[test]
fn deltas_then_same_final_message_are_rendered_snapshot() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(TURN);
    for delta in ["I will ", "first analyze the ", "request."] {
        s.feed(
            &serde_json::json!({"step": "notify", "message": {
                "method": "item/reasoning/summaryTextDelta", "params": {
                    "threadId": "thread-1", "turnId": "turn-1", "itemId": "reasoning-1",
                    "summaryIndex": 0, "delta": delta
                }
            }})
            .to_string(),
        );
    }
    let reasoning = "I will first analyze the request.";
    s.feed(&completed_reasoning(reasoning));
    for delta in ["Here is the ", "result."] {
        s.feed(&prose_delta("item/agentMessage/delta", "msg-result", delta));
    }
    let answer = "Here is the result.";
    s.assert_contains(answer);
    s.feed(&assistant_message("msg-result", answer));
    s.feed(&turn_ended("completed"));
    s.keys("ctrl+t");
    s.assert_in_order(&[reasoning, answer]);
    s.assert_text_count(reasoning, 1);
    s.assert_text_count(answer, 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:5057 unterminated_prose_is_visible_before_completion kind=snap owner=ui-pane snap=unterminated_agent_prose_preview snap=unterminated_plan_prose_preview
#[test]
fn unterminated_prose_is_visible_before_completion() {
    for method in ["item/agentMessage/delta", "item/plan/delta"] {
        let mut s = Screen::build(Owner::UiPane).size(26, 16).live().open();
        s.feed(TURN);
        for text in ["100 200 300 400 ", "500 600 700 800 ", "900 1000 1100 1200"] {
            s.feed(&prose_delta(method, "draft-1", text));
        }
        s.assert_in_order(&["100", "400", "500", "800", "900", "1200"]);
        s.assert_text_count("1200", 1);
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:5233 reasoning_delta_does_not_double_schedule_visible_status_redraw kind=ui owner=ui-pane
#[test]
fn reasoning_delta_does_not_double_schedule_visible_status_redraw() {
    kit::na(
        Owner::UiPane,
        "the oracle counts manual FrameRequester requests before coalescing; rtwk derives frames from view changes (process.md), and this screen test checks the visible header",
    );
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(&format!("{BASE}\n{TURN}"));
    let item =
        serde_json::json!({"type": "reasoning", "id": "reasoning-1", "summary": [], "content": []});
    s.feed(
        &serde_json::json!({"step": "notify", "message": {"method": "item/started", "params": {
            "threadId": "thread-1", "turnId": "turn-1", "item": item, "startedAtMs": 0
        }}})
        .to_string(),
    );
    s.feed(r#"{"step":"notify","message":{"method":"item/reasoning/summaryTextDelta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"reasoning-1","summaryIndex":0,"delta":"**Checking files**"}}}"#);
    s.assert_contains("Checking files");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5257 reasoning_delta_restores_recreated_status_indicator_header kind=snap owner=ui-pane snap=reasoning_delta_restores_recreated_status_indicator
#[test]
fn reasoning_delta_restores_recreated_status_indicator_header() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 16)
        .script(BASE)
        .open();
    let item =
        serde_json::json!({"type": "reasoning", "id": "reasoning-1", "summary": [], "content": []});
    let started = serde_json::json!({"step": "notify", "message": {"method": "item/started", "params": {
        "threadId": "thread-1", "turnId": "turn-1", "item": item, "startedAtMs": 0
    }}});
    let reasoning_delta = |delta: &str| {
        serde_json::json!({"step": "notify", "message": {
            "method": "item/reasoning/summaryTextDelta", "params": {
                "threadId": "thread-1", "turnId": "turn-1", "itemId": "reasoning-1",
                "summaryIndex": 0, "delta": delta
            }
        }})
        .to_string()
    };
    s.feed(&format!(
        "{TURN}\n{started}\n{}",
        reasoning_delta("**Checking files**")
    ));
    s.assert_contains("Checking files");
    let preamble = |delta| prose_delta("item/agentMessage/delta", "preamble", delta);
    s.feed(&preamble("\n"));
    s.assert_contains("Checking files");
    s.feed(&preamble("Preamble line"));
    s.assert_contains("Checking files");
    s.feed(&preamble("\n"));
    s.assert_contains("Preamble line");
    s.assert_absent("esc to interrupt");
    s.feed(BACKGROUND);
    s.assert_contains("Checking files");
    s.assert_contains("1 background terminal running");
    s.feed(&reasoning_delta(" and preparing a response"));
    s.assert_contains("Checking files and preparing a response");
    s.resize(80, 8);
    s.assert_snap("reasoning_delta_restores_recreated_status_indicator");
}

#[test]
fn reasoning_status_uses_wire_order_when_terminal_and_line_fit_one_frame() {
    let mut read: serde_json::Value = serde_json::from_str(BACKGROUND).expect("background script");
    let item = &mut read["message"]["params"]["item"];
    item["command"] = serde_json::json!("cat README.md");
    item["commandActions"] = serde_json::json!([{
        "type": "read", "command": "cat README.md", "name": "README.md", "path": "/tmp/project/README.md"
    }]);

    let background_read = read.to_string();
    let item = &mut read["message"]["params"]["item"];
    item["source"] = serde_json::json!("agent");
    item["processId"] = serde_json::Value::Null;

    for (command, recognized_read, background) in [
        (BACKGROUND.to_owned(), false, true),
        (background_read, true, true),
        (read.to_string(), true, false),
    ] {
        for command_last in [false, true] {
            let mut s = Screen::build(Owner::UiPane)
                .size(80, 16)
                .script(BASE)
                .open();
            s.feed(TURN);
            s.feed(&prose_delta(
                "item/agentMessage/delta",
                "answer",
                "Complete line",
            ));
            s.assert_contains("esc to interrupt");
            let newline = prose_delta("item/agentMessage/delta", "answer", "\n");
            let events = if command_last {
                format!("{newline}\n{command}")
            } else {
                format!("{command}\n{newline}")
            };
            s.feed(&events);
            s.assert_contains("Complete line");
            s.assert_present("Read README.md", recognized_read);
            s.assert_present("1 background terminal running", background);
            s.assert_present("esc to interrupt", command_last);
            if !command_last {
                s.feed(&command);
                s.assert_absent("esc to interrupt");
                s.feed(
                    &serde_json::json!({"step": "notify", "message": {
                        "method": "item/completed", "params": {
                            "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0,
                            "item": {"type": "agentMessage", "id": "answer",
                                "text": "Complete line", "phase": "commentary"}
                        }
                    }})
                    .to_string(),
                );
                s.assert_contains("esc to interrupt");
            }
        }
    }
}

#[test]
fn reasoning_status_keeps_the_new_turn_hidden_under_an_older_terminal() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 16)
        .script(BASE)
        .open();
    s.feed(&format!(
        "{TURN}\n{BACKGROUND}\n{}",
        turn_ended("completed")
    ));
    s.feed(&TURN.replace("turn-1", "turn-2"));
    let answer = prose_delta("item/agentMessage/delta", "answer", "Complete line\n");
    s.feed(&answer.replace("turn-1", "turn-2"));
    s.assert_contains("1 background terminal running");
    s.assert_contains("Complete line");
    s.assert_absent("esc to interrupt");
    s.feed(
        &BACKGROUND
            .replace("turn-1", "turn-2")
            .replace("proc-1", "proc-2"),
    );
    s.assert_contains("2 background terminal");
    s.assert_contains("esc to interrupt");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5296 user_prompt_submit_app_server_hook_notifications_render_snapshot kind=snap owner=ui-pane snap=user_prompt_submit_app_server_hook_notifications_render_snapshot
#[test]
fn user_prompt_submit_app_server_hook_notifications_render_snapshot() {
    // UserPromptSubmit starts and stops within the turn's input: the stop is written to history
    // and no activity row is left
    let frames = Frames {
        name: "user_prompt_submit_app_server_hook_notifications_render_snapshot",
        labels: &[],
        named: false,
    };
    let id = "user-prompt-submit:0:/tmp/hooks.json";
    let message = "checking go-workflow input policy";
    let mut s = chat();
    s.feed(&format!(
        "{}\n{}",
        in_turn(hook_started(id, "userPromptSubmit", Some(message))),
        in_turn(hook_completed(
            id,
            "userPromptSubmit",
            "stopped",
            Some(message),
            &[
                ("warning", "go-workflow must start from PlanMode"),
                ("stop", "prompt blocked"),
            ],
        )),
    ));
    frames.history(&s, 0, "• Hook stopped");
    s.assert_absent(message);
    s.assert_absent("Working");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5372 hook_events_render_consistently kind=ui owner=ui-pane
#[test]
fn hook_events_render_consistently() {
    for (event, id, message) in [
        (
            "interrupt",
            "interrupt:0:/tmp/hooks.json",
            "cleaning up the interrupted turn",
        ),
        (
            "preToolUse",
            "pre-tool-use:0:/tmp/hooks.json",
            "warming the shell",
        ),
        (
            "postToolUse",
            "post-tool-use:0:/tmp/hooks.json",
            "warming the shell",
        ),
        (
            "sessionStart",
            "session-start:0:/tmp/hooks.json",
            "warming the shell",
        ),
    ] {
        let mut s = chat();
        s.feed(&format!(
            "{TURN}\n{}",
            hook_started(id, event, Some(message))
        ));
        // the start updates the live row, nothing is written to history; the row waits for its delay
        s.assert_absent("Hook");
        s.assert_absent(message);
        s.wait(150);
        // the hook start renders its status in the activity row
        s.assert_row_of("Working", &[message], &[]);
        s.assert_absent("Hook");
        let mut entries = vec![("warning", "Heads up from the hook")];
        if event != "interrupt" {
            entries.push(("context", "Remember the startup checklist."));
        }
        s.feed(&hook_completed(
            id,
            event,
            "completed",
            Some(message),
            &entries,
        ));
        // the output for the user is the one history row; the context is the model's
        s.assert_has_row_trimmed("↳ Hook · Heads up from the hook");
        s.assert_absent("startup checklist");
        s.assert_rows_with("Hook", 1);
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:5400 quiet_hook_cleanup_starts_when_the_hook_is_revealed kind=ui owner=ui-pane
#[test]
fn quiet_hook_cleanup_starts_when_the_hook_is_revealed() {
    let id = "post-tool-use:0:/tmp/hooks.json";
    let quiet = hook_completed(id, "postToolUse", "completed", None, &[]);
    for delayed_redraw in [false, true] {
        let mut s = chat();
        s.feed(&hook_started(
            id,
            "postToolUse",
            Some("checking output policy"),
        ));
        // held back for its delay: no row yet
        s.assert_absent("Checking output policy");
        // revealed (a redraw at the deadline, or one that comes late)
        s.wait(if delayed_redraw { 1000 } else { 150 });
        s.assert_contains("Checking output policy");
        s.feed(&quiet);
        // a quiet end leaves no row and writes no history, then or later
        s.assert_absent("Checking output policy");
        s.wait(700);
        s.assert_absent("Checking output policy");
        s.assert_absent("Hook");
    }
    // a hook that is over before its delay never shows
    let mut s = chat();
    s.feed(&hook_started(
        id,
        "postToolUse",
        Some("checking output policy"),
    ));
    s.feed(&quiet);
    s.wait(700);
    s.assert_absent("Checking output policy");
    s.assert_absent("Hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5440 blocked_and_failed_hooks_render_feedback_and_errors kind=snap owner=ui-pane snap=hook_blocked_failed_feedback_history_snapshot
#[test]
fn blocked_and_failed_hooks_render_feedback_and_errors() {
    let frames = Frames {
        name: "hook_blocked_failed_feedback_history_snapshot",
        labels: &[],
        named: false,
    };
    let mut s = chat();
    s.feed(&hook_completed(
        "pre-tool-use:0:/tmp/hooks.json",
        "preToolUse",
        "blocked",
        None,
        &[("feedback", "run tests before touching the fixture")],
    ));
    s.feed(&hook_completed(
        "post-tool-use:1:/tmp/hooks.json",
        "postToolUse",
        "failed",
        None,
        &[("error", "hook exited with code 7")],
    ));
    s.assert_in_order(&[
        "Blocked by hook",
        "run tests before touching the fixture",
        "Hook failed",
        "hook exited with code 7",
    ]);
    frames.history(&s, 0, "• Blocked by hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5484 completed_hook_with_output_flushes_immediately kind=snap owner=ui-pane snap=completed_hook_with_output_flushes_immediately_snapshot
#[test]
fn completed_hook_with_output_flushes_immediately() {
    let frames = Frames {
        name: "completed_hook_with_output_flushes_immediately_snapshot",
        labels: &["running", "completed", "activity:", "history:"],
        named: false,
    };
    let id = "pre-tool-use:0:/tmp/hooks.json:tool-call-1";
    let mut s = chat();
    s.feed(&hook_started(id, "preToolUse", Some("checking command")));
    s.wait(150);
    // running: an activity row, nothing in history
    frames.pane(&s, 1);
    s.assert_absent("Blocked by hook");
    // completed with output: the output is in history at once
    s.feed(&hook_completed(
        id,
        "preToolUse",
        "blocked",
        None,
        &[("feedback", "command blocked by policy")],
    ));
    frames.pane(&s, 4);
    frames.history(&s, 5, "• Blocked by hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5523 completed_hook_output_precedes_following_assistant_message kind=snap owner=ui-pane snap=completed_hook_output_precedes_following_assistant_message_snapshot
#[test]
fn completed_hook_output_precedes_following_assistant_message() {
    let frames = Frames {
        name: "completed_hook_output_precedes_following_assistant_message_snapshot",
        labels: &[],
        named: false,
    };
    let id = "pre-tool-use:0:/tmp/hooks.json:tool-call-1";
    let mut s = chat();
    s.feed(&hook_started(id, "preToolUse", Some("checking command")));
    s.wait(150);
    s.feed(&hook_completed(
        id,
        "preToolUse",
        "blocked",
        None,
        &[("feedback", "command blocked by policy")],
    ));
    s.feed(&assistant_message(
        "msg-after-hook",
        "The hook feedback was applied.",
    ));
    s.assert_in_order(&["Blocked by hook", "The hook feedback was applied."]);
    frames.history(&s, 0, "• Blocked by hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5577 completed_same_id_hook_output_survives_restart kind=snap owner=ui-pane snap=completed_same_id_hook_output_survives_restart_snapshot
#[test]
fn completed_same_id_hook_output_survives_restart() {
    let frames = Frames {
        name: "completed_same_id_hook_output_survives_restart_snapshot",
        labels: &["restarted", "activity:", "history:"],
        named: false,
    };
    let id = "stop:0:/tmp/hooks.json";
    let mut s = chat();
    s.feed(&hook_started(id, "stop", Some("checking stop condition")));
    s.wait(150);
    s.feed(&hook_completed(
        id,
        "stop",
        "stopped",
        None,
        &[("stop", "continue with more context")],
    ));
    // the same id starts again: its first run's output stays in history
    s.feed(&hook_started(id, "stop", Some("checking stop condition")));
    s.wait(150);
    s.assert_in_order(&["Hook stopped", "continue with more context"]);
    frames.pane(&s, 1);
    frames.history(&s, 2, "• Hook stopped");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5627 running_hooks_fit_around_background_activity_and_finish_without_history_snapshot kind=snap owner=ui-pane snap=long_running_hook_below_background_activity_row snap=mixed_running_hooks_share_background_activity_row snap=running_hooks_share_background_activity_row
#[test]
fn running_hooks_fit_around_background_activity_and_finish_without_history_snapshot() {
    for (messages, name) in [
        (
            vec!["checking command policy"; 3],
            "running_hooks_share_background_activity_row",
        ),
        (
            vec![
                "checking command policy",
                "scanning for secrets",
                "loading project context",
            ],
            "mixed_running_hooks_share_background_activity_row",
        ),
        (
            vec![
                "checking command policy for deployment commands, production credentials, and paths containing 日本語 before continuing",
            ],
            "long_running_hook_below_background_activity_row",
        ),
    ] {
        let frames = Frames {
            name,
            labels: &["80 columns:", "120 columns:"],
            named: true,
        };
        let started: Vec<String> = messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                hook_started(&format!("hook-{index}"), "preToolUse", Some(message))
            })
            .collect();
        let finished: Vec<String> = (0..messages.len())
            .map(|index| {
                hook_completed(
                    &format!("hook-{index}"),
                    "preToolUse",
                    "completed",
                    None,
                    &[],
                )
            })
            .collect();
        for (render, width) in [(0, 80), (1, 120)] {
            let mut s = Screen::build(Owner::UiPane).size(width, 24).live().open();
            s.feed(BASE);
            // a task with one background terminal, then the hooks
            s.feed(&format!("{TURN}\n{BACKGROUND}\n{}", started.join("\n")));
            s.wait(150);
            frames.pane(&s, render);
            // the batch finishes before the next frame: no hook row, no history
            s.feed(&finished.join("\n"));
            s.assert_contains("1 background terminal running");
            s.assert_absent("Running hooks");
            s.assert_absent("checking command policy");
            s.assert_absent("Hook");
        }
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:5694 session_end_hook_has_standalone_activity_row kind=snap owner=ui-pane snap=session_end_hook_activity snap=session_end_hook_activity_without_message
#[test]
fn session_end_hook_has_standalone_activity_row() {
    for (message, name) in [
        (Some("saving session summary"), "session_end_hook_activity"),
        (None, "session_end_hook_activity_without_message"),
    ] {
        let frames = Frames {
            name,
            labels: &["before reveal:", "running:", "completed:"],
            named: false,
        };
        let mut s = chat();
        s.feed(&hook_started("session-end", "sessionEnd", message));
        frames.pane(&s, 0);
        s.wait(150);
        frames.pane(&s, 1);
        s.feed(&hook_completed(
            "session-end",
            "sessionEnd",
            "completed",
            None,
            &[],
        ));
        frames.pane(&s, 2);
        // no history is written
        s.assert_absent("Hook");
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:5732 overlapping_hook_live_cell_tracks_parallel_quiet_hooks kind=snap owner=ui-pane snap=overlapping_hook_live_cell_snapshot
#[test]
fn overlapping_hook_live_cell_tracks_parallel_quiet_hooks() {
    let frames = Frames {
        name: "overlapping_hook_live_cell_snapshot",
        labels: &[
            "pre running",
            "post running",
            "pre completed",
            "all completed",
            "activity:",
            "history:",
        ],
        named: false,
    };
    let pre = "pre-tool-use:0:/tmp/hooks.json";
    let post = "post-tool-use:1:/tmp/hooks.json";
    let mut s = chat();
    // a task that is thinking (`set_status_header("Thinking")`)
    s.feed(&format!("{TURN}\n{THINKING}"));
    s.feed(&hook_started(
        pre,
        "preToolUse",
        Some("checking command policy"),
    ));
    s.wait(150);
    frames.pane(&s, 1);
    s.feed(&hook_started(
        post,
        "postToolUse",
        Some("checking output policy"),
    ));
    s.wait(150);
    frames.pane(&s, 4);
    s.feed(&hook_completed(pre, "preToolUse", "completed", None, &[]));
    frames.pane(&s, 7);
    s.feed(&hook_completed(post, "postToolUse", "completed", None, &[]));
    frames.pane(&s, 10);
    // nothing is written to history
    s.assert_absent("Hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5806 running_hook_does_not_displace_active_exec_cell kind=snap owner=ui-pane snap=hook_runs_while_exec_active_snapshot
#[test]
fn running_hook_does_not_displace_active_exec_cell() {
    let frames = Frames {
        name: "hook_runs_while_exec_active_snapshot",
        labels: &[
            "exec running:",
            "exec and hook running:",
            "history after exec:",
            "hook running after exec:",
            "quiet hook completed:",
        ],
        named: false,
    };
    let id = "post-tool-use:0:/tmp/hooks.json";
    let mut s = chat();
    s.feed(&format!("{TURN}\n{EXEC_BEGIN}"));
    frames.history(&s, 0, "• Running echo done");
    let exec_row = row_of(&s, "• Running echo done");
    // the revealed hook is a row of the status area: the exec cell stays where it is
    s.feed(&hook_started(
        id,
        "postToolUse",
        Some("checking output policy"),
    ));
    s.wait(150);
    kit::assert_same(row_of(&s, "• Running echo done"), exec_row, &s.text());
    frames.pane_from(&s, 1, 3);
    // the exec ends: its cell goes to history, the hook still runs
    s.feed(EXEC_END);
    frames.history(&s, 2, "• Ran echo done");
    frames.pane(&s, 3);
    s.feed(&hook_completed(id, "postToolUse", "completed", None, &[]));
    frames.pane(&s, 4);
    s.assert_absent("Hook");
}

// ORACLE chatwidget/tests/status_and_layout.rs:5913 stopped_hook_hides_model_context_and_preserves_stop_reason_snapshot kind=snap owner=ui-pane snap=stopped_hook_hides_model_context_and_preserves_stop_reason
#[test]
fn stopped_hook_hides_model_context_and_preserves_stop_reason_snapshot() {
    let frames = Frames {
        name: "stopped_hook_hides_model_context_and_preserves_stop_reason",
        labels: &[],
        named: false,
    };
    let mut s = chat();
    s.feed(&hook_completed(
        "session-start:0:/tmp/hooks.json",
        "sessionStart",
        "stopped",
        None,
        &[
            (
                "context",
                "This hook context is intentionally long enough to wrap across several terminal rows while keeping the complete value available in the transcript overlay. The main conversation should stay compact even when a hook injects a large block of instructions for the model.",
            ),
            (
                "stop",
                "The hook stopped this turn for an important reason.\nThis second line must remain visible in full.",
            ),
        ],
    ));
    frames.history(&s, 0, "• Hook stopped");
    s.assert_absent("intentionally long");
}

// ORACLE chatwidget/tests/status_and_layout.rs:6029 chatwidget_exec_and_status_layout_vt100_snapshot kind=snap owner=ui-pane snap=chatwidget_exec_and_status_layout_vt100_snapshot
#[test]
fn chatwidget_exec_and_status_layout_vt100_snapshot() {
    // The oracle prints the history into a 40-row terminal (an inline viewport) and compares the
    // screen; the port's chat is full-screen, so the same rows are compared as the chat shows
    // them: the answer, the explored cell, the status row, the draft and the footer, in order (the
    // status row's spinner and elapsed seconds run on the screen's clock, the answer wraps one
    // column earlier than the oracle's history).
    let mut s = chat();
    s.feed(&assistant_message(
        "msg-search",
        "I’m going to search the repo for where “Change Approved” is rendered to update that view.",
    ));
    let command = |status: &str, output: bool| {
        serde_json::json!({"step": "notify", "message": {"method": if output { "item/completed" } else { "item/started" },
            "params": {"threadId": "thread-1", "turnId": "turn-1", "startedAtMs": 0, "completedAtMs": 0,
                "item": {"type": "commandExecution", "id": "c1", "pluginId": null, "scriptPath": null,
                    "command": "bash -lc 'rg \"Change Approved\"'", "cwd": "/tmp/project", "processId": null,
                    "source": "agent", "status": status,
                    "commandActions": [
                        {"type": "search", "command": "rg \"Change Approved\"", "query": "Change Approved", "path": null},
                        {"type": "read", "command": "cat diff_render.rs", "name": "diff_render.rs", "path": "/tmp/project/diff_render.rs"}],
                    "aggregatedOutput": null, "exitCode": if output { serde_json::json!(0) } else { serde_json::Value::Null },
                    "durationMs": if output { serde_json::json!(16000) } else { serde_json::Value::Null },
                    "startedAtMs": null, "completedAtMs": null}}}})
        .to_string()
    };
    s.feed(&command("inProgress", false));
    s.feed(&command("completed", true));
    s.feed(TURN);
    s.feed(r#"{"step":"notify","message":{"method":"item/started","params":{"threadId":"thread-1","turnId":"turn-1","startedAtMs":0,"item":{"type":"reasoning","id":"reasoning-1","summary":[],"content":[]}}}}"#);
    s.feed(r#"{"step":"notify","message":{"method":"item/reasoning/summaryTextDelta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"reasoning-1","summaryIndex":0,"delta":"**Investigating rendering code**"}}}"#);
    s.type_text("Summarize recent commits");
    s.assert_in_order(&[
        "• I’m going to search the repo for where “Change Approved” is rendered to",
        "that view.",
        "• Explored",
        "  └ Search Change Approved",
        "    Read diff_render.rs",
        "Investigating rendering code (",
        "s • esc to interrupt)",
        "› Summarize recent commits",
        "  GPT-5.6-Sol default · /tmp/project",
        "  tab to queue message",
    ]);
}

// ORACLE chatwidget/tests/status_and_layout.rs:6130 chatwidget_markdown_code_blocks_vt100_snapshot kind=snap owner=ui-pane snap=chatwidget_markdown_code_blocks_vt100_snapshot
#[test]
fn chatwidget_markdown_code_blocks_vt100_snapshot() {
    kit::pending(
        Owner::UiPane,
        "G73: item 1, the oracle draws the indented code block with its four columns of indent (`      SELECT *`); rtwk's MarkdownView code block drops the indent and pads one cell (`   SELECT *`). The fenced blocks, the nested fence and the `Worked for` row equal the oracle's",
    );
    // The oracle streams the answer in two-character deltas into a 50-row terminal, history rows
    // only; the chat shows the same rows in its transcript, and the turn's `Worked for` line
    // (its duration and clock are placeholders there).
    let source = "\n\n    -- Indented code block (4 spaces)\n    SELECT *\n    FROM \"users\"\n    WHERE \"email\" LIKE '%@example.com';\n\n````markdown\n```sh\nprintf 'fenced within fenced\\n'\n```\n````\n\n```jsonc\n{\n  // comment allowed in jsonc\n  \"path\": \"C:\\\\Program Files\\\\App\",\n  \"regex\": \"^foo.*(bar)?$\"\n}\n```\n";
    let chars: Vec<char> = source.chars().collect();
    let mut s = Screen::build(Owner::UiPane).size(80, 50).live().open();
    s.feed(BASE);
    s.feed(TURN);
    for pair in chars.chunks(2) {
        let delta: String = pair.iter().collect();
        s.feed(&prose_delta("item/agentMessage/delta", "msg-md", &delta));
    }
    s.feed(&turn_ended("completed"));
    s.assert_in_order(&[
        "•     -- Indented code block (4 spaces)",
        "      SELECT *",
        "      FROM \"users\"",
        "      WHERE \"email\" LIKE '%@example.com';",
        "  ```sh",
        "  printf 'fenced within fenced\\n'",
        "  ```",
        "  {",
        "    // comment allowed in jsonc",
        "    \"path\": \"C:\\\\Program Files\\\\App\",",
        "    \"regex\": \"^foo.*(bar)?$\"",
        "  }",
        "Worked for",
    ]);
}

// ORACLE chatwidget/tests/status_and_layout.rs:6211 chatwidget_tall kind=snap owner=ui-pane snap=chatwidget_tall
#[test]
fn chatwidget_tall() {
    kit::pending(
        Owner::UiPane,
        "pending(G-new-pane-inset): the oracle insets the bottom pane by one row at the top and cuts the preview at the room (a blank first row, 15 queued rows); the port's pane has no blank first row and draws 16 queued rows (the preview itself is cut at the room: `PendingInput` is `overflow_intended`)",
    );
    // a turn under way with thirty queued follow-ups: the status row, fifteen of the follow-ups
    // and the composer fill the 24 rows
    let mut s = chat();
    let queued: Vec<String> = (0..30).map(|i| format!("Hello, world! {i}")).collect();
    s.feed(TURN);
    s.feed(
        &serde_json::json!({"step":"screen","dir":"chat","ev":{"pending":{
            "thread": "thread-1", "queued": queued}}})
        .to_string(),
    );
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
    s.wait_for("8% left (resets");
}

// ORACLE chatwidget/tests/status_command_tests.rs:35 status_command_refresh_updates_cached_limits_for_future_status_outputs kind=ui owner=ui-pane
#[test]
fn status_command_refresh_updates_cached_limits_for_future_status_outputs() {
    // tall enough for both cards: the first card's `8% left` stays on the screen
    let mut s = Screen::build(Owner::UiPane).height(60).live().open();
    s.feed(CHATGPT).feed(&limits(92));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("8% left (resets");
    // the card's row; the usage notice in the composer gap (`⚠ 5h limit: 8% left`) is not a card
    s.assert_rows_with("8% left (resets", 1);
    // the refresh updated the cache: the next card shows the limits as they are now
    s.feed(&limits(50));
    s.type_text("/status");
    s.keys("enter");
    s.wait_for("50% left");
    s.assert_contains("8% left (resets");
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
    s.wait_for("8% left (resets");
    // one row in each card; the usage notice in the composer gap is not a card
    s.assert_rows_with("8% left (resets", 2);
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
    // The oracle's `cache_rate_limit_snapshot`: a 30-day window 35% used and a 7-day one 50% used.
    let mut s = Screen::build(Owner::UiPane)
        .size(100, 24)
        .script(include_str!("fixtures/chat_status_3/rate_limits.jsonl"))
        .open();
    // `model-name` is the oracle's alias of `model`, `monthly-limit` is the item of the window
    // the snapshot has (the 30-day one, `five-hour-limit`); the title is set atomically
    s.type_text("/title model-name monthly-limit");
    s.keys("enter");
    s.assert_title_has("monthly 65% left");
    s.assert_absent("Unknown");
    s.type_text("/statusline monthly-limit weekly-limit --no-colors");
    s.keys("enter");
    s.assert_contains("monthly 65% left · weekly 50% left");
    // both are the chat's own: nothing went to the config
    s.assert_no_request("config/batchWrite");
    s.assert_no_request("config/value/write");
    // the snapshot's second window turns monthly: `monthly-limit` is ambiguous, and the refusal
    // changes nothing
    s.feed(
        r#"{"step":"notify","message":{"method":"account/rateLimits/updated","params":{"rateLimits":{"limitId":null,"limitName":null,"normalModelSlug":null,"primary":{"usedPercent":35,"windowDurationMins":43200,"resetsAt":null},"secondary":{"usedPercent":50,"windowDurationMins":43200,"resetsAt":null},"credits":null,"individualLimit":null,"spendControlReached":null,"planType":null,"rateLimitReachedType":null}}}}"#,
    );
    s.type_text("/statusline monthly-limit");
    s.keys("enter");
    s.assert_contains(
        "Ambiguous status-line item 'monthly-limit'. Use 'five-hour-limit' or 'weekly-limit'.",
    );
    s.type_text("/statusline model reasoning");
    s.keys("enter");
    s.type_text("/statusline current-dir --colors --no-colors");
    s.keys("enter");
    s.assert_contains("Specify only one of --colors or --no-colors.");
    s.type_text("/statusline --colors");
    s.keys("enter");
    s.type_text("/title --clear");
    s.keys("enter");
    s.type_text("/statusline --clear --no-colors");
    s.keys("enter");
    s.assert_no_request("config/batchWrite");
    s.assert_no_request("config/value/write");
}
