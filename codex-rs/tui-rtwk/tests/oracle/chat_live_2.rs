//! Shard `chat_live_2` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: chatwidget/tests/backend_banners_tests.rs (12 screen-level tests here; 0 state tests left to chat-live)

/// The account read's answer (`banner_response`): a `selected_model_limit` banner on `test-model-a`
/// for the account `workspace-a`; `edit` changes the banner's fields.
fn banner_response(
    presentation: Option<&str>,
    ctas: serde_json::Value,
    edit: impl Fn(&mut serde_json::Value),
) -> serde_json::Value {
    let mut banner = serde_json::json!({
        "banner_type": "selected_model_limit", "model_slug": "test-model-a",
        "title": "Selected model usage exhausted",
        "description": "Ask your owner for credits or switch to another model.", "ctas": ctas,
    });
    if let Some(presentation) = presentation {
        banner["presentation"] = serde_json::json!(presentation);
    }
    edit(&mut banner);
    serde_json::json!({"accountId": "workspace-a", "rateLimits": {}, "rateLimitUpsell": banner})
}

/// The kit step that makes the fake backend answer the next account read with `response`.
fn answer(response: &serde_json::Value) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "respond", "method": "account/rateLimits/read", "result": response})
    )
}

/// The turn `turn-1` of the live thread starts (`set_task_running(true)`).
const TURN_STARTED: &str = r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#;

/// `set_model` (the config's model in use) and, with `read`, the account read that follows
/// the status facts' `account_usage`.
fn model_step(model: &str, read: bool) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "screen", "dir": "chat", "ev": {"status": {"model": model, "account_usage": read}}})
    )
}

/// The chat of the oracle's `make_chatwidget_manual` (agents navigation off, the status line of
/// its footer: the model and the directory) on `model`, with a ChatGPT account whose read says
/// the Reserve snapshot (`reserve_snapshot`: 5h and weekly windows of `gpt-reserve`).
fn reserve_chat(width: u16, model: &str, primary: i32, weekly: i32) -> Screen {
    reserve_chat_in(width, model, primary, weekly, None, None)
}

/// `reserve_chat` in a terminal whose default colours are `colors` (foreground, background),
/// the read carrying the banner `upsell` (`update_backend_banner`).
fn reserve_chat_in(
    width: u16,
    model: &str,
    primary: i32,
    weekly: i32,
    colors: Option<(u32, u32)>,
    upsell: Option<&serde_json::Value>,
) -> Screen {
    let open = Screen::build(Owner::ChatLive).size(width, 24).live();
    let mut s = match colors {
        Some((fg, bg)) => open.terminal_colors(fg, bg).open(),
        None => open.open(),
    };
    s.feed(r#"{"step":"screen","dir":"chat","ev":{"facts":{"agents_navigation":false}}}
{"step":"screen","dir":"status","ev":{"kind":"account","account":{"chat_gpt":{"email":"user@example.com","plan":"plus"}}}}"#);
    let window = |used: i32, mins: i32| serde_json::json!({"usedPercent": used, "windowDurationMins": mins, "resetsAt": null});
    let mut response = serde_json::json!({"rateLimits": {
        "limitId": "base_model_inference", "limitName": "gpt-reserve",
        "primary": window(primary, 300), "secondary": window(weekly, 10080)}});
    if let Some(upsell) = upsell {
        response["rateLimitUpsell"] = upsell.clone();
    }
    s.feed(&answer(&response));
    s.feed(&reserve_status(model));
    s
}

/// `set_model`: the status facts of `reserve_chat` on `model`.
fn reserve_status(model: &str) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "screen", "dir": "chat", "ev": {"status": {
            "model": model, "cwd": "/tmp/project", "items": ["model-with-reasoning", "current-dir"],
            "account_usage": true}}})
    )
}

/// The first screen row that has `text`.
fn row_with(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains(text))
        .unwrap_or_else(|| panic!("no row has `{text}`:\n{}", s.text()))
}

/// A live chat on `model` whose account read carried `response`: the banner is on screen.
fn banner_chat(width: u16, model: &str, response: &serde_json::Value) -> Screen {
    let mut s = Screen::build(Owner::ChatLive).size(width, 24).live().open();
    s.feed(&(answer(response) + &model_step(model, true)));
    s.wait_for("Selected model usage exhausted");
    s
}

const OWNER_NUDGE: &str = "account/sendAddCreditsNudgeEmail";
const CREDITS_DEPLETED: &str = "workspace_member_credits_depleted";
const USAGE_CAP_REACHED: &str = "workspace_member_usage_limit_reached";
const REQUEST_INCREASE: &str = "Request a limit increase from your owner";

fn exhausted_workspace(reached: &str) -> serde_json::Value {
    serde_json::json!({
        "limitId": "codex", "rateLimitReachedType": reached,
        "primary": {"usedPercent": 100, "windowDurationMins": 300, "resetsAt": null},
    })
}

