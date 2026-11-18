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
use crate::ids::TicketMint;
use crate::rpc::Answer;
use crate::rpc::Pending;
use crate::rpc::Reply;
use crate::screens;
use crate::screens::ScreenCommand;
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
    #[non_exhaustive]
    StartThread { ticket: Ticket },
    #[non_exhaustive]
    StartTurn {
        ticket: Ticket,
        thread: ThreadId,
        text: String,
    },
    #[non_exhaustive]
    RejectRequest { key: RequestKey },
    /// An RPC of a screen slot (`ThreadFx::request`).
    #[non_exhaustive]
    Screen {
        ticket: Ticket,
        thread: ThreadId,
        command: ScreenCommand,
    },
}

/// Effects of one `apply_app` call, as values.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct AppFx {
    pub(crate) commands: Vec<Command>,
}

impl AppFx {
    /// The commands requested, in order.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
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
    pub(crate) mint: TicketMint,
    pub(crate) outgoing: Pending<AppCtx>,
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
            mint: TicketMint::default(),
            outgoing: Pending::default(),
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

    /// The ingress router: drops what an older connection sent (R1), applies
    /// the rest, hands the effects to the shell.
    pub fn ingest(&mut self, ingress: Ingress, cx: &mut AppCx<'_, '_, CodexApp>) {
        if ingress.epoch != self.epoch() {
            cx.unchanged();
            return;
        }
        let mut fx = AppFx::default();
        apply_app(self, ingress.ev, &mut fx);
        if let Some(active) = self.session.active.clone() {
            self.thread_ui.ensure(&active);
        }
        shell::run_effects(self, fx);
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
    match ev {
        AppEv::Connected => {
            let ticket = app.mint.next();
            app.outgoing.insert(ticket, AppCtx::StartThread);
            fx.commands.push(Command::StartThread { ticket });
        }
        AppEv::StartFailed(message) | AppEv::Disconnected(message) => {
            set_status(app, Session::Failed(message));
        }
        AppEv::Backend(BackendEv::Notification(notification)) => {
            // `serverRequest/resolved`: the server answered or withdrew a
            // request first; it leaves the queue and nothing is rejected.
            if let ServerNotification::ServerRequestResolved(resolved) = &*notification {
                let key = RequestKey {
                    epoch: app.epoch(),
                    thread: ThreadId::new(&resolved.thread_id),
                    id: ServerRequestId::new(
                        serde_json::to_string(&resolved.request_id).unwrap_or_default(),
                    ),
                };
                if app.requests.contains(&key) {
                    Rc::make_mut(&mut app.requests)
                        .entries
                        .retain(|entry| entry.key != key);
                }
                return;
            }
            if let Some((thread, ev)) = threads::project(&notification) {
                apply_thread(app, &thread, ev, fx);
            }
        }
        AppEv::Backend(BackendEv::Request(request)) => {
            if let Some(pending) = pending_request(app.epoch(), &request) {
                let queue = Rc::make_mut(&mut app.requests);
                if !queue.contains(&pending.key) {
                    queue.entries.push(pending);
                }
            }
        }
        AppEv::Backend(BackendEv::Lagged(skipped)) => {
            if let Some(id) = app.session.active.clone() {
                let message = format!("the backend dropped {skipped} events");
                if let Some(thread) = app.threads.get_mut(&id) {
                    Rc::make_mut(thread).last_error = Some(message);
                }
            }
        }
        AppEv::Reply(ticket, reply) => apply_reply(app, ticket, reply, fx),
        AppEv::Intent(intent) => apply_intent(app, intent, fx),
        AppEv::Screen(ev) => screens::apply_ev(Rc::make_mut(&mut app.screens), ev, fx),
    }
}

fn set_status(app: &mut CodexApp, status: Session) {
    if app.session.status != status {
        Rc::make_mut(&mut app.session).status = status;
    }
}

fn apply_reply(app: &mut CodexApp, ticket: Ticket, reply: Reply, fx: &mut AppFx) {
    if app.outgoing.contains(ticket) {
        let Some((AppCtx::StartThread, answer)) = app.outgoing.resolve(ticket, reply) else {
            return;
        };
        match answer {
            Answer::ThreadStarted { thread } => {
                app.threads
                    .insert(thread.clone(), Rc::new(ThreadModel::new(thread.clone())));
                let session = Rc::make_mut(&mut app.session);
                session.active = Some(thread.clone());
                session.status = Session::Ready;
                for text in std::mem::take(&mut app.queued_submits) {
                    submit(app, &thread, text, fx);
                }
            }
            Answer::Failed(message) => set_status(app, Session::Failed(message)),
            Answer::TurnStarted { .. } | Answer::Screen(_) => {}
        }
        return;
    }
    // A thread RPC: the thread whose model holds the ticket takes it; a
    // ticket nobody holds is stale and dropped.
    let owner = app
        .threads
        .iter()
        .find(|(_, model)| model.outgoing.contains(ticket))
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
                None => {
                    // Before the first thread: kept until it exists; the
                    // composer's draft clears (the blank projection's ticket).
                    app.queued_submits.push(text);
                    let ticket = app.mint.next();
                    Rc::make_mut(&mut app.blank_thread).last_submit = Some(ticket);
                }
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
    apply_thread(app, thread, ThreadEv::Submit { text }, fx);
}

/// A thread event routed to its thread. Used by `apply_app` and by tests.
pub fn apply_thread(app: &mut CodexApp, thread: &ThreadId, ev: ThreadEv, fx: &mut AppFx) {
    if let Some(model) = app.threads.get_mut(thread) {
        threads::apply(
            Rc::make_mut(model),
            ev,
            &mut ThreadFx::new(thread, fx, &mut app.mint),
        );
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
