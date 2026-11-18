//! A scripted backend for screen tests: it answers our RPCs the way the
//! app-server does and plays the notifications the test scripts. It is one
//! future the shell runs on the tokio runtime and talks to the app only
//! through the ingress, as the real one does.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use codex_app_server_protocol::AgentMessageDeltaNotification;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::ItemStartedNotification;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ToolRequestUserInputParams;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStartedNotification;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::app::AppEv;
use crate::app::BackendEv;
use crate::app::Command;
use crate::app::Ingress;
use crate::app::Reply;
use crate::ids::ConnEpoch;
use crate::ids::ThreadId;
use crate::ids::TurnId;
use crate::shell::IngressTx;

/// The id of the one thread the fake starts.
pub const FAKE_THREAD: &str = "thread-1";

#[derive(Default)]
pub(crate) struct Shared {
    commands: Vec<Command>,
    /// The events to play after each turn's `TurnStarted` reply, one entry
    /// per turn, in order.
    turns: VecDeque<Vec<BackendEv>>,
    started_turns: usize,
    ingress: Option<(IngressTx, ConnEpoch)>,
}

/// The server side, handed to the app as `Backend::Fake`.
pub struct FakeServer {
    shared: Arc<Mutex<Shared>>,
}

/// The test's side: script, inspect, inject.
#[derive(Clone)]
pub struct FakeHandle {
    shared: Arc<Mutex<Shared>>,
}

impl FakeServer {
    pub fn new() -> (FakeServer, FakeHandle) {
        let shared = Arc::new(Mutex::new(Shared::default()));
        (
            FakeServer {
                shared: shared.clone(),
            },
            FakeHandle { shared },
        )
    }
}

impl FakeHandle {
    /// Plays `events` after the next unscripted turn starts.
    pub fn script_turn(&self, events: Vec<BackendEv>) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.turns.push_back(events);
        }
    }

    /// The commands the app has sent, in order.
    pub fn commands(&self) -> Vec<Command> {
        self.shared
            .lock()
            .map(|shared| shared.commands.clone())
            .unwrap_or_default()
    }

    /// The epoch the app attached the connection under.
    pub fn epoch(&self) -> Option<ConnEpoch> {
        self.shared
            .lock()
            .ok()
            .and_then(|shared| shared.ingress.as_ref().map(|(_, epoch)| *epoch))
    }

    /// Delivers `ev` as if the backend had sent it, under `epoch` (a stale
    /// epoch must be dropped).
    pub fn inject(&self, epoch: ConnEpoch, ev: AppEv) {
        if let Ok(shared) = self.shared.lock()
            && let Some((ingress, _)) = &shared.ingress
        {
            let _ = ingress.send(Ingress { epoch, ev });
        }
    }
}

pub(crate) async fn run(
    server: FakeServer,
    ingress: IngressTx,
    mut commands: UnboundedReceiver<Command>,
    epoch: ConnEpoch,
) {
    let shared = server.shared;
    if let Ok(mut guard) = shared.lock() {
        guard.ingress = Some((ingress.clone(), epoch));
    }
    let emit = |ev: AppEv| {
        let _ = ingress.send(Ingress { epoch, ev });
    };
    emit(AppEv::Connected);
    while let Some(command) = commands.recv().await {
        let scripted = {
            let Ok(mut guard) = shared.lock() else {
                return;
            };
            guard.commands.push(command.clone());
            match &command {
                Command::StartTurn { .. } => {
                    guard.started_turns += 1;
                    Some((guard.started_turns, guard.turns.pop_front()))
                }
                _ => None,
            }
        };
        match command {
            Command::StartThread { ticket } => emit(AppEv::Reply(
                ticket,
                Reply::ThreadStarted {
                    thread: ThreadId::new(FAKE_THREAD),
                },
            )),
            Command::StartTurn {
                ticket,
                thread,
                text,
            } => {
                let (n, events) = scripted.unwrap_or((0, None));
                let turn_id = format!("turn-{n}");
                emit(AppEv::Reply(
                    ticket,
                    Reply::TurnStarted {
                        thread: thread.clone(),
                        turn: TurnId::new(&turn_id),
                    },
                ));
                // As the server does: the turn starts, the user message
                // is echoed back as an item.
                emit(AppEv::Backend(turn_started(thread.as_str(), &turn_id)));
                emit(AppEv::Backend(user_item(
                    thread.as_str(),
                    &turn_id,
                    &format!("user-{n}"),
                    &text,
                )));
                for event in events.unwrap_or_default() {
                    emit(AppEv::Backend(event));
                }
            }
            Command::RejectRequest { .. } => {}
        }
    }
}

