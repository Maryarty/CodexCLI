//! Shard `chat_status_1` (screens S10, owner ui-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, ColorDepth, Owner, Screen};

/// The status facts that make the app read the account's limits (a ChatGPT login and the
/// `account_usage` fact).
const ACCOUNT_USAGE: &str = r#"{"step":"screen","dir":"status","ev":{"kind":"account","account":{"chat_gpt":{"email":"user@example.com","plan":"plus"}}}}
{"step":"screen","dir":"chat","ev":{"status":{"account_usage":true}}}"#;

/// `account/rateLimits/updated` with `snapshot`.
fn rolling(snapshot: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({"step": "notify", "message": {"method": "account/rateLimits/updated", "params": {"rateLimits": snapshot}}})
}

/// The clock of the fake shell says it is `millis` after the epoch.
fn clock(millis: i64) -> String {
    format!("{{\"step\":\"clock\",\"millis\":{millis}}}\n")
}

/// The moment the countdown tests start at.
const START: i64 = 1_000_000_000_000;

/// What `account/rateLimits/read` answers from now on: `snapshot` (the respond table of the fake).
fn read_of(snapshot: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({"step": "respond", "method": "account/rateLimits/read",
        "result": {"rateLimits": snapshot}})
}

/// `model/list` with the lower-cost model of the switch prompt, listed for the picker.
fn luna_catalog() -> String {
    let model = serde_json::json!({
        "additionalSpeedTiers": [], "availabilityNux": null, "defaultReasoningEffort": "medium",
        "defaultServiceTier": null, "description": "Fast and affordable model for easier tasks.",
        "displayName": "GPT-6 Luna", "hidden": false, "id": "gpt-6-luna",
        "inputModalities": ["text"], "isDefault": false, "model": "gpt-6-luna",
        "modelSpecialty": null, "serviceTiers": [],
        "supportedReasoningEfforts": [{"description": "Balanced", "reasoningEffort": "medium"}],
        "supportsPersonality": false, "upgrade": null, "upgradeInfo": null
    });
    serde_json::json!({"step": "respond", "method": "model/list",
        "result": {"data": [model], "nextCursor": null}})
    .to_string()
}

/// A rolling snapshot of the plan's limit at `used` percent, with the limit type it reached.
fn limit_snapshot(used: i32, reached: Option<&str>) -> serde_json::Value {
    serde_json::json!({"limitId": "codex", "rateLimitReachedType": reached,
        "primary": {"usedPercent": used, "windowDurationMins": 300, "resetsAt": null}})
}

/// The live seed's session (thread `thread-1`) with a running turn `turn-1`.
const RUNNING: &str = include_str!("fixtures/chat_status_1/running_turn.jsonl");

/// An item of the turn `turn-1` as the server sends it for a `cua_repl` computer call.
fn computer_item(id: &str, server: &str, status: &str) -> serde_json::Value {
    let done = status != "inProgress";
    serde_json::json!({
        "type": "mcpToolCall", "id": id, "server": server, "tool": "js", "status": status,
        "arguments": {"title": format!("Inspect page {id}"), "code": "await cua.getState()"},
        "appContext": null, "mcpAppUi": null, "pluginId": null, "readOnlyHint": null,
        "result": done.then(|| serde_json::json!({
            "content": [{"type": "text", "text": format!("Full output for {id}")}],
            "structuredContent": null, "_meta": null, "isError": false
        })),
        "error": null, "durationMs": done.then_some(5)
    })
}

/// A `dynamicToolCall` item of `test.lookup`.
fn dynamic_item(id: &str, status: &str) -> serde_json::Value {
    let done = status != "inProgress";
    serde_json::json!({
        "type": "dynamicToolCall", "id": id, "namespace": "test", "tool": "lookup",
        "arguments": {"id": id}, "status": status,
        "contentItems": done.then(|| serde_json::json!([{"type": "inputText", "text": format!("Result for {id}")}])),
        "success": null, "durationMs": done.then_some(25)
    })
}

/// A reasoning item with one summary line.
fn reasoning_item(summary: &str) -> serde_json::Value {
    serde_json::json!({"type": "reasoning", "id": summary, "summary": [summary], "content": []})
}

/// A reasoning item of `turn-1` with the summary parts `parts` (an empty list: a streamed one).
fn reasoning_parts(id: &str, parts: &[&str]) -> serde_json::Value {
    serde_json::json!({"type": "reasoning", "id": id, "summary": parts, "content": []})
}

/// `method` (a reasoning stream notification) of the item `item` in `turn-1`.
fn reasoning_stream(method: &str, item: &str, delta: Option<&str>) -> String {
    let mut params = serde_json::json!({
        "threadId": "thread-1", "turnId": "turn-1", "itemId": item, "summaryIndex": 0});
    if let Some(delta) = delta {
        params["delta"] = serde_json::json!(delta);
    }
    serde_json::json!({"step": "notify", "message": {"method": method, "params": params}})
        .to_string()
}

/// A summary delta of `item`.
fn summary_delta(item: &str, delta: &str) -> String {
    reasoning_stream("item/reasoning/summaryTextDelta", item, Some(delta))
}

/// `item/started` (`done` false) or `item/completed` of `item` in the turn `turn-1`.
fn lifecycle(item: &serde_json::Value, done: bool) -> String {
    let (method, at) = if done {
        ("item/completed", "completedAtMs")
    } else {
        ("item/started", "startedAtMs")
    };
    serde_json::json!({"step": "notify", "message": {"method": method, "params": {
        "threadId": "thread-1", "turnId": "turn-1", "item": item, at: 0}}})
    .to_string()
}

/// `turn/completed` of `turn-1` as `status` (`interrupted` for the oracle's `finalize_turn`).
fn turn_over(status: &str) -> String {
    serde_json::json!({"step": "notify", "message": {"method": "turn/completed", "params": {
        "threadId": "thread-1", "turn": {"id": "turn-1", "rootTurnId": null, "items": [],
        "itemsView": "full", "status": status, "error": null, "startedAt": null,
        "completedAt": null, "durationMs": null}}}})
    .to_string()
}

/// `item/completed` of an agent message `text` in the turn `turn-1`.
fn agent_message(text: &str) -> String {
    lifecycle(
        &serde_json::json!({"type": "agentMessage", "id": "msg-1", "text": text,
            "phase": "final_answer", "memoryCitation": null}),
        true,
    )
}

/// The row a cell starts at: the oracle's snapshot is the bare cell, the chat draws the
/// transcript around it.
fn cell_row(s: &Screen, head: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.starts_with(head))
        .unwrap_or_else(|| panic!("no `{head}` row:\n{}", s.text()))
}

/// `turn/completed` of the live seed's thread.
const TURN_ENDED: &str = r#"{"step":"notify","message":{"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"completed","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#;

/// `error` of the live seed's thread that is no retry: the usage limit (`"usageLimitExceeded"`)
/// or the generic rate limit (a 429 after too many attempts).
fn limit_error(usage_limit: bool) -> String {
    let info = if usage_limit {
        r#""usageLimitExceeded""#
    } else {
        r#"{"responseTooManyFailedAttempts":{"httpStatusCode":429}}"#
    };
    format!(
        r#"{{"step":"notify","message":{{"method":"error","params":{{"threadId":"thread-1","turnId":"turn-1","willRetry":false,"error":{{"message":"Usage limit reached.","codexErrorInfo":{info},"additionalDetails":null,"misalignment":null}}}}}}}}"#
    )
}

/// The oracle snapshot `name` is on the screen, its first row the one that reads `title`.
fn assert_popup(s: &Screen, name: &str, title: &str) {
    let row = s
        .rows()
        .iter()
        .position(|row| row.trim() == title)
        .unwrap_or_else(|| panic!("no `{title}` row:\n{}", s.text()));
    s.assert_snap_rows_at(name, row);
}

/// The usage warnings the transcript holds begin `Heads up, you have less than ..`.
const WARNING: &str = "Heads up, you have less than";

// ---- oracle: chatwidget/tests/computer_activity_tests.rs (4 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE chatwidget/tests/computer_activity_tests.rs:30 computer_activity_live_and_replay_group_identically kind=snap owner=ui-pane
#[test]
fn computer_activity_live_and_replay_group_identically() {
    // oracle: four completed cua_repl calls live (started + completed) and replayed (ThreadSnapshot) draw one group;
    // the live and the replayed outputs are equal, and the group is the snapshot.
    let mut live = Screen::build(Owner::UiPane)
        .size(80, 24)
        .inline(&[])
        .script(RUNNING)
        .open();
    for id in ["1", "2", "3", "4"] {
        live.feed(&lifecycle(
            &computer_item(id, "cua_repl", "inProgress"),
            false,
        ));
        live.feed(&lifecycle(
            &computer_item(id, "cua_repl", "completed"),
            true,
        ));
    }
    let items = (1..=4)
        .map(|id| computer_item(&id.to_string(), "cua_repl", "completed"))
        .collect::<Vec<_>>();
    let replayed = Screen::build(Owner::UiPane)
        .size(80, 24)
        .inline(&[])
        .replay(vec![kit::replay_turn("turn-1", serde_json::json!(items))])
        .open();
    live.assert_snap_region(
        "computer_activity_replayed",
        cell_row(&live, "• Used computer"),
        0,
    );
    replayed.assert_snap_region(
        "computer_activity_replayed",
        cell_row(&replayed, "• Used computer"),
        0,
    );
}