fn rate_limit_error(turn: &str, info: serde_json::Value) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "notify", "message": {"method": "error", "params": {
            "threadId": "thread-1", "turnId": turn, "willRetry": false,
            "error": {"message": "Workspace usage was blocked.", "codexErrorInfo": info},
        }}})
    )
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:29 backend_banner_presentation_and_cta_do_not_imply_recovery kind=snap owner=chat-live
#[test]
fn backend_banner_presentation_and_cta_do_not_imply_recovery() {
    // The oracle, per presentation: the banner shows its title; `esc to dismiss` only when
    // dismissible; key 1 queues the notify-owner email and the popup does not change.
    for presentation in [None, Some("inline"), Some("dismissible")] {
        let ctas = serde_json::json!([{"action": "notify_owner", "label": "Notify owner"}]);
        let response = banner_response(presentation, ctas, |banner| {
            if presentation == Some("inline") {
                // 17:30 (the clock below is UTC; the oracle takes today's date from the clock)
                banner["description"] =
                    serde_json::json!("Your usage resets at {time}. Ask your owner for credits.");
                banner["reset_at"] = serde_json::json!(1_795_541_400_i64);
            }
        });
        let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
        // the viewer's clock: 12-hour time in UTC, so `{time}` does not depend on the machine
        s.feed(&format!(
            "{}\n",
            serde_json::json!({"step": "screen", "dir": "cells", "ev": {"kind": "clock", "format": "twelve_hour", "utc_offset": 0}})
        ));
        s.feed(&(answer(&response) + &model_step("test-model-a", true)));
        s.wait_for("Selected model usage exhausted");
        if presentation == Some("inline") {
            // `{time}` is the reset time in the viewer's clock (12-hour by default)
            s.assert_contains("Your usage resets at 5:30 PM");
            s.assert_absent("{time}");
        }
        let dismissible = presentation == Some("dismissible");
        let initial = s.shot();
        s.assert_present("esc to dismiss", dismissible);
        s.keys("1");
        s.assert_nudges(1);
        s.assert_contains("Selected model usage exhausted");
        s.assert_same_rows(&initial);
        // the request is in flight: a second key 1 starts no other
        s.keys("1");
        s.assert_nudges(1);
        s.assert_composer("");
        // the same banner again changes nothing
        s.feed(&(answer(&response) + &model_step("test-model-a", true)));
        s.assert_same_rows(&initial);
        s.keys("esc");
        s.assert_present("Selected model usage exhausted", !dismissible);
        s.feed(&model_step("test-model-b", false));
        s.assert_absent("Selected model usage exhausted");
        s.feed(&model_step("test-model-a", false));
        s.assert_present("Selected model usage exhausted", !dismissible);
    }
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:292 backend_banner_invalid_content_and_absence_restore_fallback kind=ui owner=chat-live
#[test]
fn backend_banner_invalid_content_and_absence_restore_fallback() {
    for invalid in [
        serde_json::Value::Null,
        serde_json::json!({"presentation": "future_mode"}),
        serde_json::json!({"presentation": null}),
        serde_json::json!({"title": " "}),
    ] {
        let mut read = banner_response(Some("inline"), serde_json::json!([]), |_| {});
        read["rateLimits"] = exhausted_workspace(CREDITS_DEPLETED);
        let mut s = banner_chat(100, "test-model-a", &read);
        if let Some(fields) = invalid.as_object() {
            for (key, value) in fields {
                read["rateLimitUpsell"][key] = value.clone();
            }
        } else {
            read =
                serde_json::json!({"accountId": "workspace-a", "rateLimits": read["rateLimits"]});
        }
        s.feed(&(answer(&read) + &model_step("test-model-a", true)));
        s.assert_absent("Selected model usage exhausted");
        s.feed(&rate_limit_error(
            "turn-1",
            serde_json::json!({"responseTooManyFailedAttempts": {"httpStatusCode": 429}}),
        ));
        s.assert_contains("Ask your workspace owner to add more.");
        s.assert_no_request(OWNER_NUDGE);
        s.feed(
            r#"{"step":"respond","method":"account/sendAddCreditsNudgeEmail","result":{"status":"sent"}}"#,
        );
        s.keys("y");
        s.assert_requests_of(OWNER_NUDGE, 1);
        s.assert_last_request_has(OWNER_NUDGE, r#""creditType":"credits""#);
        s.wait_for("Workspace owner notified.");
    }
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:322 backend_banner_fallback_candidates_and_notice_follow_selected_model kind=ui owner=chat-live
#[test]
fn backend_banner_fallback_candidates_and_notice_follow_selected_model() {
    let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
    // the catalog the picker lists (a hidden model among them), a ChatGPT login, and the
    // server accepting the settings update
    let preset = |model: &str, visible: bool| serde_json::json!({"model": model, "name": model, "show_in_picker": visible});
    s.feed(&format!(
        "{}\n{}\n{}\n",
        serde_json::json!({"step": "screen", "dir": "select", "ev": {"kind": "catalog", "presets": [
            preset("hidden-model", false), preset("test-model-b", true), preset("test-model-c", true),
        ]}}),
        serde_json::json!({"step": "notify", "message": {"method": "account/updated", "params": {"authMode": "chatgpt", "planType": null}}}),
        serde_json::json!({"step": "respond", "method": "thread/settings/update", "result": {}}),
    ));
    let response = banner_response(Some("inline"), serde_json::json!([]), |banner| {
        banner["blocked_model_slug"] = serde_json::json!("test-model-a");
        banner["fallback_model_slugs"] =
            serde_json::json!(["hidden-model", "test-model-c", "test-model-b"]);
    });
    s.feed(&(answer(&response) + &model_step("test-model-a", true)));
    // the first fallback the picker lists, test-model-c, is asked of the thread
    s.wait_for("Automatically switched to test-model-c");
    s.assert_contains("due to usage limits.");
    s.assert_model_updates(&["test-model-c"]);
    // the server's `thread/settings/updated` brings the model; the banner shows for it
    s.feed(&model_step("test-model-c", false));
    s.assert_contains("Selected model usage exhausted");
    s.keys("esc");
    s.assert_contains("Selected model usage exhausted");
    // another task: the banner is inherited and follows the selected model (test-model-a hides
    // it, test-model-c shows it)
    s.feed(&model_step("test-model-a", false));
    s.assert_absent("Selected model usage exhausted");
    s.feed(&model_step("unrelated-model", false));
    s.assert_absent("Selected model usage exhausted");
    s.feed(&model_step("test-model-c", false));
    s.assert_contains("Selected model usage exhausted");
    s.assert_model_updates(&["test-model-c"]);
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:373 backend_banner_sparse_updates_preserve_visible_and_dismissed_occurrences kind=ui owner=chat-live
#[test]
fn backend_banner_sparse_updates_preserve_visible_and_dismissed_occurrences() {
    for dismiss in [false, true] {
        let mut s = banner_chat(
            70,
            "test-model-a",
            &banner_response(Some("dismissible"), serde_json::json!([]), |_| {}),
        );
        if dismiss {
            s.keys("esc");
        }
        let before = s.shot();
        // two rolling snapshots (60%, then an exhausted credits snapshot) must not change the popup
        for (used, reached) in [(60, None), (100, Some("workspace_member_credits_depleted"))] {
            let mut snapshot = serde_json::json!({"primary": {"usedPercent": used, "windowDurationMins": 300, "resetsAt": null}});
            if let Some(reached) = reached {
                snapshot["rateLimitReachedType"] = serde_json::json!(reached);
            }
            s.feed(&format!(
                "{}\n",
                serde_json::json!({"step": "notify", "message": {"method": "account/rateLimits/updated", "params": {"rateLimits": snapshot}}})
            ));
        }
        s.assert_same_rows(&before);
    }
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:396 backend_banner_changed_remedy_keeps_fallback_until_applicable_replacement kind=ui owner=chat-live
#[test]
fn backend_banner_changed_remedy_keeps_fallback_until_applicable_replacement() {
    for rolling_update in [false, true] {
        let mut read = banner_response(
            Some("inline"),
            serde_json::json!([{"action":"notify_owner","label":"Notify owner"}]),
            |_| {},
        );
        read["rateLimits"] = exhausted_workspace(CREDITS_DEPLETED);
        let mut s = banner_chat(100, "test-model-a", &read);
        s.assert_contains("Notify owner");

        // Keep the full account read in flight until the backend can offer the new remedy.
        let reads = s.requests_of("account/rateLimits/read");
        let pending = s.hold_request("account/rateLimits/read");
        if rolling_update {
            s.feed(&format!(
                "{}\n",
                serde_json::json!({"step": "notify", "message": {
                    "method": "account/rateLimits/updated",
                    "params": {"rateLimits": exhausted_workspace(USAGE_CAP_REACHED)},
                }})
            ));
            s.assert_absent("Notify owner");
        }
        s.feed(&rate_limit_error(
            "turn-1",
            serde_json::json!("usageLimitExceeded"),
        ));
        s.assert_absent("Notify owner");
        s.assert_contains(REQUEST_INCREASE);
        s.assert_requests_of("account/rateLimits/read", reads + 1);

        read["rateLimits"] = exhausted_workspace(USAGE_CAP_REACHED);
        read["rateLimitUpsell"]["title"] = serde_json::json!("Workspace spending cap reached");
        read["rateLimitUpsell"]["ctas"] =
            serde_json::json!([{"action":"request_increase","label":"Request more usage"}]);
        read["rateLimitUpsell"]["model_slug"] = serde_json::json!("test-model-b");
        s.feed(&answer(&read));
        s.release(&pending);
        s.assert_contains(REQUEST_INCREASE);
        s.assert_absent("Workspace spending cap reached");

        // A real command approval can cover the fallback when the replacement arrives.
        const APPROVAL: &str = "Would you like to run the following command?";
        if rolling_update {
            s.feed(r#"{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":7,"params":{"kind":"command","itemId":"call-cover","approvalId":"call-cover","environmentId":null,"command":"echo hi","cwd":"/tmp","threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#);
            s.wait_for(APPROVAL);
        }
        read["rateLimitUpsell"]["model_slug"] = serde_json::json!("test-model-a");
        s.feed(&(answer(&read) + &model_step("test-model-a", true)));
        if rolling_update {
            s.assert_contains(APPROVAL);
            s.keys("esc");
            s.assert_absent(APPROVAL);
        }
        s.assert_contains("Workspace spending cap reached");
        s.assert_contains("Request more usage");
        s.assert_absent(REQUEST_INCREASE);
        s.assert_no_request(OWNER_NUDGE);

        s.feed(&model_step("test-model-b", false));
        s.feed(&rate_limit_error(
            "turn-2",
            serde_json::json!("usageLimitExceeded"),
        ));
        s.assert_contains(REQUEST_INCREASE);
        s.feed(
            r#"{"step":"respond","method":"account/sendAddCreditsNudgeEmail","result":{"status":"sent"}}"#,
        );
        s.keys("y");
        s.assert_requests_of(OWNER_NUDGE, 1);
        s.assert_last_request_has(OWNER_NUDGE, r#""creditType":"usage_limit""#);
        s.wait_for("Limit increase requested.");
    }
}

// ---- oracle: chatwidget/tests/compaction_tests.rs (4 screen-level tests here; 2 state tests left to chat-live)
// ORACLE chatwidget/tests/compaction_tests.rs:31 compaction_status_survives_follow_up_and_preserves_turn_time kind=snap owner=chat-live snap=compaction_completed snap=compaction_running snap=compaction_running_narrow
#[test]
fn compaction_status_survives_follow_up_and_preserves_turn_time() {
    // The oracle's snapshots are the bottom pane with the elapsed time masked and its own status line
    // under the composer (`GPT-5.6-Sol default · /tmp/project`); the rows of the compaction are compared
    // here with the time of a compaction started 83s ago, on a turn that runs for 600s.
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/chat_live_2/compaction_since.jsonl"))
        .open();
    s.assert_screen_row_then(
        "• Compacting context (1m 23s • esc to interrupt)",
        &["  └ Making room to continue."],
    );
    s.resize(40, 24);
    s.assert_has_row("• Compacting context (1m 23s • esc to i…");
    s.resize(80, 24);
    s.type_text("keep going").keys("enter");
    s.assert_contains("Compacting context");
    s.feed(include_str!(
        "fixtures/chat_live_2/compaction_completed.jsonl"
    ));
    s.assert_has_row("• Context compacted · 1m 23s");
    s.assert_contains("Working (10m 00s • esc to interrupt)");
    s.assert_absent("Compacting context");
}

// ORACLE chatwidget/tests/compaction_tests.rs:106 compaction_status_clears_when_turn_ends_without_item_completion kind=ui owner=chat-live
#[test]
fn compaction_status_clears_when_turn_ends_without_item_completion() {
    for script in [
        include_str!("fixtures/chat_live_2/turn_end_completed.jsonl"),
        include_str!("fixtures/chat_live_2/turn_end_interrupted.jsonl"),
        include_str!("fixtures/chat_live_2/turn_end_failed.jsonl"),
    ] {
        let s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .script(script)
            .open();
        s.assert_absent("Compacting context");
        s.assert_absent("Context compacted");
        s.assert_contains("Working");
    }
}

// ORACLE chatwidget/tests/compaction_tests.rs:138 compaction_history_does_not_start_a_timer_or_finish_live_compaction kind=ui owner=chat-live
#[test]
fn compaction_history_does_not_start_a_timer_or_finish_live_compaction() {
    let history = kit::replay_turn(
        "old",
        serde_json::json!([
            {"type":"contextCompaction", "id":"old-compaction"}
        ]),
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .replay(Vec::new())
        .open();
    s.history_page(vec![history], Some("older"));
    s.assert_text_count("Context compacted", 1);
    s.assert_absent("Compacting context");

    let live = include_str!("fixtures/chat_live_2/compaction_since.jsonl")
        .replace("thread-1", kit::REPLAYED);
    let live = live
        .lines()
        .filter(|line| !line.contains("\"step\":\"seed\""))
        .collect::<Vec<_>>()
        .join("\n");
    s.feed(&live);
    s.assert_contains("Compacting context (1m 23s");
    s.assert_text_count("Context compacted", 1);
    s.wheel_up(0, 0, 60);
    s.history_page(
        vec![kit::replay_turn(
            "older",
            serde_json::json!([{"type":"contextCompaction", "id":"older-compaction"}]),
        )],
        None,
    );
    s.keys("ctrl+end");
    s.assert_contains("Compacting context (1m 23s");
    s.assert_text_count("Context compacted", 2);
    let complete = include_str!("fixtures/chat_live_2/compaction_completed.jsonl")
        .replace("thread-1", kit::REPLAYED);
    s.feed(&complete);
    s.assert_absent("Compacting context");
    s.assert_text_count("Context compacted", 3);
}

// ORACLE chatwidget/tests/compaction_tests.rs:162 compaction_snapshot_restores_elapsed_time_and_clears_on_replayed_completion kind=ui owner=chat-live
#[test]
fn compaction_snapshot_restores_elapsed_time_and_clears_on_replayed_completion() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_live_2/compaction_snapshot.jsonl"
        ))
        .open();
    // oracle: the header says Compacting context and the elapsed time counts from the server's
    // start, 12 seconds before.
    s.assert_contains("Compacting context (12s");
    s.feed(include_str!(
        "fixtures/chat_live_2/compaction_completed.jsonl"
    ));
    // oracle: the completion clears the compaction, the header is Working again and the history
    // holds the Context compacted cell.
    s.assert_absent("Compacting context");
    s.assert_contains("Working");
    s.assert_text_count("Context compacted", 1);
}

// ---- oracle: chatwidget/tests/completion_styling_tests.rs (2 screen-level tests here; 4 state tests left to chat-live)
// ORACLE chatwidget/tests/completion_styling_tests.rs:72 completion_follows_plain_and_streamed_tool_answers kind=snap owner=chat-live snap=completion_after_plain_answer snap=completion_after_streamed_tool_answer
#[test]
fn completion_follows_plain_and_streamed_tool_answers() {
    // The history rows of the turn: its answer (and the command before it) and the separator.
    let plain = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/chat_live_2/completion_plain.jsonl"))
        .open();
    plain.assert_snap_region(
        "completion_after_plain_answer",
        row_of(&plain, "• The change is ready."),
        0,
    );
    let streamed = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_live_2/completion_streamed.jsonl"
        ))
        .open();
    streamed.assert_snap_region(
        "completion_after_streamed_tool_answer",
        row_of(&streamed, "• Ran echo ready"),
        0,
    );
}

// ORACLE chatwidget/tests/completion_styling_tests.rs:255 completion_snapshot_normalization_preserves_clock_only_message_lines kind=snap owner=chat-live snap=completion_like_transcript_text
#[test]
fn completion_snapshot_normalization_preserves_clock_only_message_lines() {
    kit::na(
        Owner::ChatLive,
        "asserts the snapshot tool's timestamp normalization over a cell the test builds itself (a PlainHistoryCell and a FinalMessageSeparator), not a turn the screen draws: a unit test of the helper",
    );
}

// ---- oracle: chatwidget/tests/config_errors_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/config_errors_tests.rs:4 chained_config_error_wraps_in_history_snapshot kind=snap owner=chat-live snap=chained_config_error_wraps_in_history_snapshot
#[test]
fn chained_config_error_wraps_in_history_snapshot() {
    // The oracle adds the error cell itself; here the server refuses the write of the default
    // model with the chained message and the app words it as the save failure.
    let mut s = Screen::build(Owner::ChatLive).size(56, 24).live().open();
    let catalog = include_str!("fixtures/chat_commands_2/speed_catalog.json").trim();
    s.feed(&format!(
        r#"{{"step":"respond","method":"model/list","result":{catalog}}}
{{"step":"screen","dir":"chat","ev":{{"status":{{"model":"gpt-5.2"}}}}}}"#
    ));
    s.server_answers("thread/settings/update", serde_json::json!({}));
    s.server_refuses(
        "config/batchWrite",
        None,
        "config/batchWrite failed in TUI: Invalid configuration: features.fast_mode=true is not supported; allowed set [fast_mode=false]",
    );
    s.type_text("/model").keys("enter");
    s.keys("enter").keys("enter");
    // oracle: the error wraps at 56 columns under its marker.
    s.assert_snap_region(
        "chained_config_error_wraps_in_history_snapshot",
        row_of(&s, "■ Failed to save default model"),
        0,
    );
}

// ---- oracle: chatwidget/tests/goal_validation.rs (2 screen-level tests here; 4 state tests left to chat-live)
// ORACLE chatwidget/tests/goal_validation.rs:117 goal_slash_command_preserves_large_pasted_objective kind=ui owner=chat-live
#[test]
fn goal_slash_command_preserves_large_pasted_objective() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/goal ");
    // The oracle pastes MAX_THREAD_GOAL_OBJECTIVE_CHARS + 1 (4000 + 1) `x` cells.
    s.paste(&"x".repeat(4_001));
    s.assert_contains("[Pasted Content");
    s.keys("enter");
    // The oracle then holds the draft's pending paste to the pasted text and sends no submit.
}

// ORACLE chatwidget/tests/goal_validation.rs:159 goal_slash_command_emits_only_inserted_paste_text_element kind=ui owner=chat-live
#[test]
fn goal_slash_command_emits_only_inserted_paste_text_element() {
    let mut s = Screen::build(Owner::ChatLive).size(120, 24).live().open();
    s.type_text("/goal keep literal [Pasted Content 1001 chars] and ");
    s.paste(&"x".repeat(1_001));
    s.assert_contains("keep literal [Pasted Content 1001 chars] and [Pasted Content 1001 chars]");
    s.keys("enter");
    // The oracle then checks the draft holds one pending paste (the inserted one) and no submit.
}
// ---- oracle: chatwidget/tests/history_projection.rs (6 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/history_projection.rs:12 older_tool_projection_matches_initial_replay kind=snap owner=chat-live
#[test]
fn older_tool_projection_matches_initial_replay() {
    let command = |id: &str, command: &str, action: serde_json::Value, output: &str, ms: u32| {
        serde_json::json!({
            "type": "commandExecution", "id": id, "command": command, "cwd": "/work",
            "processId": null, "source": "agent", "status": "completed",
            "commandActions": [action], "aggregatedOutput": output, "exitCode": 0, "durationMs": ms
        })
    };
    let read = |id: &str, name: &str| {
        command(
            id,
            &format!("cat {name}"),
            serde_json::json!({"type": "read", "command": format!("cat {name}"), "name": name, "path": format!("/work/{name}")}),
            "fn main() {}",
            5,
        )
    };
    let items = serde_json::json!([
        command(
            "command",
            "cargo check",
            serde_json::json!({"type": "unknown", "command": "cargo check"}),
            &(1..=12)
                .map(|line| format!("Checking crate {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
            25
        ),
        read("one.rs", "one.rs"),
        read("two.rs", "two.rs"),
    ]);
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 60)
        .replay(vec![bare_turn("turn", items)])
        .live()
        .open();
    // oracle: `completion_only_replay_exploration_group`, the compact half (the group of the two reads), then the detailed
    s.snap("completion_only_replay_exploration_group")
        .frame("compact:")
        .at(row_of(&s, "• Explored"), 0)
        .assert();
    s.keys("ctrl+t");
    s.snap("completion_only_replay_exploration_group")
        .frame("detailed:")
        .at(row_of(&s, "$ cat one.rs"), 0)
        .assert();
}

// ORACLE chatwidget/tests/history_projection.rs:180 snapshot_formatter_reasoning_matches_compact_and_detailed_replay kind=snap owner=chat-live
#[test]
fn snapshot_formatter_reasoning_matches_compact_and_detailed_replay() {
    let mut s = resumed_raw(vec![kit::replay_turn(
        "turn",
        serde_json::json!([{
            "type": "reasoning", "id": "reasoning",
            "summary": ["**Plan**\n\nRead [src/main.rs](src/main.rs:3).", "**Checking tests**\n\n<!-- -->"],
            "content": ["Raw detail."]
        }]),
    )]);
    s.keys("ctrl+t");
    s.assert_contains("• Read src/main.rs (src/main.rs:3).");
    s.assert_contains("Raw detail.");
}

// ORACLE chatwidget/tests/history_projection.rs:250 raw_reasoning_keeps_its_own_heading kind=snap owner=chat-live
#[test]
fn raw_reasoning_keeps_its_own_heading() {
    let mut s = resumed_raw(vec![kit::replay_turn(
        "turn-1",
        serde_json::json!([{
            "type": "reasoning", "id": "raw", "summary": [],
            "content": ["**Raw investigation**\nKeep this heading and its details."]
        }]),
    )]);
    s.keys("ctrl+t");
    let at = row_of(&s, "• Raw investigation");
    s.assert_row(at, "• Raw investigation");
    s.assert_row(at + 1, "  Keep this heading and its details.");
}

// ORACLE chatwidget/tests/history_projection.rs:269 snapshot_formatter_completed_patch_needs_no_started_notification kind=snap owner=chat-live
#[test]
fn snapshot_formatter_completed_patch_needs_no_started_notification() {
    let s = resumed(vec![kit::replay_turn(
        "turn",
        serde_json::json!([patch_item("completed")]),
    )]);
    s.assert_snap_region(
        "snapshot_formatter_completed_patch",
        row_of(&s, "• Edited sample.txt"),
        0,
    );
}

// ORACLE chatwidget/tests/history_projection.rs:307 snapshot_formatter_live_and_buffered_patch_notifications_render_once kind=snap owner=chat-live
#[test]
fn snapshot_formatter_live_and_buffered_patch_notifications_render_once() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(SEED)
        .live()
        .open();
    s.feed(&turn_started("thread-1", "turn-1"));
    s.feed(&notify(
        "item/started",
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "startedAtMs": 0, "item": patch_item("inProgress")}),
    ));
    s.feed(&item_completed("thread-1", patch_item("completed")));
    s.assert_snap_region(
        "snapshot_formatter_completed_patch",
        row_of(&s, "• Edited sample.txt"),
        0,
    );
    if times(&s, "• Edited sample.txt") != 1 {
        panic!(
            "the patch is drawn {} times:\n{}",
            times(&s, "• Edited sample.txt"),
            s.text()
        );
    }
}

