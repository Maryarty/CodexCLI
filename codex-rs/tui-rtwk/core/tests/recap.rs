#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/recap` (oracle `app/recap.rs`, `app/recap_history.rs`): the excerpt of the visible exchanges,
//! the structured turn asked of the door (`structured_turn.rs` walks the run itself), and what
//! its answer becomes (a recap, the failure line, or nothing when the conversation moved on). The
//! words and the cell are the UI's: these tests read data.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Intent, apply_app};
use codex_tui_core::features::recap::{Intent as RecapIntent, Message, Note as RecapNote};
use codex_tui_core::features::{FeatureIntent, FeatureNote};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::structured::Kind;
use codex_tui_core::threads::{Item as Entry, Note, NoteKey, ThreadAsk};
use pretty_assertions::assert_eq;
use serde_json::json;

use crate::structured_turn::{TEMP, ends, says, stream, the_ask, thread_asks, unsubscribes, walk};

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
pub fn app(exchanges: &[(&str, &str)]) -> CodexApp {
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

pub fn recap(app: &mut CodexApp) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Recap(
            RecapIntent::Command {
                source: Some(ThreadId::new(T)),
            },
        ))),
    )
}

pub fn notes(app: &CodexApp) -> Vec<RecapNote> {
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

/// The slice asked for a structured turn: the first request of the run is out.
fn asked(fx: &AppFx) -> bool {
    matches!(
        thread_asks(fx).as_slice(),
        [(_, ThreadAsk::McpServers { .. })]
    )
}

/// The run goes through to the end with the model's `response`.
fn answer(app: &mut CodexApp, first: &AppFx, response: &str) {
    walk(app, first, json!({}));
    says(app, response);
    ends(app, "completed");
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
    // the oracle reads the last eight exchanges, then drops the oldest ones until the rest fits
    // the prompt's byte budget (32 KiB): eight of 6000 bytes do not
    let messages: Vec<Message> = (0..40)
        .flat_map(|i| {
            [
                Message::User(format!("request {i} {}", "x".repeat(6000))),
                Message::Assistant(format!("answer {i}")),
            ]
        })
        .collect();
    let history = codex_tui_core::features::recap::recap_history(&messages);
    assert!(history.starts_with("[Earlier exchanges omitted]"));
    assert!(history.contains("answer 39"));
    assert!(!history.contains("request 31 "));
    assert!(!history.contains("answer 0\n"));
}

#[test]
fn an_empty_conversation_has_nothing_to_recap() {
    let mut app = app(&[]);
    assert!(thread_asks(&recap(&mut app)).is_empty());
    assert_eq!(notes(&app), vec![RecapNote::EmptyHistory]);
}

/// `request_recap` with `ThreadSettingsOwner::Server`: the cloud owns the settings of its
/// conversations, so a manual recap says so and asks nothing (the first check, before the in-flight
/// and the empty-history ones).
#[test]
fn a_cloud_conversation_has_no_recap() {
    use codex_tui_core::app::{BackendEv, Command, apply_app_from};
    use codex_tui_core::ids::BackendId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::agents::Ev;
    const CLOUD: BackendId = BackendId(1);
    let mut app = app(&[]);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::CloudBackend(CLOUD))),
    );
    let fx = run(&mut app, AppEv::Intent(Intent::CycleBackend));
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
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("cloud-1"))),
    );
    let mut fx = AppFx::default();
    let note: codex_app_server_protocol::ServerNotification =
        serde_json::from_value(json!({"method": "item/completed",
        "params": {"threadId": "cloud-1", "turnId": "turn-1", "completedAtMs": 0,
        "startedAtMs": 0, "item": {"type": "userMessage", "id": "u1", "clientId": null,
        "content": [{"type": "text", "text": "fix it", "text_elements": []}]}}}))
        .unwrap();
    apply_app_from(
        &mut app,
        CLOUD,
        AppEv::Backend(BackendEv::Notification(Box::new(note))),
        &mut fx,
    );
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Recap(
            RecapIntent::Command {
                source: Some(ThreadId::new("cloud-1")),
            },
        ))),
    );
    assert!(thread_asks(&fx).is_empty(), "{:?}", fx.commands());
    assert_eq!(notes(&app), vec![RecapNote::Cloud]);
}