// ORACLE chatwidget/tests/computer_activity_tests.rs:63 computer_activity_keeps_reasoning_in_order_live_and_replayed kind=snap owner=ui-pane
#[test]
fn computer_activity_keeps_reasoning_in_order_live_and_replayed() {
    // oracle: computer calls 1 and 2 interleaved with reasoning summaries "Inspecting action N" and "Checking action N",
    // live and replayed, in the same order.
    let mut live = Screen::build(Owner::UiPane)
        .size(80, 40)
        .script(RUNNING)
        .open();
    let mut recorded = Vec::new();
    for id in ["1", "2"] {
        live.feed(&lifecycle(
            &computer_item(id, "cua_repl", "inProgress"),
            false,
        ));
        if id == "1" {
            live.feed(&lifecycle(
                &computer_item(id, "cua_repl", "completed"),
                true,
            ));
        }
        recorded.push(computer_item(id, "cua_repl", "completed"));
        for part in ["Inspecting", "Checking"] {
            let summary = reasoning_item(&format!("{part} action {id}"));
            live.feed(&lifecycle(&summary, true));
            recorded.push(summary);
        }
        if id == "2" {
            live.feed(&lifecycle(
                &computer_item(id, "cua_repl", "completed"),
                true,
            ));
        }
    }
    let mut replayed = Screen::build(Owner::UiPane)
        .size(80, 40)
        .replay(vec![kit::replay_turn(
            "turn-1",
            serde_json::json!(recorded),
        )])
        .open();
    for s in [&mut live, &mut replayed] {
        s.assert_in_order(&[
            "• Used computer · 2 actions",
            "├ Inspect page 1",
            "└ Inspect page 2",
        ]);
        // Raw: both calls with their output, no reasoning
        s.keys("alt+r");
        s.assert_screen_row_then(
            "• Called cua_repl.js({\"title\":\"Inspect page 1\",\"code\":\"await cua.getState()\"})",
            &[
                "  └ Full output for 1",
                "• Called cua_repl.js({\"title\":\"Inspect page 2\",\"code\":\"await cua.getState()\"})",
                "  └ Full output for 2",
            ],
        );
        s.keys("alt+r");
        // Expanded: a blank row above each reasoning, none above the call after one
        s.keys("ctrl+t");
        s.assert_screen_row_then(
            "• Called cua_repl.js({\"title\":\"Inspect page 1\",\"code\":\"await cua.getState()\"})",
            &[
                "  └ Full output for 1",
                "",
                "• Inspecting action 1",
                "",
                "• Checking action 1",
                "• Called cua_repl.js({\"title\":\"Inspect page 2\",\"code\":\"await cua.getState()\"})",
                "  └ Full output for 2",
                "",
                "• Inspecting action 2",
                "",
                "• Checking action 2",
            ],
        );
    }
}

// ORACLE chatwidget/tests/computer_activity_tests.rs:127 computer_activity_preserves_boundaries_and_expanded_output kind=ui owner=ui-pane
#[test]
fn computer_activity_preserves_boundaries_and_expanded_output() {
    // oracle: computer call 1 completes -> "Used computer · 1 action"; a call from another server starts its own group
    // (its name shows); call 3 is the active cell whose expanded transcript holds the output and the code.
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 40)
        .script(RUNNING)
        .open();
    s.feed(&lifecycle(
        &computer_item("1", "cua_repl", "completed"),
        true,
    ));
    s.feed(&agent_message("Between"));
    s.assert_contains("Used computer · 1 action");
    s.feed(&lifecycle(
        &computer_item("2", "cua_repl", "completed"),
        true,
    ));
    s.feed(&lifecycle(
        &computer_item("other", "another_server", "completed"),
        true,
    ));
    s.assert_contains("another_server");
    s.feed(&lifecycle(
        &computer_item("3", "cua_repl", "completed"),
        true,
    ));
    s.keys("ctrl+t");
    s.assert_contains("Full output for 3");
    s.assert_contains("await cua.getState()");
}

// ORACLE chatwidget/tests/computer_activity_tests.rs:151 computer_activity_out_of_order_completion_and_interruption kind=snap owner=ui-pane
#[test]
fn computer_activity_out_of_order_completion_and_interruption() {
    // oracle: calls 1 and 2 start, 2 completes out of order; the active cell says "Using computer"; the turn is
    // finalized as interrupted and one history cell is inserted.
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    s.feed(&lifecycle(
        &computer_item("1", "cua_repl", "inProgress"),
        false,
    ));
    s.feed(&lifecycle(
        &computer_item("2", "cua_repl", "inProgress"),
        false,
    ));
    s.feed(&lifecycle(
        &computer_item("2", "cua_repl", "completed"),
        true,
    ));
    s.assert_contains("Using computer");
    s.feed(&turn_over("interrupted"));
    s.assert_snap_region(
        "computer_activity_partial_interruption",
        cell_row(&s, "• Used computer"),
        0,
    );
}

// ---- oracle: chatwidget/tests/dynamic_activity_tests.rs (3 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE chatwidget/tests/dynamic_activity_tests.rs:34 terminal_dynamic_activity_retains_calls_across_both_fallbacks kind=snap owner=ui-pane
#[test]
fn terminal_dynamic_activity_retains_calls_across_both_fallbacks() {
    // oracle: the same two calls on the terminal-transcript fallback (alt screen always) and on the owned fallback
    // (fullscreen transcript on, alt screen never) draw the same rows.
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    for id in ["call-1", "call-2"] {
        s.feed(&lifecycle(&dynamic_item(id, "inProgress"), false));
    }
    s.feed(&lifecycle(&dynamic_item("call-2", "completed"), true));
    s.feed(&lifecycle(&dynamic_item("call-1", "failed"), true));
    s.assert_in_order(&[
        "Failed test.lookup · 25ms",
        "Result for call-1",
        "Called test.lookup · 25ms",
        "Result for call-2",
    ]);
}

// ORACLE chatwidget/tests/dynamic_activity_tests.rs:85 owned_dynamic_activity_updates_the_retained_row_after_config_refresh kind=ui owner=ui-pane
#[test]
fn owned_dynamic_activity_updates_the_retained_row_after_config_refresh() {
    // oracle: owned transcript mode; the call row reads "Calling test.lookup", then after completion it reads
    // "Called test.lookup · 25ms" with "Result for call-1" beneath it, and no live call remains.
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    s.feed(&lifecycle(&dynamic_item("call-1", "inProgress"), false));
    s.assert_contains("• Calling test.lookup");
    s.feed(&lifecycle(&dynamic_item("call-1", "completed"), true));
    s.assert_absent("Calling test.lookup");
    s.assert_contains("• Called test.lookup · 25ms");
    s.assert_contains("Result for call-1");
}

// ORACLE chatwidget/tests/dynamic_activity_tests.rs:114 turn_finalization_drains_lifecycle_queue_before_interrupting_remaining_calls kind=snap owner=ui-pane
#[test]
fn turn_finalization_drains_lifecycle_queue_before_interrupting_remaining_calls() {
    // oracle: a blocking user-input question is queued, a completed call and an unfinished call are queued; on finalize
    // the queue drains and the unfinished call is reported as interrupted; the cell is a 64-wide buffer.
    let mut s = Screen::build(Owner::UiPane)
        .size(64, 24)
        .color_depth(ColorDepth::Ansi16)
        .script(RUNNING)
        .open();
    s.feed(
        &serde_json::json!({"step": "request", "message": {
        "method": "item/tool/requestUserInput", "id": 1, "params": {
            "threadId": "thread-1", "turnId": "turn-1", "itemId": "question-1",
            "isBlocking": true, "autoResolutionMs": null,
            "questions": [{"id": "scope", "header": "Scope",
                "question": "Which change should I review?",
                "isOther": false, "isSecret": false, "options": null}]}}})
        .to_string(),
    );
    s.feed(&lifecycle(&dynamic_item("completed", "inProgress"), false));
    s.feed(&lifecycle(&dynamic_item("completed", "completed"), true));
    s.feed(&lifecycle(&dynamic_item("unfinished", "inProgress"), false));
    s.feed(&turn_over("interrupted"));
    s.assert_contains("Result for completed");
    s.assert_contains("Interrupted before this tool returned a result.");
    // The snapshot is a `Buffer` dump: its rows (the cell styles of its blank cells, and the accent
    // of a test terminal with no colours, are not what a real terminal's cells say) and its styles
    // by the marks that show: a red bold bullet, the verb bold, the output row dim.
    let at = cell_row(&s, "• Interrupted test.lookup");
    s.assert_snap_rows_from("interrupted_dynamic_activity", at, 2, false);
    s.assert_style(0, at, "fg=Red bold");
    s.assert_text_style("Interrupted", "bold");
    s.assert_text_style("Interrupted before this tool returned a result.", "dim");
}

