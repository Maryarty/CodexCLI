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
    turn_error, turn_failed, turn_started, user_item, user_item_with_receipt,
};
use codex_tui_core::ids::{
    ConnEpoch, ItemId, RequestKey, ServerRequestId, ThreadId, Ticket, TurnId,
};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::chat::{
    McpHeader, McpStartup, McpStatus, McpWarning, Retry, StatusHeader,
};
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, ScreenReply, ScreenThreadIntent, chat};
use codex_tui_core::threads::UserMessage;
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

/// The texts waiting in the queue of thread `T`.
fn queued(app: &CodexApp) -> Vec<String> {
    thread(app).queue().iter().map(|m| m.text.clone()).collect()
}

/// The `(ticket, text)` of every `StartTurn` in `fx` for thread `T`.
fn start_turns(fx: &AppFx) -> Vec<(Ticket, String)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn {
                ticket,
                thread,
                message,
                ..
            } => {
                assert_eq!(*thread, ThreadId::new(T));
                Some((*ticket, message.text.clone()))
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

/// The usage menu's refresh of the account's limits: asks a read.
fn usage_read(app: &mut CodexApp) -> AppFx {
    use codex_tui_core::screens::usage::Intent as UsageIntent;
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Usage(
            UsageIntent::RefreshRateLimits,
        ))),
    )
}

/// `/status` typed in the shown thread: the session command the chat sends.
fn status_command(app: &mut CodexApp) -> AppFx {
    use codex_tui_core::commands::{SessionIntent, SlashCommand};
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Status,
            intent: SessionIntent::Status,
        }))),
    )
}

/// The newest status card of the shown thread's transcript.
#[allow(clippy::expect_used)]
fn status_card(app: &CodexApp) -> std::sync::Arc<codex_tui_core::screens::status::StatusCard> {
    thread(app)
        .transcript()
        .last_status()
        .expect("a status card in the transcript")
        .clone()
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
            message: UserMessage::text(text),
        }),
    )
}

/// An intent of the chat slot that reaches the thread (`Intent::ThreadScreen`).
fn chat_intent(app: &mut CodexApp, intent: chat::ThreadIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: ScreenThreadIntent::Chat(intent),
        }),
    )
}

/// Tab: the message waits behind the running turn instead of steering it.
fn tab(app: &mut CodexApp, text: &str) -> AppFx {
    chat_intent(
        app,
        chat::ThreadIntent::Queue(UserMessage::text(text).into()),
    )
}

/// The `(ticket, turn, text)` of every `turn/steer` in `fx`.
fn steers(fx: &AppFx) -> Vec<(Ticket, String, String)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                ticket,
                command: ScreenCommand::Chat(chat::Command::Steer { turn, message, .. }),
                ..
            } => Some((*ticket, turn.to_string(), message.text.clone())),
            _ => None,
        })
        .collect()
}

fn steer_receipt(fx: &AppFx) -> String {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Steer { client_id, .. }),
                ..
            } => Some(client_id.clone()),
            _ => None,
        })
        .unwrap()
}

/// The texts of the steers the server has not yet echoed.
fn pending_steers(app: &CodexApp) -> Vec<String> {
    thread(app)
        .pending_steers()
        .iter()
        .map(|steer| steer.message().text.clone())
        .collect()
}

fn rejected_steers(app: &CodexApp) -> Vec<String> {
    thread(app)
        .rejected_steers()
        .iter()
        .map(|message| message.text.clone())
        .collect()
}

fn steered(turn: &str) -> Reply {
    Reply::screen(ScreenReply::Chat(chat::Reply::Steered(TurnId::new(turn))))
}

#[test]
fn connected_starts_the_first_thread() {
    let mut app = app();
    let fx = run(&mut app, AppEv::Connected);
    // The thread start, the startup page's launch facts (startup-flow), the
    // model catalog the /model page and its shortcuts read, and the
    // collaboration modes `/plan` and Shift+Tab choose from.
    assert_eq!(fx.commands().len(), 4);
    assert!(
        fx.commands()
            .iter()
            .any(|c| matches!(c, Command::Startup { .. })),
        "{:?}",
        fx.commands()
    );
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
            message: UserMessage::text("early"),
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

/// A `!cmd` before the first thread is the same as a message: kept in order
/// with them (the oracle queues it until the session is configured) and run
/// when the thread starts.
#[test]
fn a_shell_command_before_ready_waits_in_order_and_runs_when_the_thread_starts() {
    let mut app = app();
    let shell = |command: &str| {
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Shell {
            thread: None,
            command: command.to_string(),
        })))
    };
    let early = |text: &str| {
        AppEv::Intent(Intent::Submit {
            thread: None,
            message: UserMessage::text(text),
        })
    };
    assert!(run(&mut app, shell("ls -la")).commands().is_empty());
    run(&mut app, early("then"));
    assert!(run(&mut app, shell("pwd")).commands().is_empty());
    let fx = run(&mut app, AppEv::Connected);
    let ticket = start_thread_ticket(&fx);
    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    let sent: Vec<String> = fx
        .commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Shell(command)),
                ..
            } => Some(format!("!{command}")),
            Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(sent, ["!ls -la", "then", "!pwd"], "{:?}", fx.commands());
}

#[test]
fn a_second_submit_while_the_turn_starts_is_queued_not_refused() {
    let mut app = ready();
    assert_eq!(start_turns(&submit(&mut app, "one")).len(), 1);
    let first = thread(&app).last_submit();
    let fx = submit(&mut app, "two");
    assert!(fx.commands().is_empty());
    assert_eq!(queued(&app), vec!["two".to_string()]);
    assert_ne!(thread(&app).last_submit(), first, "the intent was accepted");
}

/// Enter during a running turn steers it (`turn/steer`); the message waits
/// as a pending steer until the server echoes it, and is not in the queue.
#[test]
fn enter_while_a_turn_runs_steers_it() {
    let mut app = ready();
    let first = thread(&app).last_submit();
    notification(&mut app, turn_started(T, "turn-1"));
    let fx = submit(&mut app, "two");
    let sent = steers(&fx);
    assert_eq!(sent.len(), 1, "{:?}", fx.commands());
    assert_eq!((sent[0].1.as_str(), sent[0].2.as_str()), ("turn-1", "two"));
    assert!(queued(&app).is_empty());
    assert_eq!(pending_steers(&app), vec!["two".to_string()]);
    assert_ne!(thread(&app).last_submit(), first);
}

