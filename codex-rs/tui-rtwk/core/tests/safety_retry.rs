#![allow(clippy::expect_used)]

//! The app's safety retry owns one checked source turn and one connection across explicit
//! confirmation, the interrupt receipt, history verification, and the fork.

use codex_app_server_protocol::{RequestId, ServerNotification, Thread};
use codex_tui_core::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, Ingress, Intent, apply_app, apply_app_from,
};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::fixtures::builders::{agent_delta, user_item_with_receipt};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket, TurnId};
use codex_tui_core::protocol;
use codex_tui_core::rpc::{CloudSubmission, Reply};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, agents, chat};
use codex_tui_core::threads::{Queued, ThreadAsk, ThreadReply, UserMessage, safety};
use serde_json::json;

const SOURCE: &str = "thread-1";
const TURN: &str = "checked";
const PROMPT: &str = "Summarize the public example.";

fn apply(app: &mut CodexApp, event: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, event, &mut fx);
    fx
}

fn notify(app: &mut CodexApp, notification: serde_json::Value) -> AppFx {
    let notification: ServerNotification =
        serde_json::from_value(notification).expect("notification");
    apply(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
    )
}

fn start(app: &mut CodexApp, event: AppEv, expected: fn(&Command) -> bool) -> Ticket {
    apply(app, event)
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread { ticket, .. } | Command::StartTurn { ticket, .. }
                if expected(command) =>
            {
                Some(*ticket)
            }
            _ => None,
        })
        .expect("expected request")
}

fn offered() -> (CodexApp, safety::RetryOffer) {
    offered_with_app(CodexApp::new())
}

fn offered_with_app(mut app: CodexApp) -> (CodexApp, safety::RetryOffer) {
    let ticket = start(&mut app, AppEv::Connected, |cmd| {
        matches!(cmd, Command::StartThread { .. })
    });
    let source = ThreadId::new(SOURCE);
    apply(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(source.clone())),
    );
    let ticket = start(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: Some(source.clone()),
            message: UserMessage::text(PROMPT),
        }),
        |cmd| matches!(cmd, Command::StartTurn { .. }),
    );
    apply(
        &mut app,
        AppEv::Reply(ticket, Reply::turn_started(source, TurnId::new(TURN))),
    );
    notify(
        &mut app,
        json!({"method": "model/safetyBuffering/updated", "params": {
            "threadId": SOURCE, "turnId": TURN, "model": "default", "useCases": [],
            "reasons": [], "showBufferingUi": true, "fasterModel": "faster"
        }}),
    );
    let offer = app
        .active_thread()
        .safety()
        .buffering()
        .and_then(safety::Buffering::retry_offer)
        .cloned()
        .expect("retry offer for locally submitted turn");
    (app, offer)
}

fn choose(app: &mut CodexApp, offer: safety::RetryOffer) -> AppFx {
    apply(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Safety {
            thread: ThreadId::new(SOURCE),
            verb: safety::Verb::Retry(offer),
        }))),
    )
}

fn interrupt(fx: &AppFx) -> Option<Ticket> {
    fx.commands().iter().find_map(|command| match command {
        Command::Screen {
            ticket,
            command:
                ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::RetryInterrupt {
                    thread,
                    turn,
                })),
            ..
        } if thread.as_str() == SOURCE && turn.as_str() == TURN => Some(*ticket),
        _ => None,
    })
}

fn history_read(fx: &AppFx) -> usize {
    fx.commands()
        .iter()
        .filter(|command| {
            matches!(command, Command::Screen {
        command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Read { thread })), ..
    } if thread == SOURCE)
        })
        .count()
}

fn ended(app: &mut CodexApp, status: &str) -> AppFx {
    notify(
        app,
        json!({"method": "turn/completed", "params": {
            "threadId": SOURCE, "turn": {"id": TURN, "status": status, "itemsView": "full", "items": []}
        }}),
    )
}

#[test]
fn source_requires_both_receipts_and_wrong_kind_cannot_spend_the_interrupt() {
    for terminal_first in [false, true] {
        let (mut app, offer) = offered();
        let ticket = interrupt(&choose(&mut app, offer.clone())).expect("source interrupt");
        assert!(app.active_thread().safety().retrying());
        assert!(interrupt(&choose(&mut app, offer)).is_none());
        let ack = || AppEv::Reply(ticket, Reply::thread(ThreadReply::RetryInterrupted));
        let mut foreign = AppFx::default();
        apply_app_from(&mut app, BackendId(8), ack(), &mut foreign);
        assert_eq!(history_read(&foreign), 0);
        for reply in [
            Reply::thread(ThreadReply::Done),
            Reply::cloud_submission(CloudSubmission::Duplicate),
        ] {
            assert_eq!(
                history_read(&apply(&mut app, AppEv::Reply(ticket, reply))),
                0
            );
        }
        let (one, two) = if terminal_first {
            (ended(&mut app, "interrupted"), apply(&mut app, ack()))
        } else {
            (apply(&mut app, ack()), ended(&mut app, "interrupted"))
        };
        assert_eq!(history_read(&one), 0);
        assert_eq!(history_read(&two), 1);
        assert_eq!(history_read(&ended(&mut app, "interrupted")), 0);
        assert_eq!(history_read(&apply(&mut app, ack())), 0);
    }
}

