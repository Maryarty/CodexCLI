//! ADR-003 conformance tests: one `ThreadModel` per thread, one writer, a
//! screen asks `OpenThread(id)` and the app decides. The mandatory tests of
//! the ADR: R2 switch round-trip, R4 resume mid-turn, R5 end-path matrix, R7
//! `assert_consistent` after every event, R8 lifecycle x intent, R9/R10
//! permutations; and the one open-rest skipped: thread A keeps receiving
//! events while B is shown.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, Session, apply_app};
use codex_tui_core::fixtures::builders::{
    agent_completed, agent_delta, turn_completed, turn_failed, turn_started, user_item,
};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::agents::{Ev, Page, Reply as AgentsReply};
use codex_tui_core::screens::{ScreenEv, ScreenReply};
use codex_tui_core::threads::{Item, ThreadAsk, TurnState};
use serde_json::json;

const A: &str = "thread-1";
const B: &str = "00000000-0000-0000-0000-000000000002";

fn thread(id: &str) -> Thread {
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn send(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn note(app: &mut CodexApp, ev: BackendEv) {
    send(app, AppEv::Backend(ev));
}

/// A is the chat's thread (seeded); the center lists A and B.
fn app_with_a_and_b() -> CodexApp {
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    let mut app = build(&format!("{script}\n")).expect("plays").app;
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
            threads: vec![thread(A), thread(B)],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    app
}

fn open(app: &mut CodexApp, id: &str) -> AppFx {
    send(app, AppEv::Intent(Intent::OpenThread(ThreadId::new(id))))
}

/// The resume `fx` asks for, if any: its ticket.
fn resume_ticket(fx: &AppFx, id: &str) -> Option<codex_tui_core::ids::Ticket> {
    fx.commands().iter().find_map(|command| match command {
        Command::Screen {
            ticket,
            command:
                codex_tui_core::screens::ScreenCommand::Agents(
                    codex_tui_core::screens::agents::Command::Thread(ThreadAsk::Resume { thread }),
                ),
            ..
        } if thread == id => Some(*ticket),
        _ => None,
    })
}

fn answer_resume(app: &mut CodexApp, ticket: codex_tui_core::ids::Ticket, id: &str) {
    let reply = Reply::screen(ScreenReply::Agents(AgentsReply::Resumed(Box::new(thread(
        id,
    )))));
    send(app, AppEv::Reply(ticket, reply));
}

fn transcript_len(app: &CodexApp, id: &str) -> usize {
    app.thread(&ThreadId::new(id)).unwrap().transcript().len()
}

/// R2: opening B resumes it once; A -> B -> A -> B asks the backend for no
/// second resume (the server has one writer per thread), and each thread
/// keeps its own model.
#[test]
fn open_thread_resumes_once_then_switches_without_a_request() {
    let mut app = app_with_a_and_b();
    assert_eq!(app.session().active, Some(ThreadId::new(A)));
    let fx = open(&mut app, B);
    let ticket = resume_ticket(&fx, B).expect("B is resumed");
    answer_resume(&mut app, ticket, B);
    assert_eq!(app.session().active, Some(ThreadId::new(B)));
    for next in [A, B, A, B] {
        let fx = open(&mut app, next);
        assert!(fx.commands().is_empty(), "{next}: {:?}", fx.commands());
        assert_eq!(app.session().active, Some(ThreadId::new(next)));
        assert_eq!(app.session().status, Session::Ready);
    }
}

/// open-rest's missing test: A keeps receiving events while B is shown, and
/// when A is shown again its transcript holds them.
#[test]
fn a_thread_keeps_receiving_events_while_another_is_shown() {
    let mut app = app_with_a_and_b();
    let ticket = resume_ticket(&open(&mut app, B), B).unwrap();
    answer_resume(&mut app, ticket, B);
    note(&mut app, turn_started(A, "a-turn"));
    note(&mut app, user_item(A, "a-turn", "a-user", "hello"));
    note(&mut app, agent_delta(A, "a-turn", "a-reply", "wor"));
    note(&mut app, agent_delta(A, "a-turn", "a-reply", "king"));
    assert_eq!(app.session().active, Some(ThreadId::new(B)));
    assert!(
        app.active_thread().transcript().is_empty(),
        "B is untouched"
    );
    assert!(!app.active_thread().is_running());
    assert_eq!(transcript_len(&app, A), 2);
    note(&mut app, turn_completed(A, "a-turn"));
    open(&mut app, A);
    let shown = app.active_thread();
    assert!(!shown.is_running());
    assert_eq!(shown.transcript().len(), 2);
}

/// R4: a thread resumed while its turn runs: the running turn's live events
/// upsert into one transcript, never push twice.
#[test]
fn resume_mid_turn_then_live_events_upsert_once() {
    let mut app = app_with_a_and_b();
    let ticket = resume_ticket(&open(&mut app, B), B).unwrap();
    answer_resume(&mut app, ticket, B);
    note(&mut app, turn_started(B, "b-turn"));
    note(&mut app, agent_delta(B, "b-turn", "b-reply", "a"));
    note(&mut app, agent_completed(B, "b-turn", "b-reply", "ab"));
    // The same item arrives again (a snapshot after a reconnect).
    note(&mut app, agent_completed(B, "b-turn", "b-reply", "ab"));
    assert_eq!(transcript_len(&app, B), 1);
    assert!(app.active_thread().is_running());
}

/// R5: every way a turn ends leaves it idle, and the item stays.
#[test]
fn every_turn_end_path_leaves_the_thread_idle() {
    type End = fn(&str, &str) -> BackendEv;
    let ends: [(&str, End); 2] = [("completed", turn_completed), ("failed", turn_failed)];
    for (name, end) in ends {
        let mut app = app_with_a_and_b();
        note(&mut app, turn_started(A, "t"));
        note(&mut app, agent_delta(A, "t", "i", "x"));
        assert!(app.active_thread().is_running(), "{name}");
        note(&mut app, end(A, "t"));
        assert_eq!(app.active_thread().turn(), &TurnState::Idle, "{name}");
        assert_eq!(transcript_len(&app, A), 1, "{name}");
        // A late start of the ended turn must not revive it.
        note(&mut app, turn_started(A, "t"));
        assert_eq!(
            app.active_thread().turn(),
            &TurnState::Idle,
            "{name} late start"
        );
    }
}

/// R8: every lifecycle state of the session takes every app intent without
/// breaking an invariant.
#[test]
fn every_intent_in_every_lifecycle_state_keeps_the_model_consistent() {
    let intents: [fn() -> Intent; 5] = [
        || Intent::Submit {
            thread: None,
            text: "hi".into(),
        },
        || Intent::Submit {
            thread: Some(ThreadId::new("nowhere")),
            text: "hi".into(),
        },
        || Intent::CycleBackend,
        || Intent::OpenThread(ThreadId::new(B)),
        || Intent::OpenThread(ThreadId::new("unknown")),
    ];
    let starts: [fn() -> CodexApp; 3] = [
        CodexApp::new,
        || {
            let mut app = CodexApp::new();
            send(&mut app, AppEv::StartFailed("no server".into()));
            app
        },
        app_with_a_and_b,
    ];
    for start in starts {
        for intent in intents {
            let mut app = start();
            send(&mut app, AppEv::Intent(intent()));
            send(&mut app, AppEv::Disconnected);
        }
    }
}

/// R9/R10: the events of two threads in any interleaving leave each thread
/// with what its own events say.
#[test]
fn any_interleaving_of_two_threads_events_gives_each_its_own_transcript() {
    let a = [
        turn_started(A, "ta"),
        agent_delta(A, "ta", "ia", "one"),
        turn_completed(A, "ta"),
    ];
    let b = [
        turn_started(B, "tb"),
        agent_delta(B, "tb", "ib", "two"),
        turn_completed(B, "tb"),
    ];
    // Every merge of the two ordered sequences (6 choose 3 = 20).
    for mask in 0u32..64 {
        if mask.count_ones() != 3 {
            continue;
        }
        let (mut ia, mut ib) = (0, 0);
        let mut app = app_with_a_and_b();
        let ticket = resume_ticket(&open(&mut app, B), B).unwrap();
        answer_resume(&mut app, ticket, B);
        for step in 0..6 {
            let from_a = mask >> step & 1 == 1;
            let ev = if from_a {
                ia += 1;
                a[ia - 1].clone()
            } else {
                ib += 1;
                b[ib - 1].clone()
            };
            note(&mut app, ev);
        }
        let texts = |id: &str| -> Vec<String> {
            app.thread(&ThreadId::new(id))
                .unwrap()
                .transcript()
                .iter()
                .map(|(_, item)| match item {
                    Item::Agent { text, .. } => text.clone(),
                    other => format!("{other:?}"),
                })
                .collect()
        };
        assert_eq!(texts(A), vec!["one".to_owned()], "mask {mask:b}");
        assert_eq!(texts(B), vec!["two".to_owned()], "mask {mask:b}");
        assert!(!app.thread(&ThreadId::new(A)).unwrap().is_running());
        assert!(!app.thread(&ThreadId::new(B)).unwrap().is_running());
    }
}