// ORACLE chatwidget/tests/history_projection.rs:344 snapshot_formatter_empty_success_and_failed_patch_keep_their_outcomes kind=snap owner=chat-live snap=empty
#[test]
fn snapshot_formatter_empty_success_and_failed_patch_keep_their_outcomes() {
    let empty = serde_json::json!({"type": "fileChange", "id": "empty", "changes": [], "status": "completed"});
    let turns = vec![kit::replay_turn(
        "turn",
        serde_json::json!([empty, patch_item("failed")]),
    )];
    let assert_failure = |s: &Screen| {
        s.assert_has_row("✘ Failed to apply patch");
        s.assert_text_count("Failed to apply patch", 1);
        for phantom in ["Edited", "old sample", "new sample"] {
            s.assert_absent(phantom);
        }
    };

    let inline = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .replay(turns.clone())
        .live()
        .inline(&[])
        .open();
    assert_failure(&inline);
    inline.assert_absent("error details unavailable");

    let mut owned = resumed(turns);
    for detailed in [false, true] {
        if detailed {
            owned.keys("ctrl+t");
        }
        assert_failure(&owned);
        let at = row_of(&owned, "✘ Failed to apply patch");
        owned.assert_row(at + 1, "  └ (error details unavailable)");
    }
}

#[test]
fn repeated_completed_turn_keeps_one_recorded_patch_failure() {
    let turn = |patch: serde_json::Value| {
        kit::replay_turn("failed-patch-turn", serde_json::json!([patch]))
    };
    let failed = turn(patch_item("failed"));
    let mut s = resumed(vec![failed.clone()]);
    let completion = |turn| {
        notify(
            "turn/completed",
            serde_json::json!({"threadId": kit::REPLAYED, "turn": turn}),
        )
    };

    for _ in 0..2 {
        s.assert_text_count("Failed to apply patch", 1);
        for phantom in ["Edited", "old sample", "new sample"] {
            s.assert_absent(phantom);
        }
        s.feed(&completion(failed.clone()));
    }
    s.assert_text_count("Failed to apply patch", 1);
    s.assert_absent("Edited");

    s.feed(&completion(turn(patch_item("completed"))));
    s.assert_text_count("Edited sample.txt", 1);
    s.assert_contains("new sample");
    s.assert_absent("Failed to apply patch");

    let mut corrected = patch_item("completed");
    corrected["changes"][0]["diff"] =
        serde_json::json!("@@ -1 +1 @@\n-old sample\n+authoritative replacement\n");
    s.feed(&completion(turn(corrected)));
    s.assert_text_count("Edited sample.txt", 1);
    s.assert_contains("authoritative replacement");
    s.assert_absent("new sample");
}