#[test]
fn completion_response_or_disconnect_retires_the_exact_interrupt() {
    for reason in ["completed", "response", "lost"] {
        let (mut app, offer) = offered();
        let ticket = interrupt(&choose(&mut app, offer.clone())).expect("source interrupt");
        let cancelled = match reason {
            "completed" => ended(&mut app, "completed"),
            "response" => apply(
                &mut app,
                AppEv::Backend(agent_delta(SOURCE, TURN, "answer", "Notes.")),
            ),
            _ => apply(
                &mut app,
                AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
            ),
        };
        assert!(!app.active_thread().safety().retrying(), "{reason}");
        assert_eq!(history_read(&cancelled), 0, "{reason}");
        assert_eq!(
            history_read(&apply(
                &mut app,
                AppEv::Reply(ticket, Reply::thread(ThreadReply::RetryInterrupted))
            )),
            0,
            "{reason}"
        );
        assert!(interrupt(&choose(&mut app, offer)).is_none(), "{reason}");
    }
}

#[test]
fn refusal_restores_the_exact_original_draft_to_the_source() {
    let (mut app, offer) = offered();
    let ticket = interrupt(&choose(&mut app, offer)).expect("source interrupt");
    let fx = apply(
        &mut app,
        AppEv::Reply(ticket, Reply::failed("server refused interruption")),
    );
    assert_eq!(history_read(&fx), 0);
    assert!(!app.active_thread().safety().retrying());
    let source = app.active_thread();
    let restore = source.restore().expect("original draft restored");
    assert_eq!(restore.messages(), &[UserMessage::text(PROMPT)]);
}

#[test]
fn revisiting_the_source_requires_a_fresh_confirmation() {
    let (mut app, offer) = offered();
    let interrupt_ticket = interrupt(&choose(&mut app, offer.clone())).expect("source interrupt");
    let source = ThreadId::new(SOURCE);
    let other = ThreadId::new("other");
    let ticket = start(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(source.clone()),
            command: SlashCommand::New,
            intent: SessionIntent::New { name: None },
        }))),
        |command| matches!(command, Command::StartThread { .. }),
    );
    apply(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(other.clone())),
    );
    assert_eq!(app.session().active.as_ref(), Some(&other));
    assert!(
        app.thread(&source)
            .is_some_and(|model| !model.safety().retrying())
    );
    apply(&mut app, AppEv::Intent(Intent::OpenThread(source.clone())));
    assert_eq!(app.session().active.as_ref(), Some(&source));
    assert!(interrupt(&choose(&mut app, offer)).is_none());
    let old_ack = AppEv::Reply(
        interrupt_ticket,
        Reply::thread(ThreadReply::RetryInterrupted),
    );
    assert_eq!(history_read(&apply(&mut app, old_ack)), 0);
}

#[test]
fn only_the_retry_owners_transport_can_settle_its_live_interrupt() {
    let a = BackendId::default();
    let b = BackendId(1);
    for replace in [false, true] {
        let mut app = CodexApp::new();
        let a_epoch = app.attach_backend(a);
        let b_epoch = app.attach_backend(b);
        let (mut app, offer) = offered_with_app(app);
        let ticket = interrupt(&choose(&mut app, offer)).expect("source interrupt");
        let ack = || Reply::thread(ThreadReply::RetryInterrupted);
        assert!(
            app.ingest(Ingress::transport_reply(b, b_epoch, b_epoch, ticket, ack()))
                .is_none()
        );
        let foreign_loss = Ingress::transport_replaced(b, b_epoch, b_epoch.next());
        assert!(app.ingest(foreign_loss).is_some());
        assert!(app.active_thread().safety().retrying());

        let own_loss = if replace {
            Ingress::transport_replaced(a, a_epoch, a_epoch.next())
        } else {
            Ingress::of(a, a_epoch, AppEv::Disconnected)
        };
        let fx = app.ingest(own_loss).expect("source transport ended");
        assert_eq!(history_read(&fx), 0);
        assert!(!app.active_thread().safety().retrying());
        assert!(
            app.ingest(Ingress::transport_reply(a, a_epoch, a_epoch, ticket, ack()))
                .is_none()
        );
    }
}

