#![allow(clippy::expect_used)]

use super::*;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ErrorNotification;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::ModelSafetyBufferingUpdatedNotification;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnItemsView;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;
use codex_tui_core::app::{AppFx, CodexApp, Intent, apply_app_from};
use codex_tui_core::fixtures::builders::{turn_completed, turn_started, user_item_with_receipt};
use codex_tui_core::protocol;
use codex_tui_core::screens::{ScreenCommand, ScreenThreadIntent, chat};
use codex_tui_core::threads::{Queued, TurnState, UserMessage, cloud_submit_params};

const THREAD: &str = "cloud-thread";
const CLOUD: BackendId = BackendId(1);

fn step(app: &mut CodexApp, ev: AppEv) -> AppFx {
    step_from(app, CLOUD, ev)
}

fn step_from(app: &mut CodexApp, backend: BackendId, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app_from(app, backend, ev, &mut fx);
    fx
}

fn thread_started(fx: &AppFx) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("thread start")
}

fn chat(running: bool) -> CodexApp {
    let mut app = CodexApp::new();
    let local = BackendId::default();
    let fx = step_from(&mut app, local, AppEv::Connected);
    step_from(
        &mut app,
        local,
        AppEv::Reply(
            thread_started(&fx),
            Reply::thread_started(ThreadId::new("local")),
        ),
    );
    app.attach_backend(CLOUD);
    step(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::CloudBackend(CLOUD))),
    );
    step(&mut app, AppEv::Connected);
    let fx = step_from(&mut app, local, AppEv::Intent(Intent::CycleBackend));
    step(
        &mut app,
        AppEv::Reply(
            thread_started(&fx),
            Reply::thread_started(ThreadId::new(THREAD)),
        ),
    );
    if running {
        step(&mut app, AppEv::Backend(turn_started(THREAD, "previous")));
    }
    app
}

fn type_message(app: &mut CodexApp, text: &str) -> AppFx {
    step(
        app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(THREAD)),
            message: UserMessage::text(text),
        }),
    )
}

fn chat_intent(app: &mut CodexApp, intent: chat::ThreadIntent) -> AppFx {
    step(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(THREAD),
            intent: ScreenThreadIntent::Chat(intent),
        }),
    )
}

fn submission(fx: &AppFx) -> (Ticket, &str, ClientRequest) {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::StartTurn {
                ticket, client_id, ..
            }
            | Command::Screen {
                ticket,
                command: ScreenCommand::Chat(chat::Command::Steer { client_id, .. }),
                ..
            } => Some((
                *ticket,
                client_id.as_str(),
                protocol::client_request(command, RequestId::Integer(1), None)
                    .expect("submission on the wire"),
            )),
            _ => None,
        })
        .expect("user input submitted")
}

fn receive(
    app: &mut CodexApp,
    ticket: Ticket,
    id: &str,
    request: ClientRequest,
    result: Result<CloudUserMessageReceipt, TypedRequestError>,
) -> AppFx {
    let params = cloud_submit_params(request).expect("native submission on the wire");
    assert_eq!(params.thread_id, THREAD);
    assert_eq!(params.client_user_message_id.as_deref(), Some(id));
    assert!(matches!(
        params.input.as_slice(),
        [UserInput::Text { text, .. }] if text == "first input"
    ));
    let event = submission_reply(ticket, result);
    let (ingress, mut events) = tokio::sync::mpsc::unbounded_channel();
    let epoch = app.epoch_of(CLOUD);
    emit_from(&ingress, CLOUD, epoch, event);
    let event = events.try_recv().expect("reply at host ingress");
    assert_eq!(event.epoch, epoch);
    let mut fx = AppFx::default();
    apply_app_from(app, event.backend, event.ev, &mut fx);
    app.assert_consistent();
    fx
}

fn active(turn: &str) -> Turn {
    Turn {
        id: turn.to_owned(),
        root_turn_id: None,
        items: Vec::new(),
        items_view: TurnItemsView::NotLoaded,
        status: TurnStatus::InProgress,
        error: None,
        started_at: None,
        completed_at: None,
        duration_ms: None,
    }
}

fn server_error(code: i64) -> TypedRequestError {
    TypedRequestError::Server {
        method: "submitUserMessage".to_owned(),
        source: JSONRPCErrorError {
            code,
            message: "submission rejected or lost".to_owned(),
            data: None,
        },
    }
}

