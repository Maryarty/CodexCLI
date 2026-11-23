#![allow(clippy::expect_used)]

//! State tests of the provider-error lock (core-pass for chunk-live-provider-error): a turn or
//! a steer the server refuses for the chat's model provider keeps the rejected message, locks
//! the thread and survives navigation (oracle `show_provider_error`).

use codex_app_server_protocol::ModelProviderError;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::screens::seed_ev;
use codex_tui_core::ids::TurnId;
use codex_tui_core::fixtures::builders::turn_started;
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::protocol::reply_from_server;
use codex_tui_core::rpc::{Reply, ServerError};
use codex_tui_core::screens::chat;
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, ScreenReply};
use codex_tui_core::threads::UserMessage;
use serde_json::json;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn submit(app: &mut CodexApp, text: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            message: UserMessage::text(text),
        }),
    )
}

fn not_configured() -> ServerError {
    ServerError {
        code: Some(-32600),
        message: "refused".into(),
        data: Some(json!({"type": "modelProviderNotConfigured", "modelProvider": "old-provider"})),
    }
}

fn the_turn(fx: &AppFx) -> (Ticket, Command) {
    fx.commands()
        .iter()
        .find_map(|c| match c {
            Command::StartTurn { ticket, .. } => Some((*ticket, c.clone())),
            _ => None,
        })
        .expect("a StartTurn")
}

#[test]
fn a_refused_first_prompt_waits_first_in_the_queue_and_locks_the_thread() {
    let mut app = ready();
    let (ticket, command) = the_turn(&submit(&mut app, "Keep this prompt"));
    let (answered, reply) = reply_from_server(&command, Err(not_configured())).expect("a reply");
    assert_eq!(answered, ticket);
    let fx = run(&mut app, AppEv::Reply(ticket, reply));
    let thread = app.active_thread();
    assert_eq!(
        thread.provider_error(),
        Some(&ModelProviderError::ModelProviderNotConfigured {
            model_provider: "old-provider".into()
        })
    );
    let queued: Vec<_> = thread.queue().iter().map(|m| m.text.as_str()).collect();
    assert_eq!(queued, ["Keep this prompt"]);
    // nothing goes out: the queue is not sent on its own
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    // and the thread takes no input any more
    let fx = submit(&mut app, "ignored");
    assert!(fx.commands().is_empty());
    assert_eq!(app.active_thread().queue().len(), 1);
}

#[test]
fn another_failure_is_not_a_provider_error() {
    let mut app = ready();
    let (ticket, command) = the_turn(&submit(&mut app, "hello"));
    let error = ServerError {
        code: Some(-32000),
        message: "boom".into(),
        data: Some(json!({"unrelated": true})),
    };
    let (_, reply) = reply_from_server(&command, Err(error)).expect("a reply");
    run(&mut app, AppEv::Reply(ticket, reply));
    assert!(app.active_thread().provider_error().is_none());
    assert!(app.active_thread().last_error().is_some());
}

#[test]
fn a_refused_steer_goes_back_alone() {
    let mut app = ready();
    run(
        &mut app,
        AppEv::Backend(turn_started(T, "turn-1")),
    );
    let first = submit(&mut app, "Focus on the backend");
    let second = submit(&mut app, "Also add tests");
    let steer = |fx: &AppFx| {
        fx.commands()
            .iter()
            .find_map(|c| match c {
                Command::Screen {
                    ticket,
                    command: ScreenCommand::Chat(chat::Command::Steer { .. }),
                    ..
                } => Some((*ticket, c.clone())),
                _ => None,
            })
            .expect("a steer")
    };
    let (accepted, _) = steer(&first);
    let (rejected, command) = steer(&second);
    run(
        &mut app,
        AppEv::Reply(
            accepted,
            Reply::screen(ScreenReply::Chat(chat::Reply::Steered(TurnId::new("turn-1")))),
        ),
    );
    let (_, reply) = reply_from_server(&command, Err(not_configured())).expect("a reply");
    run(&mut app, AppEv::Reply(rejected, reply));
    let thread = app.active_thread();
    assert!(thread.provider_error().is_some());
    let pending: Vec<_> = thread
        .pending_steers()
        .iter()
        .map(|steer| steer.message().text.as_str())
        .collect();
    assert_eq!(pending, ["Focus on the backend"]);
    let rejected: Vec<_> = thread
        .rejected_steers()
        .iter()
        .map(|m| m.text.as_str())
        .collect();
    assert_eq!(rejected, ["Also add tests"]);
}

fn saved(app: &mut CodexApp, path: Option<&str>) {
    let notification: codex_app_server_protocol::ServerNotification =
        serde_json::from_value(json!({
            "method": "thread/started",
            "params": {"thread": {
                "id": T, "environments": null, "extra": null, "sessionId": T,
                "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
                "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
                "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
                "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
                "recencyAt": 0, "status": {"type": "idle"}, "path": path, "cwd": "/tmp",
                "cliVersion": "0.0.0", "originator": null, "source": "cli",
                "canAcceptDirectInput": true, "threadSource": null,
                "agentNickname": null, "agentRole": null, "gitInfo": null, "name": null,
                "daybreakEnabled": null, "turns": []
            }}
        }))
        .expect("thread/started");
    run(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
    );
}

fn refuse(app: &mut CodexApp) {
    let seed = seed_ev(
        "chat",
        json!({"provider_error": {"thread": T, "error": {
            "type": "modelProviderMismatch",
            "modelProvider": "custom-gateway", "currentModelProvider": "amazon-bedrock-runtime"}}}),
    )
    .expect("a seed");
    run(app, AppEv::Screen(seed));
}

fn continue_chat(app: &mut CodexApp) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::New,
            intent: SessionIntent::ContinueChat {
                message: "Read it".into(),
            },
        }))),
    )
}

#[test]
fn the_continue_key_opens_a_fresh_chat_with_the_text_in_its_composer() {
    let mut app = ready();
    saved(&mut app, Some("/tmp/saved.jsonl"));
    refuse(&mut app);
    assert!(app.active_thread().provider_error().is_some());
    let fx = continue_chat(&mut app);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a fresh thread is asked for");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-2"))),
    );
    let fresh = app.active_thread();
    assert_eq!(fresh.id().as_str(), "thread-2");
    assert!(fresh.provider_error().is_none());
    let restore = fresh.restore().expect("the text is given back");
    assert_eq!(restore.merged().text, "Read it");
    // the refused chat stays locked, ready to be shown again
    assert!(
        app.thread(&ThreadId::new(T))
            .expect("held")
            .provider_error()
            .is_some()
    );
}

#[test]
fn the_continue_key_needs_a_saved_chat() {
    let mut app = ready();
    saved(&mut app, None);
    refuse(&mut app);
    assert!(continue_chat(&mut app).commands().is_empty());
}