fn retry_ask(fx: &AppFx, predicate: impl Fn(&ThreadAsk) -> bool) -> (&Command, Ticket) {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
                ..
            } if predicate(ask) => Some((command, *ticket)),
            _ => None,
        })
        .expect("verified retry request")
}

fn saved_thread(id: &str, forked_from: Option<&str>) -> Box<Thread> {
    Box::new(
        serde_json::from_value(json!({
            "id": id, "sessionId": id, "forkedFromId": forked_from, "preview": "",
            "ephemeral": false, "historyMode": "legacy", "modelProvider": "openai",
            "createdAt": 0, "updatedAt": 0, "recencyAt": 0, "status": {"type": "idle"},
            "cwd": "/tmp/project", "cliVersion": "0.0.0", "source": "cli",
            "canAcceptDirectInput": true, "turns": []
        }))
        .expect("thread metadata"),
    )
}

#[test]
fn ambiguous_child_admission_stays_owned_until_an_exact_live_echo() {
    let child = ThreadId::new("child");
    let source = ThreadId::new(SOURCE);
    let (mut app, offer) = offered();
    let interrupt = interrupt(&choose(&mut app, offer)).expect("source interrupt");
    apply(
        &mut app,
        AppEv::Reply(interrupt, Reply::thread(ThreadReply::RetryInterrupted)),
    );
    let fx = ended(&mut app, "interrupted");
    let (_, read) = retry_ask(&fx, |ask| matches!(ask, ThreadAsk::Read { .. }));
    let fx = apply(
        &mut app,
        AppEv::Reply(
            read,
            Reply::thread(ThreadReply::Read {
                thread: saved_thread(SOURCE, None),
                unread: false,
            }),
        ),
    );
    let (_, turns) = retry_ask(&fx, |ask| matches!(ask, ThreadAsk::RetryTurns(_)));
    let page = serde_json::from_value(json!({
        "data": [{"id": TURN, "status": "interrupted", "itemsView": "full", "items": []}],
        "nextCursor": null,
    }))
    .expect("terminal source history");
    let fx = apply(
        &mut app,
        AppEv::Reply(
            turns,
            Reply::thread(ThreadReply::RetryTurns(Box::new(page))),
        ),
    );
    let (_, fork) = retry_ask(&fx, |ask| matches!(ask, ThreadAsk::RetryFork(_)));
    let fx = apply(
        &mut app,
        AppEv::Reply(
            fork,
            Reply::thread(ThreadReply::RetryForked(saved_thread(
                child.as_str(),
                Some(SOURCE),
            ))),
        ),
    );
    let (command, ticket) = retry_ask(&fx, |ask| matches!(ask, ThreadAsk::RetryStart(_)));
    let request = serde_json::to_value(protocol::client_request(
        command,
        RequestId::Integer(1),
        None,
    ))
    .expect("real wire request");
    let client_id = request["params"]["clientUserMessageId"]
        .as_str()
        .expect("submission ID");
    assert_eq!(request["params"]["threadId"], child.as_str());
    assert_eq!(app.session().active.as_ref(), Some(&child));
    assert!(app.thread(&child).is_some_and(|thread| thread.busy()));

    apply(&mut app, AppEv::Intent(Intent::OpenThread(source.clone())));
    assert_eq!(app.session().active.as_ref(), Some(&source));
    let no_resend = |fx: &AppFx| {
        assert!(!fx.commands().iter().any(|command| matches!(
            command,
            Command::StartTurn { .. }
                | Command::Screen {
                    command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::RetryStart(
                        _
                    ))),
                    ..
                }
        )));
    };
    no_resend(&apply(
        &mut app,
        AppEv::Reply(ticket, Reply::submission_unconfirmed("connection dropped")),
    ));
    let held = |app: &CodexApp| {
        app.thread(&child).is_some_and(|thread| matches!(
            thread.queue(),
            [Queued::Unconfirmed { client_id: id, message, .. }] if id == client_id && message.text == PROMPT
        ))
    };
    assert!(held(&app));
    let echo = |receipt: &str| {
        user_item_with_receipt(
            child.as_str(),
            "child-turn",
            "item",
            PROMPT,
            Some(receipt.to_owned()),
        )
    };
    no_resend(&apply(&mut app, AppEv::Backend(echo("other-submission"))));
    assert!(held(&app));
    no_resend(&apply(&mut app, AppEv::Backend(echo(client_id))));
    assert!(
        app.thread(&child)
            .is_some_and(|thread| thread.queue().is_empty())
    );
    assert_eq!(app.session().active.as_ref(), Some(&source));
}
