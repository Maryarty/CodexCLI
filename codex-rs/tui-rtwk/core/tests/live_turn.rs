#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The end of a live turn (oracle `chatwidget/turn_runtime.rs` `finalize_active_cell_as_failed`,
//! `chatwidget/tests/review_mode.rs` `interrupt_exec_marks_failed_snapshot`): the turn's own
//! `turn/completed` lists its items as `full` with none in it, and the calls the notifications
//! drew stay in the transcript, a running command finished as failed.

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::threads::exploration::Outcome;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const T: &str = "thread-1";

fn notify(message: Value) -> String {
    format!(r#"{{"step":"notify","message":{message}}}"#)
}

fn turn(id: &str, status: &str, items: Value) -> Value {
    json!({"id": id, "rootTurnId": null, "items": items, "itemsView": "full", "status": status,
        "error": null, "startedAt": null, "completedAt": null, "durationMs": null})
}

fn started() -> String {
    notify(json!({"method": "turn/started", "params": {
        "threadId": T, "turn": turn("turn-1", "inProgress", json!([]))}}))
}

fn ended(status: &str) -> String {
    notify(json!({"method": "turn/completed", "params": {
        "threadId": T, "turn": turn("turn-1", status, json!([]))}}))
}

fn running_read() -> String {
    notify(json!({"method": "item/started", "params": {
        "threadId": T, "turnId": "turn-1", "startedAtMs": 0, "item": {
            "type": "commandExecution", "id": "c1", "command": "cat a.rs", "cwd": "/tmp",
            "processId": null, "source": "agent", "status": "inProgress",
            "commandActions": [{"type": "read", "command": "cat a.rs", "name": "a.rs",
                "path": "/tmp/a.rs"}],
            "aggregatedOutput": null, "exitCode": null, "durationMs": null}}}))
}

fn outcomes(status: &str) -> Vec<Outcome> {
    let script = [
        r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_string(),
        started(),
        running_read(),
        ended(status),
    ];
    let sandbox = Sandbox::play(&script.join("\n")).expect("plays");
    let groups = sandbox
        .app()
        .active_thread()
        .transcript()
        .exploration_groups();
    assert_eq!(groups.len(), 1, "{groups:?}");
    groups[0].calls.iter().map(|call| call.outcome).collect()
}

#[test]
fn an_interrupted_turn_with_an_empty_full_list_keeps_its_running_call_as_failed() {
    assert_eq!(
        outcomes("interrupted"),
        vec![Outcome::Failed { exit_code: 1 }]
    );
}

#[test]
fn a_failed_turn_with_an_empty_full_list_keeps_its_running_call_as_failed() {
    assert_eq!(outcomes("failed"), vec![Outcome::Failed { exit_code: 1 }]);
}
