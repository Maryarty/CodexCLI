//! State tests: `apply_app` and `threads::apply` on the plain model, no
//! terminal, no backend (ADR-003: the model is testable without the UI).
//! Tickets cannot be forged: a test takes them from the commands the model
//! emitted, as the shell does.

use std::rc::Rc;

use codex_tui_rtwk::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, Intent, PendingRequest, Session, apply_app,
};
use codex_tui_rtwk::ids::{
    ConnEpoch, ItemId, RequestKey, ServerRequestId, ThreadId, Ticket, TurnId,
};
use codex_tui_rtwk::rpc::Reply;
use codex_tui_rtwk::shell::Backend;
use codex_tui_rtwk::shell::fake::{
    agent_completed, agent_delta, server_request_resolved, tool_input_request, turn_completed,
    turn_started, user_item,
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

/// The ticket of the one `StartThread` in `fx`.
fn start_thread_ticket(fx: &AppFx) -> Ticket {
    let found: Vec<Ticket> = fx
        .commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "one StartThread in {:?}", fx.commands());
    found[0]
}

/// The `(ticket, text)` of every `StartTurn` in `fx` for thread `T`.
fn start_turns(fx: &AppFx) -> Vec<(Ticket, String)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn {
                ticket,
                thread,
                text,
                ..
            } => {
                assert_eq!(*thread, ThreadId::new(T));
                Some((*ticket, text.clone()))
            }
            _ => None,
        })
        .collect()
}

/// The app with its thread started, as after `Connected` and the reply.
fn ready() -> CodexApp {
    let mut app = app();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = start_thread_ticket(&fx);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
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

fn submit(app: &mut CodexApp, text: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            text: text.into(),
        }),
    )
}

#[test]
fn connected_starts_the_first_thread() {
    let mut app = app();
    let fx = run(&mut app, AppEv::Connected);
    assert_eq!(fx.commands().len(), 1);
    start_thread_ticket(&fx);
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
    assert!(fx.commands().is_empty());
    let fx = run(&mut app, AppEv::Connected);
    let ticket = start_thread_ticket(&fx);
    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].1, "early");
}

#[test]
fn a_second_submit_while_the_turn_starts_is_queued_not_refused() {
    let mut app = ready();
    assert_eq!(start_turns(&submit(&mut app, "one")).len(), 1);
    let first = thread(&app).last_submit;
    let fx = submit(&mut app, "two");
    assert!(fx.commands().is_empty());
    assert_eq!(thread(&app).queue, vec!["two".to_string()]);
    assert_ne!(thread(&app).last_submit, first, "the intent was accepted");
}

#[test]
fn a_submit_while_a_turn_runs_is_queued_and_accepted() {
    let mut app = ready();
    let first = thread(&app).last_submit;
    notification(&mut app, turn_started(T, "turn-1"));
    let fx = submit(&mut app, "two");
    assert!(fx.commands().is_empty());
    assert_eq!(thread(&app).queue, vec!["two".to_string()]);
    assert_ne!(thread(&app).last_submit, first);
}

#[test]
fn queued_messages_go_out_in_order_one_per_turn_end() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    submit(&mut app, "two");
    submit(&mut app, "three");
    assert_eq!(
        thread(&app).queue,
        vec!["two".to_string(), "three".to_string()]
    );
    let fx = notification(&mut app, turn_completed(T, "turn-1"));
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "one per turn end");
    assert_eq!(turns[0].1, "two");
    assert_eq!(thread(&app).queue, vec!["three".to_string()]);
    // The turn for "two" is accepted and runs; its end sends "three".
    run(
        &mut app,
        AppEv::Reply(
            turns[0].0,
            Reply::turn_started(ThreadId::new(T), TurnId::new("turn-2")),
        ),
    );
    assert!(thread(&app).is_running());
    let fx = notification(&mut app, turn_completed(T, "turn-2"));
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].1, "three");
    assert!(thread(&app).queue.is_empty());
}

#[test]
fn a_queue_survives_a_second_submit_and_keeps_its_order() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    submit(&mut app, "two");
    submit(&mut app, "three");
    submit(&mut app, "four");
    assert_eq!(
        thread(&app).queue,
        ["two", "three", "four"].map(String::from).to_vec()
    );
}

#[test]
fn a_turn_that_fails_to_start_still_sends_the_next_queued_message() {
    let mut app = ready();
    let fx = submit(&mut app, "one");
    let (ticket, _) = start_turns(&fx).remove(0);
    submit(&mut app, "two");
    let fx = run(&mut app, AppEv::Reply(ticket, Reply::failed("refused")));
    assert_eq!(thread(&app).last_error.as_deref(), Some("refused"));
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].1, "two");
}

#[test]
fn a_reply_with_a_spent_ticket_is_dropped() {
    let mut app = ready();
    let fx = run(&mut app, AppEv::Connected);
    let spent = start_thread_ticket(&fx);
    run(
        &mut app,
        AppEv::Reply(spent, Reply::thread_started(ThreadId::new("other"))),
    );
    let before = thread(&app);
    run(
        &mut app,
        AppEv::Reply(
            spent,
            Reply::turn_started(ThreadId::new(T), TurnId::new("turn-1")),
        ),
    );
    assert_eq!(*thread(&app), *before);
}

#[test]
fn turn_reply_and_notification_in_either_order_give_one_running_turn() {
    for reply_first in [true, false] {
        let mut app = ready();
        let fx = submit(&mut app, "hi");
        let (ticket, _) = start_turns(&fx).remove(0);
        let reply = AppEv::Reply(
            ticket,
            Reply::turn_started(ThreadId::new(T), TurnId::new("turn-1")),
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
    assert!(
        matches!(fx.commands(), [Command::RejectRequest { key: k, .. }] if *k == key),
        "{:?}",
        fx.commands()
    );
    assert!(app.requests.entries.is_empty());
    let fx = run(&mut app, AppEv::Intent(Intent::DismissRequest(key)));
    assert!(fx.commands().is_empty());
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

#[test]
fn a_resolved_server_request_closes_its_dialog_without_a_reject() {
    let mut app = ready();
    run(&mut app, AppEv::Backend(tool_input_request(T, "turn-1", 7)));
    assert_eq!(app.requests.entries.len(), 1);
    let fx = notification(&mut app, server_request_resolved(T, 7));
    assert!(app.requests.entries.is_empty());
    assert!(fx.commands().is_empty(), "nothing is rejected: it is gone");
}

#[test]
fn a_second_request_waits_behind_the_first() {
    let mut app = ready();
    run(&mut app, AppEv::Backend(tool_input_request(T, "turn-1", 7)));
    run(&mut app, AppEv::Backend(tool_input_request(T, "turn-1", 8)));
    assert_eq!(app.requests.entries.len(), 2);
    assert_eq!(
        app.requests.head().map(|r| r.key.id.clone()),
        Some(ServerRequestId::new("7"))
    );
    notification(&mut app, server_request_resolved(T, 7));
    assert_eq!(
        app.requests.head().map(|r| r.key.id.clone()),
        Some(ServerRequestId::new("8"))
    );
}
