//! State tests: `apply_app` and `threads::apply` on the plain model, no
//! terminal, no backend (ADR-003: the model is testable without the UI).

use std::rc::Rc;

use codex_tui_rtwk::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, Intent, PendingRequest, Reply, Session, apply_app,
};
use codex_tui_rtwk::ids::{
    ConnEpoch, ItemId, RequestKey, ServerRequestId, ThreadId, Ticket, TurnId,
};
use codex_tui_rtwk::shell::Backend;
use codex_tui_rtwk::shell::fake::{
    agent_completed, agent_delta, tool_input_request, turn_completed, turn_started, user_item,
};
use codex_tui_rtwk::threads::{self, Item, ItemKey, TurnEnd, TurnState};

const T: &str = "thread-1";

fn app() -> CodexApp {
    CodexApp::new(Backend::None)
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn notification(app: &mut CodexApp, ev: BackendEv) -> AppFx {
    run(app, AppEv::Backend(ev))
}

/// The app with its thread started, as after `Connected` and the reply.
fn ready() -> CodexApp {
    let mut app = app();
    run(&mut app, AppEv::Connected);
    run(
        &mut app,
        AppEv::Reply(
            Ticket(1),
            Reply::ThreadStarted {
                thread: ThreadId::new(T),
            },
        ),
    );
    app
}

fn thread(app: &CodexApp) -> Rc<codex_tui_rtwk::threads::ThreadModel> {
    app.active_thread()
}

fn key(item: &str) -> ItemKey {
    ItemKey {
        turn: TurnId::new("turn-1"),
        item: ItemId::new(item),
    }
}

#[test]
fn connected_starts_the_first_thread() {
    let mut app = app();
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(
        fx.commands,
        vec![Command::StartThread { ticket: Ticket(1) }]
    );
    assert_eq!(app.session.status, Session::Starting);
}

#[test]
fn thread_started_reply_makes_the_session_ready() {
    let app = ready();
    assert_eq!(app.session.status, Session::Ready);
    assert_eq!(app.session.active, Some(ThreadId::new(T)));
}

#[test]
fn a_submit_before_ready_waits_and_goes_out_when_the_thread_starts() {
    let mut app = app();
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: None,
            text: "early".into(),
        }),
    );
    assert!(fx.commands.is_empty());
    run(&mut app, AppEv::Connected);
    let fx = run(
        &mut app,
        AppEv::Reply(
            Ticket(1),
            Reply::ThreadStarted {
                thread: ThreadId::new(T),
            },
        ),
    );
    assert_eq!(
        fx.commands,
        vec![Command::StartTurn {
            ticket: Ticket(2),
            thread: ThreadId::new(T),
            text: "early".into(),
        }]
    );
}

#[test]
fn a_second_submit_waits_for_the_turn_to_be_accepted() {
    let mut app = ready();
    let submit = |text: &str| {
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            text: text.into(),
        })
    };
    assert_eq!(run(&mut app, submit("one")).commands.len(), 1);
    assert!(run(&mut app, submit("two")).commands.is_empty());
}

#[test]
fn a_reply_with_an_unknown_ticket_is_dropped() {
    let mut app = ready();
    let before = thread(&app);
    run(
        &mut app,
        AppEv::Reply(
            Ticket(99),
            Reply::TurnStarted {
                thread: ThreadId::new(T),
                turn: TurnId::new("turn-1"),
            },
        ),
    );
    assert_eq!(*thread(&app), *before);
}

#[test]
fn turn_reply_and_notification_in_either_order_give_one_running_turn() {
    for reply_first in [true, false] {
        let mut app = ready();
        run(
            &mut app,
            AppEv::Intent(Intent::Submit {
                thread: Some(ThreadId::new(T)),
                text: "hi".into(),
            }),
        );
        let reply = AppEv::Reply(
            Ticket(2),
            Reply::TurnStarted {
                thread: ThreadId::new(T),
                turn: TurnId::new("turn-1"),
            },
        );
        let started = AppEv::Backend(turn_started(T, "turn-1"));
        if reply_first {
            run(&mut app, reply);
            run(&mut app, started);
        } else {
            run(&mut app, started);
            run(&mut app, reply);
        }
        assert!(thread(&app).is_running(), "reply_first={reply_first}");
        assert!(thread(&app).outgoing.is_empty());
    }
}

