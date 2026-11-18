//! A scripted backend for screen tests: it answers our RPCs the way the
//! app-server does and plays the notifications the test scripts. It is one
//! future the shell runs on the tokio runtime and talks to the app only
//! through the ingress, as the real one does.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::BackendEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::Ingress;
use codex_tui_core::fixtures::builders::turn_started;
use codex_tui_core::fixtures::builders::user_item;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::ids::TurnId;
use codex_tui_core::rpc::Reply;
use tokio::sync::mpsc::UnboundedReceiver;

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
            Command::StartThread { ticket, .. } => emit(AppEv::Reply(
                ticket,
                Reply::thread_started(ThreadId::new(FAKE_THREAD)),
            )),
            Command::StartTurn {
                ticket,
                thread,
                text,
                ..
            } => {
                let (n, events) = scripted.unwrap_or((0, None));
                let turn_id = format!("turn-{n}");
                emit(AppEv::Reply(
                    ticket,
                    Reply::turn_started(thread.clone(), TurnId::new(&turn_id)),
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
            // Slot RPCs are recorded; a test replies with `inject`.
            Command::RejectRequest { .. } | Command::Worktree { .. } | Command::Screen { .. } => {}
        }
    }
}
