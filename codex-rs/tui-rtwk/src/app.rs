//! `CodexApp`, the model of the rtwk app (ADR-003): plain data, one writer.
//!
//! Backend events, RPC replies and user intents are `AppEv` values that enter
//! one ordered ingress stream (R10); `apply_app` changes the model and returns
//! effects as data (`AppFx`); the shell runs them (R1). Nothing here touches
//! the network, the clock or the file system (R12).

use std::collections::BTreeMap;
use std::rc::Rc;

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use rtwk::GlobalKey;
use rtwk::app::AppCx;

use crate::ids::ConnEpoch;
use crate::ids::RequestKey;
use crate::ids::ServerRequestId;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TurnId;
use crate::screens;
use crate::screens::ScreenEv;
use crate::screens::ScreenIntent;
use crate::screens::ScreenStates;
use crate::shell;
use crate::threads;
use crate::threads::ThreadEv;
use crate::threads::ThreadFx;
use crate::threads::ThreadModel;
use crate::ui::thread_ui::ThreadUiMap;

/// Where the connection to the app-server stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Session {
    /// The client is starting or the first thread is being created; the
    /// composer is usable as a draft, a submit waits for `Ready`.
    Starting,
    Ready,
    Failed(String),
}

/// The session projection provided to the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSlice {
    pub status: Session,
    pub epoch: ConnEpoch,
    pub active: Option<ThreadId>,
}

/// A request waiting for the user: approvals, elicitations, user input. The
/// model's queue is the only source the presenter opens dialogs from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRequest {
    pub key: RequestKey,
    /// The wire method, e.g. `item/commandExecution/requestApproval`.
    pub method: String,
}

/// The queue of requests the user answers, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestQueue {
    pub entries: Vec<PendingRequest>,
}

impl RequestQueue {
    pub fn head(&self) -> Option<&PendingRequest> {
        self.entries.first()
    }

    pub fn contains(&self, key: &RequestKey) -> bool {
        self.entries.iter().any(|entry| &entry.key == key)
    }
}

/// An event from the backend, as the shell received it.
#[derive(Clone, Debug)]
pub enum BackendEv {
    Notification(Box<ServerNotification>),
    Request(Box<ServerRequest>),
    Lagged(usize),
}

/// The reply of one of our RPCs; it travels back with its `Ticket`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    ThreadStarted { thread: ThreadId },
    TurnStarted { thread: ThreadId, turn: TurnId },
    Failed(String),
}

/// What the user asks for. Intents that touch backend-shared or
/// cross-thread state are values sent into the ingress (R10).
#[derive(Clone, Debug)]
pub enum Intent {
    /// Send `text` to `thread`; `None` is the startup draft, which waits for
    /// the session to be `Ready`.
    Submit {
        thread: Option<ThreadId>,
        text: String,
    },
    /// Close a request without answering it (the skeleton's only answer).
    DismissRequest(RequestKey),
    Screen(ScreenIntent),
}

/// Everything that reaches `apply_app`, through one ordered stream.
#[derive(Clone, Debug)]
pub enum AppEv {
    /// The backend client is up.
    Connected,
    StartFailed(String),
    Disconnected(String),
    Backend(BackendEv),
    Reply(Ticket, Reply),
    Intent(Intent),
    Screen(ScreenEv),
}

/// An event with the connection epoch it was attached under.
#[derive(Clone, Debug)]
pub struct Ingress {
    pub epoch: ConnEpoch,
    pub ev: AppEv,
}

/// An RPC or reply the shell performs. Thread RPCs are built through
/// `ThreadFx`, which stamps the thread id (R3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    StartThread {
        ticket: Ticket,
    },
    StartTurn {
        ticket: Ticket,
        thread: ThreadId,
        text: String,
    },
    RejectRequest {
        key: RequestKey,
    },
}

/// Effects of one `apply_app` call, as values.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct AppFx {
    pub commands: Vec<Command>,
}

/// What an outgoing app-scoped RPC was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppCtx {
    StartThread,
}

/// The model.
pub struct CodexApp {
    pub session: Rc<SessionSlice>,
    pub threads: BTreeMap<ThreadId, Rc<ThreadModel>>,
    /// The projection of "no thread yet": the provider list stays fixed.
    pub blank_thread: Rc<ThreadModel>,
    pub requests: Rc<RequestQueue>,
    pub screens: Rc<ScreenStates>,
    /// Per-thread UI controllers (R2, R14); never seen by `apply`.
    pub thread_ui: ThreadUiMap,
    /// Open the debug menu at start (`--debug-screens`).
    pub debug_screens: bool,
    /// Keeps the `Navigator`'s stack across view runs (ADR-002, G15).
    pub nav_key: GlobalKey,
    pub(crate) link: shell::Link,
    pub(crate) next_ticket: u64,
    pub(crate) outgoing: BTreeMap<Ticket, AppCtx>,
    pub(crate) queued_submits: Vec<String>,
}

impl CodexApp {
    pub fn new(backend: shell::Backend) -> Self {
        Self {
            session: Rc::new(SessionSlice {
                status: Session::Starting,
                epoch: ConnEpoch::default(),
                active: None,
            }),
            threads: BTreeMap::new(),
            blank_thread: Rc::new(ThreadModel::blank()),
            requests: Rc::new(RequestQueue::default()),
            screens: Rc::new(ScreenStates::default()),
            thread_ui: ThreadUiMap::default(),
            debug_screens: false,
            nav_key: GlobalKey::with_label("navigator"),
            link: shell::Link::new(backend),
            next_ticket: 0,
            outgoing: BTreeMap::new(),
            queued_submits: Vec::new(),
        }
    }

