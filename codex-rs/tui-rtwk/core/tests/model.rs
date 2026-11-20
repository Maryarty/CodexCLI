//! State tests: `apply_app` and `threads::apply` on the plain model, no
//! terminal, no backend (ADR-003: the model is testable without the UI).
//! Tickets cannot be forged: a test takes them from the commands the model
//! emitted, as the shell does.

use std::rc::Rc;

use codex_tui_core::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, Intent, PendingRequest, Session, apply_app,
};
use codex_tui_core::fixtures::builders::{
    agent_completed, agent_delta, server_request_resolved, tool_input_request, turn_completed,
    turn_error, turn_failed, turn_started, user_item,
};
use codex_tui_core::ids::{
    ConnEpoch, ItemId, RequestKey, ServerRequestId, ThreadId, Ticket, TurnId,
};
use codex_tui_core::rpc::Reply;
use codex_tui_core::threads::{self, Failure, Item, ItemKey, TurnEnd, TurnState};

const T: &str = "thread-1";

fn app() -> CodexApp {
    CodexApp::new()
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

fn thread(app: &CodexApp) -> Rc<codex_tui_core::threads::ThreadModel> {
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
    assert_eq!(app.session().status, Session::Starting);
}

#[test]
fn thread_started_reply_makes_the_session_ready() {
    let app = ready();
    assert_eq!(app.session().status, Session::Ready);
    assert_eq!(app.session().active, Some(ThreadId::new(T)));
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
    let first = thread(&app).last_submit();
    let fx = submit(&mut app, "two");
    assert!(fx.commands().is_empty());
    assert_eq!(thread(&app).queue(), vec!["two".to_string()]);
    assert_ne!(thread(&app).last_submit(), first, "the intent was accepted");
}

#[test]
fn a_submit_while_a_turn_runs_is_queued_and_accepted() {
    let mut app = ready();
    let first = thread(&app).last_submit();
    notification(&mut app, turn_started(T, "turn-1"));
    let fx = submit(&mut app, "two");
    assert!(fx.commands().is_empty());
    assert_eq!(thread(&app).queue(), vec!["two".to_string()]);
    assert_ne!(thread(&app).last_submit(), first);
}

#[test]
fn queued_messages_go_out_in_order_one_per_turn_end() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    submit(&mut app, "two");
    submit(&mut app, "three");
    assert_eq!(
        thread(&app).queue(),
        vec!["two".to_string(), "three".to_string()]
    );
    let fx = notification(&mut app, turn_completed(T, "turn-1"));
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "one per turn end");
    assert_eq!(turns[0].1, "two");
    assert_eq!(thread(&app).queue(), vec!["three".to_string()]);
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
    assert!(thread(&app).queue().is_empty());
}

#[test]
fn a_queue_survives_a_second_submit_and_keeps_its_order() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    submit(&mut app, "two");
    submit(&mut app, "three");
    submit(&mut app, "four");
    assert_eq!(
        thread(&app).queue(),
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
    assert_eq!(
        thread(&app).last_error().cloned(),
        Some(Failure::Message("refused".into()))
    );
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].1, "two");
}

/// After a `Lagged` gap the end of a turn can be seen before the reply that
/// starts it: the reply is then what finds the turn already over.
#[test]
fn a_queued_message_goes_out_when_the_turn_end_was_seen_before_its_start_reply() {
    let mut app = ready();
    let fx = submit(&mut app, "one");
    let (ticket, _) = start_turns(&fx).remove(0);
    submit(&mut app, "two");
    let fx = notification(&mut app, turn_completed(T, "turn-1"));
    assert!(start_turns(&fx).is_empty(), "the start is still pending");
    let fx = run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::turn_started(ThreadId::new(T), TurnId::new("turn-1")),
        ),
    );
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "the queue must not stall");
    assert_eq!(turns[0].1, "two");
    assert!(thread(&app).queue().is_empty());
    assert!(!thread(&app).is_running(), "the ended turn is not revived");
}

#[test]
fn a_later_submit_does_not_overtake_older_queued_text() {
    let mut app = ready();
    let fx = submit(&mut app, "one");
    let (ticket, _) = start_turns(&fx).remove(0);
    submit(&mut app, "two");
    notification(&mut app, turn_completed(T, "turn-1"));
    let fx = run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::turn_started(ThreadId::new(T), TurnId::new("turn-1")),
        ),
    );
    assert_eq!(start_turns(&fx)[0].1, "two");
    // "two" is starting: "three" waits behind it.
    let fx = submit(&mut app, "three");
    assert!(start_turns(&fx).is_empty());
    assert_eq!(thread(&app).queue(), vec!["three".to_string()]);
}