fn note(notification: ServerNotification) -> BackendEv {
    BackendEv::Notification(Box::new(notification))
}

fn turn(id: &str, status: TurnStatus) -> Turn {
    Turn {
        id: id.to_string(),
        root_turn_id: None,
        items: Vec::new(),
        items_view: Default::default(),
        status,
        error: None,
        started_at: None,
        completed_at: None,
        duration_ms: None,
    }
}

/// `turn/started`, as the server sends it.
pub fn turn_started(thread: &str, turn_id: &str) -> BackendEv {
    note(ServerNotification::TurnStarted(TurnStartedNotification {
        thread_id: thread.to_string(),
        turn: turn(turn_id, TurnStatus::InProgress),
    }))
}

/// `item/started` of a user message.
pub fn user_item(thread: &str, turn_id: &str, item: &str, text: &str) -> BackendEv {
    note(ServerNotification::ItemStarted(ItemStartedNotification {
        item: ThreadItem::untimed(ThreadItemPayload::UserMessage {
            id: item.to_string(),
            client_id: None,
            content: vec![UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            }],
        }),
        thread_id: thread.to_string(),
        turn_id: turn_id.to_string(),
        started_at_ms: 0,
    }))
}

/// `item/agentMessage/delta`.
pub fn agent_delta(thread: &str, turn_id: &str, item: &str, delta: &str) -> BackendEv {
    note(ServerNotification::AgentMessageDelta(
        AgentMessageDeltaNotification {
            thread_id: thread.to_string(),
            turn_id: turn_id.to_string(),
            item_id: item.to_string(),
            delta: delta.to_string(),
        },
    ))
}

/// `item/completed` of an agent message.
pub fn agent_completed(thread: &str, turn_id: &str, item: &str, text: &str) -> BackendEv {
    note(ServerNotification::ItemCompleted(
        ItemCompletedNotification {
            item: ThreadItem::untimed(ThreadItemPayload::AgentMessage {
                moderation: None,
                id: item.to_string(),
                text: text.to_string(),
                phase: None,
                memory_citation: None,
                delivery: None,
                questions: None,
            }),
            thread_id: thread.to_string(),
            turn_id: turn_id.to_string(),
            content_token_count: None,
            completed_at_ms: 0,
        },
    ))
}

/// `turn/completed`.
pub fn turn_completed(thread: &str, turn_id: &str) -> BackendEv {
    note(ServerNotification::TurnCompleted(
        TurnCompletedNotification {
            thread_id: thread.to_string(),
            turn: turn(turn_id, TurnStatus::Completed),
        },
    ))
}

/// A `item/tool/requestUserInput` server request, the simplest kind that
/// carries a thread id.
/// `serverRequest/resolved`, as the server sends it when a request is
/// answered or withdrawn.
pub fn server_request_resolved(thread: &str, request_id: i64) -> BackendEv {
    note(ServerNotification::ServerRequestResolved(
        codex_app_server_protocol::ServerRequestResolvedNotification {
            thread_id: thread.to_string(),
            request_id: RequestId::Integer(request_id),
        },
    ))
}

pub fn tool_input_request(thread: &str, turn_id: &str, request_id: i64) -> BackendEv {
    BackendEv::Request(Box::new(ServerRequest::ToolRequestUserInput {
        request_id: RequestId::Integer(request_id),
        params: ToolRequestUserInputParams {
            thread_id: thread.to_string(),
            turn_id: turn_id.to_string(),
            item_id: "tool-1".to_string(),
            questions: Vec::new(),
            is_blocking: true,
            auto_resolution_ms: None,
        },
    }))
}
