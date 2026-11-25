//! The startup draft's confirmation, core half (oracle `chatwidget/startup_submission.rs`,
//! `input_submission.rs`): the model gate a submit and the queue's drain go through, and the
//! rules of when a confirmed draft may go out. The draft itself is the page's.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::Notice;
use codex_tui_core::fixtures::builders::{turn_completed, turn_started};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::{ScreenThreadIntent, chat};
use codex_tui_core::threads::startup::{confirm_ready, confirms, model_unavailable};
use codex_tui_core::threads::{Item, Note, NoteKey, UserMessage};

const T: &str = "thread-1";
const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

/// `thread/settings/updated` naming `model`.
fn settings(model: &str) -> String {
    serde_json::json!({"step": "notify", "message": {
        "method": "thread/settings/updated",
        "params": {"threadId": T, "threadSettings": {
            "cwd": "/tmp/project", "approvalPolicy": "on-request", "approvalsReviewer": "user",
            "sandboxPolicy": {"type": "dangerFullAccess"}, "activePermissionProfile": null,
            "model": model, "modelProvider": "openai", "serviceTier": null, "effort": null,
            "summary": null, "personality": null,
            "collaborationMode": {"mode": "default", "settings": {
                "model": model, "reasoning_effort": null, "developer_instructions": null}}
        }}
    }})
    .to_string()
}

/// A session whose thread names `model`, or none.
fn app(model: Option<&str>) -> CodexApp {
    let mut script = SESSION.to_string();
    if let Some(model) = model {
        script = format!("{script}\n{}", settings(model));
    }
    build(&format!("{script}\n")).expect("plays").app
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
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

fn tab(app: &mut CodexApp, text: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(
                UserMessage::text(text).into(),
            )),
        }),
    )
}

fn event(app: &mut CodexApp, ev: BackendEv) -> AppFx {
    run(app, AppEv::Backend(ev))
}

fn turns(fx: &AppFx) -> Vec<String> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect()
}

fn refusals(app: &CodexApp) -> usize {
    app.active_thread()
        .transcript()
        .iter()
        .filter(|(_, item)| {
            matches!(
                item,
                Item::Note(Note {
                    key: NoteKey::Command(Notice::ThreadModelUnavailable),
                    ..
                })
            )
        })
        .count()
}

fn restored(app: &CodexApp) -> Vec<String> {
    app.active_thread()
        .restore()
        .map(|restore| restore.messages().iter().map(|m| m.text.clone()).collect())
        .unwrap_or_default()
}

fn queued(app: &CodexApp) -> Vec<String> {
    app.active_thread()
        .queue()
        .iter()
        .map(|m| m.text.clone())
        .collect()
}

#[test]
fn only_a_model_known_to_be_empty_is_unavailable() {
    assert!(model_unavailable(Some("")));
    assert!(model_unavailable(Some("  ")));
    assert!(!model_unavailable(Some("gpt-5.2")));
    // a thread never told its model runs on the config's
    assert!(!model_unavailable(None));
}

#[test]
fn a_submit_with_no_model_says_so_and_sends_nothing() {
    let mut app = app(Some(""));
    let fx = submit(&mut app, "hello");
    assert!(turns(&fx).is_empty(), "{:?}", fx.commands());
    assert_eq!(refusals(&app), 1);
    // the line stays where it was typed: the thread takes no submit and gives nothing back
    assert!(restored(&app).is_empty());
    // the model comes back: the same line goes out
    run_settings(&mut app, "gpt-5.2");
    assert_eq!(turns(&submit(&mut app, "hello")), ["hello"]);
}

#[test]
fn a_thread_that_was_never_told_its_model_is_not_refused() {
    let mut app = app(None);
    assert_eq!(turns(&submit(&mut app, "hello")), ["hello"]);
    assert_eq!(refusals(&app), 0);
}

#[test]
fn the_drain_gives_the_message_back_and_keeps_the_rest() {
    let mut app = app(None);
    event(&mut app, turn_started(T, "turn-1"));
    tab(&mut app, "first");
    tab(&mut app, "second");
    assert_eq!(queued(&app), ["first", "second"]);
    run_settings(&mut app, "");
    let fx = event(&mut app, turn_completed(T, "turn-1"));
    assert!(turns(&fx).is_empty(), "{:?}", fx.commands());
    assert_eq!(refusals(&app), 1);
    assert_eq!(restored(&app), ["first"]);
    assert_eq!(queued(&app), ["second"]);
}

#[test]
fn the_drain_goes_on_when_the_model_is_back() {
    let mut app = app(None);
    event(&mut app, turn_started(T, "turn-1"));
    tab(&mut app, "first");
    run_settings(&mut app, "");
    event(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(restored(&app), ["first"]);
    run_settings(&mut app, "gpt-5.2");
    assert_eq!(turns(&submit(&mut app, "first")), ["first"]);
}

#[test]
fn the_startup_draft_confirms_text_and_leaves_commands_to_run() {
    assert!(confirms("hello"));
    assert!(confirms("!ls -la"));
    assert!(confirms("  /not a command"));
    assert!(confirms("/usr/bin/env is a path"));
    assert!(!confirms(""));
    assert!(!confirms("  \n "));
    assert!(!confirms("/model"));
    assert!(!confirms("/plan do it"));
}

#[test]
fn the_ready_rules_follow_the_session_and_the_thread() {
    let mut app = app(None);
    let ready = |app: &CodexApp| confirm_ready(app.session(), &app.active_thread());
    assert!(ready(&app));
    // a turn that runs holds the draft until it ends
    event(&mut app, turn_started(T, "turn-1"));
    assert!(!ready(&app));
    event(&mut app, turn_completed(T, "turn-1"));
    assert!(ready(&app));
    // a model known to be empty holds it until one arrives
    run_settings(&mut app, "");
    assert!(!ready(&app));
    run_settings(&mut app, "gpt-5.2");
    assert!(ready(&app));
}

#[test]
fn the_session_that_has_not_started_is_not_ready() {
    let mut app = CodexApp::new();
    app.attach();
    let blank = app.active_thread();
    assert!(!confirm_ready(app.session(), &blank));
}

/// The server's settings of the thread name `model`.
fn run_settings(app: &mut CodexApp, model: &str) {
    let line: serde_json::Value = serde_json::from_str(&settings(model)).expect("json");
    let notification: codex_app_server_protocol::ServerNotification =
        serde_json::from_value(line["message"].clone()).expect("a notification");
    event(app, BackendEv::Notification(Box::new(notification)));
}