#[test]
fn a_lagged_gap_is_a_failure_the_view_words() {
    let mut app = ready();
    notification(&mut app, BackendEv::Lagged(3));
    assert_eq!(thread(&app).last_error().cloned(), Some(Failure::Lagged(3)));
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
        assert!(thread(&app).outgoing().is_empty());
    }
}

#[test]
fn a_late_turn_started_does_not_revive_an_ended_turn() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(thread(&app).turn(), &TurnState::Idle);
    notification(&mut app, turn_started(T, "turn-1"));
    assert_eq!(thread(&app).turn(), &TurnState::Idle);
}

#[test]
fn a_delta_before_its_item_creates_the_entry() {
    let mut app = ready();
    notification(&mut app, agent_delta(T, "turn-1", "msg-1", "Hel"));
    notification(&mut app, agent_delta(T, "turn-1", "msg-1", "lo"));
    let transcript = thread(&app).transcript().clone();
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
        .transcript()
        .iter()
        .map(|(_, item)| item.clone())
        .collect();
    assert_eq!(
        items,
        vec![
            Item::User(codex_tui_core::threads::UserPrompt {
                text: "hi".into(),
                ..Default::default()
            }),
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
    let mut transcript = thread(&app).transcript().clone();
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
    assert_eq!(app.requests().entries.len(), 1);
    let PendingRequest { key, method, .. } = app.requests().entries[0].clone();
    assert_eq!(method, "item/tool/requestUserInput");
    assert_eq!(
        key,
        RequestKey {
            backend: Default::default(),
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
    assert!(app.requests().entries.is_empty());
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
        threads::ThreadEv::TurnCompleted {
            timing: None,
            turn: TurnId::new("turn-3"),
            end: TurnEnd::Completed,
            snapshot: None,
            error: None,
        }
    );
}

#[test]
fn a_resolved_server_request_closes_its_dialog_without_a_reject() {
    let mut app = ready();
    run(&mut app, AppEv::Backend(tool_input_request(T, "turn-1", 7)));
    assert_eq!(app.requests().entries.len(), 1);
    let fx = notification(&mut app, server_request_resolved(T, 7));
    assert!(app.requests().entries.is_empty());
    assert!(fx.commands().is_empty(), "nothing is rejected: it is gone");
}

#[test]
fn a_second_request_waits_behind_the_first() {
    let mut app = ready();
    run(&mut app, AppEv::Backend(tool_input_request(T, "turn-1", 7)));
    run(&mut app, AppEv::Backend(tool_input_request(T, "turn-1", 8)));
    assert_eq!(app.requests().entries.len(), 2);
    assert_eq!(
        app.requests().head().map(|r| r.key.id.clone()),
        Some(ServerRequestId::new("7"))
    );
    notification(&mut app, server_request_resolved(T, 7));
    assert_eq!(
        app.requests().head().map(|r| r.key.id.clone()),
        Some(ServerRequestId::new("8"))
    );
}

#[test]
fn request_keys_preserve_numeric_and_escaped_string_ids() {
    use codex_app_server_protocol::{
        RequestId, ServerNotification, ServerRequest, ServerRequestResolvedNotification,
    };
    use codex_tui_core::protocol::reject_target;

    let mut app = ready();
    let string_id = RequestId::String("7\"\n".to_owned());
    let BackendEv::Request(mut request) = tool_input_request(T, "turn-1", 7) else {
        panic!("expected a request");
    };
    let ServerRequest::ToolRequestUserInput { request_id, .. } = request.as_mut() else {
        panic!("expected a user-input request");
    };
    *request_id = string_id.clone();
    notification(&mut app, BackendEv::Request(request));
    notification(&mut app, tool_input_request(T, "turn-1", 7));
    assert_eq!(app.requests().entries.len(), 2);
    assert_eq!(
        app.requests()
            .head()
            .and_then(|req| reject_target(&req.key)),
        Some(string_id.clone())
    );
    notification(&mut app, server_request_resolved(T, 7));
    assert_eq!(app.requests().entries.len(), 1);
    let resolved = ServerNotification::ServerRequestResolved(ServerRequestResolvedNotification {
        thread_id: T.to_owned(),
        request_id: string_id,
    });
    notification(&mut app, BackendEv::Notification(Box::new(resolved)));
    assert!(app.requests().entries.is_empty());
}

/// A failed turn says why: the server's `error` notification carries the
/// message, which the status line shows instead of a bare "the turn failed".
#[test]
fn a_failed_turn_keeps_the_servers_error_message() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_error(T, "turn-1", "no network", false));
    notification(&mut app, turn_failed(T, "turn-1"));
    assert_eq!(
        thread(&app).last_error().cloned(),
        Some(Failure::Message("no network".into()))
    );
    assert_eq!(thread(&app).turn(), &TurnState::Idle);
}

#[test]
fn a_failed_turn_without_a_message_says_so_plainly() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_failed(T, "turn-1"));
    assert_eq!(
        thread(&app).last_error().cloned(),
        Some(Failure::TurnFailed)
    );
}

