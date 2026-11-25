//! Screen tests of the `backtrack` slot. A slice adds its tests here, with its
//! snapshots in `tests/snapshots/backtrack/`.
//!
//! The edit of a past prompt end to end: Esc Esc, Enter on the marked prompt,
//! `thread/read` and `thread/revert` answered by the scripted server, the
//! prompt back in the composer; and the 10 s the server gets to confirm the
//! revert with `thread/reverted`.

#![allow(clippy::expect_used)]

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::turn_completed;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use serde_json::{Value, json};

use crate::common::{Harness, ms, slot, start};

#[test]
fn backtrack_slot_is_registered_and_empty() {
    slot("backtrack");
}

fn thread_with_one_turn(prompt: &str) -> Value {
    let mut thread = json!({
        "id": FAKE_THREAD, "environments": null, "extra": null, "sessionId": FAKE_THREAD,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": prompt, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000
    });
    let rest = json!({
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": null,
        "daybreakEnabled": null
    });
    let message = json!({"type": "userMessage", "id": "user-1", "clientId": null,
        "content": [{"type": "text", "text": prompt, "text_elements": []}]});
    let turn = json!({"id": "turn-1", "itemsView": "full", "status": "completed",
        "error": null, "items": [message]});
    let (Some(thread_map), Some(rest)) = (thread.as_object_mut(), rest.as_object()) else {
        unreachable!("objects")
    };
    thread_map.extend(rest.clone());
    thread_map.insert("turns".into(), json!([turn]));
    thread
}

/// The turn the revert kept: the prompt before the edited one, and its answer.
fn kept_turn() -> Value {
    json!({"id": "turn-0", "rootTurnId": null, "itemsView": "full", "status": "completed",
    "error": null, "startedAt": null, "completedAt": null, "durationMs": null,
    "items": [
        {"type": "userMessage", "id": "user-0", "clientId": null,
         "content": [{"type": "text", "text": "kept prompt", "text_elements": []}]},
        {"type": "agentMessage", "id": "agent-0", "text": "kept answer"}
    ]})
}