// ---- oracle: chatwidget/tests/program_status.rs (1 screen-level tests here; 1 state tests left to ui-pane)
// ORACLE chatwidget/tests/program_status.rs:10 program_status_tracks_live_results_and_resolved_approvals kind=ui owner=ui-pane
#[test]
fn program_status_tracks_live_results_and_resolved_approvals() {
    kit::na(
        Owner::UiPane,
        "asserts the ProgramStatus values of desired_program_status; the status goes out as OSC 7501/21337 sequences to the terminal, not drawn on the screen grid",
    );
}

// ---- oracle: chatwidget/tests/reasoning_status_tests.rs (3 screen-level tests here; 4 state tests left to ui-pane)
// ORACLE chatwidget/tests/reasoning_status_tests.rs:37 reasoning_status_accepts_bold_text_with_a_plain_suffix kind=snap owner=ui-pane
#[test]
fn reasoning_status_accepts_bold_text_with_a_plain_suffix() {
    // oracle: header "Working" before the delta; after the delta "**: running suite" the header reads "Checking tests: running suite".
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    s.feed(&lifecycle(
        &reasoning_parts("reasoning", &["**Checking tests"]),
        false,
    ));
    s.assert_contains("Working");
    s.assert_absent("Checking tests");
    s.feed(&summary_delta("reasoning", "**: running suite"));
    s.assert_contains("Checking tests: running suite");
}

// ORACLE chatwidget/tests/reasoning_status_tests.rs:76 reasoning_status_tracks_items_and_restores_after_tool_activity kind=snap owner=ui-pane
#[test]
fn reasoning_status_tracks_items_and_restores_after_tool_activity() {
    // oracle: the header sequence Working, Researching backend freshness (x4), No summary heading is
    // available., Preparing evidence report (the unified-exec tool step between is `terminal_wait`'s test).
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    let complete = |s: &mut Screen, id: &str| {
        s.feed(&lifecycle(&reasoning_parts(id, &[]), true));
    };
    s.feed(&lifecycle(&reasoning_parts("first", &[]), false));
    s.feed(&summary_delta("first", "**Researching backend"));
    s.assert_contains("Working");
    s.feed(&summary_delta("first", " freshness**"));
    s.assert_contains("Researching backend freshness");
    complete(&mut s, "first");
    s.feed(&summary_delta("first", "**Late old header**"));
    s.assert_contains("Researching backend freshness");
    s.assert_absent("Late old header");
    s.feed(&lifecycle(&reasoning_parts("second", &[]), false));
    s.assert_contains("Researching backend freshness");
    s.feed(&reasoning_stream(
        "item/reasoning/summaryPartAdded",
        "first",
        None,
    ));
    complete(&mut s, "first");
    s.feed(&summary_delta("first", "**Stale heading**"));
    s.feed(&summary_delta("second", "No summary heading is available."));
    s.assert_absent("Stale heading");
    s.assert_contains("No summary heading is available.");
    s.feed(&summary_delta("second", "\n**Preparing evidence report**"));
    s.assert_contains("Preparing evidence report");
}

// ORACLE chatwidget/tests/reasoning_status_tests.rs:271 completed_reasoning_stays_in_expanded_transcript_for_live_and_replay kind=snap owner=ui-pane
#[test]
fn completed_reasoning_stays_in_expanded_transcript_for_live_and_replay() {
    // oracle: three reasoning items, live and replayed, give the same compact and expanded history; the expanded
    // transcript keeps every summary line; the third item's heading is the live status.
    let summaries = [
        "**Inspecting repository structure**",
        "Mapping the app structure",
        "**Tracing terminal components**\n\nThe playback clock preserves elapsed time.",
    ];
    let items = ["first", "second", "third"].map(|id| {
        let at = ["first", "second", "third"]
            .iter()
            .position(|name| *name == id)
            .unwrap_or(0);
        serde_json::json!({"type": "reasoning", "id": id, "summary": [summaries[at]],
            "content": ["private reasoning"]})
    });
    let mut live = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    for item in &items {
        // the oracle's live path ignores a completion of an item it never saw start
        live.feed(&lifecycle(item, false));
        live.feed(&lifecycle(item, true));
    }
    let mut replayed = Screen::build(Owner::UiPane)
        .size(80, 24)
        .replay(vec![kit::replay_turn("turn-1", serde_json::json!(items))])
        .open();
    live.assert_contains("The playback clock preserves elapsed time.");
    for s in [&mut live, &mut replayed] {
        s.assert_absent("Inspecting repository structure");
        s.keys("ctrl+t");
        s.assert_in_order(&[
            "• Inspecting repository structure",
            "• Mapping the app structure",
            "• The playback clock preserves elapsed time.",
        ]);
    }
}

// ---- oracle: chatwidget/tests/status_and_layout.rs (47 screen-level tests here; 40 state tests left to ui-pane)
// ORACLE chatwidget/tests/status_and_layout.rs:16 finalized_voice_transcript_renders_beside_the_streamed_cell kind=snap owner=ui-pane snap=voice_live_transcript_and_stream snap=voice_partial_transcript_hidden
#[test]
fn finalized_voice_transcript_renders_beside_the_streamed_cell() {
    let mut s = Screen::build(Owner::UiPane).size(60, 24).live().open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"animations":false}}}"#);
    s.voice_available().type_text("/voice").keys("enter");
    let session = s.connect_voice();
    s.feed(r#"{"step":"notify","message":{"method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"answer","delta":"Agent answer arriving"}}}"#);
    s.voice_caption(false, &session, "speech", "user", "");
    s.voice_delta("speech", "pick a number");

    let partial = kit::snap_rows("voice_partial_transcript_hidden", "");
    let finalized = kit::snap_rows("voice_live_transcript_and_stream", "");
    s.wait_for("Agent answer arriving");
    s.assert_has_row(partial[1].trim_end());
    s.assert_absent(finalized[4].trim());
    s.assert_contains("voice ● listen");

    s.voice_caption(true, &session, "speech", "user", "pick a number");
    s.wait_for("pick a number");
    s.assert_has_row(finalized[1].trim_end());
    s.assert_has_row(finalized[4].trim_end());
    s.assert_in_order(&[finalized[1].trim(), finalized[4].trim(), "voice ● listen"]);
}

// ORACLE chatwidget/tests/status_and_layout.rs:78 resumed_session_hides_unknown_token_usage_until_an_update_arrives kind=snap owner=ui-pane
#[test]
fn resumed_session_hides_unknown_token_usage_until_an_update_arrives() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .replay(vec![kit::agent_turn("turn-1", &["Loaded previously."])])
        .open();
    s.assert_contains("Loaded previously.");
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"items":["context-remaining","context-used","total-input-tokens","total-output-tokens"]}}}"#);
    s.assert_absent("Context");
    s.assert_absent("0 in · 0 out");
    s.feed(r#"{"step":"notify","message":{"method":"thread/tokenUsage/updated","params":{"threadId":"replayed-1","turnId":"turn-1","tokenUsage":{"total":{"totalTokens":12700,"inputTokens":0,"cachedInputTokens":0,"outputTokens":0,"reasoningOutputTokens":0},"last":{"totalTokens":12700,"inputTokens":0,"cachedInputTokens":0,"outputTokens":0,"reasoningOutputTokens":0},"modelContextWindow":13000}}}}"#);
    s.assert_contains("Context 30% left · Context 70% used · 0 in · 0 out");
}

// ORACLE chatwidget/tests/status_and_layout.rs:124 app_server_cyber_policy_error_renders_dedicated_notice kind=ui owner=ui-pane
#[test]
fn app_server_cyber_policy_error_renders_dedicated_notice() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(r#"{"step":"notify","message":{"method":"error","params":{"threadId":"thread-1","turnId":"turn-1","willRetry":false,"error":{"message":"server fallback message","codexErrorInfo":"cyberPolicy","additionalDetails":null,"misalignment":null}}}}"#);
    s.assert_contains("This content can’t be shown");
    s.assert_reads("We take extra care with some cybersecurity requests");
    s.assert_contains("Apply for Daybreak");
    s.assert_absent("server fallback message");
}

fn notify_daybreak(method: &str, params: serde_json::Value) -> String {
    serde_json::json!({"step": "notify", "message": {"method": method, "params": params}})
        .to_string()
}

