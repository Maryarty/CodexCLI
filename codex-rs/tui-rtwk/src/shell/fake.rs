//! A scripted backend for screen tests: it answers our RPCs the way the
//! app-server does and plays the notifications the test scripts. It is one
//! future the shell runs on the tokio runtime and talks to the app only
//! through the ingress, as the real one does.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

use codex_app_server_protocol::ClientRequest;
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
use codex_tui_core::screens::agents::DirKeys;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::shell::IngressTx;
use crate::shell::rpc::Ids;
use crate::shell::rpc::answer;
use crate::shell::workspace::Boxed;
use crate::shell::workspace::Checkout;
use crate::shell::workspace::Facts;
use crate::shell::workspace::Workspace;
use crate::shell::workspace::backend_ev;
use crate::shell::workspace::directories;
use crate::shell::workspace::worktree;

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
    /// What the backend is, told to the app when it connects.
    facts: Option<Facts>,
    /// Answers our RPCs of the screens; without one they stay unanswered
    /// (a test replies with `inject`).
    responder: Option<Responder>,
    /// The wire requests that went out, as JSON, in order.
    requests: Vec<serde_json::Value>,
}

/// What the scripted server answers to a wire request (as JSON): `None`
/// leaves it unanswered.
pub type Responder =
    Arc<dyn Fn(&serde_json::Value) -> Option<Result<serde_json::Value, String>> + Send + Sync>;

/// A scripted machine: the directory keys it knows, the outcome of creating
/// a worktree, and what was asked of it.
#[derive(Default)]
pub struct FakeWorkspace {
    state: Mutex<WorkspaceState>,
}

#[derive(Default)]
struct WorkspaceState {
    keys: Vec<(String, DirKeys)>,
    create: Option<Result<Checkout, String>>,
    bind: Option<Result<(), String>>,
    key_reads: Vec<String>,
    created: Vec<Option<String>>,
    bound: Vec<(Checkout, String, String)>,
}

impl FakeWorkspace {
    /// The keys of `cwd`.
    pub fn set_keys(&self, cwd: &str, keys: DirKeys) {
        if let Ok(mut state) = self.state.lock() {
            state.keys.push((cwd.to_owned(), keys));
        }
    }

    /// What creating a worktree gives (default: a failure).
    pub fn set_create(&self, result: Result<Checkout, String>) {
        if let Ok(mut state) = self.state.lock() {
            state.create = Some(result);
        }
    }

    /// What binding the task gives (default: success).
    pub fn set_bind(&self, result: Result<(), String>) {
        if let Ok(mut state) = self.state.lock() {
            state.bind = Some(result);
        }
    }

    /// The `cwd`s whose keys were read.
    pub fn key_reads(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|state| state.key_reads.clone())
            .unwrap_or_default()
    }

    /// The source directories worktrees were created for.
    pub fn created(&self) -> Vec<Option<String>> {
        self.state
            .lock()
            .map(|state| state.created.clone())
            .unwrap_or_default()
    }

    /// The `(checkout, thread, started cwd)` of each bind.
    pub fn bound(&self) -> Vec<(Checkout, String, String)> {
        self.state
            .lock()
            .map(|state| state.bound.clone())
            .unwrap_or_default()
    }
}

impl Workspace for FakeWorkspace {
    fn dir_keys(&self, cwd: &str) -> Option<DirKeys> {
        let mut state = self.state.lock().ok()?;
        state.key_reads.push(cwd.to_owned());
        state
            .keys
            .iter()
            .find(|(held, _)| held == cwd)
            .map(|(_, keys)| keys.clone())
    }

    fn create_worktree(&self, source_cwd: Option<String>) -> Boxed<Result<Checkout, String>> {
        let result = self
            .state
            .lock()
            .map(|mut state| {
                state.created.push(source_cwd);
                state
                    .create
                    .clone()
                    .unwrap_or_else(|| Err("no worktree scripted".to_owned()))
            })
            .unwrap_or_else(|_| Err("poisoned".to_owned()));
        Box::pin(std::future::ready(result))
    }