#[test]
fn a_recap_asks_one_structured_turn_and_says_the_answer() {
    let mut app = app(&[("fix the bug", "done"), ("ship it", "shipped")]);
    let fx = recap(&mut app);
    assert!(asked(&fx), "{:?}", fx.commands());
    let requests = walk(&mut app, &fx, json!({}));
    let Some(ThreadAsk::StructuredTurn {
        thread,
        kind,
        prompt,
        schema,
        effort,
    }) = requests.last()
    else {
        panic!("expected the turn of the run, got {requests:?}");
    };
    assert_eq!((thread.as_str(), *kind, effort), (TEMP, Kind::Recap, &None));
    assert!(
        prompt.ends_with(
            "User: fix the bug\n\nAssistant: done\n\nUser: ship it\n\nAssistant: shipped"
        )
    );
    assert_eq!(schema["required"], json!(["summary", "next_action"]));
    // a second one while it generates says so and asks nothing
    assert!(thread_asks(&recap(&mut app)).is_empty());
    says(
        &mut app,
        r#"{"summary": " Shipped the fix. ", "next_action": " "}"#,
    );
    let fx = ends(&mut app, "completed");
    assert_eq!(unsubscribes(&fx), [TEMP], "the temporary thread is let go");
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
    assert!(asked(&recap(&mut app)));
}

#[test]
fn a_recap_says_the_next_action_when_there_is_one() {
    let mut app = app(&[("a", "b")]);
    let fx = recap(&mut app);
    answer(
        &mut app,
        &fx,
        r#"{"summary": "Fixed.", "next_action": "Run the tests."}"#,
    );
    assert_eq!(
        notes(&app).last(),
        Some(&RecapNote::Recap {
            summary: "Fixed.".to_owned(),
            next_action: Some("Run the tests.".to_owned())
        })
    );
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
        let fx = recap(&mut app);
        answer(&mut app, &fx, response);
        assert_eq!(notes(&app), vec![RecapNote::Failed], "{response}");
    }
}

#[test]
fn a_request_the_server_refuses_is_the_failure_line() {
    let mut app = app(&[("a", "b")]);
    let fx = recap(&mut app);
    let (ticket, _) = the_ask(&fx);
    crate::structured_turn::refuse(&mut app, ticket, "no server");
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
    // the request is over: another can be made
    assert!(asked(&recap(&mut app)));
}

#[test]
fn an_answer_for_a_conversation_that_moved_on_is_dropped() {
    let mut app = app(&[("a", "b")]);
    let fx = recap(&mut app);
    walk(&mut app, &fx, json!({}));
    stream(
        &mut app,
        json!({"method": "item/completed",
            "params": {"threadId": T, "turnId": "turn-2", "completedAtMs": 0,
            "startedAtMs": 0, "item": {"type": "agentMessage", "id": "late", "text": "more",
            "phase": null}}}),
    );
    says(&mut app, r#"{"summary": "x", "next_action": null}"#);
    let fx = ends(&mut app, "completed");
    assert_eq!(
        unsubscribes(&fx),
        [TEMP],
        "the temporary thread is let go all the same"
    );
    assert_eq!(notes(&app), vec![]);
}

#[test]
fn a_conversation_that_moved_on_before_the_turn_starts_gets_no_model_call() {
    use codex_tui_core::screens::agents;

    let mut app = app(&[("a", "b")]);
    let fx = recap(&mut app);
    let (ticket, _) = the_ask(&fx);
    let fx = crate::structured_turn::answer(
        &mut app,
        ticket,
        agents::Reply::McpServers(Default::default()),
    );
    let (ticket, _) = the_ask(&fx);
    // the conversation moves while the temporary thread starts
    stream(
        &mut app,
        json!({"method": "item/completed",
            "params": {"threadId": T, "turnId": "turn-2", "completedAtMs": 0,
            "startedAtMs": 0, "item": {"type": "agentMessage", "id": "late", "text": "more",
            "phase": null}}}),
    );
    let fx = crate::structured_turn::answer(
        &mut app,
        ticket,
        agents::Reply::Temporary {
            thread: TEMP.to_owned(),
            as_asked: true,
        },
    );
    assert_eq!(
        thread_asks(&fx)
            .into_iter()
            .map(|(_, ask)| ask)
            .filter(|ask| matches!(ask, ThreadAsk::StructuredTurn { .. }))
            .count(),
        0,
        "no turn is started for an answer that would be dropped"
    );
    assert_eq!(unsubscribes(&fx), [TEMP], "the temporary thread is let go");
    assert_eq!(notes(&app), vec![], "and nothing is said");
    // the request is over: another can be made
    assert!(asked(&recap(&mut app)));
}