#[test]
fn a_fresh_failed_patch_keeps_its_live_preview_and_outcome() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(SEED)
        .live()
        .open();
    s.feed(&turn_started("thread-1", "turn-1"));
    s.feed(&notify(
        "item/started",
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1",
            "startedAtMs": 0, "item": patch_item("inProgress")}),
    ));
    s.assert_text_count("Edited sample.txt", 1);
    s.assert_contains("new sample");
    s.assert_absent("Failed to apply patch");
    s.feed(&item_completed("thread-1", patch_item("failed")));
    s.assert_text_count("Edited sample.txt", 1);
    s.assert_text_count("Failed to apply patch", 1);
}

// ---- oracle: chatwidget/tests/history_replay.rs (24 screen-level tests here; 11 state tests left to chat-live)

/// A chat of a resumed thread whose recorded turns are `turns` (`replay_turn`s), oldest first as
/// the oracle's tests give them; the server answers newest first.
fn resumed(mut turns: Vec<serde_json::Value>) -> Screen {
    turns.reverse();
    Screen::build(Owner::ChatLive)
        .size(80, 24)
        .replay(turns)
        .live()
        .open()
}

/// `resumed` with the config's `show_raw_agent_reasoning` on.
fn resumed_raw(mut turns: Vec<serde_json::Value>) -> Screen {
    turns.reverse();
    Screen::build(Owner::ChatLive)
        .size(80, 24)
        .replay(turns)
        .raw_reasoning()
        .live()
        .open()
}

/// The screen row of the first row that starts with `text`.
fn row_of(s: &Screen, text: &str) -> usize {
    let Some(at) = s
        .rows()
        .iter()
        .position(|row| row.trim_start().starts_with(text))
    else {
        panic!("no row starts with {text:?} on the screen:\n{}", s.text());
    };
    at
}

/// A turn as the oracle's `app_server_turn` makes one: no start, end or duration (so no
/// `Worked for` separator).
fn bare_turn(id: &str, items: serde_json::Value) -> serde_json::Value {
    let mut turn = kit::replay_turn(id, items);
    turn["startedAt"] = serde_json::Value::Null;
    turn["completedAt"] = serde_json::Value::Null;
    turn["durationMs"] = serde_json::Value::Null;
    turn
}

/// `bare_turn` ended as `status` (`failed`, `interrupted`).
fn turn_ended(id: &str, status: &str, items: serde_json::Value) -> serde_json::Value {
    let mut turn = bare_turn(id, items);
    turn["status"] = serde_json::json!(status);
    turn
}

