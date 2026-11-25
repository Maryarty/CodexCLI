//! The hook runs under way and the rows they leave (oracle `chatwidget/hook_lifecycle.rs`,
//! `history_cell/hook_cell.rs`): a started run is on the list until its end; a finished run with
//! something to say is a transcript row at its place in the event order, and a restart of the same
//! id never drops it; the edges of a turn drop the runs whose end never came. The 300 ms delay
//! before the status row names a run is the view's (ADR-012: core reads no clock), so here a run
//! only has a `seq` to tell a restart from the first run.

#![allow(clippy::expect_used)]

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, apply_app};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::threads::Item;
use codex_tui_core::threads::hooks::{self, RunningHook};
use serde_json::{Value, json};

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let mut fx = AppFx::default();
    apply_app(&mut app, AppEv::Connected, &mut fx);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread command");
    apply_app(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
        &mut AppFx::default(),
    );
    app
}

fn notified(app: &mut CodexApp, notification: Value) {
    let notification: ServerNotification =
        serde_json::from_value(notification).expect("notification");
    apply_app(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        &mut AppFx::default(),
    );
}

fn run(id: &str, status: &str, message: Option<&str>, entries: Value) -> Value {
    json!({
        "id": id, "eventName": "preToolUse", "handlerType": "command", "executionMode": "sync",
        "scope": "turn", "sourcePath": "/tmp/hooks.json", "source": "user", "displayOrder": 0,
        "status": status, "statusMessage": message, "startedAt": 1,
        "completedAt": if status == "running" { Value::Null } else { json!(11) },
        "durationMs": if status == "running" { Value::Null } else { json!(10) },
        "entries": entries,
    })
}

fn started(app: &mut CodexApp, id: &str, message: Option<&str>) {
    notified(
        app,
        json!({"method": "hook/started", "params": {
            "threadId": "thread-1", "turnId": null,
            "run": run(id, "running", message, json!([]))}}),
    );
}

fn ended(app: &mut CodexApp, id: &str, status: &str, entries: Value) {
    notified(
        app,
        json!({"method": "hook/completed", "params": {
            "threadId": "thread-1", "turnId": null,
            "run": run(id, status, None, entries)}}),
    );
}

fn turn_started(app: &mut CodexApp) {
    notified(
        app,
        json!({"method": "turn/started", "params": {"threadId": "thread-1", "turn": {
            "id": "turn-1", "items": [], "itemsView": "full", "status": "inProgress",
            "error": null}}}),
    );
}

fn turn_ended(app: &mut CodexApp, status: &str) {
    let error = (status == "failed").then(|| json!({"message": "the model failed"}));
    notified(
        app,
        json!({"method": "turn/completed", "params": {"threadId": "thread-1", "turn": {
            "id": "turn-1", "items": [], "itemsView": "full", "status": status,
            "error": error}}}),
    );
}

fn answered(app: &mut CodexApp, text: &str) {
    notified(
        app,
        json!({"method": "item/completed", "params": {
            "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0,
            "item": {"type": "agentMessage", "id": "msg-1", "text": text, "phase": null}}}),
    );
}

/// The ids of the runs under way, in the order they started.
fn running(app: &CodexApp) -> Vec<String> {
    app.active_thread()
        .running_hooks()
        .iter()
        .map(|run| run.id.clone())
        .collect()
}

/// The transcript's rows by kind, in order.
fn kinds(app: &CodexApp) -> Vec<&'static str> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Hook(_) => Some("hook"),
            Item::Agent { .. } => Some("agent"),
            _ => None,
        })
        .collect()
}

#[test]
fn a_started_hook_is_under_way_until_it_completes() {
    // oracle hook_lifecycle.rs `handle_hook_started` / `handle_hook_completed`
    let mut app = ready();
    started(&mut app, "pre-tool-use:0", Some("checking command policy"));
    started(&mut app, "post-tool-use:1", None);
    assert_eq!(running(&app), ["pre-tool-use:0", "post-tool-use:1"]);
    let thread = app.active_thread();
    let runs = thread.running_hooks();
    assert_eq!(runs[0].message.as_deref(), Some("checking command policy"));
    assert_eq!(runs[1].message, None);

    ended(&mut app, "pre-tool-use:0", "completed", json!([]));
    assert_eq!(running(&app), ["post-tool-use:1"]);
    ended(&mut app, "post-tool-use:1", "completed", json!([]));
    assert!(running(&app).is_empty());
}

#[test]
fn a_restart_of_the_same_id_is_one_run_with_a_new_seq() {
    // oracle hook_cell.rs `start_run`: a second begin of an id replaces the run and resets its
    // reveal delay, so the view needs to tell the two apart
    let mut app = ready();
    started(&mut app, "stop:0", Some("checking stop condition"));
    let first = app.active_thread().running_hooks()[0].seq;
    started(&mut app, "stop:0", Some("checking stop condition"));
    let thread = app.active_thread();
    let runs = thread.running_hooks();
    assert_eq!(runs.len(), 1);
    assert_ne!(runs[0].seq, first);

    // and after a completion in between
    ended(&mut app, "stop:0", "completed", json!([]));
    started(&mut app, "stop:0", None);
    let again = app.active_thread().running_hooks()[0].seq;
    assert_ne!(again, first);
}