#[test]
fn an_error_that_will_be_retried_is_not_shown() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_error(T, "turn-1", "reconnecting", true));
    assert_eq!(thread(&app).last_error().cloned(), None);
}

/// The oracle ends with the error when the server does not start
/// (`startup_orchestration.rs`: the daemon's `start_with_features_classified`
/// error, `run_main_inner` returning it): the model fails the session and
/// says the app ends; a started connection does not.
#[test]
fn a_start_failure_ends_the_app() {
    let mut app = app();
    let fx = run(&mut app, AppEv::StartFailed("no daemon package".into()));
    assert!(fx.ends_app());
    assert_eq!(
        app.session().status,
        Session::Failed(Failure::Message("no daemon package".into()))
    );
    let mut app = self::app();
    assert!(!run(&mut app, AppEv::Connected).ends_app());
}

#[test]
fn interrupt_waits_for_the_turn_id_and_is_sent_once() {
    for notification_first in [false, true] {
        let mut app = ready();
        let start = start_turns(&submit(&mut app, "first"))[0].0;
        let interrupt = || AppEv::Intent(Intent::Interrupt(ThreadId::new(T)));
        assert!(run(&mut app, interrupt()).commands().is_empty());
        let started = || {
            AppEv::Reply(
                start,
                Reply::turn_started(ThreadId::new(T), TurnId::new("turn-1")),
            )
        };
        let fx = if notification_first {
            notification(&mut app, turn_started(T, "turn-1"))
        } else {
            run(&mut app, started())
        };
        let [
            Command::InterruptTurn {
                ticket,
                thread: target,
                turn,
                ..
            },
        ] = fx.commands()
        else {
            panic!("expected one interrupt, got {:?}", fx.commands());
        };
        assert_eq!(target, &ThreadId::new(T));
        assert_eq!(turn, &TurnId::new("turn-1"));
        let ticket = *ticket;

        if notification_first {
            assert!(run(&mut app, started()).commands().is_empty());
        } else {
            assert!(
                notification(&mut app, turn_started(T, "turn-1"))
                    .commands()
                    .is_empty()
            );
        }
        notification(&mut app, agent_delta(T, "turn-1", "msg-1", "partial"));
        assert!(run(&mut app, interrupt()).commands().is_empty());
        assert!(start_turns(&submit(&mut app, "queued")).is_empty());
        let fx = notification(&mut app, turn_completed(T, "turn-1"));
        assert_eq!(start_turns(&fx)[0].1, "queued");
        assert!(thread(&app).last_error().is_none());
        run(
            &mut app,
            AppEv::Reply(ticket, Reply::failed("late interrupt")),
        );
        assert!(thread(&app).last_error().is_none());
        assert_eq!(
            thread(&app)
                .transcript()
                .committed_agent_text(&key("msg-1")),
            Some("partial")
        );
    }
}

#[test]
fn ended_before_start_reply_skips_interrupt_and_sends_one_queued_turn() {
    let mut app = ready();
    let start = start_turns(&submit(&mut app, "first"))[0].0;
    run(&mut app, AppEv::Intent(Intent::Interrupt(ThreadId::new(T))));
    submit(&mut app, "queued");
    assert!(
        notification(&mut app, turn_completed(T, "turn-1"))
            .commands()
            .is_empty()
    );
    let fx = run(
        &mut app,
        AppEv::Reply(
            start,
            Reply::turn_started(ThreadId::new(T), TurnId::new("turn-1")),
        ),
    );
    assert!(matches!(fx.commands(), [Command::StartTurn { text, .. }] if text == "queued"));
}