/// Tab during a running turn queues the message behind it.
#[test]
fn tab_while_a_turn_runs_queues_the_message() {
    let mut app = ready();
    let first = thread(&app).last_submit();
    notification(&mut app, turn_started(T, "turn-1"));
    let fx = tab(&mut app, "two");
    assert!(fx.commands().is_empty());
    assert_eq!(queued(&app), vec!["two".to_string()]);
    assert!(pending_steers(&app).is_empty());
    assert_ne!(thread(&app).last_submit(), first);
}

#[test]
fn queued_messages_go_out_in_order_one_per_turn_end() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    tab(&mut app, "two");
    tab(&mut app, "three");
    assert_eq!(queued(&app), vec!["two".to_string(), "three".to_string()]);
    let fx = notification(&mut app, turn_completed(T, "turn-1"));
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "one per turn end");
    assert_eq!(turns[0].1, "two");
    assert_eq!(queued(&app), vec!["three".to_string()]);
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
    assert!(queued(&app).is_empty());
}

#[test]
fn a_queue_survives_a_second_submit_and_keeps_its_order() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    tab(&mut app, "two");
    tab(&mut app, "three");
    tab(&mut app, "four");
    assert_eq!(
        queued(&app),
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
    assert!(queued(&app).is_empty());
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
    assert_eq!(queued(&app), vec!["three".to_string()]);
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
            text: "Hello".into(),
            phase: None,
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
                text: "Hello".into(),
                phase: None,
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
                phase: None,
            },
        )],
    );
    assert_eq!(transcript.len(), 1);
    assert_eq!(
        transcript.get(0).map(|(k, i)| (k.clone(), i.clone())),
        Some((
            key("msg-1"),
            Item::Agent {
                text: "fresh".into(),
                phase: None,
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
/// message, which the transcript keeps as an error row (oracle `on_error`).
#[test]
fn a_failed_turn_keeps_the_servers_error_message() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_error(T, "turn-1", "no network", false));
    notification(&mut app, turn_failed(T, "turn-1"));
    assert_eq!(
        thread(&app)
            .transcript()
            .iter()
            .filter_map(|(_, item)| match item {
                Item::Error { message } => Some(message.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        ["no network"]
    );
    assert_eq!(thread(&app).last_error().cloned(), None);
    assert_eq!(thread(&app).turn(), &TurnState::Idle);
}

/// Oracle `on_server_overloaded_error`: an overloaded server that said
/// nothing is a typed high-load row (the UI words it), once per turn.
#[test]
fn an_overloaded_server_that_said_nothing_is_a_high_load_row() {
    use codex_app_server_protocol::{
        CodexErrorInfo, ErrorNotification, ServerNotification, TurnError,
    };
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(
        &mut app,
        BackendEv::Notification(Box::new(ServerNotification::Error(ErrorNotification {
            thread_id: T.to_string(),
            turn_id: "turn-1".to_string(),
            will_retry: false,
            error: TurnError {
                message: String::new(),
                codex_error_info: Some(CodexErrorInfo::ServerOverloaded),
                additional_details: None,
                misalignment: None,
            },
        }))),
    );
    notification(&mut app, turn_failed(T, "turn-1"));
    let rows = thread(&app)
        .transcript()
        .iter()
        .filter(|(_, item)| matches!(item, Item::HighLoad | Item::Error { .. }))
        .map(|(_, item)| matches!(item, Item::HighLoad))
        .collect::<Vec<_>>();
    assert_eq!(rows, [true]);
}

/// Oracle `on_error`: a non-retried error notification ends the turn (finalize, the
/// error row, the next queued input); the failed turn's completion after it is a
/// no-op: no second error row, the queue is not sent twice.
#[test]
fn a_turn_error_ends_the_turn_and_sends_the_next_queued_message_once() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    tab(&mut app, "two");
    tab(&mut app, "three");
    let fx = notification(&mut app, turn_error(T, "turn-1", "no network", false));
    assert_eq!(thread(&app).turn(), &TurnState::Idle);
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "{:?}", fx.commands());
    assert_eq!(turns[0].1, "two");
    assert_eq!(queued(&app), vec!["three".to_string()]);
    let fx = notification(&mut app, turn_failed(T, "turn-1"));
    assert!(start_turns(&fx).is_empty(), "{:?}", fx.commands());
    assert_eq!(queued(&app), vec!["three".to_string()]);
    let errors = thread(&app)
        .transcript()
        .iter()
        .filter(|(_, item)| matches!(item, Item::Error { .. }))
        .count();
    assert_eq!(errors, 1);
}

/// The oracle shows nothing for a failed turn that has no error (`finalize_turn` only).
#[test]
fn a_failed_turn_without_a_message_shows_nothing() {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_failed(T, "turn-1"));
    assert_eq!(thread(&app).last_error().cloned(), None);
    assert!(thread(&app).transcript().is_empty());
    assert_eq!(thread(&app).turn(), &TurnState::Idle);
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
        assert!(start_turns(&tab(&mut app, "queued")).is_empty());
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
    assert!(
        matches!(fx.commands(), [Command::StartTurn { message, .. }] if message.text == "queued")
    );
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
fn rate_limit_notification_supersedes_the_outstanding_startup_read() {
    use codex_tui_core::ids::BackendId;
    use codex_tui_core::screens::status::RateLimits;

    let mut app = ready();
    // The first facts of an account that has limits start the startup read.
    let fx = run(
        &mut app,
        AppEv::Screen(codex_tui_core::screens::ScreenEv::Chat(chat::Ev::Status(
            Box::new(chat::StatusFacts {
                account_usage: true,
                ..chat::StatusFacts::default()
            }),
        ))),
    );
    let [
        Command::ReadRateLimits {
            ticket, backend, ..
        },
    ] = fx.commands()
    else {
        panic!("expected the startup account read: {:?}", fx.commands());
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

    assert_eq!(
        app.session().rate_limits[backend]
            .primary
            .expect("primary")
            .used_percent,
        30
    );
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
    status_command(&mut app);
    let card = status_card(&app);
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
            data: None,
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

/// `turn/completed` of an interrupted turn (the fixtures' builders have none).
fn turn_interrupted(thread: &str, turn_id: &str) -> BackendEv {
    use codex_app_server_protocol::{
        ServerNotification, Turn, TurnCompletedNotification, TurnItemsView, TurnStatus,
    };
    BackendEv::Notification(Box::new(ServerNotification::TurnCompleted(
        TurnCompletedNotification {
            thread_id: thread.to_string(),
            turn: Turn {
                id: turn_id.to_string(),
                root_turn_id: None,
                items: Vec::new(),
                items_view: TurnItemsView::NotLoaded,
                status: TurnStatus::Interrupted,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    )))
}

/// A running thread with `turn-1`.
fn running() -> CodexApp {
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    app
}

fn restored(app: &CodexApp) -> Vec<String> {
    thread(app)
        .restore()
        .map(|restore| restore.messages().iter().map(|m| m.text.clone()).collect())
        .unwrap_or_default()
}

/// The server echoes the steer as the user's item: the pending steer is done,
/// oldest first, and the echo is the transcript's, never a second copy.
#[test]
fn the_echo_of_a_steer_clears_its_pending_entry_in_order() {
    let mut app = running();
    let first_fx = submit(&mut app, "one");
    let second_fx = submit(&mut app, "two");
    let first = steers(&first_fx);
    let second = steers(&second_fx);
    assert_eq!(first.len() + second.len(), 2, "a double Enter sends both");
    assert_eq!(
        pending_steers(&app),
        ["one", "two"].map(String::from).to_vec()
    );
    run(&mut app, AppEv::Reply(first[0].0, steered("turn-1")));
    notification(&mut app, user_item(T, "turn-1", "other-client", "one"));
    assert_eq!(pending_steers(&app), ["one", "two"]);
    notification(
        &mut app,
        user_item_with_receipt(T, "turn-1", "item-1", "one", Some(steer_receipt(&first_fx))),
    );
    assert_eq!(pending_steers(&app), ["two"]);
    notification(
        &mut app,
        user_item_with_receipt(
            T,
            "turn-1",
            "item-2",
            "two",
            Some(steer_receipt(&second_fx)),
        ),
    );
    assert!(pending_steers(&app).is_empty());
    assert!(queued(&app).is_empty());
}

/// A reply that comes after the echo, or after the turn ended, changes nothing.
#[test]
fn a_late_steer_reply_is_harmless() {
    let mut app = running();
    let sent_fx = submit(&mut app, "one");
    let sent = steers(&sent_fx);
    notification(
        &mut app,
        user_item_with_receipt(T, "turn-1", "item-1", "one", Some(steer_receipt(&sent_fx))),
    );
    assert!(pending_steers(&app).is_empty());
    let fx = run(&mut app, AppEv::Reply(sent[0].0, steered("turn-1")));
    assert!(fx.commands().is_empty());
    assert!(pending_steers(&app).is_empty());
    assert!(thread(&app).last_error().is_none());
}

/// The server refuses a steer of a review or compact turn: the message goes
/// to the rejected queue and is sent first, as one message, when the turn ends.
#[test]
fn a_refused_steer_is_sent_when_the_turn_ends() {
    let mut app = running();
    let one = steers(&submit(&mut app, "one"));
    let two = steers(&submit(&mut app, "two"));
    tab(&mut app, "later");
    run(
        &mut app,
        AppEv::Reply(one[0].0, Reply::failed("cannot steer a review turn")),
    );
    run(
        &mut app,
        AppEv::Reply(two[0].0, Reply::failed("cannot steer a review turn")),
    );
    assert!(pending_steers(&app).is_empty());
    assert_eq!(
        rejected_steers(&app),
        ["one", "two"].map(String::from).to_vec()
    );
    assert!(thread(&app).last_error().is_none(), "not an error");
    let fx = notification(&mut app, turn_completed(T, "turn-1"));
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "{:?}", fx.commands());
    assert_eq!(turns[0].1, "one\ntwo");
    assert_eq!(queued(&app), vec!["later".to_string()]);
    assert!(rejected_steers(&app).is_empty());
}

/// "no active turn to steer": the turn is over, the message starts a turn.
#[test]
fn a_steer_with_no_active_turn_starts_a_turn() {
    let mut app = running();
    let sent = steers(&submit(&mut app, "one"));
    let fx = run(
        &mut app,
        AppEv::Reply(sent[0].0, Reply::failed("no active turn to steer")),
    );
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "{:?}", fx.commands());
    assert_eq!(turns[0].1, "one");
    assert!(pending_steers(&app).is_empty());
    assert!(restored(&app).is_empty());
}

/// A stale turn id: the steer goes again to the turn the server names, once.
#[test]
fn a_steer_retries_once_with_the_turn_the_server_names() {
    let mut app = running();
    let sent = steers(&submit(&mut app, "one"));
    let fx = run(
        &mut app,
        AppEv::Reply(
            sent[0].0,
            Reply::failed("expected active turn id `turn-1` but found `turn-2`"),
        ),
    );
    let again = steers(&fx);
    assert_eq!(again.len(), 1, "{:?}", fx.commands());
    assert_eq!(
        (again[0].1.as_str(), again[0].2.as_str()),
        ("turn-2", "one")
    );
    assert_eq!(pending_steers(&app), vec!["one".to_string()]);
    let fx = run(
        &mut app,
        AppEv::Reply(
            again[0].0,
            Reply::failed("expected active turn id `turn-2` but found `turn-3`"),
        ),
    );
    assert!(steers(&fx).is_empty(), "retried once only");
    assert!(pending_steers(&app).is_empty());
    assert!(thread(&app).last_error().is_some());
    assert_eq!(restored(&app), vec!["one".to_string()]);
}

/// Any other failure puts the message back in the composer with the error.
#[test]
fn a_failed_steer_goes_back_to_the_composer() {
    let mut app = running();
    let sent = steers(&submit(&mut app, "one"));
    run(&mut app, AppEv::Reply(sent[0].0, Reply::failed("boom")));
    assert_eq!(
        thread(&app).last_error().cloned(),
        Some(Failure::Message("boom".into()))
    );
    assert!(pending_steers(&app).is_empty());
    assert_eq!(restored(&app), vec!["one".to_string()]);
}

/// The interrupted turn gives the queue back to the composer instead of
/// sending it (`on_interrupted_turn`).
#[test]
fn an_interrupted_turn_restores_the_queue_to_the_composer() {
    let mut app = running();
    tab(&mut app, "a");
    tab(&mut app, "b");
    let fx = notification(&mut app, turn_interrupted(T, "turn-1"));
    assert!(start_turns(&fx).is_empty(), "{:?}", fx.commands());
    assert!(queued(&app).is_empty());
    assert_eq!(restored(&app), ["a", "b"].map(String::from).to_vec());
    assert_eq!(thread(&app).turn(), &TurnState::Idle);
}

/// Pending steers come back too: rejected first, then pending, then queued.
#[test]
fn an_interrupted_turn_restores_rejected_pending_and_queued_in_order() {
    let mut app = running();
    let one = steers(&submit(&mut app, "rejected"));
    run(
        &mut app,
        AppEv::Reply(one[0].0, Reply::failed("cannot steer a review turn")),
    );
    submit(&mut app, "pending");
    tab(&mut app, "queued");
    let fx = notification(&mut app, turn_interrupted(T, "turn-1"));
    assert!(start_turns(&fx).is_empty());
    assert_eq!(
        restored(&app),
        ["rejected", "pending", "queued"].map(String::from).to_vec()
    );
    assert!(pending_steers(&app).is_empty() && rejected_steers(&app).is_empty());
}

/// Esc with pending steers interrupts and then sends them as one message.
#[test]
fn esc_with_pending_steers_interrupts_and_submits_them_merged() {
    let mut app = running();
    submit(&mut app, "one");
    submit(&mut app, "two");
    tab(&mut app, "queued");
    let fx = chat_intent(&mut app, chat::ThreadIntent::InterruptForSteers);
    assert!(
        matches!(fx.commands(), [Command::InterruptTurn { .. }]),
        "{:?}",
        fx.commands()
    );
    let fx = notification(&mut app, turn_interrupted(T, "turn-1"));
    let turns = start_turns(&fx);
    assert_eq!(turns.len(), 1, "{:?}", fx.commands());
    assert_eq!(turns[0].1, "one\ntwo");
    assert!(pending_steers(&app).is_empty());
    assert_eq!(queued(&app), vec!["queued".to_string()], "the queue waits");
    assert!(restored(&app).is_empty());
}

/// With no steer pending the key is the plain Esc: nothing is sent.
#[test]
fn the_steer_interrupt_without_steers_does_not_interrupt() {
    let mut app = running();
    let fx = chat_intent(&mut app, chat::ThreadIntent::InterruptForSteers);
    assert!(fx.commands().is_empty());
}

/// The edit-queued key takes the newest queued message back into the composer.
#[test]
fn edit_queued_takes_the_newest_message_back() {
    let mut app = running();
    tab(&mut app, "a");
    tab(&mut app, "b");
    chat_intent(&mut app, chat::ThreadIntent::EditQueued);
    assert_eq!(queued(&app), vec!["a".to_string()]);
    assert_eq!(restored(&app), vec!["b".to_string()]);
    assert!(
        thread(&app)
            .restore()
            .is_some_and(codex_tui_core::threads::Restore::replaces)
    );
}

/// While a turn is only starting (no turn id yet) Enter waits in the queue.
#[test]
fn enter_while_the_turn_is_starting_queues() {
    let mut app = ready();
    start_turns(&submit(&mut app, "one"));
    let fx = submit(&mut app, "two");
    assert!(steers(&fx).is_empty());
    assert_eq!(queued(&app), vec!["two".to_string()]);
}

/// Restored images are renumbered in order so the merged placeholders agree.
#[test]
fn restored_messages_merge_with_renumbered_image_placeholders() {
    use codex_tui_core::threads::LocalImage;
    let image = |start, end| LocalImage {
        path: "/tmp/a.png".into(),
        range: start..end,
    };
    let mut app = running();
    let with_image = UserMessage {
        text: "[Image #1] first".into(),
        images: vec![image(0, 10)],
        remote_images: Vec::new(),
        mentions: Vec::new(),
        tasks: Box::default(),
    };
    chat_intent(
        &mut app,
        chat::ThreadIntent::Queue(with_image.clone().into()),
    );
    chat_intent(&mut app, chat::ThreadIntent::Queue(with_image.into()));
    notification(&mut app, turn_interrupted(T, "turn-1"));
    let restore = thread(&app).restore().cloned().expect("restore");
    let merged = restore.merged();
    assert_eq!(merged.text, "[Image #1] first\n[Image #2] first");
    assert_eq!(merged.images.len(), 2);
    assert_eq!(merged.images[1].range, 17..27);
}

// ---- MCP startup (oracle chatwidget/mcp_startup.rs), the retry header and the
// high-load error

fn mcp(name: &str, status: &str, error: Option<&str>) -> BackendEv {
    use codex_app_server_protocol::{
        McpServerStartupState, McpServerStatusUpdatedNotification, ServerNotification,
    };
    BackendEv::Notification(Box::new(ServerNotification::McpServerStatusUpdated(
        McpServerStatusUpdatedNotification {
            thread_id: Some(T.to_string()),
            name: name.to_string(),
            status: match status {
                "starting" => McpServerStartupState::Starting,
                "ready" => McpServerStartupState::Ready,
                "cancelled" => McpServerStartupState::Cancelled,
                _ => McpServerStartupState::Failed,
            },
            error: error.map(str::to_string),
            failure_reason: None,
        },
    )))
}

fn startup_warnings(app: &CodexApp) -> Vec<McpWarning> {
    thread(app)
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::StartupWarning(warning) => Some(warning.clone()),
            _ => None,
        })
        .collect()
}

fn header(app: &CodexApp) -> Option<StatusHeader> {
    thread(app).screen_slots().chat.header()
}

fn servers(names: &[&str]) -> Vec<String> {
    names.iter().map(ToString::to_string).collect()
}

fn failed(server: &str, error: &str) -> McpWarning {
    McpWarning::Failed {
        server: server.to_string(),
        error: Some(error.to_string()),
    }
}

fn starting(completed: usize, total: usize, names: &[&str]) -> Option<StatusHeader> {
    Some(StatusHeader::Mcp(McpHeader::Starting {
        completed,
        total,
        servers: servers(names),
        more: false,
    }))
}

#[test]
fn mcp_startup_header_names_the_servers_still_starting() {
    let mut app = ready();
    notification(&mut app, mcp("beta", "starting", None));
    assert_eq!(
        header(&app),
        Some(StatusHeader::Mcp(McpHeader::Booting {
            server: "beta".to_string()
        }))
    );
    notification(&mut app, mcp("alpha", "starting", None));
    assert_eq!(header(&app), starting(0, 2, &["alpha", "beta"]));
    notification(&mut app, mcp("alpha", "ready", None));
    assert_eq!(header(&app), starting(1, 2, &["beta"]));
    notification(&mut app, mcp("beta", "ready", None));
    assert_eq!(header(&app), None);
    assert!(startup_warnings(&app).is_empty());
}

#[test]
fn mcp_startup_header_cuts_a_long_list_after_three_servers() {
    let mut app = ready();
    for name in ["a", "b", "c", "d"] {
        notification(&mut app, mcp(name, "starting", None));
    }
    assert_eq!(
        header(&app),
        Some(StatusHeader::Mcp(McpHeader::Starting {
            completed: 0,
            total: 4,
            servers: servers(&["a", "b", "c"]),
            more: true,
        }))
    );
}

#[test]
fn mcp_startup_reports_a_failure_once_and_the_round_incomplete() {
    let mut app = ready();
    for ev in [
        mcp("beta", "starting", None),
        mcp("alpha", "starting", None),
        mcp("alpha", "failed", Some("handshake failed")),
        mcp("alpha", "failed", Some("handshake failed")),
    ] {
        notification(&mut app, ev);
    }
    assert_eq!(
        startup_warnings(&app),
        vec![failed("alpha", "handshake failed")]
    );
    notification(&mut app, mcp("beta", "ready", None));
    assert_eq!(
        startup_warnings(&app),
        vec![
            failed("alpha", "handshake failed"),
            McpWarning::Incomplete {
                servers: servers(&["alpha"])
            }
        ]
    );
}

#[test]
fn mcp_cancelled_servers_are_summarised_before_failed_ones() {
    let mut app = ready();
    for ev in [
        mcp("a", "starting", None),
        mcp("b", "starting", None),
        mcp("a", "cancelled", None),
        mcp("b", "ready", None),
    ] {
        notification(&mut app, ev);
    }
    assert_eq!(
        startup_warnings(&app),
        vec![McpWarning::Interrupted {
            servers: servers(&["a"])
        }]
    );
}

#[test]
fn an_outcome_without_a_start_is_reported_at_once() {
    let mut app = ready();
    notification(&mut app, mcp("alpha", "failed", Some("late")));
    assert_eq!(
        startup_warnings(&app),
        vec![
            failed("alpha", "late"),
            McpWarning::Incomplete {
                servers: servers(&["alpha"])
            }
        ]
    );
    assert_eq!(header(&app), None);
}

#[test]
fn a_message_sent_while_mcp_servers_start_waits_and_goes_out_when_they_settle() {
    let mut app = ready();
    notification(&mut app, mcp("slow", "starting", None));
    let fx = submit(&mut app, "hello");
    assert!(start_turns(&fx).is_empty(), "{:?}", fx.commands());
    assert_eq!(queued(&app), vec!["hello".to_string()]);
    let fx = notification(&mut app, mcp("slow", "ready", None));
    assert_eq!(
        start_turns(&fx)
            .into_iter()
            .map(|(_, text)| text)
            .collect::<Vec<_>>(),
        vec!["hello".to_string()]
    );
}

#[test]
fn a_lag_settles_the_round_and_forgets_the_reported_failures() {
    let broke = || McpStatus::Failed {
        error: Some("handshake failed".to_string()),
    };
    let mut round = McpStartup::default();
    round.update("beta".to_string(), McpStatus::Starting);
    round.update("alpha".to_string(), McpStatus::Starting);
    assert_eq!(
        round.update("alpha".to_string(), broke()),
        vec![failed("alpha", "handshake failed")]
    );
    // the lag settles the round: the summary
    assert_eq!(
        round.finish_after_lag(),
        vec![McpWarning::Incomplete {
            servers: servers(&["alpha"])
        }]
    );
    assert!(!round.is_starting());
    // a failure the lag made us forget is a new one
    assert_eq!(round.update("alpha".to_string(), broke()).len(), 2);
}

#[test]
fn a_server_that_failed_without_words_reports_itself_by_name() {
    let mut round = McpStartup::default();
    assert_eq!(
        round.update("alpha".to_string(), McpStatus::Failed { error: None })[0],
        McpWarning::Failed {
            server: "alpha".to_string(),
            error: None
        }
    );
}

#[test]
fn a_retrying_error_sets_the_header_until_the_next_notification() {
    use codex_app_server_protocol::{
        CodexErrorInfo, ErrorNotification, ServerNotification, TurnError,
    };
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(
        &mut app,
        BackendEv::Notification(Box::new(ServerNotification::Error(ErrorNotification {
            thread_id: T.to_string(),
            turn_id: "turn-1".to_string(),
            will_retry: true,
            error: TurnError {
                message: "Reconnecting... 2/5".to_string(),
                codex_error_info: Some(CodexErrorInfo::ResponseStreamDisconnected {
                    http_status_code: Some(429),
                }),
                additional_details: None,
                misalignment: None,
            },
        }))),
    );
    assert_eq!(
        header(&app),
        Some(StatusHeader::Retry(Retry::Busy {
            attempt: "2/5".to_string()
        }))
    );
    notification(&mut app, agent_delta(T, "turn-1", "msg", "hello"));
    assert_eq!(header(&app), None);
}

/// The edit of a past prompt reverted the server's history (`thread/revert`):
/// the thread drops every row it held and reads the retained turns again
/// (oracle: the thread's replay store is replaced).
#[test]
fn a_reverted_thread_clears_its_transcript_and_reads_the_retained_turns() {
    use codex_app_server_protocol::{ServerNotification, ThreadRevertedNotification};
    use codex_tui_core::screens::agents::Command as AgentsCommand;
    use codex_tui_core::threads::ThreadAsk;

    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, user_item(T, "turn-1", "u1", "first"));
    notification(&mut app, agent_completed(T, "turn-1", "a1", "answer"));
    notification(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(thread(&app).transcript().len(), 2);
    let revision = thread(&app).transcript().layout_revision();

    let fx = notification(
        &mut app,
        BackendEv::Notification(Box::new(ServerNotification::ThreadReverted(
            ThreadRevertedNotification {
                thread_id: T.to_string(),
            },
        ))),
    );

    assert_eq!(thread(&app).transcript().len(), 0, "the old rows are gone");
    assert_ne!(thread(&app).transcript().layout_revision(), revision);
    assert!(
        fx.commands().iter().any(|c| matches!(
            c,
            Command::Screen {
                command: ScreenCommand::Agents(AgentsCommand::Thread(ThreadAsk::History {
                    thread,
                    cursor: None,
                })),
                ..
            } if thread == T
        )),
        "the retained turns are read again: {:?}",
        fx.commands()
    );
}

/// The banner's numbered rows (oracle `handle_inline_banner_key`): a row whose action is a
/// page asks the host to open it; a stale ticket or a row past the last does nothing, and
/// choosing leaves the banner where it is (`dismiss_on_select: false`).
#[test]
fn choosing_a_banner_row_opens_its_page_and_keeps_the_banner() {
    use codex_tui_core::ids::{BackendId, Ticket};
    use codex_tui_core::protocol;
    let mut app = ready();
    let fx = usage_read(&mut app);
    let [read @ Command::ReadRateLimits { backend, .. }] = fx.commands() else {
        panic!("expected the account read: {:?}", fx.commands());
    };
    let backend: BackendId = *backend;
    let (read_ticket, reply) = protocol::reply(
        read,
        Ok(serde_json::json!({
            "rateLimits": {},
            "rateLimitUpsell": {
                "banner_type": "credits",
                "title": "Usage limit",
                "description": "Add credits.",
                "ctas": [{"action": "view_usage", "label": "Usage"}],
                "presentation": "dismissible",
            },
        })),
    )
    .expect("a reply");
    run(&mut app, AppEv::Reply(read_ticket, reply));
    let shown: Ticket = app.session().backend_banners[&backend].ticket;
    let pick = |app: &mut CodexApp, ticket: Ticket, row: usize| {
        run(
            app,
            AppEv::Intent(Intent::PickBannerCta {
                backend,
                ticket,
                row,
            }),
        )
    };
    let fx = pick(&mut app, shown, 0);
    assert!(
        matches!(
            fx.commands(),
            [Command::OpenUrl { url, .. }] if url == "https://chatgpt.com/settings/usage"
        ),
        "{:?}",
        fx.commands()
    );
    assert!(app.session().backend_banners[&backend].is_visible(None));
    assert!(pick(&mut app, shown, 1).commands().is_empty());
    // the read's own ticket is not the banner occurrence's
    assert!(pick(&mut app, read_ticket, 0).commands().is_empty());
}

/// The account read answered with `banner` as its upsell (a `Status` open asks
/// for it); returns the backend the banner belongs to.
fn read_banner(app: &mut CodexApp, banner: serde_json::Value) -> codex_tui_core::ids::BackendId {
    use codex_tui_core::protocol;

    let fx = usage_read(app);
    let [read @ Command::ReadRateLimits { backend, .. }] = fx.commands() else {
        panic!("expected the account read: {:?}", fx.commands());
    };
    let backend = *backend;
    let Some((ticket, reply)) = protocol::reply(
        read,
        Ok(
            serde_json::json!({"accountId": "workspace-a", "rateLimits": {}, "rateLimitUpsell": banner}),
        ),
    ) else {
        panic!("the read has a reply");
    };
    run(app, AppEv::Reply(ticket, reply));
    backend
}

fn model_is(app: &mut CodexApp, model: &str) {
    run(
        app,
        AppEv::Screen(codex_tui_core::screens::ScreenEv::Chat(chat::Ev::Status(
            Box::new(chat::StatusFacts {
                model: Some(model.to_owned()),
                ..chat::StatusFacts::default()
            }),
        ))),
    );
}

fn model_banner(presentation: &str) -> serde_json::Value {
    serde_json::json!({
        "banner_type": "selected_model_limit", "model_slug": "test-model-a",
        "title": "Selected model usage exhausted",
        "description": "Ask your owner for credits or switch to another model.",
        "ctas": [], "presentation": presentation,
    })
}

/// Oracle `dismiss_backend_banner_for_new_turn`: a submit dismisses a dismissible banner the user was
/// shown, and only that one; an inline banner stays, and so does one the model in use hides.
#[test]
fn a_submit_dismisses_only_the_dismissible_banner_that_was_shown() {
    let mut app = ready();
    model_is(&mut app, "test-model-a");
    let backend = read_banner(&mut app, model_banner("dismissible"));
    assert!(app.session().backend_banners[&backend].is_visible(Some("test-model-a")));
    submit(&mut app, "hello");
    assert!(!app.session().backend_banners[&backend].is_visible(Some("test-model-a")));

    let mut inline = ready();
    model_is(&mut inline, "test-model-a");
    let backend = read_banner(&mut inline, model_banner("inline"));
    submit(&mut inline, "hello");
    assert!(inline.session().backend_banners[&backend].is_visible(Some("test-model-a")));

    // another model hides the dismissible banner without dismissing it: the turn on
    // that model leaves the occurrence for when the first model is back.
    let mut hidden = ready();
    model_is(&mut hidden, "test-model-b");
    let backend = read_banner(&mut hidden, model_banner("dismissible"));
    assert!(!hidden.session().backend_banners[&backend].is_visible(Some("test-model-b")));
    submit(&mut hidden, "hello");
    assert!(hidden.session().backend_banners[&backend].is_visible(Some("test-model-a")));
}

/// The oracle's `backend_banner_invalid_content_and_absence_restore_fallback`, banner half: a
/// replacement the client cannot show (an unknown or null presentation, a blank title) and the
/// banner's absence both drop the shown occurrence, as the account read is authoritative.
#[test]
fn an_unusable_or_absent_banner_replaces_the_shown_occurrence() {
    for replacement in [
        serde_json::Value::Null,
        serde_json::json!({"presentation": "future_mode"}),
        serde_json::json!({"presentation": null}),
        serde_json::json!({"title": " "}),
    ] {
        let mut app = ready();
        model_is(&mut app, "test-model-a");
        let backend = read_banner(&mut app, model_banner("inline"));
        assert!(app.session().backend_banners[&backend].is_visible(Some("test-model-a")));
        let mut banner = model_banner("inline");
        if replacement.is_null() {
            banner = serde_json::Value::Null;
        } else {
            for (key, value) in replacement.as_object().expect("an object") {
                banner[key.as_str()] = value.clone();
            }
        }
        read_banner(&mut app, banner);
        assert!(
            !app.session().backend_banners.contains_key(&backend),
            "{replacement}: {:?}",
            app.session().backend_banners
        );
    }
}

fn reserve_banner(presentation: &str) -> serde_json::Value {
    serde_json::json!({
        "banner_type": "luna_reserve", "presentation": presentation,
        "title": "You’re now using Luna, a faster model for simpler tasks.",
        "description": "Add credits or upgrade to continue using the most advanced models.",
        "ctas": [{"action": "add_credits", "label": "Add credits"},
                 {"action": "open_pricing_dialog", "label": "Upgrade"}],
    })
}

/// Oracle `luna_reserve_selector_supports_arrows_enter_shortcuts_and_escape_without_losing_draft`,
/// state half: the selector's Esc and every pick are a local dismissal (`picker_dismissed`) whatever
/// the presentation says; a pick of a purchase action also opens its page, and a periodic read of the
/// same occurrence does not bring the selector back.
#[test]
fn a_reserve_selector_closes_on_a_pick_or_on_escape_and_stays_closed() {
    for (action, presentation) in [
        (Some(1), "dismissible"),
        (Some(0), "inline"),
        (None, "inline"),
    ] {
        let mut app = ready();
        model_is(&mut app, "gpt-reserve");
        let backend = read_banner(&mut app, reserve_banner(presentation));
        let ticket = app.session().backend_banners[&backend].ticket;
        assert!(app.session().backend_banners[&backend].is_visible(Some("gpt-reserve")));
        let fx = match action {
            Some(row) => run(
                &mut app,
                AppEv::Intent(Intent::PickBannerCta {
                    backend,
                    ticket,
                    row,
                }),
            ),
            None => run(
                &mut app,
                AppEv::Intent(Intent::DismissBackendBanner { backend, ticket }),
            ),
        };
        match action {
            Some(row) => {
                let urls = [
                    "https://chatgpt.com/settings/usage?credits_modal=true",
                    "https://chatgpt.com/?cta_tab=personal&highlight_plan=plus#pricing",
                ];
                assert!(
                    matches!(fx.commands(), [Command::OpenUrl { url, .. }] if url == urls[row]),
                    "{:?}",
                    fx.commands()
                );
            }
            None => assert!(fx.commands().is_empty()),
        }
        assert!(
            !app.session().backend_banners[&backend].is_visible(Some("gpt-reserve")),
            "{action:?} {presentation}: the selector is closed"
        );
        read_banner(&mut app, reserve_banner(presentation));
        assert!(
            !app.session().backend_banners[&backend].is_visible(Some("gpt-reserve")),
            "{action:?} {presentation}: a read of the same occurrence keeps it closed"
        );
    }
}

fn notices(app: &CodexApp) -> Vec<codex_tui_core::threads::Notice> {
    thread(app)
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Notice(notice) => Some(notice.clone()),
            _ => None,
        })
        .collect()
}

/// Oracle `backend_banner_presentation_and_cta_do_not_imply_recovery` and
/// `finish_add_credits_nudge_email_request`: the notify-owner action asks the backend to send the
/// e-mail, one at a time; the reply (or its failure) leaves one notice in the transcript and frees
/// the action, and the banner stays.
#[test]
fn the_notify_owner_action_sends_one_nudge_at_a_time_and_tells_the_outcome() {
    use codex_tui_core::protocol;
    use codex_tui_core::screens::chat::{Nudge, NudgeOutcome};
    use codex_tui_core::threads::Notice;

    for (action, nudge) in [
        ("notify_owner", Nudge::Credits),
        ("request_increase", Nudge::UsageLimit),
    ] {
        let mut app = ready();
        model_is(&mut app, "test-model-a");
        let mut banner = model_banner("inline");
        banner["ctas"] = serde_json::json!([{"action": action, "label": "Notify"}]);
        let backend = read_banner(&mut app, banner);
        let ticket = app.session().backend_banners[&backend].ticket;
        let pick = |app: &mut CodexApp| {
            run(
                app,
                AppEv::Intent(Intent::PickBannerCta {
                    backend,
                    ticket,
                    row: 0,
                }),
            )
        };
        let fx = pick(&mut app);
        let [send @ Command::SendAddCreditsNudge { nudge: asked, .. }] = fx.commands() else {
            panic!("{action}: expected the nudge: {:?}", fx.commands());
        };
        assert_eq!(*asked, nudge);
        assert!(
            pick(&mut app).commands().is_empty(),
            "{action}: one nudge at a time"
        );
        let wire =
            protocol::client_request(send, codex_app_server_protocol::RequestId::Integer(1), None)
                .expect("a request");
        assert!(
            format!("{wire:?}").contains("SendAddCreditsNudgeEmail"),
            "{action}: {wire:?}"
        );
        let (reply_ticket, reply) =
            protocol::reply(send, Ok(serde_json::json!({"status": "cooldown_active"})))
                .expect("a reply");
        run(&mut app, AppEv::Reply(reply_ticket, reply));
        assert_eq!(
            notices(&app),
            [Notice::AddCreditsNudge {
                nudge,
                outcome: NudgeOutcome::Cooldown
            }]
        );
        assert!(app.session().backend_banners[&backend].is_visible(Some("test-model-a")));
        // freed: the next pick asks again, and a failed request says so
        let fx = pick(&mut app);
        let [again] = fx.commands() else {
            panic!("{action}: asks again: {:?}", fx.commands());
        };
        let (reply_ticket, reply) =
            protocol::reply(again, Err("offline".to_owned())).expect("a reply");
        run(&mut app, AppEv::Reply(reply_ticket, reply));
        assert_eq!(
            notices(&app).last(),
            Some(&Notice::AddCreditsNudge {
                nudge,
                outcome: NudgeOutcome::Failed
            })
        );
    }
}

/// Oracle `owner_notification_completion_cannot_cross_account_change`, state half: an account
/// update drops the banner and the request in flight, so the old account's reply tells nothing and
/// the new account may ask again.
#[test]
fn an_account_update_drops_the_banner_and_the_nudge_in_flight() {
    use codex_app_server_protocol::{AccountUpdatedNotification, ServerNotification};
    use codex_tui_core::app::BackendEv;
    use codex_tui_core::protocol;

    let mut app = ready();
    model_is(&mut app, "test-model-a");
    let mut banner = model_banner("inline");
    banner["ctas"] = serde_json::json!([{"action": "notify_owner", "label": "Notify"}]);
    let backend = read_banner(&mut app, banner.clone());
    let ticket = app.session().backend_banners[&backend].ticket;
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::PickBannerCta {
            backend,
            ticket,
            row: 0,
        }),
    );
    let [previous] = fx.commands() else {
        panic!("expected the nudge: {:?}", fx.commands());
    };
    let previous = previous.clone();
    run(
        &mut app,
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::AccountUpdated(AccountUpdatedNotification {
                auth_mode: None,
                plan_type: None,
            }),
        ))),
    );
    assert!(!app.session().backend_banners.contains_key(&backend));
    let (stale_ticket, reply) =
        protocol::reply(&previous, Ok(serde_json::json!({"status": "sent"}))).expect("a reply");
    run(&mut app, AppEv::Reply(stale_ticket, reply));
    assert!(
        notices(&app).is_empty(),
        "the old account's reply tells nothing"
    );
    // the new account's read brings its banner back, and its pick is not held by the old request
    let backend = read_banner(&mut app, banner);
    let ticket = app.session().backend_banners[&backend].ticket;
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::PickBannerCta {
            backend,
            ticket,
            row: 0,
        }),
    );
    assert!(
        matches!(fx.commands(), [Command::SendAddCreditsNudge { .. }]),
        "{:?}",
        fx.commands()
    );
}