/// A `notify` step: a server notification of `method` with `params`.
fn notify(method: &str, params: serde_json::Value) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "notify", "message": {"method": method, "params": params}})
    )
}

/// `turn/started` of `turn` in `thread`.
fn turn_started(thread: &str, turn: &str) -> String {
    notify(
        "turn/started",
        serde_json::json!({"threadId": thread, "turn": {
            "id": turn, "rootTurnId": null, "items": [], "itemsView": "full",
            "status": "inProgress", "error": null, "startedAt": 0, "completedAt": null, "durationMs": null
        }}),
    )
}

/// `item/completed` of `item` in `turn-1` of `thread`.
fn item_completed(thread: &str, item: serde_json::Value) -> String {
    notify(
        "item/completed",
        serde_json::json!({"threadId": thread, "turnId": "turn-1", "item": item,
            "startedAtMs": 0, "completedAtMs": 0}),
    )
}

/// An `mcpToolCall` item of `server`'s `tool`.
fn mcp_call(
    id: &str,
    server: &str,
    tool: &str,
    status: &str,
    arguments: serde_json::Value,
    result: serde_json::Value,
    duration: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "type": "mcpToolCall", "id": id, "server": server, "tool": tool, "status": status,
        "arguments": arguments, "result": result, "error": null, "durationMs": duration
    })
}

/// The oracle's `patch_item`: `sample.txt` updated, one line changed.
fn patch_item(status: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "fileChange", "id": "patch", "status": status,
        "changes": [{"path": "sample.txt", "kind": {"type": "update", "move_path": null},
            "diff": "@@ -1 +1 @@\n-old sample\n+new sample\n"}]
    })
}

/// How often the screen shows `text`.
fn times(s: &Screen, text: &str) -> usize {
    s.text().matches(text).count()
}

const SEED: &str = "{\"step\":\"seed\",\"seed\":\"session\",\"thread\":\"thread-1\"}\n";

/// A recorded user message `text` with the extra inputs `more`.
fn user_message(id: &str, text: &str, more: serde_json::Value) -> serde_json::Value {
    let mut content = vec![serde_json::json!({"type": "text", "text": text, "text_elements": []})];
    content.extend(more.as_array().cloned().unwrap_or_default());
    serde_json::json!({"type": "userMessage", "id": id, "content": content})
}
// ORACLE chatwidget/tests/history_replay.rs:18 resumed_initial_messages_render_history kind=snap owner=chat-live
#[test]
fn resumed_initial_messages_render_history() {
    let turn = kit::replay_turn(
        "turn-1",
        serde_json::json!([
            user_message("user-1", "hello from user", serde_json::json!([])),
            {"type": "agentMessage", "id": "assistant-1", "text": "assistant reply", "phase": null, "memoryCitation": null}
        ]),
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .replay(vec![turn])
        .replay_daybreak()
        .live()
        .open();
    s.assert_in_order(&[
        "OpenAI Codex",
        "Daybreak is on for new turns.",
        "hello from user",
        "assistant reply",
    ]);
    s.assert_text_count("Daybreak is on for new turns.", 1);
}

// ORACLE chatwidget/tests/history_replay.rs:130 replayed_failed_turns_preserve_overload_warnings_between_retries kind=snap owner=chat-live
#[test]
fn replayed_failed_turns_preserve_overload_warnings_between_retries() {
    let prompt = "The workspace also looks super confusing with its separator.";
    let failed = |id: &str, item: &str| {
        let mut turn = turn_ended(
            id,
            "failed",
            serde_json::json!([user_message(item, prompt, serde_json::json!([]))]),
        );
        turn["error"] = serde_json::json!({
            "message": "Selected model is at capacity. Please try a different model.",
            "codexErrorInfo": "serverOverloaded", "additionalDetails": null
        });
        turn["completedAt"] = serde_json::Value::Null;
        turn["durationMs"] = serde_json::Value::Null;
        turn
    };
    let s = resumed(vec![failed("turn-1", "user-1"), failed("turn-2", "user-2")]);
    s.assert_contains("Selected model is at capacity. Please try a different model.");
    // oracle: the history rows alone, a blank row before the first.
    s.assert_snap_region(
        "replayed_failed_turns_preserve_overload_warnings_between_retries",
        row_of(&s, "› The workspace") - 1,
        0,
    );
}

// ORACLE chatwidget/tests/history_replay.rs:181 restored_conversation_ultra_remains_selected_after_switching_to_plan kind=ui owner=chat-live
#[test]
fn restored_conversation_ultra_remains_selected_after_switching_to_plan() {
    kit::na(
        Owner::ChatLive,
        "asserts the active collaboration mode kind and the current reasoning effort after Shift+Tab, neither drawn as text of its own",
    );
}

// ORACLE chatwidget/tests/history_replay.rs:219 replayed_user_messages_seed_composer_history kind=ui owner=chat-live
#[test]
fn replayed_user_messages_seed_composer_history() {
    let mut s = resumed(vec![
        kit::replay_turn(
            "turn-1",
            serde_json::json!([user_message(
                "user-1",
                "use $sample",
                serde_json::json!([{"type": "mention", "name": "Sample Plugin", "path": "plugin://sample@test"}])
            )]),
        ),
        kit::replay_turn(
            "turn-2",
            serde_json::json!([user_message(
                "user-2",
                "use $google-calendar",
                serde_json::json!([{"type": "mention", "name": "Google Calendar", "path": "app://google_calendar"}])
            )]),
        ),
    ]);
    s.keys("up");
    s.assert_composer("use $google-calendar");
    s.keys("up");
    s.assert_composer("use $sample");
}

// ORACLE chatwidget/tests/history_replay.rs:309 replayed_review_prompt_does_not_seed_composer_history kind=ui owner=chat-live
#[test]
fn replayed_review_prompt_does_not_seed_composer_history() {
    let mut s = resumed(vec![kit::replay_turn(
        "turn-1",
        serde_json::json!([
            {"type": "enteredReviewMode", "id": "review-start", "review": "changes against main"},
            user_message("review-prompt", "Review the code changes against the base branch 'main'.", serde_json::json!([]))
        ]),
    )]);
    s.keys("up");
    s.assert_composer("");
}

// ORACLE chatwidget/tests/history_replay.rs:333 replayed_delegated_tool_output_is_attributed_without_seeding_composer_history kind=snap owner=chat-live snap=replayed_delegated_tool_output
#[test]
fn replayed_delegated_tool_output_is_attributed_without_seeding_composer_history() {
    let item = serde_json::json!({
        "type": "functionCallOutput", "id": "delegation-1", "name": "send_message_to_thread",
        "namespace": "codex_tui",
        "output": concat!(
            "<codex_delegation>\n  <source_thread_id>source-task</source_thread_id>\n  <input>",
            "**Follow up** on [the PR](https://example.com/pr).\n\n",
            "- Review `&lt;up&gt; &amp; report`.\n",
            "- Share the result.",
            "</input>\n</codex_delegation>",
        )
    });
    let mut s = resumed(vec![kit::replay_turn("turn-1", serde_json::json!([item]))]);
    s.assert_snap_region(
        "replayed_delegated_tool_output",
        row_of(&s, "• Sent by Codex"),
        0,
    );
    s.assert_link("the PR", "https://example.com/pr");
    s.keys("up");
    s.assert_composer("");
}

// ORACLE chatwidget/tests/history_replay.rs:410 replayed_nested_review_prompts_do_not_render_or_seed_composer_history kind=snap owner=chat-live
#[test]
fn replayed_nested_review_prompts_do_not_render_or_seed_composer_history() {
    let hint = "current changes";
    let prompt = "Review the current code changes (staged, unstaged, and untracked files).";
    let marker = |id: &str| {
        bare_turn(
            id,
            serde_json::json!([
                {"type": "enteredReviewMode", "id": format!("{id}-start"), "review": hint},
                {"type": "exitedReviewMode", "id": format!("{id}-end"), "review": "review complete"}
            ]),
        )
    };
    let mut interrupted = turn_ended(
        "turn-interrupted",
        "interrupted",
        serde_json::json!([
            user_message("interrupted-prompt", hint, serde_json::json!([])),
            user_message("interrupted-steer", hint, serde_json::json!([]))
        ]),
    );
    interrupted["completedAt"] = serde_json::json!(1);
    let mut child = turn_ended(
        "turn-review-child",
        "interrupted",
        serde_json::json!([
            user_message("review-prompt-1", prompt, serde_json::json!([])),
            user_message("review-prompt-2", prompt, serde_json::json!([])),
            {"type": "agentMessage", "id": "review-result", "text": "review result is retained",
             "phase": "final_answer", "memoryCitation": null}
        ]),
    );
    child["completedAt"] = serde_json::Value::Null;
    // taller than `resumed`: the oracle's frame is the history alone, 19 rows
    let mut turns = vec![
        marker("turn-review-before-steer"),
        interrupted,
        marker("turn-review"),
        child,
    ];
    turns.reverse();
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 30)
        .replay(turns)
        .live()
        .open();
    s.assert_absent(prompt);
    s.assert_snap_region(
        "replayed_nested_review_prompts",
        row_of(&s, ">> Code review started"),
        0,
    );
    s.keys("up");
    s.assert_composer(hint);
}