#[test]
fn the_edges_of_a_turn_drop_the_runs_whose_end_never_came() {
    // oracle turn_runtime.rs: `on_task_started`, `on_task_complete`, `handle_turn_interrupted`,
    // `on_error` call `clear_active_hook_cell`
    for (edge, status) in [
        ("started", ""),
        ("completed", "completed"),
        ("interrupted", "interrupted"),
        ("failed", "failed"),
    ] {
        let mut app = ready();
        if edge == "started" {
            started(&mut app, "pre-tool-use:0", Some("checking command policy"));
            turn_started(&mut app);
        } else {
            turn_started(&mut app);
            started(&mut app, "pre-tool-use:0", Some("checking command policy"));
            assert_eq!(running(&app).len(), 1, "{edge}");
            turn_ended(&mut app, status);
        }
        assert!(running(&app).is_empty(), "a turn that {edge}");
        // the end that arrives late finds nothing to take off, and writes no row
        ended(&mut app, "pre-tool-use:0", "completed", json!([]));
        assert!(kinds(&app).is_empty(), "a turn that {edge}");
    }
}

#[test]
fn a_hook_started_in_a_running_turn_survives_another_turn_started_event() {
    // only a turn that starts from idle clears: the run of the live turn is still its own
    let mut app = ready();
    turn_started(&mut app);
    started(&mut app, "pre-tool-use:0", None);
    assert_eq!(running(&app).len(), 1);
}

#[test]
fn a_finished_hook_with_output_is_a_row_before_the_message_that_follows() {
    // oracle `flush_completed_hook_output`: the row is written at the hook's end, in event order
    let mut app = ready();
    started(&mut app, "pre-tool-use:0", Some("checking command"));
    ended(
        &mut app,
        "pre-tool-use:0",
        "blocked",
        json!([{"kind": "feedback", "text": "command blocked by policy"}]),
    );
    answered(&mut app, "The hook feedback was applied.");
    assert_eq!(kinds(&app), ["hook", "agent"]);
    assert!(running(&app).is_empty());
}

#[test]
fn a_restart_of_the_same_id_keeps_the_row_the_first_run_left() {
    // oracle `completed_same_id_hook_output_survives_restart`
    let mut app = ready();
    started(&mut app, "stop:0", Some("checking stop condition"));
    ended(
        &mut app,
        "stop:0",
        "stopped",
        json!([{"kind": "stop", "text": "continue with more context"}]),
    );
    started(&mut app, "stop:0", Some("checking stop condition"));
    assert_eq!(kinds(&app), ["hook"]);
    assert_eq!(running(&app), ["stop:0"]);
}

#[test]
fn a_quiet_hook_leaves_no_row() {
    // oracle `context_only_hook_completed_before_reveal_stays_out_of_history`: the context is the
    // model's, a hook with nothing else to say is not history
    let mut app = ready();
    started(&mut app, "session-start:0", Some("warming the shell"));
    ended(
        &mut app,
        "session-start:0",
        "completed",
        json!([{"kind": "context", "text": "Remember the startup checklist."}]),
    );
    assert!(kinds(&app).is_empty());
    assert!(running(&app).is_empty());

    started(&mut app, "session-start:1", None);
    ended(&mut app, "session-start:1", "completed", json!([]));
    assert!(kinds(&app).is_empty());
}

fn hook(seq: u32, id: &str, message: Option<&str>) -> RunningHook {
    RunningHook {
        seq,
        id: id.to_string(),
        message: message.map(str::to_string),
    }
}

#[test]
fn the_status_summary_is_the_shared_message_else_how_many_run() {
    // oracle `HookCell::running_status_summary`
    let none: [RunningHook; 0] = [];
    assert_eq!(hooks::summary(&none), None);
    assert_eq!(
        hooks::summary(&[hook(0, "a", Some("checking command policy"))]).as_deref(),
        Some("checking command policy")
    );
    assert_eq!(
        hooks::summary(&[hook(0, "a", None)]).as_deref(),
        Some("Running hook")
    );
    assert_eq!(
        hooks::summary(&[hook(0, "a", Some("  "))]).as_deref(),
        Some("Running hook")
    );
    let same = [
        hook(0, "a", Some("checking command policy")),
        hook(1, "b", Some(" checking command policy ")),
        hook(2, "c", Some("checking command policy")),
    ];
    assert_eq!(
        hooks::summary(&same).as_deref(),
        Some("checking command policy")
    );
    let mixed = [
        hook(0, "a", Some("checking command policy")),
        hook(1, "b", Some("scanning for secrets")),
    ];
    assert_eq!(hooks::summary(&mixed).as_deref(), Some("Running hooks"));
    let one_silent = [hook(0, "a", Some("checking command policy")), hook(1, "b", None)];
    assert_eq!(hooks::summary(&one_silent).as_deref(), Some("Running hooks"));
    let silent = [hook(0, "a", None), hook(1, "b", None)];
    assert_eq!(hooks::summary(&silent).as_deref(), Some("Running hooks"));
}