#[test]
fn a_late_turn_started_does_not_revive_an_ended_turn() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(thread(&app).turn, TurnState::Idle);
    notification(&mut app, turn_started(T, "turn-1"));
    assert_eq!(thread(&app).turn, TurnState::Idle);
}

#[test]
fn a_delta_before_its_item_creates_the_entry() {
    let mut app = ready();
    notification(&mut app, agent_delta(T, "turn-1", "msg-1", "Hel"));
    notification(&mut app, agent_delta(T, "turn-1", "msg-1", "lo"));
    let transcript = thread(&app).transcript.clone();
    assert_eq!(
        transcript.get(0).map(|(_, item)| item.clone()),
        Some(Item::Agent {
            text: "Hello".into()
        })
    );
}

#[test]
fn the_completed_item_replaces_the_streamed_text_in_place() {
    let mut app = ready();
    notification(&mut app, user_item(T, "turn-1", "user-1", "hi"));
    notification(&mut app, agent_delta(T, "turn-1", "msg-1", "Hel"));
    notification(&mut app, agent_completed(T, "turn-1", "msg-1", "Hello"));
    let model = thread(&app);
    let items: Vec<_> = model
        .transcript
        .iter()
        .map(|(_, item)| item.clone())
        .collect();
    assert_eq!(
        items,
        vec![
            Item::User { text: "hi".into() },
            Item::Agent {
                text: "Hello".into()
            },
        ]
    );
}

#[test]
fn replace_turn_takes_the_snapshot_wholesale() {
    let mut app = ready();
    notification(&mut app, agent_delta(T, "turn-1", "msg-1", "stale"));
    notification(&mut app, agent_delta(T, "turn-1", "msg-2", "gone"));
    let mut transcript = thread(&app).transcript.clone();
    transcript.replace_turn(
        &TurnId::new("turn-1"),
        vec![(
            ItemId::new("msg-1"),
            Item::Agent {
                text: "fresh".into(),
            },
        )],
    );
    assert_eq!(transcript.len(), 1);
    assert_eq!(
        transcript.get(0).map(|(k, i)| (k.clone(), i.clone())),
        Some((
            key("msg-1"),
            Item::Agent {
                text: "fresh".into()
            }
        ))
    );
}

#[test]
fn a_server_request_is_queued_once_and_rejected_once() {
    let mut app = ready();
    let request = || AppEv::Backend(tool_input_request(T, "turn-1", 7));
    run(&mut app, request());
    run(&mut app, request());
    assert_eq!(app.requests.entries.len(), 1);
    let PendingRequest { key, method } = app.requests.entries[0].clone();
    assert_eq!(method, "item/tool/requestUserInput");
    assert_eq!(
        key,
        RequestKey {
            epoch: ConnEpoch(0),
            thread: ThreadId::new(T),
            id: ServerRequestId::new("7"),
        }
    );
    let fx = run(&mut app, AppEv::Intent(Intent::DismissRequest(key.clone())));
    assert_eq!(
        fx.commands,
        vec![Command::RejectRequest { key: key.clone() }]
    );
    assert!(app.requests.entries.is_empty());
    let fx = run(&mut app, AppEv::Intent(Intent::DismissRequest(key)));
    assert!(fx.commands.is_empty());
}

#[test]
fn the_projection_maps_what_the_skeleton_shows() {
    let BackendEv::Notification(n) = turn_completed(T, "turn-3") else {
        panic!("a notification");
    };
    let (thread, ev) = threads::project(&n).expect("projected");
    assert_eq!(thread, ThreadId::new(T));
    assert_eq!(
        ev,
        threads::ThreadEv::TurnEnded {
            turn: TurnId::new("turn-3"),
            end: TurnEnd::Completed,
        }
    );
}