// ORACLE chatwidget/tests/history_replay.rs:853 replayed_user_message_with_only_remote_images_renders_history_cell kind=ui owner=chat-live
#[test]
fn replayed_user_message_with_only_remote_images_renders_history_cell() {
    kit::na(
        Owner::ChatLive,
        "asserts the stored message of the replayed user cell (empty) and its remote image urls, not text drawn",
    );
}

// ORACLE chatwidget/tests/history_replay.rs:914 replayed_user_message_with_only_local_images_renders_history_cell kind=ui owner=chat-live
#[test]
fn replayed_user_message_with_only_local_images_renders_history_cell() {
    kit::na(
        Owner::ChatLive,
        "asserts the stored message of the replayed user cell (empty) and its local image paths, not text drawn",
    );
}

/// The session's `thread/start` answer for a conversation `id` that the server named `name`: the
/// thread `/fork` copies from (`emit_forked_thread_event` reads its title).
fn parent_thread(id: &str, name: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "thread": {
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
            "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
            "status": {"type": "idle"}, "path": null, "cwd": "/work", "cliVersion": "0.0.0",
            "originator": null, "source": "cli", "canAcceptDirectInput": true,
            "threadSource": null, "agentNickname": null, "agentRole": null, "gitInfo": null,
            "name": name, "daybreakEnabled": null, "turns": []
        },
        "model": "gpt-5", "modelProvider": "openai", "serviceTier": null, "cwd": "/work",
        "instructionSources": [], "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

// ORACLE chatwidget/tests/history_replay.rs:973 forked_thread_history_line_includes_name_and_id_snapshot kind=snap owner=chat-live snap=forked_thread_history_line
#[test]
fn forked_thread_history_line_includes_name_and_id_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .thread_start(parent_thread(
            "e9f18a88-8081-4e51-9d4e-8af5cde2d8dd",
            Some("named-thread"),
        ))
        .script(include_str!("fixtures/chat_live_2/fork_named.jsonl"))
        .open();
    s.type_text("/fork");
    s.keys("enter");
    s.assert_snap_region(
        "forked_thread_history_line",
        row_of(&s, "• Thread forked from"),
        0,
    );
}

// ORACLE chatwidget/tests/history_replay.rs:1003 forked_thread_history_line_without_name_shows_id_once_snapshot kind=snap owner=chat-live snap=forked_thread_history_line_without_name
#[test]
fn forked_thread_history_line_without_name_shows_id_once_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .thread_start(parent_thread("019c2d47-4935-7423-a190-05691f566092", None))
        .script(include_str!("fixtures/chat_live_2/fork_unnamed.jsonl"))
        .open();
    s.type_text("/fork");
    s.keys("enter");
    s.assert_snap_region(
        "forked_thread_history_line_without_name",
        row_of(&s, "• Thread forked from"),
        0,
    );
}

// ORACLE chatwidget/tests/history_replay.rs:1091 prompt_edit_thread_history_line_snapshot kind=snap owner=chat-live snap=prompt_edit_thread_history_line
#[test]
fn prompt_edit_thread_history_line_snapshot() {
    // The oracle calls `emit_prompt_edit_thread_event` and snapshots the cell it sends; the edit
    // is run whole here: Esc Esc, Enter, the server's revert and its `thread/reverted`.
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_live_2/prompt_edit_revert.jsonl"
        ))
        .open();
    s.keys("esc");
    s.keys("esc");
    s.keys("enter");
    // the server reverted: its notification comes after the answer of `thread/revert`
    s.feed(
        r#"{"step":"notify","message":{"method":"thread/reverted","params":{"threadId":"thread-1"}}}"#,
    );
    s.assert_snap_region(
        "prompt_edit_thread_history_line",
        row_of(&s, "• Conversation reverted"),
        0,
    );
}

// ORACLE chatwidget/tests/history_replay.rs:1113 app_server_forked_thread_history_line_uses_app_server_title_snapshot kind=snap owner=chat-live snap=app_server_forked_thread_history_line
#[test]
fn app_server_forked_thread_history_line_uses_app_server_title_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        // wide enough for the oracle's one-line snapshot (the line is 84 columns)
        .size(100, 24)
        .thread_start(parent_thread(
            "e9f18a88-8081-4e51-9d4e-8af5cde2d8dd",
            Some("app-server-parent-thread"),
        ))
        .script(include_str!(
            "fixtures/chat_live_2/fork_server_titled.jsonl"
        ))
        .open();
    s.type_text("/fork");
    s.keys("enter");
    s.assert_snap_region(
        "app_server_forked_thread_history_line",
        row_of(&s, "• Thread forked from"),
        0,
    );
    s.assert_absent("stale-local-thread");
}

// ORACLE chatwidget/tests/history_replay.rs:1153 app_server_forked_thread_history_line_without_app_server_name_ignores_local_snapshot kind=snap owner=chat-live snap=app_server_forked_thread_history_line_without_app_server_name
#[test]
fn app_server_forked_thread_history_line_without_app_server_name_ignores_local_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .thread_start(parent_thread("019c2d47-4935-7423-a190-05691f566092", None))
        .script(include_str!(
            "fixtures/chat_live_2/fork_server_unnamed.jsonl"
        ))
        .open();
    s.type_text("/fork");
    s.keys("enter");
    s.assert_absent("stale-local-thread");
    s.assert_snap_region(
        "app_server_forked_thread_history_line_without_app_server_name",
        row_of(&s, "• Thread forked from"),
        0,
    );
}

// ORACLE chatwidget/tests/history_replay.rs:1195 thread_snapshot_replay_preserves_agent_message_during_review_mode kind=ui owner=chat-live
#[test]
fn thread_snapshot_replay_preserves_agent_message_during_review_mode() {
    let s = resumed(vec![kit::replay_turn(
        "turn-1",
        serde_json::json!([
            {"type": "enteredReviewMode", "id": "review-start", "review": "current changes"},
            {"type": "agentMessage", "id": "review-message", "text": "Review progress update", "phase": null, "memoryCitation": null}
        ]),
    )]);
    s.assert_contains("Review progress update");
}

// ORACLE chatwidget/tests/history_replay.rs:1276 replayed_reasoning_item_preserves_summary_parts_and_hides_raw_reasoning_when_disabled kind=ui owner=chat-live
#[test]
fn replayed_reasoning_item_preserves_summary_parts_and_hides_raw_reasoning_when_disabled() {
    let mut s = resumed(vec![kit::replay_turn(
        "turn-1",
        serde_json::json!([{
            "type": "reasoning", "id": "reasoning-1",
            "summary": ["**Plan**\n\ndone", "**Checking tests**\n\n<!-- -->"],
            "content": ["Raw reasoning"]
        }]),
    )]);
    s.keys("ctrl+t");
    s.assert_contains("• done");
    s.assert_absent("Raw reasoning");
}

// ORACLE chatwidget/tests/history_replay.rs:1328 replayed_reasoning_item_shows_raw_reasoning_when_enabled kind=ui owner=chat-live
#[test]
fn replayed_reasoning_item_shows_raw_reasoning_when_enabled() {
    let mut s = resumed_raw(vec![kit::replay_turn(
        "turn-1",
        serde_json::json!([{
            "type": "reasoning", "id": "reasoning-1",
            "summary": ["Summary only"], "content": ["Raw reasoning"]
        }]),
    )]);
    s.keys("ctrl+t");
    s.assert_contains("Raw reasoning");
}

// ORACLE chatwidget/tests/history_replay.rs:1376 replayed_mcp_tool_call_stays_active_until_completion kind=ui owner=chat-live
#[test]
fn replayed_mcp_tool_call_stays_active_until_completion() {
    let call = |status: &str| {
        mcp_call(
            "mcp-1",
            "copilot-bridge",
            "copilot",
            status,
            serde_json::json!({"action": "wait"}),
            serde_json::Value::Null,
            serde_json::Value::Null,
        )
    };
    let mut s = resumed(vec![turn_ended(
        "turn-1",
        "inProgress",
        serde_json::json!([call("inProgress")]),
    )]);
    s.assert_contains("Calling");
    s.assert_absent("MCP tool call completed without a result");
    s.feed(&item_completed(kit::REPLAYED, call("completed")));
    s.assert_contains("MCP tool call completed without a result");
}