/// A chat that holds the finished turn of `go`, its server answering the read and the
/// revert of the edit.
fn chat_with_a_prompt() -> Harness {
    let mut h = start();
    h.wait_idle();
    h.fake.respond(|request| match request["method"].as_str()? {
        "thread/read" => Some(Ok(json!({"thread": thread_with_one_turn("go")}))),
        "thread/revert" => Some(Ok(json!({"thread": thread_with_one_turn("go")}))),
        // what the revert kept, read again once the server says it reverted
        "thread/turns/list" if request["params"]["limit"].as_u64() != Some(1) => {
            Some(Ok(json!({"data": [kept_turn()], "nextCursor": null})))
        }
        _ => None,
    });
    h.fake.script_turn(vec![]);
    h.t.type_text("go");
    h.press("enter");
    h.wait_for("the prompt in the transcript", |s| {
        s.contains("› go") && s.contains("Ask Codex to do anything")
    });
    let epoch = h.fake.epoch().expect("attached");
    h.fake
        .inject(epoch, AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1")));
    h.wait_idle();
    h
}

fn methods(h: &Harness) -> Vec<String> {
    h.fake
        .requests()
        .iter()
        .filter_map(|request| request["method"].as_str().map(str::to_string))
        .collect()
}

/// The edit is chosen: Esc Esc, Enter; the server has read and reverted, and the prompt
/// is in the composer.
fn edit_the_prompt(h: &mut Harness) {
    h.wait_for("the turn to end", |s| s.contains("? for shortcuts"));
    h.press("esc");
    h.wait_for("the first Esc to prime", |s| {
        s.contains("esc again to edit")
    });
    h.press("esc");
    h.press("enter");
    h.wait_until("the revert", |h| {
        methods(h).iter().any(|method| method == "thread/revert")
    });
    h.wait_for("the prompt in the composer", in_the_composer);
}

/// The prompt twice: in the transcript and in the composer.
fn in_the_composer(screen: &str) -> bool {
    screen
        .lines()
        .filter(|row| row.trim_start() == "› go")
        .count()
        == 2
}

#[test]
fn enter_on_the_marked_prompt_reverts_before_it_and_puts_it_in_the_composer() {
    let mut h = chat_with_a_prompt();
    assert!(!in_the_composer(&h.t.screen()));
    edit_the_prompt(&mut h);
    let methods = methods(&h);
    let read = methods
        .iter()
        .position(|m| m == "thread/read")
        .expect("read");
    let revert = methods
        .iter()
        .position(|m| m == "thread/revert")
        .expect("revert");
    assert!(read < revert, "{methods:?}");
    let screen = h.t.screen();
    assert!(in_the_composer(&screen), "{screen}");
}

#[test]
fn ten_seconds_without_the_servers_notice_say_so_and_the_composer_keeps_the_text() {
    let mut h = chat_with_a_prompt();
    edit_the_prompt(&mut h);
    // the revert's answer comes in real time: the ten seconds run from the moment it is in
    h.wait_until("the wait for the notice", |h| {
        h.t.read_app(|app: &UiApp| {
            app.core
                .active_thread()
                .screen_slots()
                .backtrack
                .awaiting()
                .is_some()
        })
    });
    h.t.pump_for(ms(100));
    assert!(
        !h.t.screen().contains("could not be confirmed"),
        "{}",
        h.t.screen()
    );
    h.t.pump_for(ms(10_500));
    // the expiry goes through the ingress, which the backend thread drains in real time
    h.wait_for("the error line", |screen| {
        screen.contains("prompt edit could not be confirmed")
    });
    let screen = h.t.screen();
    assert!(in_the_composer(&screen), "{screen}");
}

#[test]
fn the_servers_notice_in_time_leaves_no_error() {
    let mut h = chat_with_a_prompt();
    edit_the_prompt(&mut h);
    let epoch = h.fake.epoch().expect("attached");
    let reverted = serde_json::from_value(
        json!({"method": "thread/reverted", "params": {"threadId": FAKE_THREAD}}),
    )
    .expect("notification");
    h.fake.inject(
        epoch,
        AppEv::Backend(codex_tui_core::app::BackendEv::Notification(Box::new(
            reverted,
        ))),
    );
    h.settle();
    h.t.pump_for(ms(11_000));
    assert!(
        !h.t.screen().contains("could not be confirmed"),
        "{}",
        h.t.screen()
    );
}

/// `thread/reverted` as the server sends it, for the one thread.
fn reverted_notice(h: &Harness) {
    let epoch = h.fake.epoch().expect("attached");
    let reverted = serde_json::from_value(
        json!({"method": "thread/reverted", "params": {"threadId": FAKE_THREAD}}),
    )
    .expect("notification");
    h.fake.inject(
        epoch,
        AppEv::Backend(codex_tui_core::app::BackendEv::Notification(Box::new(
            reverted,
        ))),
    );
}

const REVERTED_LINE: &str = "• Conversation reverted to this point. File changes are unchanged.";

#[test]
fn the_edit_says_the_conversation_was_reverted_after_the_history_it_kept() {
    // oracle: `emit_prompt_edit_thread_event`, the line the revert's end inserts after the
    // retained history (`owned_prompt_revert_retained_history`)
    let mut h = chat_with_a_prompt();
    edit_the_prompt(&mut h);
    reverted_notice(&h);
    h.wait_for("the line after the kept history", |screen| {
        screen.contains(REVERTED_LINE) && screen.contains("kept answer")
    });
    let screen = h.t.screen();
    let rows: Vec<&str> = screen.lines().collect();
    let kept = rows
        .iter()
        .position(|row| row.contains("kept answer"))
        .expect("the kept answer");
    let line = rows
        .iter()
        .position(|row| row.trim_start() == REVERTED_LINE)
        .expect("the line");
    assert!(kept < line, "{screen}");
}

#[test]
fn a_revert_nobody_here_asked_for_says_nothing() {
    let mut h = chat_with_a_prompt();
    reverted_notice(&h);
    h.settle();
    assert!(
        !h.t.screen().contains("Conversation reverted"),
        "{}",
        h.t.screen()
    );
}