#[test]
fn streaming_commits_complete_utf8_lines_and_keeps_a_preview() {
    let mut app = ready();
    let key = key("msg-1");
    for (delta, stable) in [
        ("перв", ""),
        ("ая\nвто", "первая\n"),
        ("рая\nтретья", "первая\nвторая\n"),
    ] {
        notification(&mut app, agent_delta(T, "turn-1", "msg-1", delta));
        assert!(thread(&app).transcript().is_agent_streaming(&key));
        assert_eq!(
            thread(&app).transcript().committed_agent_text(&key),
            Some(stable)
        );
    }
    notification(
        &mut app,
        agent_completed(T, "turn-1", "msg-1", "первая\nвторая\nтретья"),
    );
    assert!(!thread(&app).transcript().is_agent_streaming(&key));
    assert_eq!(
        thread(&app).transcript().committed_agent_text(&key),
        Some("первая\nвторая\nтретья")
    );
}

#[test]
fn rate_limit_notification_supersedes_the_outstanding_account_read() {
    use codex_tui_core::ids::BackendId;
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::screens::status::{Intent as StatusIntent, RateLimits};

    let mut app = ready();
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Status(StatusIntent::Open))),
    );
    let [
        Command::ReadRateLimits {
            ticket, backend, ..
        },
    ] = fx.commands()
    else {
        panic!(
            "expected the status card's account read: {:?}",
            fx.commands()
        );
    };
    assert_eq!(*backend, BackendId::default());
    let update = serde_json::from_value(serde_json::json!({
        "method": "account/rateLimits/updated",
        "params": {"rateLimits": {"primary": {"usedPercent": 30, "windowDurationMins": 300, "resetsAt": 2000000000}}}
    })).expect("rate limits notification");
    notification(&mut app, BackendEv::Notification(Box::new(update)));
    run(
        &mut app,
        AppEv::Reply(*ticket, Reply::rate_limits(RateLimits::default())),
    );

    let card = app.screens().status.card.as_ref().expect("status card");
    assert_eq!(card.limits[0].used_percent, 30);
    assert_eq!(
        app.session().rate_limits[backend]
            .primary
            .expect("primary")
            .resets_at,
        Some(2000000000)
    );
}

#[test]
fn status_reads_token_usage_from_its_thread_slot() {
    use codex_tui_core::screens::{ScreenIntent, status};

    let mut app = ready();
    let usage = serde_json::json!({
        "totalTokens": 15000,
        "inputTokens": 14000,
        "cachedInputTokens": 500,
        "outputTokens": 1000,
        "reasoningOutputTokens": 250
    });
    let update = serde_json::from_value(serde_json::json!({
        "method": "thread/tokenUsage/updated",
        "params": {
            "threadId": T, "turnId": "turn-1",
            "tokenUsage": {"total": usage, "last": usage, "modelContextWindow": 22000}
        }
    }))
    .expect("token usage notification");
    notification(&mut app, BackendEv::Notification(Box::new(update)));
    run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Status(status::Intent::Open))),
    );
    let card = app.screens().status.card.as_ref().expect("status card");
    assert_eq!(card.tokens.blended_total(), 14500);
    assert_eq!(card.context.expect("context").percent_left, 70);
}

#[test]
fn interrupt_retries_once_when_the_server_names_a_different_active_turn() {
    use codex_tui_core::protocol::reply_from_server;
    use codex_tui_core::rpc::ServerError;

    let mut app = ready();
    notification(&mut app, turn_started(T, "old"));
    let fx = run(&mut app, AppEv::Intent(Intent::Interrupt(ThreadId::new(T))));
    let refusal = |expected, actual| {
        Err(ServerError {
            code: Some(-32602),
            message: format!("expected active turn id {expected} but found {actual}"),
        })
    };
    let (ticket, reply) =
        reply_from_server(&fx.commands()[0], refusal("old", "actual")).expect("reply");
    let retry = run(&mut app, AppEv::Reply(ticket, reply));
    assert!(
        matches!(retry.commands(), [Command::InterruptTurn { turn, .. }] if turn.as_str() == "actual")
    );
    let (ticket, reply) =
        reply_from_server(&retry.commands()[0], refusal("actual", "later")).expect("reply");
    assert!(
        run(&mut app, AppEv::Reply(ticket, reply))
            .commands()
            .is_empty()
    );
    assert!(thread(&app).last_error().is_some());
}