const FAILED_REPL_OUTPUT: &str = r#"{"exit_code":0,"output":"ready","chunk_id":"chunk-1"}"#;

fn failed_repl(server: &str) -> serde_json::Value {
    mcp_call(
        "mcp-failed",
        server,
        "js",
        "failed",
        serde_json::json!({"title": "Inspect workspace"}),
        serde_json::json!({"content": [
            {"type": "text", "text": "Script failed"},
            {"type": "text", "text": FAILED_REPL_OUTPUT},
            {"type": "text", "text": "Script error:\npermission denied"}
        ], "structuredContent": null, "_meta": null}),
        serde_json::json!(5),
    )
}

// ORACLE chatwidget/tests/history_replay.rs:1419 failed_repl_mcp_tool_call_preserves_status_and_result kind=snap owner=chat-live
#[test]
fn failed_repl_mcp_tool_call_preserves_status_and_result() {
    for server in ["node_repl", "cua_repl"] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .script(SEED)
            .live()
            .open();
        s.feed(&turn_started("thread-1", "turn-1"));
        s.feed(&item_completed("thread-1", failed_repl(server)));
        if server == "cua_repl" {
            let at = row_of(&s, "• Used computer");
            s.assert_row(at, "• Used computer · 1 action · 1 failed");
            s.assert_row(at + 1, "  └ Failed: Inspect workspace — permission denied");
            s.assert_absent(FAILED_REPL_OUTPUT);
        } else {
            let at = row_of(&s, "• Inspect workspace");
            s.assert_row(at, "• Inspect workspace");
            s.assert_row(at + 1, "  └ Script failed");
            s.assert_row(
                at + 2,
                r#"    {"exit_code": 0, "output": "ready", "chunk_id": "chunk-1"}"#,
            );
            s.assert_row(at + 3, "    Script error:");
            s.assert_row(at + 4, "    +1 line");
            s.assert_absent("permission denied");
            s.click_text("+1 line").wait_for("permission denied");
            s.click_text("− Hide details").wait_for("+1 line");
        }
        s.keys("ctrl+t");
        s.wait_for(&format!("Called {server}.js("));
        s.assert_in_order(&[
            "Inspect workspace",
            "Script failed",
            FAILED_REPL_OUTPUT,
            "Script error:",
            "permission denied",
        ]);
        s.assert_absent("+1 line");
    }
}

#[test]
fn failed_repl_inline_preview_matches_live_and_replay() {
    for server in ["node_repl", "cua_repl"] {
        let mut live = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .inline(&[])
            .script(SEED)
            .live()
            .open();
        live.feed(&turn_started("thread-1", "turn-1"));
        live.feed(&item_completed("thread-1", failed_repl(server)));
        let replay = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .inline(&[])
            .replay(vec![kit::replay_turn(
                "turn-1",
                serde_json::json!([failed_repl(server)]),
            )])
            .open();
        for shown in [&live, &replay] {
            if server == "cua_repl" {
                shown.assert_snap_region(
                    "failed_computer_activity",
                    row_of(shown, "• Used computer"),
                    0,
                );
            } else {
                let at = row_of(shown, "• Inspect workspace");
                shown.assert_row(at, "• Inspect workspace");
                shown.assert_row(at + 1, "  └ Script failed");
                shown.assert_row(
                    at + 2,
                    r#"    {"exit_code": 0, "output": "ready", "chunk_id": "chunk-1"}"#,
                );
                shown.assert_row(at + 3, "    Script error:");
                shown.assert_row(at + 4, "    +1 line (⌃t to view transcript)");
                shown.assert_absent("permission denied");
            }
        }
        if server == "node_repl" {
            live.resize(40, 24);
            live.assert_contains("+2 lines (⌃t to view transcript)");
            live.assert_absent("Script error:");
            live.resize(80, 24);
        }
        live.keys("ctrl+t");
        live.wait_for(&format!("Called {server}.js("));
        live.assert_in_order(&[FAILED_REPL_OUTPUT, "Script error:", "permission denied"]);
        live.assert_absent("+1 line (⌃t to view transcript)");
    }
}

// ORACLE chatwidget/tests/history_replay.rs:1482 deferred_mcp_lifecycle_events_keep_fifo_after_stream_finishes kind=ui owner=chat-live
#[test]
fn deferred_mcp_lifecycle_events_keep_fifo_after_stream_finishes() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(SEED)
        .live()
        .open();
    s.feed(&turn_started("thread-1", "turn-1"));
    let call = |status: &str, result: serde_json::Value, duration: serde_json::Value| {
        mcp_call(
            "mcp-deferred",
            "copilot-bridge",
            "copilot",
            status,
            serde_json::json!({"action": "wait"}),
            result,
            duration,
        )
    };
    s.feed(&notify(
        "item/started",
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "startedAtMs": 0,
            "item": call("inProgress", serde_json::Value::Null, serde_json::Value::Null)}),
    ));
    s.feed(&item_completed(
        "thread-1",
        call(
            "completed",
            serde_json::json!({"content": [{"type": "text", "text": "deferred result"}], "structuredContent": null, "_meta": null}),
            serde_json::json!(5),
        ),
    ));
    s.assert_contains("deferred result");
}

// ORACLE chatwidget/tests/history_replay.rs:1544 live_reasoning_summary_is_not_rendered_twice_when_item_completes kind=ui owner=chat-live
#[test]
fn live_reasoning_summary_is_not_rendered_twice_when_item_completes() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(SEED)
        .live()
        .open();
    s.feed(&turn_started("thread-1", "turn-1"));
    s.feed(&notify(
        "item/started",
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "startedAtMs": 0,
            "item": {"type": "reasoning", "id": "reasoning-1", "summary": [], "content": []}}),
    ));
    s.feed(&notify(
        "item/reasoning/summaryTextDelta",
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": "reasoning-1",
            "delta": "Summary only", "summaryIndex": 0}),
    ));
    s.feed(&item_completed(
        "thread-1",
        serde_json::json!({"type": "reasoning", "id": "reasoning-1", "summary": ["Summary only"], "content": []}),
    ));
    s.keys("ctrl+t");
    // the status row of the running turn says the summary too: it is not the transcript's cell
    let drawn = s
        .rows()
        .iter()
        .filter(|row| row.contains("Summary only") && !row.contains("esc to interrupt"))
        .count();
    if drawn != 1 {
        panic!("the summary is drawn {drawn} times:\n{}", s.text());
    }
}

// ORACLE chatwidget/tests/history_replay.rs:1605 live_reasoning_summary_drops_empty_parts_without_losing_content kind=ui owner=chat-live
#[test]
fn live_reasoning_summary_drops_empty_parts_without_losing_content() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(SEED)
        .live()
        .open();
    s.feed(&turn_started("thread-1", "turn-1"));
    s.feed(&notify(
        "item/started",
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "startedAtMs": 0,
            "item": {"type": "reasoning", "id": "reasoning-1", "summary": [], "content": []}}),
    ));
    for (index, delta) in [
        (0, "**Plan**\n\ndone"),
        (1, "**Checking tests**\n\n<!-- -->"),
    ] {
        s.feed(&notify(
            "item/reasoning/summaryPartAdded",
            serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": "reasoning-1", "summaryIndex": index}),
        ));
        s.feed(&notify(
            "item/reasoning/summaryTextDelta",
            serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": "reasoning-1",
                "delta": delta, "summaryIndex": index}),
        ));
    }
    s.feed(&item_completed(
        "thread-1",
        serde_json::json!({"type": "reasoning", "id": "reasoning-1",
            "summary": ["**Plan**\n\ndone", "**Checking tests**\n\n<!-- -->"], "content": []}),
    ));
    s.keys("ctrl+t");
    s.assert_contains("• done");
}

// ORACLE chatwidget/tests/history_replay.rs:1685 thread_snapshot_replayed_turn_started_marks_task_running kind=snap owner=chat-live
#[test]
fn thread_snapshot_replayed_turn_started_marks_task_running() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(SEED)
        .live()
        .open();
    s.feed(&turn_started("thread-1", "turn-1"));
    // oracle: `• Working (<elapsed> • esc to interrupt)`, the elapsed time counted from the turn's start
    s.assert_contains("• Working (");
    s.assert_contains(" • esc to interrupt)");
}