    fn bind_worktree(&self, checkout: &Checkout, thread: &str, cwd: &str) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "poisoned".to_owned())?;
        state
            .bound
            .push((checkout.clone(), thread.to_owned(), cwd.to_owned()));
        state.bind.clone().unwrap_or(Ok(()))
    }
}

/// The server side, handed to the app as `Backend::Fake`.
pub struct FakeServer {
    shared: Arc<Mutex<Shared>>,
    workspace: Arc<FakeWorkspace>,
}

/// The test's side: script, inspect, inject.
#[derive(Clone)]
pub struct FakeHandle {
    shared: Arc<Mutex<Shared>>,
    workspace: Arc<FakeWorkspace>,
}

impl FakeServer {
    pub fn new() -> (FakeServer, FakeHandle) {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let workspace = Arc::new(FakeWorkspace::default());
        (
            FakeServer {
                shared: shared.clone(),
                workspace: workspace.clone(),
            },
            FakeHandle { shared, workspace },
        )
    }
}

impl FakeHandle {
    /// The scripted machine (git and the file system).
    pub fn workspace(&self) -> &Arc<FakeWorkspace> {
        &self.workspace
    }

    /// Tells the app what the backend is, as the host does once it is
    /// connected (`Ev::Backend`): once now if the app is connected, and again
    /// at each later connection.
    pub fn announce(&self, facts: Facts) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.facts = Some(facts);
            if let Some((ingress, epoch)) = &shared.ingress {
                let _ = ingress.send(Ingress {
                    epoch: *epoch,
                    ev: backend_ev(facts),
                });
            }
        }
    }

    /// Answers the wire requests of the screens with `responder`.
    pub fn respond(
        &self,
        responder: impl Fn(&serde_json::Value) -> Option<Result<serde_json::Value, String>>
        + Send
        + Sync
        + 'static,
    ) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.responder = Some(Arc::new(responder));
        }
    }

    /// The wire requests that went out (a page of the list is one per
    /// source), as JSON.
    pub fn requests(&self) -> Vec<serde_json::Value> {
        self.shared
            .lock()
            .map(|shared| shared.requests.clone())
            .unwrap_or_default()
    }

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
    let facts = shared.lock().ok().and_then(|guard| guard.facts);
    if let Some(facts) = facts {
        emit(backend_ev(facts));
    }
    let ids = Ids::default();
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
            // Slot RPCs are recorded; a test replies with `inject`, or the
            // responder answers them as the client does.
            Command::RejectRequest { .. } => {}
            Command::Directories { cwds, .. } => {
                let workspace = server.workspace.clone();
                let ingress = ingress.clone();
                tokio::spawn(async move {
                    let ev = directories(workspace, cwds).await;
                    let _ = ingress.send(Ingress { epoch, ev });
                });
            }
            other @ (Command::Worktree { .. } | Command::Screen { .. }) => {
                let (responder, facts) = shared
                    .lock()
                    .map(|guard| (guard.responder.clone(), guard.facts))
                    .unwrap_or((None, None));
                let Some(responder) = responder else { continue };
                let facts = facts.unwrap_or(Facts {
                    cloud: false,
                    worktrees: false,
                });
                let workspace = server.workspace.clone();
                let ingress = ingress.clone();
                let ids = ids.clone();
                let log = shared.clone();
                tokio::spawn(async move {
                    let send = move |request: ClientRequest| {
                        let json = serde_json::to_value(&request).unwrap_or_default();
                        if let Ok(mut guard) = log.lock() {
                            guard.requests.push(json.clone());
                        }
                        let answered = responder(&json);
                        async move {
                            match answered {
                                Some(result) => result,
                                None => std::future::pending().await,
                            }
                        }
                    };
                    match other {
                        Command::Worktree { cwd, .. } => {
                            let ev = worktree(workspace, cwd, ids, send).await;
                            let _ = ingress.send(Ingress { epoch, ev });
                        }
                        command => {
                            if let Some((ticket, reply)) =
                                answer(command, facts, ids, None, send).await
                            {
                                let _ = ingress.send(Ingress {
                                    epoch,
                                    ev: AppEv::Reply(ticket, reply),
                                });
                            }
                        }
                    }
                });
            }
        }
    }
}
