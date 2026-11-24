#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/recap` (oracle `app/recap.rs`, `app/recap_history.rs`): the excerpt of the visible exchanges,
//! the structured turn asked of the host, and what its answer becomes (a recap, the failure line,
//! or nothing when the conversation moved on). The words and the cell are the UI's: these tests
//! read data.

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::recap::{Intent as RecapIntent, Message, Note as RecapNote};
use codex_tui_core::features::{FeatureIntent, FeatureNote};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Item as Entry, Note, NoteKey};
use pretty_assertions::assert_eq;
use serde_json::json;

const T: &str = "thread-1";

fn line(value: serde_json::Value) -> String {
    json!({"step": "notify", "message": value}).to_string()
}

fn user(id: &str, text: &str) -> String {
    line(
        json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
        "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "userMessage", "id": id, "clientId": null,
            "content": [{"type": "text", "text": text, "text_elements": []}]}}}),
    )
}

fn said(id: &str, text: &str) -> String {
    line(
        json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
        "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "agentMessage", "id": id, "text": text, "phase": null}}}),
    )
}

/// An app whose thread went through `exchanges`: a request and its answer each.
fn app(exchanges: &[(&str, &str)]) -> CodexApp {
    let mut script = vec![r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_owned()];
    for (i, (asked, answered)) in exchanges.iter().enumerate() {
        script.push(user(&format!("u{i}"), asked));
        script.push(said(&format!("a{i}"), answered));
    }
    build(&script.join("\n")).expect("plays").app
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn recap(app: &mut CodexApp) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Recap(
            RecapIntent::Command {
                source: Some(ThreadId::new(T)),
            },
        ))),
    )
}

fn asks(fx: &AppFx) -> Vec<(u64, Ask)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
            _ => None,
        })
        .collect()
}

fn notes(app: &CodexApp) -> Vec<RecapNote> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: NoteKey::Feature(FeatureNote::Recap(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

fn answer(app: &mut CodexApp, request: u64, response: &str) {
    run(
        app,
        AppEv::Effect {
            request,
            done: Done::Structured(Ok(response.to_owned())),
        },
    );
}

#[test]
fn the_history_labels_exchanges_and_a_pending_request() {
    let messages = [
        Message::User("first".to_owned()),
        Message::Assistant("one".to_owned()),
        Message::User("second".to_owned()),
        Message::User("and also".to_owned()),
    ];
    // adjacent user messages are one request; with no answer it is the pending one
    assert_eq!(
        codex_tui_core::features::recap::recap_history(&messages),
        "User: first\n\nAssistant: one\n\nPending user request: second\n\nand also"
    );
    assert_eq!(codex_tui_core::features::recap::recap_history(&[]), "");
}

#[test]
fn a_long_history_keeps_the_newest_exchanges_and_says_what_it_left_out() {
    let messages: Vec<Message> = (0..40)
        .flat_map(|i| {
            [
                Message::User(format!("request {i} {}", "x".repeat(2000))),
                Message::Assistant(format!("answer {i}")),
            ]
        })
        .collect();
    let history = codex_tui_core::features::recap::recap_history(&messages);
    assert!(history.starts_with("[Earlier exchanges omitted]"));
    assert!(history.contains("answer 39"));
    assert!(!history.contains("answer 0\n"));
}

#[test]
fn an_empty_conversation_has_nothing_to_recap() {
    let mut app = app(&[]);
    assert!(asks(&recap(&mut app)).is_empty());
    assert_eq!(notes(&app), vec![RecapNote::EmptyHistory]);
}

#[test]
fn a_recap_asks_one_structured_turn_and_says_the_answer() {
    let mut app = app(&[("fix the bug", "done"), ("ship it", "shipped")]);
    let fx = recap(&mut app);
    let sent = asks(&fx);
    let [
        (
            request,
            Ask::StructuredTurn {
                thread,
                prompt,
                schema,
            },
        ),
    ] = sent.as_slice()
    else {
        panic!("expected one structured turn, got {sent:?}");
    };
    assert_eq!(thread, &ThreadId::new(T));
    assert!(
        prompt.ends_with(
            "User: fix the bug\n\nAssistant: done\n\nUser: ship it\n\nAssistant: shipped"
        )
    );
    assert_eq!(schema["required"], json!(["summary", "next_action"]));
    // a second one while it generates says so and asks nothing
    assert!(asks(&recap(&mut app)).is_empty());
    answer(
        &mut app,
        *request,
        r#"{"summary": " Shipped the fix. ", "next_action": " "}"#,
    );
    assert_eq!(
        notes(&app),
        vec![
            RecapNote::InProgress,
            RecapNote::Recap {
                summary: "Shipped the fix.".to_owned(),
                next_action: None
            }
        ]
    );
    // the request is over: another can be made
    assert_eq!(asks(&recap(&mut app)).len(), 1);
}

#[test]
fn an_answer_that_does_not_parse_is_the_failure_line() {
    for response in [
        "not json",
        r#"{"summary": "x"}"#,
        r#"{"summary": "", "next_action": null}"#,
        r#"{"summary": "x", "next_action": null, "extra": 1}"#,
        r#"{"summary": "x", "next_action": 3}"#,
    ] {
        let mut app = app(&[("a", "b")]);
        let request = asks(&recap(&mut app))[0].0;
        answer(&mut app, request, response);
        assert_eq!(notes(&app), vec![RecapNote::Failed], "{response}");
    }
    let mut app = app(&[("a", "b")]);
    let request = asks(&recap(&mut app))[0].0;
    run(
        &mut app,
        AppEv::Effect {
            request,
            done: Done::Structured(Err("no server".to_owned())),
        },
    );
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

#[test]
fn an_answer_for_a_conversation_that_moved_on_is_dropped() {
    let mut app = app(&[("a", "b")]);
    let request = asks(&recap(&mut app))[0].0;
    run(
        &mut app,
        AppEv::Backend(BackendEv::Notification(Box::new(
            serde_json::from_value::<ServerNotification>(json!({"method": "item/completed",
                "params": {"threadId": T, "turnId": "turn-2", "completedAtMs": 0,
                "startedAtMs": 0, "item": {"type": "agentMessage", "id": "late", "text": "more",
                "phase": null}}}))
            .unwrap(),
        ))),
    );
    answer(
        &mut app,
        request,
        r#"{"summary": "x", "next_action": null}"#,
    );
    assert_eq!(notes(&app), vec![]);
}