    pub fn with_debug_screens(mut self, on: bool) -> Self {
        self.debug_screens = on;
        self
    }

    pub fn epoch(&self) -> ConnEpoch {
        self.session.epoch
    }

    /// The projection of the active thread, or the blank one.
    pub fn active_thread(&self) -> Rc<ThreadModel> {
        self.session
            .active
            .as_ref()
            .and_then(|id| self.threads.get(id))
            .cloned()
            .unwrap_or_else(|| self.blank_thread.clone())
    }

    pub(crate) fn fresh_ticket(&mut self) -> Ticket {
        self.next_ticket += 1;
        Ticket(self.next_ticket)
    }

    /// The ingress router: drops what an older connection sent (R1), applies
    /// the rest, hands the effects to the shell.
    pub fn ingest(&mut self, ingress: Ingress, cx: &mut AppCx<'_, '_, CodexApp>) {
    todo!("red: the parity tests come first")
}

    /// Puts an event into the ingress stream under the current epoch.
    pub fn send(&self, ev: AppEv) {
        shell::send(
            self,
            Ingress {
                epoch: self.epoch(),
                ev,
            },
        );
    }
}

/// Applies one event to the app-scoped model, thread events to their thread
/// model (`threads::apply`), and returns effects as data (R3, R10).
pub fn apply_app(app: &mut CodexApp, ev: AppEv, fx: &mut AppFx) {
    todo!("red: the parity tests come first")
}

fn set_status(app: &mut CodexApp, status: Session) {
    if app.session.status != status {
        Rc::make_mut(&mut app.session).status = status;
    }
}

fn apply_reply(app: &mut CodexApp, ticket: Ticket, reply: Reply, fx: &mut AppFx) {
    if let Some(AppCtx::StartThread) = app.outgoing.remove(&ticket) {
        match reply {
            Reply::ThreadStarted { thread } => {
                app.threads
                    .insert(thread.clone(), Rc::new(ThreadModel::new(thread.clone())));
                let session = Rc::make_mut(&mut app.session);
                session.active = Some(thread.clone());
                session.status = Session::Ready;
                for text in std::mem::take(&mut app.queued_submits) {
                    submit(app, &thread, text, fx);
                }
            }
            Reply::Failed(message) => set_status(app, Session::Failed(message)),
            Reply::TurnStarted { .. } => {}
        }
        return;
    }
    // A thread RPC: the thread whose model holds the ticket takes it; a
    // ticket nobody holds is stale and dropped.
    let owner = app
        .threads
        .iter()
        .find(|(_, model)| model.outgoing.contains_key(&ticket))
        .map(|(id, _)| id.clone());
    if let Some(thread) = owner {
        apply_thread(app, &thread, ThreadEv::Reply { ticket, reply }, fx);
    }
}

fn apply_intent(app: &mut CodexApp, intent: Intent, fx: &mut AppFx) {
    match intent {
        Intent::Submit { thread, text } => {
            let target = thread.or_else(|| app.session.active.clone());
            match target {
                Some(thread) => submit(app, &thread, text, fx),
                None => app.queued_submits.push(text),
            }
        }
        Intent::DismissRequest(key) => {
            let queue = Rc::make_mut(&mut app.requests);
            if queue.contains(&key) {
                queue.entries.retain(|entry| entry.key != key);
                fx.commands.push(Command::RejectRequest { key });
            }
        }
        Intent::Screen(intent) => screens::apply_intent(Rc::make_mut(&mut app.screens), intent, fx),
    }
}

fn submit(app: &mut CodexApp, thread: &ThreadId, text: String, fx: &mut AppFx) {
    let text = text.trim().to_string();
    if text.is_empty() || !app.threads.contains_key(thread) {
        return;
    }
    let ticket = app.fresh_ticket();
    apply_thread(app, thread, ThreadEv::Submit { ticket, text }, fx);
}

/// A thread event routed to its thread. Used by `apply_app` and by tests.
pub fn apply_thread(app: &mut CodexApp, thread: &ThreadId, ev: ThreadEv, fx: &mut AppFx) {
    if let Some(model) = app.threads.get_mut(thread) {
        threads::apply(Rc::make_mut(model), ev, &mut ThreadFx::new(thread, fx));
    }
}

/// The request a server request is, keyed by connection, thread and id (R3).
/// Read through the wire form: the request carries its thread id in its
/// params, whatever its kind.
fn pending_request(epoch: ConnEpoch, request: &ServerRequest) -> Option<PendingRequest> {
    let wire = serde_json::to_value(request).ok()?;
    let method = wire.get("method")?.as_str()?.to_string();
    let id = wire.get("id")?.to_string();
    let thread = wire
        .get("params")
        .and_then(|params| params.get("threadId"))
        .and_then(|thread| thread.as_str())
        .unwrap_or_default();
    Some(PendingRequest {
        key: RequestKey {
            epoch,
            thread: ThreadId::new(thread),
            id: ServerRequestId::new(id),
        },
        method,
    })
}