// ---- step 7: interruption notices, goal budget, the review prompt, the review steer gate

fn items_of(app: &CodexApp) -> Vec<Item> {
    thread(app)
        .transcript()
        .iter()
        .map(|(_, item)| item.clone())
        .collect()
}

/// `item/started` of the review banner (the oracle's `handle_entered_review_mode`).
fn entered_review(thread: &str, turn: &str, review: &str) -> BackendEv {
    use codex_app_server_protocol::{
        ItemStartedNotification, ServerNotification, ThreadItem, ThreadItemPayload,
    };
    BackendEv::Notification(Box::new(ServerNotification::ItemStarted(
        ItemStartedNotification {
            item: ThreadItem::untimed(ThreadItemPayload::EnteredReviewMode {
                id: "review-start".into(),
                review: review.into(),
            }),
            thread_id: thread.into(),
            turn_id: turn.into(),
            started_at_ms: 0,
        },
    )))
}

/// `thread/goal/updated` with `status` for `turn`.
fn goal_updated(
    thread: &str,
    turn: Option<&str>,
    status: codex_app_server_protocol::ThreadGoalStatus,
) -> BackendEv {
    use codex_app_server_protocol::{
        ServerNotification, ThreadGoal, ThreadGoalUpdatedNotification,
    };
    BackendEv::Notification(Box::new(ServerNotification::ThreadGoalUpdated(
        ThreadGoalUpdatedNotification {
            thread_id: thread.into(),
            turn_id: turn.map(str::to_owned),
            goal: ThreadGoal {
                thread_id: thread.into(),
                objective: "Run until the token budget is limited".into(),
                status,
                token_budget: Some(10_000),
                tokens_used: 10_500,
                time_used_seconds: 0,
                created_at: 0,
                updated_at: 1,
            },
        },
    )))
}