fn legacy_completion() -> AppEv {
    AppEv::Backend(BackendEv::Notification(Box::new(
        ServerNotification::ItemCompleted(ItemCompletedNotification {
            thread_id: THREAD.to_owned(),
            turn_id: "previous".to_owned(),
            item: ThreadItem::untimed(ThreadItemPayload::UserMessage {
                id: "unrelated-legacy-item".to_owned(),
                client_id: None,
                content: vec![UserInput::Text {
                    text: "first input".to_owned(),
                    text_elements: Vec::new(),
                }],
            }),
            content_token_count: None,
            completed_at_ms: 0,
        }),
    )))
}

#[test]
fn cloud_admission_decides_whether_input_starts_or_steers() {
    for local_running in [true, false] {
        let mut app = chat(local_running);
        let fx = type_message(&mut app, "first input");
        let (ticket, id, request) = submission(&fx);
        assert_eq!(
            matches!(request, ClientRequest::TurnSteer { .. }),
            local_running
        );
        let receipt = if local_running {
            CloudUserMessageReceipt::Started(active("admitted"))
        } else {
            CloudUserMessageReceipt::Steered(active("admitted"))
        };
        receive(&mut app, ticket, id, request, Ok(receipt));
        let thread = app.active_thread();
        assert!(matches!(
            thread.turn(),
            TurnState::Running(running) if running.turn.as_str() == "admitted"
        ));
        assert_eq!(thread.pending_steers().len(), usize::from(!local_running));
        step(
            &mut app,
            AppEv::Backend(user_item_with_receipt(
                THREAD,
                "admitted",
                "echo",
                "first input",
                Some(id.to_owned()),
            )),
        );
        assert!(app.active_thread().pending_steers().is_empty());
    }
}

#[test]
fn cloud_duplicate_and_unknown_wait_for_exact_live_confirmation() {
    for receipt in [
        Ok(CloudUserMessageReceipt::Duplicate {
            turn_id: Some("unverified".to_owned()),
        }),
        Err(server_error(-32000)),
        Err(TypedRequestError::Transport {
            method: "submitUserMessage".to_owned(),
            source: std::io::Error::other("channel closed"),
        }),
    ] {
        let mut app = chat(true);
        let fx = type_message(&mut app, "first input");
        let (ticket, id, request) = submission(&fx);
        step(&mut app, legacy_completion());
        assert_eq!(app.active_thread().pending_steers().len(), 1);
        assert!(
            receive(&mut app, ticket, id, request, receipt)
                .commands()
                .is_empty()
        );
        assert!(matches!(
            app.active_thread().turn(),
            TurnState::Running(running) if running.turn.as_str() == "previous"
        ));
        assert!(type_message(&mut app, "follow-up").commands().is_empty());
        for echo in [None, Some("same text, different input".to_owned())] {
            step(
                &mut app,
                AppEv::Backend(user_item_with_receipt(
                    THREAD,
                    "previous",
                    "unrelated",
                    "first input",
                    echo,
                )),
            );
            assert!(matches!(
                app.active_thread().queue(),
                [Queued::Unconfirmed { client_id, .. }, Queued::Message(message)]
                    if client_id == id && message.text == "follow-up"
            ));
        }
        assert!(
            step(
                &mut app,
                AppEv::Backend(user_item_with_receipt(
                    THREAD,
                    "previous",
                    "exact",
                    "first input",
                    Some(id.to_owned()),
                )),
            )
            .commands()
            .is_empty()
        );
        let fx = step(&mut app, AppEv::Backend(turn_completed(THREAD, "previous")));
        assert!(matches!(
            fx.commands(),
            [Command::StartTurn { message, client_id, .. }]
                if message.text == "follow-up" && client_id != id
        ));
    }
}

#[test]
fn cloud_preflight_rejection_returns_the_draft_without_replaying_it() {
    for local_running in [false, true] {
        for loss_first in [false, true] {
            let mut app = chat(local_running);
            let fx = type_message(&mut app, "first input");
            let (ticket, id, request) = submission(&fx);
            if loss_first {
                step(&mut app, stream_lost());
                assert!(matches!(
                    app.active_thread().queue(),
                    [Queued::Unconfirmed { .. }]
                ));
            }
            let fx = receive(&mut app, ticket, id, request, Err(server_error(-32602)));
            assert!(fx.commands().is_empty());
            let thread = app.active_thread();
            assert!(thread.queue().is_empty());
            assert!(thread.pending_steers().is_empty());
            assert_eq!(
                thread
                    .restore()
                    .expect("input returned to composer")
                    .messages(),
                [UserMessage::text("first input")]
            );
        }
    }
}

