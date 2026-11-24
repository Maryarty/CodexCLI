#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Proofs that upstream `openai/codex` TUI issues are fixed in the port: each test names its
//! issue and fails if the port still has the bug. (Screen-level proofs of the other issues
//! are in `tests/pages/issue_proofs.rs`.)

use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::fixtures::builders::turn_started;
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::agents::{Ev, Page, Reply as AgentsReply};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenReply};
use serde_json::{Value, json};

const OLD: &str = "00000000-0000-0000-0000-000000000001";

fn dispatch(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

/// A thread of the server's list with the recorded `turns` and the server's `status`.
fn thread(status: Value, turns: Vec<Value>) -> Thread {
    serde_json::from_value(json!({
        "id": OLD, "environments": null, "extra": null, "sessionId": OLD,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "old", "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000, "status": status,
        "path": null, "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": "old",
        "daybreakEnabled": null, "turns": turns
    }))
    .expect("a thread")
}

/// A recorded turn of the thread still `inProgress`, with one user message.
fn unfinished_turn() -> Value {
    json!({
        "id": "turn-1", "rootTurnId": null, "itemsView": "full", "status": "inProgress",
        "error": null, "startedAt": 1_700_000_000, "completedAt": null, "durationMs": null,
        "items": [{"type": "userMessage", "id": "item-1", "clientId": null,
            "content": [{"type": "text", "text": "go", "text_elements": []}]}]
    })
}

/// The app after the resume of `OLD` was answered with `thread`.
fn resumed(thread: Thread) -> CodexApp {
    let mut app = CodexApp::new();
    dispatch(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
            threads: vec![thread.clone()],
            unread: vec![],
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    let fx = dispatch(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(OLD))),
    );
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(_),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("a resume request");
    dispatch(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(AgentsReply::Resumed(Box::new(thread)))),
        ),
    );
    assert_eq!(app.session().active, Some(ThreadId::new(OLD)));
    app
}

/// openai/codex#43952 (resume paints a permanent "Working" footer when the thread is Idle): a
/// snapshot whose last turn is `inProgress` while the server says the thread is idle (the
/// turn's end was never recorded) starts no turn: nothing runs, nothing can be interrupted,
/// until a live `turn/started` comes.
#[test]
#[ignore = "RED #43952: an idle thread resumed with an in-progress turn shows Working"]
fn issue_43952_an_idle_thread_resumed_with_an_unfinished_turn_does_not_work_until_a_live_start() {
    let mut app = resumed(thread(json!({"type": "idle"}), vec![unfinished_turn()]));
    let model = app.active_thread();
    assert!(
        !model.is_running(),
        "the idle thread's stale in-progress turn shows the Working footer"
    );
    assert!(
        !model.can_interrupt(),
        "the idle thread offers an interrupt"
    );

    dispatch(&mut app, AppEv::Backend(turn_started(OLD, "turn-2")));
    let model = app.active_thread();
    assert!(model.is_running(), "a live turn/started runs the thread");
    assert!(model.can_interrupt());
}

/// The side effect to check on #43952: a thread that is really running when it is resumed (the
/// server says `active`) shows its turn as running at once.
#[test]
fn issue_43952_a_thread_resumed_while_really_running_shows_its_turn_at_once() {
    let app = resumed(thread(
        json!({"type": "active", "activeFlags": []}),
        vec![unfinished_turn()],
    ));
    let model = app.active_thread();
    assert!(
        model.is_running(),
        "a really running thread must show Working"
    );
    assert!(model.can_interrupt());
}