/// A plain interruption says so: `new_interruption_notice`.
#[test]
fn an_interrupted_turn_ends_with_the_interruption_notice() {
    let mut app = running();
    notification(&mut app, agent_delta(T, "turn-1", "msg-1", "partial"));
    notification(&mut app, turn_interrupted(T, "turn-1"));
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::Notice(threads::Notice::Interrupted))
    );
    // a completed turn adds none
    notification(&mut app, turn_started(T, "turn-2"));
    notification(&mut app, turn_completed(T, "turn-2"));
    assert_eq!(
        items_of(&app)
            .iter()
            .filter(|item| matches!(item, Item::Notice(threads::Notice::Interrupted)))
            .count(),
        1
    );
}

/// Esc with steers waiting: the notice names the steer, and the steers go out merged.
#[test]
fn an_interrupt_for_steers_says_the_model_was_interrupted_to_submit_them() {
    let mut app = running();
    submit(&mut app, "one");
    chat_intent(&mut app, chat::ThreadIntent::InterruptForSteers);
    let fx = notification(&mut app, turn_interrupted(T, "turn-1"));
    assert_eq!(start_turns(&fx).len(), 1);
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::Notice(threads::Notice::InterruptedForSteer))
    );
}

/// The goal's budget stopped the turn: the interruption reads as the budget message,
/// once (`take_budget_limited`).
#[test]
fn a_budget_limited_turn_ends_with_the_budget_notice_once() {
    let mut app = running();
    notification(
        &mut app,
        goal_updated(
            T,
            Some("turn-1"),
            codex_app_server_protocol::ThreadGoalStatus::BudgetLimited,
        ),
    );
    notification(&mut app, turn_interrupted(T, "turn-1"));
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::Notice(threads::Notice::GoalBudgetReached))
    );
    notification(&mut app, turn_started(T, "turn-2"));
    notification(&mut app, turn_interrupted(T, "turn-2"));
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::Notice(threads::Notice::Interrupted)),
        "the mark was consumed by the first turn"
    );
}