// ORACLE chatwidget/tests/history_replay.rs:1723 replayed_in_progress_turn_marks_task_running kind=snap owner=chat-live snap=replayed_in_progress_turn
#[test]
fn replayed_in_progress_turn_marks_task_running() {
    for items_view in ["full", "notLoaded"] {
        let mut turn = turn_ended("turn-1", "inProgress", serde_json::json!([]));
        turn["itemsView"] = serde_json::json!(items_view);
        turn["startedAt"] = serde_json::json!(1_700_000_000);
        // the oracle replays it without the thread's status: the scenario is a thread that is
        // really running (an idle thread's unfinished turn starts nothing, openai/codex#43952)
        let mut s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .replay(vec![turn])
            .replay_running()
            .live()
            .open();
        // oracle: no history cell, the bottom pane `• Working (<elapsed> • esc to interrupt)`
        s.assert_contains("• Working (");
        s.assert_contains(" • esc to interrupt)");
        s.assert_absent("Worked for");
        s.keys("ctrl+c");
        s.assert_interrupts(1);
    }
}

// ---- oracle: chatwidget/tests/list_spacing_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/list_spacing_tests.rs:9 list_spacing_streams_compact_then_reflows_on_completion_or_interruption kind=snap owner=chat-live
#[test]
fn list_spacing_streams_compact_then_reflows_on_completion_or_interruption() {
    // The oracle streams a four-item numbered list at 26 or 28 cells (Plan mode: a proposed
    // plan), interrupted or completed, in owned and retained transcript modes; the snapshot
    // holds every stage. Its `Completed` stage separates the items by a blank row where one
    // wraps and the `Wide` stage (100 cells) keeps them compact: the list spacing the source
    // is laid out with at the width, which the port's Markdown cannot draw.
    kit::pending(
        Owner::ChatLive,
        "G73: item 2, the oracle reflows a completed list at the width of the cell, blank rows between the items of a list that wraps (chatwidget list_spacing_tests, markdown_render.rs:1084) and compact when none does; rtwk's MarkdownView has no list spacing option (a list is tight or loose by its source), and the kit has no section form for the 12 joined stages",
    );
}

// ---- oracle: chatwidget/tests/luna_reserve_usage_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:30 luna_reserve_selector_supports_arrows_enter_shortcuts_and_escape_without_losing_draft kind=snap owner=chat-live snap=luna_reserve_selector_second_choice
#[test]
fn luna_reserve_selector_supports_arrows_enter_shortcuts_and_escape_without_losing_draft() {
    let credits = "https://chatgpt.com/settings/usage?credits_modal=true";
    let pricing = "https://chatgpt.com/?cta_tab=personal&highlight_plan=plus#pricing";
    for (keys, destination) in [
        (vec!["down", "enter"], Some(pricing)),
        (vec!["down", "up", "enter"], Some(credits)),
        (vec!["1"], Some(credits)),
        (vec!["2"], Some(pricing)),
        (vec!["down", "esc"], None),
    ] {
        let mut s = Screen::build(Owner::ChatLive).size(90, 24).live().open();
        s.type_text("saved draft");
        let response = serde_json::json!({
            "accountId": "account-preview", "rateLimits": {},
            "rateLimitUpsell": {
                "banner_type": "luna_reserve", "presentation": "dismissible",
                "title": "You\u{2019}re now using Luna, a faster model for simpler tasks.",
                "description": "Add credits or upgrade to continue using the most advanced models.",
                "ctas": [{"action": "add_credits", "label": "Add credits"},
                    {"action": "open_pricing_dialog", "label": "Upgrade"}]
            }
        });
        s.feed(&(answer(&response) + &model_step("gpt-reserve", true)));
        s.wait_for("esc to continue working");
        s.assert_absent("saved draft");
        let title = s
            .rows()
            .iter()
            .position(|row| row.contains("You\u{2019}re now using Luna"))
            .expect("the selector's title");
        for key in &keys {
            s.keys(key);
            if *key == "down" {
                s.assert_snap_region("luna_reserve_selector_second_choice", title, 0);
            }
        }
        s.assert_opened_urls(&destination.into_iter().collect::<Vec<_>>());
        s.assert_no_turn();
        s.assert_composer("saved draft");
        // A periodic usage reply must not reopen a selector the user just closed.
        s.feed(&(answer(&response) + &model_step("gpt-reserve", true)));
        s.assert_absent("You\u{2019}re now using Luna");
        s.assert_contains("saved draft");
    }
}

// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:109 luna_reserve_status_tracks_the_active_model kind=snap owner=chat-live snap=luna_reserve_usage_wide snap=luna_reserve_usage_narrow
#[test]
fn luna_reserve_status_tracks_the_active_model() {
    let mut s = reserve_chat(80, "gpt-5.6-sol", 25, 60);
    s.assert_absent("Luna Reserve");
    s.feed(&reserve_status("gpt-reserve"));
    s.assert_contains("Luna Reserve default");
    let at = row_with(&s, "› Ask Codex");
    s.assert_snap_region("luna_reserve_usage_wide", at, 0);
    // the oracle draws the pane alone, in the rows its `desired_height` asks (five at 34 columns, the first blank):
    // a status line that has one row is cut by the ellipsis, where the chat's room wraps it
    s.resize(34, 5);
    let at = row_with(&s, "› Ask Codex");
    s.assert_snap_region("luna_reserve_usage_narrow", at, 0);
    s.resize(80, 24);
    s.feed(&reserve_status("gpt-5.6-sol"));
    s.assert_absent("Luna Reserve");
    s.feed(&reserve_status("gpt-reserve"));
    s.assert_contains("Luna Reserve");
}

// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:141 luna_reserve_usage_survives_banner_dismissal_and_typing_during_a_turn kind=snap owner=chat-live snap=luna_reserve_usage_with_upsell snap=luna_reserve_usage_running
#[test]
fn luna_reserve_usage_survives_banner_dismissal_and_typing_during_a_turn() {
    let banner = serde_json::json!({
        "banner_type": "luna_reserve", "presentation": "dismissible",
        "title": "You\u{2019}re now using Luna, a faster model for simpler tasks.",
        "description": "Add credits or upgrade to continue using the most advanced models.",
        "ctas": [{"action": "add_credits", "label": "Add credits"},
            {"action": "open_pricing_dialog", "label": "Upgrade"}]
    });
    let upsell = |colors| {
        let mut s = reserve_chat_in(80, "gpt-reserve", 48, 20, colors, Some(&banner));
        s.wait_for("esc to continue working");
        s
    };
    let mut s = upsell(None);
    let at = row_with(&s, "You\u{2019}re now using Luna");
    s.assert_snap_region("luna_reserve_usage_with_upsell", at, 0);
    s.keys("esc");
    s.assert_absent("You\u{2019}re now using Luna");
    s.feed(TURN_STARTED);
    s.type_text("x");
    s.keys("left");
    s.assert_composer("x");
    let at = row_with(&s, "\u{2022} Working");
    s.assert_snap_region("luna_reserve_usage_running", at, 0);
    // the copy is in the terminal's own foreground, not dimmed, under a dark and a light palette
    for (fg, bg, rgb) in [
        (0xe6_e6_e6, 0x14_14_14, "Rgb(230,230,230)"),
        (0x19_19_19, 0xff_ff_ff, "Rgb(25,25,25)"),
    ] {
        let s = upsell(Some((fg, bg)));
        for text in ["You\u{2019}re now using Luna", "Add credits or upgrade"] {
            s.assert_style(2, row_with(&s, text), &format!("fg={rgb} !dim"));
        }
    }
}

// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:217 luna_reserve_prompt_preserves_the_composer_palette_on_exit kind=ui owner=chat-live
#[test]
fn luna_reserve_prompt_preserves_the_composer_palette_on_exit() {
    for (fg, bg) in [(0xe6_e6_e6, 0x14_14_14), (0x19_19_19, 0xff_ff_ff)] {
        let mut s = reserve_chat_in(80, "gpt-reserve", 25, 60, Some((fg, bg)), None);
        // the prompt row is on the ordinary user-message surface (a background, the row above
        // it the same), on the Reserve model and after it leaves
        let surface = |s: &Screen| {
            let row = row_with(s, "\u{203a} Ask Codex");
            let style = |y| s.grid().style(0, y).bg;
            kit::assert_same(style(row), style(row - 1), "the surface above the prompt");
            (row, style(row))
        };
        let (row, active) = surface(&s);
        kit::assert_same(active != "Reset", true, "the prompt row has a surface");
        s.feed(&reserve_status("gpt-5.6-sol"));
        s.assert_absent("Luna Reserve");
        let (restored_row, restored) = surface(&s);
        kit::assert_same(restored_row, row, "the prompt row stays");
        kit::assert_same(restored, active, "the surface after the model leaves");
    }
}