fn exact_echo(id: &str) -> AppEv {
    AppEv::Backend(user_item_with_receipt(
        THREAD,
        "previous",
        "exact",
        "first input",
        Some(id.to_owned()),
    ))
}

fn stream_lost() -> AppEv {
    let error: ErrorNotification = serde_json::from_value(serde_json::json!({
        "threadId": THREAD,
        "turnId": "",
        "willRetry": false,
        "error": { "message": "Cloud stream ended.", "codexErrorInfo": null, "additionalDetails": null }
    })).expect("Cloud loss notification");
    AppEv::Backend(BackendEv::StreamLost {
        thread: ThreadId::new(THREAD),
        error: Box::new(error),
    })
}

#[test]
fn the_whole_cloud_connection_retires_lost_stream_admission_tickets() {
    let mut app = chat(false);
    let fx = type_message(&mut app, "first input");
    let (ticket, id, request) = submission(&fx);
    step(&mut app, stream_lost());
    let fx = step(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(THREAD))),
    );
    let resume = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen { ticket, .. }
                if matches!(
                    protocol::client_request(command, RequestId::Integer(1), None),
                    Some(ClientRequest::ThreadResume { .. })
                ) =>
            {
                Some(*ticket)
            }
            _ => None,
        })
        .expect("reopen the lost stream");
    step(
        &mut app,
        AppEv::Reply(resume, Reply::failed("attach failed")),
    );
    assert!(app.active_thread().view().stream_lost());
    step(&mut app, AppEv::Disconnected);
    assert!(!app.active_thread().busy());
    let fx = receive(&mut app, ticket, id, request, Err(server_error(-32602)));
    assert!(fx.commands().is_empty());
    assert!(matches!(
        app.active_thread().queue(),
        [Queued::Unconfirmed { client_id, .. }] if client_id == id
    ));
    assert!(app.active_thread().restore().is_none());
}

#[test]
fn cloud_turn_end_waits_for_admission_without_draining_or_merging_input() {
    for end in [TurnStatus::Completed, TurnStatus::Interrupted] {
        for echo_first in [false, true] {
            let mut app = chat(true);
            let fx = type_message(&mut app, "first input");
            let (ticket, id, request) = submission(&fx);
            chat_intent(
                &mut app,
                chat::ThreadIntent::Queue(Queued::Message(UserMessage::text("follow-up"))),
            );
            if end == TurnStatus::Interrupted {
                chat_intent(&mut app, chat::ThreadIntent::InterruptForSteers);
            }
            let completed = TurnCompletedNotification {
                thread_id: THREAD.to_owned(),
                turn: Turn {
                    status: end.clone(),
                    ..active("previous")
                },
            };
            let finished = step(
                &mut app,
                AppEv::Backend(BackendEv::Notification(Box::new(
                    ServerNotification::TurnCompleted(completed),
                ))),
            );
            assert!(finished.commands().is_empty());
            assert!(app.active_thread().restore().is_none());
            if echo_first {
                assert!(step(&mut app, exact_echo(id)).commands().is_empty());
            }
            let fx = receive(&mut app, ticket, id, request, Err(server_error(-32000)));
            let fx = if echo_first {
                fx
            } else {
                assert!(fx.commands().is_empty());
                assert!(matches!(
                    app.active_thread().queue(),
                    [Queued::Unconfirmed { client_id, .. }, Queued::Message(_)] if client_id == id
                ));
                step(&mut app, exact_echo(id))
            };
            assert!(matches!(
                fx.commands(),
                [Command::StartTurn { message, client_id, .. }]
                    if message.text == "follow-up" && client_id != id
            ));
        }
    }
}

#[test]
fn a_cloud_steer_receipt_preserves_the_active_turn_safety_check() {
    let mut app = chat(true);
    let fx = type_message(&mut app, "first input");
    let (ticket, id, request) = submission(&fx);
    let buffering = ModelSafetyBufferingUpdatedNotification {
        thread_id: THREAD.to_owned(),
        turn_id: "previous".to_owned(),
        model: "test-model".to_owned(),
        use_cases: Vec::new(),
        reasons: Vec::new(),
        show_buffering_ui: true,
        faster_model: None,
    };
    step(
        &mut app,
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::ModelSafetyBufferingUpdated(buffering),
        ))),
    );
    assert!(app.active_thread().safety().is_waiting());
    receive(
        &mut app,
        ticket,
        id,
        request,
        Ok(CloudUserMessageReceipt::Steered(active("previous"))),
    );
    assert!(app.active_thread().safety().is_waiting());
}