/// Only a budget-limited goal with a turn marks it; an active goal does not.
#[test]
fn only_a_budget_limited_goal_marks_the_turn() {
    let mut app = running();
    notification(
        &mut app,
        goal_updated(
            T,
            Some("turn-1"),
            codex_app_server_protocol::ThreadGoalStatus::Active,
        ),
    );
    notification(
        &mut app,
        goal_updated(
            T,
            None,
            codex_app_server_protocol::ThreadGoalStatus::BudgetLimited,
        ),
    );
    notification(&mut app, turn_interrupted(T, "turn-1"));
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::Notice(threads::Notice::Interrupted))
    );
}

/// In review mode the server's user item is the review prompt: it adds no row
/// (`on_committed_user_message`: `!self.review.is_review_mode`).
#[test]
fn the_review_prompt_user_item_adds_no_row_in_review_mode() {
    let mut app = running();
    notification(
        &mut app,
        entered_review(T, "turn-1", "changes against 'main'"),
    );
    notification(
        &mut app,
        user_item(T, "turn-1", "review-prompt", "Review the code changes."),
    );
    assert!(
        !items_of(&app)
            .iter()
            .any(|item| matches!(item, Item::User(_))),
        "{:?}",
        items_of(&app)
    );
    // outside review mode the same item is a row
    let mut app = running();
    notification(&mut app, user_item(T, "turn-1", "user-1", "hello"));
    assert!(
        items_of(&app)
            .iter()
            .any(|item| matches!(item, Item::User(_)))
    );
}