fn daybreak_refusal_screen() -> Screen {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .live()
        .daybreak(&[("gpt-5.6-sol", true, true)])
        .open();
    let source = include_str!("fixtures/chat_status_3/mixed_status_popup.jsonl")
        .lines()
        .filter(|line| line.contains(r#""method":"thread/started""#))
        .collect::<Vec<_>>()
        .join("\n");
    s.feed(
        &source
            .replace(r#""model":null"#, r#""model":"gpt-5.6-sol""#)
            .replace(r#""daybreakEnabled":true"#, r#""daybreakEnabled":false"#),
    );
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"status":{"items":["daybreak"]}}}"#);
    s.server_answers("thread/metadata/update", serde_json::json!({}));
    s
}

fn start_daybreak_turn(s: &mut Screen, turn: &str) {
    s.feed(&notify_daybreak(
        "turn/started",
        serde_json::json!({
            "threadId": "thread-1",
            "turn": {"id": turn, "items": [], "itemsView": "full", "status": "inProgress"}
        }),
    ));
}

fn daybreak_error() -> serde_json::Value {
    serde_json::json!({
        "message": "The server declined this request.", "codexErrorInfo": "cyberPolicy",
        "misalignment": null, "additionalDetails": null
    })
}

fn fail_daybreak_turn(s: &mut Screen, turn: &str) {
    s.feed(&notify_daybreak(
        "turn/completed",
        serde_json::json!({
            "threadId": "thread-1",
            "turn": {"id": turn, "items": [], "itemsView": "full", "status": "failed",
                "error": daybreak_error()}
        }),
    ));
}

fn offer_daybreak_alternatives(s: &mut Screen, turn: &str) {
    s.feed(&notify_daybreak("turn/moderationMetadata", serde_json::json!({
        "threadId": "thread-1", "turnId": turn,
        "metadata": {"safety_recourse": {
            "schema_version": 1, "domain": "cyber", "source": "user_request",
            "source_message": {"id": "1", "kind": "ordinary_message", "role": "user"},
            "topic": "unauthorized_account_or_system_access",
            "suggestions": [
                {"label": "Overview", "prompt_text": "Review ordinary account safety settings."},
                {"label": "Recovery", "prompt_text": "Find official account recovery guidance."},
                {"label": "Prevention", "prompt_text": "Summarize the account safety checklist."}
            ]
        }}
    })));
}

// ORACLE chatwidget/tests/status_and_layout.rs:143 daybreak_refusal_offers_enable_for_the_next_turn kind=snap owner=ui-pane snap=daybreak_recourse_picker snap=daybreak_refusal_enable_picker snap=daybreak_refusal_parent_owned
#[test]
fn daybreak_refusal_offers_enable_for_the_next_turn() {
    let mut s = daybreak_refusal_screen();
    start_daybreak_turn(&mut s, "blocked");
    s.feed(&notify_daybreak("error", serde_json::json!({
        "threadId": "thread-1", "turnId": "blocked", "willRetry": false, "error": daybreak_error()
    })));
    s.assert_contains("Turn on Daybreak for your next request?");
    s.assert_contains("Enable Daybreak");
    s.assert_requests_of("thread/metadata/update", 0);
    s.assert_no_turn();
    s.keys("enter");
    s.assert_requests_of("thread/metadata/update", 1);
    s.assert_last_request_has("thread/metadata/update", r#""threadId":"thread-1""#);
    s.assert_last_request_has("thread/metadata/update", r#""daybreakEnabled":true"#);
    s.assert_has_row_trimmed("Daybreak on");
    s.assert_absent("Turn on Daybreak for your next request?");
    s.assert_no_turn();
}

#[test]
fn daybreak_recourse_reviews_alternatives_and_opens_safety_consent() {
    let mut s = daybreak_refusal_screen();
    start_daybreak_turn(&mut s, "blocked");
    offer_daybreak_alternatives(&mut s, "blocked");
    s.assert_absent("1. Overview");
    fail_daybreak_turn(&mut s, "blocked");
    s.assert_contains("1. Overview");
    s.assert_contains("4. Enable Daybreak");
    s.assert_contains("5. Give feedback");
    s.keys("1");
    s.assert_contains("Review ordinary account safety settings.");
    s.assert_no_turn();
    s.keys("q");
    s.assert_contains("1. Overview");
    s.keys("5");
    s.assert_contains("Include logs and session history?");
    s.assert_requests_of("feedback/upload", 0);
    s.keys("esc");
    s.assert_contains("1. Overview");
    s.keys("4");
    s.assert_requests_of("thread/metadata/update", 1);
    s.assert_no_turn();
}

#[test]
fn a_new_turn_retires_the_refusals_actions_before_a_key_can_apply_them() {
    let mut s = daybreak_refusal_screen();
    start_daybreak_turn(&mut s, "blocked");
    offer_daybreak_alternatives(&mut s, "blocked");
    fail_daybreak_turn(&mut s, "blocked");
    s.assert_contains("4. Enable Daybreak");
    start_daybreak_turn(&mut s, "new-owner-turn");
    s.assert_absent("4. Enable Daybreak");
    s.keys("4");
    s.assert_requests_of("thread/metadata/update", 0);
    s.assert_requests_of("feedback/upload", 0);
    s.assert_no_turn();
}

#[test]
fn a_different_thread_cannot_use_the_refusals_actions() {
    let mut s = daybreak_refusal_screen();
    start_daybreak_turn(&mut s, "blocked");
    offer_daybreak_alternatives(&mut s, "blocked");
    fail_daybreak_turn(&mut s, "blocked");
    s.assert_contains("4. Enable Daybreak");
    s.server_resumes("thread-2").visit_thread("thread-2");
    s.assert_active_thread("thread-2");
    s.assert_absent("4. Enable Daybreak");
    s.keys("4");
    s.assert_requests_of("thread/metadata/update", 0);
    s.assert_requests_of("feedback/upload", 0);
    s.assert_no_turn();
}

#[test]
fn dismissing_a_refusal_survives_switching_threads_and_allows_the_next_refusal() {
    let mut s = daybreak_refusal_screen();
    start_daybreak_turn(&mut s, "blocked");
    offer_daybreak_alternatives(&mut s, "blocked");
    fail_daybreak_turn(&mut s, "blocked");
    s.assert_contains("4. Enable Daybreak");
    s.keys("esc");
    s.assert_absent("1. Overview");

    s.server_resumes("thread-2").visit_thread("thread-2");
    s.assert_active_thread("thread-2");
    s.server_resumes("thread-1").visit_thread("thread-1");
    s.assert_active_thread("thread-1");
    s.assert_contains("This content can’t be shown");
    s.assert_absent("1. Overview");
    s.type_text("draft after return");
    s.assert_composer("draft after return");
    s.assert_requests_of("thread/metadata/update", 0);
    s.assert_no_turn();

    start_daybreak_turn(&mut s, "blocked-again");
    offer_daybreak_alternatives(&mut s, "blocked-again");
    fail_daybreak_turn(&mut s, "blocked-again");
    s.assert_contains("1. Overview");
    s.keys("esc");
    s.assert_composer("draft after return");
}

// ORACLE chatwidget/tests/status_and_layout.rs:370 app_server_model_verification_renders_warning kind=ui owner=ui-pane
#[test]
fn app_server_model_verification_renders_warning() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(r#"{"step":"notify","message":{"method":"model/verification","params":{"threadId":"thread-1","turnId":"turn-1","verifications":["trustedAccessForCyber"]}}}"#);
    s.keys("f2");
    for text in [
        "multiple flags for possible cybersecurity risk",
        "extra safety checks are on",
        "Trusted Access for Cyber",
        "https://chatgpt.com/cyber",
    ] {
        s.assert_reads(text);
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:464 status_line_git_summary_items_render_values kind=ui owner=ui-pane
#[test]
fn status_line_git_summary_items_render_values() {
    // oracle: git summary PR 20252 and +143 -22 render as the PR number and branch-changes status values.
    let s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_status_1/git_summary.jsonl"))
        .open();
    s.assert_contains("PR #20252");
    s.assert_contains("+143 -22");
    s.assert_link("PR #20252", "https://github.com/openai/codex/pull/20252");
}

// ORACLE chatwidget/tests/status_and_layout.rs:505 status_line_branch_changes_render_no_changes kind=ui owner=ui-pane
#[test]
fn status_line_branch_changes_render_no_changes() {
    // oracle: a zero diff renders "No changes" in the branch-changes item.
    let s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_status_1/git_no_changes.jsonl"))
        .open();
    s.assert_contains("No changes");
}

// ORACLE chatwidget/tests/status_and_layout.rs:546 raw_output_mode_can_change_without_inserting_notice kind=ui owner=ui-pane
#[test]
fn raw_output_mode_can_change_without_inserting_notice() {
    // oracle: enabling raw output inserts no history; disabling it inserts the notice.
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_status_1/raw_output.jsonl"))
        .open();
    s.assert_absent("raw output");
    // Alt+R turns it on without a word in the transcript
    s.keys("alt+r");
    s.assert_contains("raw output");
    s.assert_absent("Raw output mode");
    // `/raw` turns it off and says so
    s.type_text("/raw").keys("enter");
    s.assert_contains("Raw output mode off: rich transcript rendering restored.");
    s.assert_absent("raw output ");
}

// ORACLE chatwidget/tests/status_and_layout.rs:617 flush_answer_stream_requests_scrollback_reflow_for_tables kind=ui owner=ui-pane
#[test]
fn flush_answer_stream_requests_scrollback_reflow_for_tables() {
    kit::na(
        Owner::UiPane,
        "asserts the ConsolidateAgentMessage scrollback-reflow request, an app event that is never drawn; the table rows themselves are the streamed-answer screen tests",
    );
}

const PLAN_TABLE: &str = "| Step | Owner |\n| --- | --- |\n| Verify | Codex |\n";

fn plan_delta(s: &mut Screen, delta: &str) {
    s.feed(
        &serde_json::json!({
            "step": "notify",
            "message": {"method": "item/plan/delta", "params": {
                "threadId": "thread-1", "turnId": "turn-1", "itemId": "plan-1", "delta": delta
            }}
        })
        .to_string(),
    );
}

fn stream_plan_table(s: &mut Screen) {
    for delta in PLAN_TABLE.split_inclusive('\n') {
        plan_delta(s, delta);
    }
    s.assert_text_count("Verify", 1);
}

fn finish_plan_table(s: &mut Screen, text: &str) {
    s.feed(
        &serde_json::json!({
            "step": "notify",
            "message": {"method": "item/completed", "params": {
                "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0,
                "item": {"type": "plan", "id": "plan-1", "text": text}
            }}
        })
        .to_string(),
    );
}

fn assert_one_plan_table(s: &Screen) {
    // MarkdownView uses box-drawing rules (G73).
    s.assert_row_of("Verify", &["Codex", "│"], &[]);
    s.assert_text_count("Verify", 1);
}

fn finalize_streamed_plan(s: &mut Screen) {
    stream_plan_table(s);
    finish_plan_table(s, "");
    finish_plan_table(s, "");
    plan_delta(s, "Late row");
    assert_one_plan_table(s);
    s.assert_absent("Late row");
}

// ORACLE chatwidget/tests/status_and_layout.rs:688 completed_plan_table_tail_skips_provisional_history_insert kind=ui owner=ui-pane
#[test]
fn completed_plan_table_tail_skips_provisional_history_insert() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .inline(&[])
        .script(RUNNING)
        .open();
    finalize_streamed_plan(&mut s);
    let mut owned = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    finalize_streamed_plan(&mut owned);
    owned.feed(TURN_ENDED);
    owned.keys("ctrl+t");
    assert_one_plan_table(&owned);
}

#[test]
fn completed_plan_table_with_full_payload_replaces_the_live_tail() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .script(RUNNING)
        .open();
    stream_plan_table(&mut s);
    finish_plan_table(&mut s, &PLAN_TABLE.replace("Verify", "Publish"));
    finish_plan_table(&mut s, "");
    s.assert_row_of("Publish", &["Codex", "│"], &[]);
    s.assert_text_count("Publish", 1);
    s.assert_absent("Verify");
}

// ORACLE chatwidget/tests/status_and_layout.rs:839 rate_limit_usage_warnings_early_threshold_is_scoped_and_deduplicated kind=snap owner=ui-pane
#[test]
fn rate_limit_usage_warnings_early_threshold_is_scoped_and_deduplicated() {
    for (plan, minutes, early) in [
        (Some("plus"), Some(300), true),
        (Some("team"), Some(299), true),
        (Some("pro"), Some(300), false),
        (Some("business"), Some(300), false),
        (Some("self_serve_business_prolite"), Some(300), false),
        (None, Some(300), false),
        (Some("plus"), Some(10080), false),
        (Some("team"), None, false),
    ] {
        // the account read (49% used, the plan learned there), then rolling updates without a plan
        let snapshot = |used: i32, plan: Option<&str>| {
            serde_json::json!({"limitId": "codex", "planType": plan,
                "primary": {"usedPercent": used, "windowDurationMins": minutes, "resetsAt": null}})
        };
        let read = serde_json::json!({"step": "respond", "method": "account/rateLimits/read",
            "result": {"rateLimits": snapshot(49, plan)}});
        let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
        s.feed(&format!("{read}\n{ACCOUNT_USAGE}\n"));
        s.assert_requests_of("account/rateLimits/read", 1);
        s.assert_rows_with(WARNING, 0);
        let roll = |s: &mut Screen, used: i32| {
            s.feed(&format!("{}\n", rolling(&snapshot(used, None))));
        };
        // the 50% crossing warns on the early-threshold plans only
        roll(&mut s, 50);
        let seen = usize::from(early);
        s.assert_rows_with(WARNING, seen);
        if early {
            s.assert_has_row(
                "⚠ Heads up, you have less than 50% of your 5h limit left. Run /status for a",
            );
            s.assert_has_row("  breakdown.");
        }
        roll(&mut s, 50);
        s.assert_rows_with(WARNING, seen);
        // each further threshold warns once, however often it is repeated
        for (n, used) in [75, 90, 95].into_iter().enumerate() {
            roll(&mut s, used);
            s.assert_rows_with(WARNING, seen + n + 1);
            roll(&mut s, used);
            s.assert_rows_with(WARNING, seen + n + 1);
        }
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:906 rate_limit_duration_labels_only_render_supported_windows kind=ui owner=ui-pane
#[test]
fn rate_limit_duration_labels_only_render_supported_windows() {
    // oracle: a 2-hour window has no label (the generic `usage`); 24 hours is "daily"; 365 days is
    // "annual". The labels are the warning's: each window crosses 75% used on a screen of its own.
    for (minutes, label) in [
        (120, "usage"),
        (24 * 60, "daily"),
        (365 * 24 * 60, "annual"),
    ] {
        let mut s = Screen::build(Owner::UiPane).size(120, 24).live().open();
        let snapshot = serde_json::json!({"limitId": "codex",
            "primary": {"usedPercent": 75, "windowDurationMins": minutes, "resetsAt": null}});
        s.feed(&format!("{ACCOUNT_USAGE}\n{}\n", rolling(&snapshot)));
        s.assert_contains(&format!(
            "Heads up, you have less than 25% of your {label} limit left."
        ));
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:1550 rate_limit_switch_prompt_clears_when_account_usage_recovers kind=snap owner=ui-pane snap=rate_limit_switch_prompt_recovered
#[test]
fn rate_limit_switch_prompt_clears_when_account_usage_recovers() {
    // oracle: a 95% snapshot shows the switch prompt; recovery to 0% starts the countdown "Dismissing in 1 second…";
    // the prompt is gone at the deadline; the recovered prompt is the snapshot. A read that a rolling update
    // overtook (the oracle's stale read) confirms no recovery.
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(&clock(START));
    s.feed(&format!(
        "{}\n{}\n{}\n",
        luna_catalog(),
        read_of(&limit_snapshot(0, None)),
        ACCOUNT_USAGE
    ));
    // a rolling 95% over a read of 0% in flight: the read is answered stale, the prompt stays
    let hold = s.hold_request("account/rateLimits/read");
    let reached = limit_snapshot(95, Some("rate_limit_reached"));
    s.feed(&format!("{}\n{}\n", rolling(&reached), TURN_ENDED));
    s.assert_contains("Approaching rate limits");
    s.feed(&format!("{}\n", rolling(&limit_snapshot(95, None))));
    s.release(&hold);
    s.assert_contains("Approaching rate limits");
    s.assert_absent("Dismissing in");
    // a read made after the last update confirms it
    s.feed(&format!("{}\n", rolling(&reached)));
    assert_popup(
        &s,
        "rate_limit_switch_prompt_recovered",
        "You’re no longer close to your usage limit.",
    );
    s.feed(&clock(START + 1_000));
    s.assert_contains("Dismissing in 4 seconds…");
    s.feed(&clock(START + 4_000));
    s.assert_contains("Dismissing in 1 second…");
    s.feed(&clock(START + 5_000));
    s.assert_absent("Dismissing in");
    s.assert_absent("no longer close");
}

// ORACLE chatwidget/tests/status_and_layout.rs:1585 rate_limit_recovery_prompt_can_be_dismissed_manually kind=ui owner=ui-pane
#[test]
fn rate_limit_recovery_prompt_can_be_dismissed_manually() {
    // oracle: Enter and Esc both dismiss the recovery prompt (each on a fresh chat).
    for key in ["enter", "esc"] {
        let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
        s.feed(&clock(START));
        s.feed(&format!(
            "{}\n{}\n{}\n",
            luna_catalog(),
            read_of(&limit_snapshot(95, None)),
            ACCOUNT_USAGE
        ));
        s.feed(&format!("{TURN_ENDED}\n"));
        s.assert_contains("Approaching rate limits");
        // the usage is read again: low
        s.feed(&format!(
            "{}\n{}\n",
            read_of(&limit_snapshot(0, None)),
            rolling(&limit_snapshot(95, Some("rate_limit_reached")))
        ));
        s.assert_contains("Dismissing in 5 seconds…");
        s.keys(key);
        s.assert_absent("Dismissing in");
        s.assert_absent("no longer close");
        // dismissed for good: the countdown is not drawn again when it would have run out
        s.feed(&clock(START + 6_000));
        s.assert_absent("Approaching rate limits");
        s.assert_absent("no longer close");
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:1779 rate_limit_usage_warnings_keep_newly_reached_workspace_limit kind=ui owner=ui-pane
#[test]
fn rate_limit_usage_warnings_keep_newly_reached_workspace_limit() {
    // oracle: a newly reached workspace limit on the codex limit id warns and shows the request popup; another limit id does not.
    for (limit_id, should_warn) in [("codex", true), ("codex_other", false)] {
        let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
        let mut funded = limit_snapshot(0, None);
        funded["credits"] =
            serde_json::json!({"hasCredits": true, "unlimited": false, "balance": null});
        funded["spendControlReached"] = serde_json::json!(false);
        s.feed(&format!(
            "{}\n{}\n{}\n",
            luna_catalog(),
            read_of(&funded),
            ACCOUNT_USAGE
        ));
        let mut capped = limit_snapshot(95, Some("workspace_member_usage_limit_reached"));
        capped["limitId"] = serde_json::json!(limit_id);
        capped["credits"] = funded["credits"].clone();
        // the app reads the limits again at a reached limit: the read says what the update did
        s.feed(&format!("{}\n{}\n", read_of(&capped), rolling(&capped)));
        s.assert_rows_with(WARNING, usize::from(should_warn));
        // the prompt that waits for the end of the turn is the plan's own limit's
        s.feed(&format!("{TURN_ENDED}\n"));
        s.assert_present("Approaching rate limits", should_warn);
        if should_warn {
            s.keys("esc");
        }
        s.feed(&format!("{}\n", limit_error(true)));
        s.assert_contains("Request a limit increase from your owner");
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:1891 rolling_credits_preserve_depleted_workspace_error_routing kind=ui owner=ui-pane
#[test]
fn rolling_credits_preserve_depleted_workspace_error_routing() {
    // oracle: depleted workspace member credits route the error to the credits prompt, however a
    // rolling update that carries credits (a zero balance, 95% used) reads.
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    let blocked = limit_snapshot(0, Some("workspace_member_credits_depleted"));
    let mut rolling_limits = limit_snapshot(95, None);
    rolling_limits["credits"] =
        serde_json::json!({"hasCredits": true, "unlimited": false, "balance": "0"});
    rolling_limits["spendControlReached"] = serde_json::json!(false);
    s.feed(&format!(
        "{}\n{}\n{}\n",
        rolling(&blocked),
        rolling(&rolling_limits),
        limit_error(false)
    ));
    s.assert_contains("Ask your workspace owner to add more");
}

// ORACLE chatwidget/tests/status_and_layout.rs:2059 rate_limit_switch_prompt_popup_snapshot kind=snap owner=ui-pane snap=rate_limit_switch_prompt_popup
#[test]
fn rate_limit_switch_prompt_popup_snapshot() {
    // oracle: a 92% snapshot shows the switch prompt popup at width 80, once the turn is over.
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(&format!(
        "{}\n{}\n{}\n",
        luna_catalog(),
        rolling(&limit_snapshot(92, None)),
        TURN_ENDED
    ));
    assert_popup(
        &s,
        "rate_limit_switch_prompt_popup",
        "Approaching rate limits",
    );
}

// ORACLE chatwidget/tests/status_and_layout.rs:2071 workspace_member_credits_depleted_prompts_and_sends_credits kind=snap owner=ui-pane snap=workspace_member_credits_depleted_prompt
#[test]
fn workspace_member_credits_depleted_prompts_and_sends_credits() {
    // oracle: the depleted-credits prompt at width 90; y sends the credits nudge.
    let mut s = Screen::build(Owner::UiPane).size(90, 24).live().open();
    let limits = limit_snapshot(100, Some("workspace_member_credits_depleted"));
    s.feed(&format!("{}\n{}\n", rolling(&limits), limit_error(false)));
    assert_popup(
        &s,
        "workspace_member_credits_depleted_prompt",
        "You've reached your workspace credit limit",
    );
    s.keys("y");
    s.assert_requests_of("account/sendAddCreditsNudgeEmail", 1);
}

#[test]
fn promoted_owner_confirmation_uses_remapped_picker_keys_and_keeps_the_draft() {
    let limits = limit_snapshot(100, Some("workspace_member_credits_depleted"));
    for accept in [false, true] {
        let mut s = Screen::build(Owner::UiPane)
            .size(40, 16)
            .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
            .live()
            .open();
        s.type_text("draft in progress");
        s.feed(&format!("{}\n{}\n", rolling(&limits), limit_error(false)));
        s.wait_for("Yes (y)");
        s.assert_contains("f3 select");
        s.assert_contains("f2 back");
        s.assert_requests_of("account/sendAddCreditsNudgeEmail", 0);
        s.keys(if accept { "up f3" } else { "f2" });
        s.assert_absent("Yes (y)");
        s.assert_requests_of("account/sendAddCreditsNudgeEmail", usize::from(accept));
        s.assert_composer("draft in progress");
        s.assert_no_turn();
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2090 workspace_member_usage_limit_prompts_and_sends_usage_limit kind=snap owner=ui-pane snap=workspace_member_usage_limit_prompt
#[test]
fn workspace_member_usage_limit_prompts_and_sends_usage_limit() {
    // oracle: the usage-limit prompt at width 100; y sends the usage-limit nudge.
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    let limits = limit_snapshot(100, Some("workspace_member_usage_limit_reached"));
    s.feed(&format!("{}\n{}\n", rolling(&limits), limit_error(true)));
    assert_popup(
        &s,
        "workspace_member_usage_limit_prompt",
        "Usage limit reached",
    );
    s.keys("y");
    s.assert_requests_of("account/sendAddCreditsNudgeEmail", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:2109 sparse_rate_limit_snapshot_preserves_member_limit_type_for_error_prompt kind=ui owner=ui-pane
#[test]
fn sparse_rate_limit_snapshot_preserves_member_limit_type_for_error_prompt() {
    // oracle: a sparse rolling snapshot keeps the member limit type, so the usage-limit prompt still shows; y sends the usage nudge.
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    let limits = limit_snapshot(100, Some("workspace_member_usage_limit_reached"));
    s.feed(&format!(
        "{}\n{}\n{}\n",
        rolling(&limits),
        rolling(&limit_snapshot(100, None)),
        limit_error(true)
    ));
    s.assert_contains("Request a limit increase from your owner");
    s.keys("y");
    s.assert_requests_of("account/sendAddCreditsNudgeEmail", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:2136 usage_limit_error_remaps_stale_member_credits_state_to_usage_limit_prompt kind=ui owner=ui-pane
#[test]
fn usage_limit_error_remaps_stale_member_credits_state_to_usage_limit_prompt() {
    // oracle: a stale member-credits state is remapped to the usage-limit prompt by a usage-limit error; y sends the usage nudge.
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    let limits = limit_snapshot(100, Some("workspace_member_credits_depleted"));
    s.feed(&format!("{}\n{}\n", rolling(&limits), limit_error(true)));
    s.assert_contains("Request a limit increase from your owner");
    s.keys("y");
    s.assert_requests_of("account/sendAddCreditsNudgeEmail", 1);
}

// ORACLE chatwidget/tests/status_and_layout.rs:2158 workspace_owner_limit_states_do_not_prompt_for_owner_nudge kind=ui owner=ui-pane
#[test]
fn workspace_owner_limit_states_do_not_prompt_for_owner_nudge() {
    // oracle: each of the owner-depleted, owner-usage and plain rate-limit states (fresh chat each) shows no owner nudge.
    for (reached, usage_limit) in [
        ("workspace_owner_credits_depleted", false),
        ("workspace_owner_usage_limit_reached", true),
        ("rate_limit_reached", false),
    ] {
        let mut s = Screen::build(Owner::UiPane).size(90, 24).live().open();
        let limits = limit_snapshot(100, Some(reached));
        s.feed(&format!(
            "{}\n{}\n",
            rolling(&limits),
            limit_error(usage_limit)
        ));
        s.assert_absent("workspace owner");
        s.assert_absent("Request a limit increase");
        s.assert_requests_of("account/sendAddCreditsNudgeEmail", 0);
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2186 workspace_owner_limit_states_render_state_specific_messages kind=snap owner=ui-pane snap=workspace_owner_limit_state_messages
#[test]
fn workspace_owner_limit_states_render_state_specific_messages() {
    // oracle: each owner limit state renders its own message (the joined snapshot has one case per state),
    // not the server's words.
    for (reached, usage_limit, expected) in [
        (
            "workspace_owner_credits_depleted",
            false,
            "■ You're out of credits. Your workspace is out of credits. Add credits to continue using Codex.",
        ),
        (
            "workspace_owner_usage_limit_reached",
            true,
            "■ Usage limit reached. You've reached your usage limit. Increase your limits to continue using codex.",
        ),
    ] {
        let mut s = Screen::build(Owner::UiPane).size(120, 24).live().open();
        s.feed(&format!(
            "{}\n{}\n",
            rolling(&limit_snapshot(100, Some(reached))),
            limit_error(usage_limit)
        ));
        s.assert_has_row(expected);
        if !usage_limit {
            s.assert_absent("Usage limit reached");
        }
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2223 academic_researchers_usage_limit_renders_admin_guidance kind=snap owner=ui-pane
#[test]
fn academic_researchers_usage_limit_renders_admin_guidance() {
    // oracle: a UsageLimit error on the research plan says to ask the admin. The words are codex-protocol's
    // `UsageLimitReachedError` display (the server's message); the chat draws it as the error cell.
    let mut s = Screen::build(Owner::UiPane).size(120, 24).live().open();
    let error = r#"{"step":"notify","message":{"method":"error","params":{"threadId":"thread-1","turnId":"turn-1","willRetry":false,"error":{"message":"You’ve hit your usage limit. To get more access now, send a request to your admin or try again later.","codexErrorInfo":"usageLimitExceeded","additionalDetails":null,"misalignment":null}}}}"#;
    s.feed(&format!("{error}\n"));
    s.assert_has_row(
        "■ You’ve hit your usage limit. To get more access now, send a request to your admin or try again later.",
    );
}

// ORACLE chatwidget/tests/status_and_layout.rs:2246 missing_rate_limit_reached_type_does_not_prompt_or_refresh kind=ui owner=ui-pane
#[test]
fn missing_rate_limit_reached_type_does_not_prompt_or_refresh() {
    // oracle: without a rate_limit_reached_type no owner prompt shows (the refresh is an event, not drawn).
    let mut s = Screen::build(Owner::UiPane).size(90, 24).live().open();
    s.feed(&format!(
        "{}\n{}\n",
        rolling(&limit_snapshot(100, None)),
        limit_error(true)
    ));
    s.assert_absent("workspace owner");
    s.assert_absent("Request a limit increase");
}

// ORACLE chatwidget/tests/status_and_layout.rs:2260 workspace_owner_nudge_default_no_dismisses_without_sending kind=ui owner=ui-pane
#[test]
fn workspace_owner_nudge_default_no_dismisses_without_sending() {
    kit::na(
        Owner::UiPane,
        "asserts that Enter sends no add-credits nudge and no rate-limit refresh (app events), nothing drawn",
    );
}

// ORACLE chatwidget/tests/status_and_layout.rs:2276 workspace_owner_nudge_reappears_after_dismissing_no kind=ui owner=ui-pane
#[test]
fn workspace_owner_nudge_reappears_after_dismissing_no() {
    // oracle: Enter dismisses the first nudge (no request); the next usage-limit error shows the request popup again.
    let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
    let limits = limit_snapshot(100, Some("workspace_member_usage_limit_reached"));
    s.feed(&format!("{}\n{}\n", rolling(&limits), limit_error(true)));
    s.assert_contains("Request a limit increase from your owner");
    s.keys("enter");
    s.assert_absent("Request a limit increase from your owner");
    s.assert_requests_of("account/sendAddCreditsNudgeEmail", 0);
    s.feed(&format!("{}\n", limit_error(true)));
    s.assert_contains("Request a limit increase from your owner");
}

// ORACLE chatwidget/tests/status_and_layout.rs:2301 workspace_owner_credits_nudge_completion_renders_feedback kind=snap owner=ui-pane snap=workspace_owner_credits_nudge_completion_feedback
#[test]
fn workspace_owner_credits_nudge_completion_renders_feedback() {
    // oracle: the completed credits nudge says "Workspace owner notified.", the cooldown says "...already notified recently."
    // (one fresh chat each; the failed request is the fake backend's gap: it answers no error).
    for (status, text) in [
        ("sent", "Workspace owner notified."),
        (
            "cooldown_active",
            "Workspace owner was already notified recently.",
        ),
    ] {
        let mut s = Screen::build(Owner::UiPane).size(90, 24).live().open();
        let limits = limit_snapshot(100, Some("workspace_member_credits_depleted"));
        let answer = serde_json::json!({"step": "respond", "method": "account/sendAddCreditsNudgeEmail",
            "result": {"status": status}});
        s.feed(&format!(
            "{answer}\n{}\n{}\n",
            rolling(&limits),
            limit_error(false)
        ));
        s.keys("y");
        s.assert_has_row(&format!("• {text}"));
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2339 workspace_owner_usage_limit_nudge_completion_renders_feedback kind=snap owner=ui-pane snap=workspace_owner_usage_limit_nudge_completion_feedback
#[test]
fn workspace_owner_usage_limit_nudge_completion_renders_feedback() {
    // oracle: the completed usage-limit nudge says "Limit increase requested.", the cooldown "...already requested recently."
    // (one fresh chat each; the failed request is the fake backend's gap: it answers no error).
    for (status, text) in [
        ("sent", "Limit increase requested."),
        (
            "cooldown_active",
            "A limit increase was already requested recently.",
        ),
    ] {
        let mut s = Screen::build(Owner::UiPane).size(100, 24).live().open();
        let limits = limit_snapshot(100, Some("workspace_member_usage_limit_reached"));
        let answer = serde_json::json!({"step": "respond", "method": "account/sendAddCreditsNudgeEmail",
            "result": {"status": status}});
        s.feed(&format!(
            "{answer}\n{}\n{}\n",
            rolling(&limits),
            limit_error(true)
        ));
        s.keys("y");
        s.assert_has_row(&format!("• {text}"));
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2410 streaming_final_answer_keeps_task_running_state kind=ui owner=ui-pane
#[test]
fn streaming_final_answer_keeps_task_running_state() {
    // oracle: a final answer streams while the task runs; the status indicator is hidden; Tab queues the composer text;
    // Ctrl+C sends an interrupt (an event, not drawn).
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    // needs feed: turn started, agent message delta "Final answer line", commit tick
    s.type_text("queued submission");
    s.keys("tab");
    s.assert_contains("queued submission");
    s.keys("ctrl+c");
}

// ORACLE chatwidget/tests/status_and_layout.rs:2442 answer_phase_controls_working_status_snapshot kind=snap owner=ui-pane snap=single_line_final_answer_hides_working_status snap=single_line_partial_answer_keeps_working_status
#[test]
fn answer_phase_controls_working_status_snapshot() {
    for (phase, working) in [
        (Some("final_answer"), false),
        (Some("partial_answer"), true),
        (Some("commentary"), true),
        (None, false),
    ] {
        let mut s = Screen::build(Owner::UiPane)
            .size(40, 10)
            .script(include_str!("fixtures/chat_status_1/running_turn.jsonl"))
            .open();
        s.assert_contains("Working");
        s.feed(
            &serde_json::json!({"step": "notify", "message": {
                "method": "item/started", "params": {
                    "threadId": "thread-1", "turnId": "turn-1",
                    "item": {"type": "agentMessage", "id": "msg-1", "text": "", "phase": phase},
                    "startedAtMs": 0
                }
            }})
            .to_string(),
        );
        s.assert_contains("Working");
        s.assert_absent("The answer is one.");
        s.feed(&serde_json::json!({"step": "notify", "message": {
            "method": "item/completed", "params": {
                "threadId": "thread-1", "turnId": "turn-1",
                "item": {"type": "agentMessage", "id": "msg-1", "text": "The answer is one.", "phase": phase},
                "completedAtMs": 0
            }
        }}).to_string());
        s.assert_contains("The answer is one.");
        s.assert_present("Working", working);
        s.feed(r#"{"step":"notify","message":{"method":"warning","params":{"threadId":"thread-1","message":"Unrelated warning after answer."}}}"#);
        s.assert_present("Working", working);
        s.type_text("queued submission");
        s.keys("tab");
        s.assert_composer("");
        s.assert_contains("queued submission");
        s.assert_no_turn();
        s.assert_present("Working", working);
        if !working {
            s.feed(r#"{"step":"notify","message":{"method":"item/started","params":{"threadId":"thread-1","turnId":"turn-1","item":{"type":"commandExecution","id":"after-final","command":"echo done","cwd":"/tmp","processId":null,"source":"agent","status":"inProgress","commandActions":[{"type":"unknown","command":"echo done"}],"aggregatedOutput":null,"exitCode":null,"durationMs":null},"startedAtMs":0}}}"#);
            s.assert_contains("Working");
        }
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2490 ctrl_c_interrupt_pauses_active_goal_turn kind=ui owner=ui-pane
#[test]
fn ctrl_c_interrupt_pauses_active_goal_turn() {
    kit::na(
        Owner::UiPane,
        "asserts Op::Interrupt and the goal-paused app event after Ctrl+C (events), nothing drawn in this test",
    );
}

// ORACLE chatwidget/tests/status_and_layout.rs:2501 program_status_stays_working_when_input_acknowledges_an_active_goal kind=ui owner=ui-pane
#[test]
fn program_status_stays_working_when_input_acknowledges_an_active_goal() {
    kit::na(
        Owner::UiPane,
        "asserts the ProgramStatus values of desired_program_status; the status goes out as OSC 7501/21337 sequences to the terminal, not drawn on the screen grid",
    );
}

// ORACLE chatwidget/tests/status_and_layout.rs:2532 esc_interrupt_pauses_active_goal_turn kind=snap owner=ui-pane snap=esc_interrupt_goal_paused_footer
#[test]
fn esc_interrupt_pauses_active_goal_turn() {
    // oracle: Esc interrupts an active goal turn; the goal is paused (the server tells the new
    // status, `update_thread_goal`); the goal-paused footer is the snapshot (width 80).
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(include_str!("fixtures/chat_status_1/goal_turn.jsonl"));
    s.keys("esc");
    s.feed(
        r#"{"step":"notify","message":{"method":"thread/goal/updated","params":{"threadId":"thread-1","turnId":null,"goal":{"threadId":"thread-1","objective":"Keep improving the benchmark","status":"paused","tokenBudget":null,"tokensUsed":0,"timeUsedSeconds":0,"createdAt":0,"updatedAt":0}}}}"#,
    );
    let working = s
        .rows()
        .iter()
        .position(|row| row.contains("Working (0s"))
        .unwrap_or_else(|| panic!("no working row:\n{}", s.text()));
    // the spinner glyph follows the clock
    s.assert_snap_rows_from("esc_interrupt_goal_paused_footer", working - 1, 8, true);
}

// ORACLE chatwidget/tests/status_and_layout.rs:2559 request_user_input_interrupt_pauses_active_goal_turn kind=ui owner=ui-pane
#[test]
fn request_user_input_interrupt_pauses_active_goal_turn() {
    kit::na(
        Owner::UiPane,
        "asserts Op::Interrupt and the goal-paused app event after Esc or Ctrl+C during a user-input request (events), nothing drawn in this test",
    );
}

// ORACLE chatwidget/tests/status_and_layout.rs:2644 final_answer_completion_restores_status_indicator_for_pending_steer kind=ui owner=ui-pane
#[test]
fn final_answer_completion_restores_status_indicator_for_pending_steer() {
    // oracle: Enter during a running turn queues a steer; the final answer keeps the Working status; the steer's user message clears the queue.
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    // needs feed: turn started, two partial deltas and commit ticks
    s.type_text("Please summarize the rest more briefly.");
    s.keys("enter");
    s.assert_contains("Please summarize the rest more briefly.");
    // needs feed: final answer message, then the steer's user message
    s.assert_contains("Working");
}

// ORACLE chatwidget/tests/status_and_layout.rs:2732 ui_snapshots_small_heights_idle kind=snap owner=ui-pane
#[test]
fn ui_snapshots_small_heights_idle() {
    // oracle: width 40, heights 1, 2 and 3 for the idle chat.
    for (h, snap) in [
        (1u16, "chat_small_idle_h1"),
        (2, "chat_small_idle_h2"),
        (3, "chat_small_idle_h3"),
    ] {
        let s = Screen::build(Owner::UiPane).size(40, h).live().open();
        s.assert_snap(snap);
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2748 ui_small_heights_hide_running_state kind=ui owner=ui-pane
#[test]
fn ui_small_heights_hide_running_state() {
    // oracle: at width 40 and heights 1, 2 and 3, the running chat (turn started, reasoning "**Thinking**") draws the same rows as the idle chat.
    // (the reasoning delta only sets the header, which is hidden with the row)
    for h in [1u16, 2, 3] {
        let idle = Screen::build(Owner::UiPane).size(40, h).live().open();
        let running = Screen::build(Owner::UiPane)
            .size(40, h)
            .script(include_str!("fixtures/chat_status_1/running_turn.jsonl"))
            .open();
        running.assert_same_rows(&idle.shot());
    }
}

// ORACLE chatwidget/tests/status_and_layout.rs:2774 ambient_pet_stays_hidden_until_a_pet_is_selected kind=ui owner=ui-pane
#[test]
fn ambient_pet_stays_hidden_until_a_pet_is_selected() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 20)
        .kitty()
        .live()
        .open();
    s.assert_ambient_pet(None);
    s.type_text("/pets codex").keys("enter");
    s.pets_catalog();
    s.pets_preview("codex");
    s.assert_ambient_pet(None);
    s.pets_save_accepted("codex");
    s.pets_ambient(Some("codex"), false);
    s.assert_ambient_pet(Some((51, 14, 9, 5)));
    s.feed(RUNNING);
    s.assert_ambient_pet(Some((51, 14, 9, 5)));
}

// ORACLE chatwidget/tests/status_and_layout.rs:2815 ambient_pet_screen_bottom_anchor_uses_terminal_bottom kind=ui owner=ui-pane
#[test]
fn ambient_pet_screen_bottom_anchor_uses_terminal_bottom() {
    kit::pending(
        Owner::UiPane,
        "rtwk inline graphics are clipped to the active viewport; the physical bottom cannot be addressed when the viewport ends earlier",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .kitty()
        .live()
        .open();
    s.pets_ambient(None, true);
    s.assert_ambient_pet(Some((71, 18, 9, 5)));
}

// ORACLE chatwidget/tests/status_and_layout.rs:2849 added_history_uses_pet_adjusted_terminal_width kind=snap owner=ui-pane
#[test]
fn added_history_uses_pet_adjusted_terminal_width() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 10)
        .kitty()
        .live()
        .open();
    s.pets_ambient(None, false);
    s.feed(RUNNING);
    s.feed(&agent_message(
        "width-sensitive history uses available columns for its answer, and wraps here finally.",
    ));
    s.feed(TURN_ENDED);
    s.assert_contains("width-sensitive history uses available columns for its answer, and");
    s.assert_absent("and wraps here finally.");
    s.assert_contains("wraps here finally.");
}

// ORACLE chatwidget/tests/status_and_layout.rs:2917 ambient_pet_reduces_stream_width_and_composer_text_width kind=ui owner=ui-pane
#[test]
fn ambient_pet_reduces_stream_width_and_composer_text_width() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 12)
        .kitty()
        .live()
        .open();
    let text =
        "Minim commodo esse elit Lorem exercitation elit ipsum proident labore. Esse culpa aliqua";
    s.pets_ambient(None, false);
    s.type_text(text);
    s.assert_contains("Minim commodo esse elit Lorem exercitation elit ipsum proident");
    s.assert_absent("proident labore.");
    s.assert_contains("labore. Esse culpa aliqua");
    s.keys("ctrl+u");
    s.type_text("/pets disable").keys("enter");
    s.pets_save_accepted("disabled");
    s.type_text(text);
    s.assert_contains("proident labore.");
}

// ORACLE chatwidget/tests/status_and_layout.rs:2988 ambient_pet_draw_uses_terminal_screen_area_not_short_inline_viewport kind=ui owner=ui-pane
#[test]
fn ambient_pet_draw_uses_terminal_screen_area_not_short_inline_viewport() {
    let mut small = Screen::build(Owner::UiPane)
        .size(80, 3)
        .kitty()
        .inline(&[])
        .live()
        .open();
    small.pets_ambient(None, false);
    small.assert_ambient_pet(None);
    let mut full = Screen::build(Owner::UiPane)
        .size(80, 24)
        .kitty()
        .live()
        .open();
    full.pets_ambient(None, true);
    full.assert_ambient_pet(Some((71, 18, 9, 5)));
}

// ORACLE chatwidget/tests/status_and_layout.rs:3019 ambient_pet_hides_notification_text_overlay kind=ui owner=ui-pane
#[test]
fn ambient_pet_hides_notification_text_overlay() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 20)
        .kitty()
        .live()
        .open();
    s.pets_ambient(None, false);
    s.feed(RUNNING);
    s.assert_ambient_pet(Some((51, 14, 9, 5)));
    s.assert_absent("Running");
    s.assert_absent("Needs input");
    s.feed(TURN_ENDED);
    s.assert_ambient_pet(Some((51, 14, 9, 5)));
    s.assert_absent("Ready");
    s.assert_absent("Blocked");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3047 status_widget_and_approval_modal_snapshot kind=snap owner=ui-pane snap=status_widget_and_approval_modal
#[test]
fn status_widget_and_approval_modal_snapshot() {
    // oracle: width 100 at the pane's desired height (14 rows); the modal replaces the status row.
    let s = Screen::build(Owner::UiPane)
        .size(100, 14)
        .script(include_str!(
            "fixtures/chat_status_1/running_exec_approval.jsonl"
        ))
        .open();
    s.assert_snap("status_widget_and_approval_modal");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3095 status_widget_active_snapshot kind=snap owner=ui-pane snap=status_widget_active
#[test]
fn status_widget_active_snapshot() {
    // Probed colours keep the pulsing glyph fixed while the clock starts with this turn.
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 8)
        .terminal_colors(0xffffff, 0x000000)
        .live()
        .open();
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}
{"step":"screen","dir":"chat","ev":{"status":{"model":"GPT-5.6-Sol","cwd":"/tmp/project","items":["model-with-reasoning","current-dir"]}}}
{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#);
    s.feed(&lifecycle(&reasoning_parts("analysis", &[]), false));
    s.feed(&summary_delta("analysis", "**Analyzing**"));
    s.assert_snap("status_widget_active");
}

// ORACLE chatwidget/tests/status_and_layout.rs:3157 warning_event_adds_warning_history_cell kind=ui owner=ui-pane
#[test]
fn warning_event_adds_warning_history_cell() {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(r#"{"step":"notify","message":{"method":"warning","params":{"threadId":"thread-1","message":"test warning message"}}}"#);
    s.assert_absent("test warning message");
    s.keys("f2");
    s.assert_text_count("test warning message", 1);
}