/// A steer's echo is the user's own message, drawn even in review mode.
#[test]
fn the_echo_of_a_steer_is_drawn_in_review_mode() {
    let mut app = running();
    notification(&mut app, entered_review(T, "turn-1", "current changes"));
    let fx = submit(&mut app, "follow-up");
    notification(
        &mut app,
        user_item_with_receipt(T, "turn-1", "echo", "follow-up", Some(steer_receipt(&fx))),
    );
    assert!(
        items_of(&app)
            .iter()
            .any(|item| matches!(item, Item::User(_)))
    );
}

/// Esc with a steer waiting during /review does not interrupt: it warns that
/// steers are not supported, and keeps the steer.
#[test]
fn esc_with_steers_in_review_mode_warns_and_does_not_interrupt() {
    let mut app = running();
    notification(&mut app, entered_review(T, "turn-1", "feature branch"));
    submit(&mut app, "review follow-up");
    let fx = chat_intent(&mut app, chat::ThreadIntent::InterruptForSteers);
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    assert_eq!(pending_steers(&app), vec!["review follow-up".to_string()]);
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::LocalWarning(
            threads::LocalWarning::ReviewSteerUnavailable
        ))
    );
    // a refused steer parked in the rejected list gives the same warning
    let mut app = running();
    notification(&mut app, entered_review(T, "turn-1", "feature branch"));
    let sent = steers(&submit(&mut app, "refused"));
    run(
        &mut app,
        AppEv::Reply(sent[0].0, Reply::failed("cannot steer a review turn")),
    );
    assert_eq!(rejected_steers(&app), vec!["refused".to_string()]);
    let fx = chat_intent(&mut app, chat::ThreadIntent::InterruptForSteers);
    assert!(fx.commands().is_empty());
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::LocalWarning(
            threads::LocalWarning::ReviewSteerUnavailable
        ))
    );
}

/// Esc Esc with no prompt to edit: the chat says so, as an info line of the
/// history (`NO_PREVIOUS_MESSAGE_TO_EDIT`).
#[test]
fn no_previous_message_is_a_line_of_the_history() {
    let mut app = running();
    chat_intent(&mut app, chat::ThreadIntent::NoPreviousMessage);
    assert_eq!(
        items_of(&app).last(),
        Some(&Item::Notice(threads::Notice::NoPreviousMessage))
    );
}
