//! `CodexApp`, the core model of the app (ADR-003): plain data, one writer.
//! It holds no UI state (the UI crate's `UiApp` wraps it) and names no rtwk.
//!
//! Backend events, RPC replies and user intents are `AppEv` values that enter
//! one ordered ingress stream (R10); `apply_app` changes the model and returns
//! effects as data (`AppFx`); the shell runs them (R1). Nothing here touches
//! the network, the clock or the file system (R12).

use std::collections::BTreeMap;
use std::rc::Rc;

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;

use crate::ids::BackendId;
use crate::ids::ConnEpoch;
use crate::ids::RequestKey;
use crate::ids::ServerRequestId;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TicketMint;
use crate::rpc::Answer;
use crate::rpc::Pending;
use crate::rpc::Reply;
use crate::rpc::RpcError;
use crate::screens;
use crate::screens::ScreenCommand;
use crate::screens::ScreenCtx;
use crate::screens::ScreenEv;
use crate::screens::ScreenIntent;
use crate::screens::ScreenReply;
use crate::screens::ScreenStates;
use crate::screens::agents;
use crate::threads;
use crate::threads::Failure;
use crate::threads::ThreadEv;
use crate::threads::ThreadFx;
use crate::threads::ThreadModel;

/// Where the connection to the app-server stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Session {
    /// The client is starting or the first thread is being created; the
    /// composer is usable as a draft, a submit waits for `Ready`.
    Starting,
    Ready,
    Failed(Failure),
}

/// The session projection provided to the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSlice {
    pub status: Session,
    /// The first backend's connection epoch (`BackendId::default()`); the
    /// other backends' are `epochs`.
    pub epoch: ConnEpoch,
    /// The epoch of each further backend's connection: one per backend, so
    /// a reconnect of one never drops the other's events.
    pub epochs: BTreeMap<BackendId, ConnEpoch>,
    pub active: Option<ThreadId>,
    /// The backend control above the composer (oracle:
    /// `visible_new_backend_control`): the displayed backend's name, while
    /// the conversation is empty and another backend is connected.
    pub cycle: Option<BackendLabel>,
    /// How many times an empty conversation moved to another backend, and
    /// the thread it left: the composer's draft goes with it.
    pub switches: u32,
    pub switched_from: Option<ThreadId>,
}

/// The name of a backend as the control shows it (oracle:
/// `BackendSessions::displayed_name`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendLabel {
    Local,
    Cloud,
    /// The n-th connection, counting from 1.
    Connection(u32),
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
    /// Move the empty conversation to the next connected backend (oracle:
    /// `CycleNewConversationBackend`).
    CycleBackend,
    Screen(ScreenIntent),
}

/// Everything that reaches `apply_app`, through one ordered stream.
#[derive(Clone, Debug)]
pub enum AppEv {
    /// The backend client is up.
    Connected,
    /// The client did not start; the client's message.
    StartFailed(String),
    /// The backend stopped.
    Disconnected,
    Backend(BackendEv),
    Reply(Ticket, Reply),
    Intent(Intent),
    Screen(ScreenEv),
}

/// An event with the connection epoch it was attached under, and the
/// backend it came from (each backend has its own epoch).
#[derive(Clone, Debug)]
pub struct Ingress {
    pub backend: BackendId,
    pub epoch: ConnEpoch,
    pub ev: AppEv,
}

impl Ingress {
    /// An event of the first backend.
    pub fn new(epoch: ConnEpoch, ev: AppEv) -> Self {
        Self::of(BackendId::default(), epoch, ev)
    }

    /// An event of `backend`.
    pub fn of(backend: BackendId, epoch: ConnEpoch, ev: AppEv) -> Self {
        Self { backend, epoch, ev }
    }
}

/// An RPC or reply the shell performs. Thread RPCs are built through
/// `ThreadFx`, which stamps the thread id (R3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    #[non_exhaustive]
    StartThread { ticket: Ticket, backend: BackendId },
    #[non_exhaustive]
    StartTurn {
        ticket: Ticket,
        thread: ThreadId,
        text: String,
    },
    #[non_exhaustive]
    RejectRequest { key: RequestKey },
    /// A new worktree in `cwd`'s repository with a task in it: git work the
    /// shell does; it answers with the agents screen's `Ev::Notice`.
    #[non_exhaustive]
    Worktree { cwd: Option<String> },
    /// The directory keys of these `cwd`s (canonical path, repository):
    /// file-system reads the shell does; it answers with the agents
    /// screen's `Ev::Directories`.
    #[non_exhaustive]
    Directories { cwds: Vec<String> },
    /// Writes `tui.agents_overview_grouping` (`project`, `status`, `model`):
    /// a config edit the shell does; a failure comes back as the agents
    /// screen's `Ev::Notice`.
    #[non_exhaustive]
    SaveGrouping { grouping: String },
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
    /// The app cannot go on: the connection did not start (the oracle ends
    /// with the error, `run_main_inner` returning it).
    pub(crate) ends: bool,
}

impl AppFx {
    /// The commands, taken: what the shell runs.
    pub fn into_commands(self) -> Vec<Command> {
        self.commands
    }

    /// Whether the app ends: its connection did not start.
    pub fn ends_app(&self) -> bool {
        self.ends
    }

    /// The commands requested, in order.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}

/// What an outgoing app-scoped RPC was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppCtx {
    /// The first thread of a connection, or one a cycle started on `backend`
    /// (`None`: the first).
    StartThread { switch: Option<BackendId> },
    /// An RPC of an app-scoped screen (the agents command center).
    Screen(ScreenCtx),
}

/// The model.
pub struct CodexApp {
    pub session: Rc<SessionSlice>,
    pub threads: BTreeMap<ThreadId, Rc<ThreadModel>>,
    /// The projection of "no thread yet": the provider list stays fixed.
    pub blank_thread: Rc<ThreadModel>,
    pub requests: Rc<RequestQueue>,
    pub screens: Rc<ScreenStates>,
    pub(crate) mint: TicketMint,
    pub(crate) outgoing: Pending<AppCtx>,
    pub(crate) queued_submits: Vec<String>,
    /// Whether connecting starts the first thread. `codex agents` opens the
    /// overview instead (oracle: `agents_overview`), and a task starts when
    /// the user asks for one.
    pub start_thread_on_connect: bool,
}

impl Default for CodexApp {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexApp {
    pub fn new() -> Self {
        Self {
            session: Rc::new(SessionSlice {
                status: Session::Starting,
                epoch: ConnEpoch::default(),
                epochs: BTreeMap::new(),
                active: None,
                cycle: None,
                switches: 0,
                switched_from: None,
            }),
            threads: BTreeMap::new(),
            blank_thread: Rc::new(ThreadModel::blank()),
            requests: Rc::new(RequestQueue::default()),
            screens: Rc::new(ScreenStates::default()),
            mint: TicketMint::default(),
            outgoing: Pending::default(),
            queued_submits: Vec::new(),
            start_thread_on_connect: true,
        }
    }

    /// The app-scoped RPCs whose answer is still awaited.
    pub fn in_flight(&self) -> usize {
        self.outgoing.len()
    }

    pub fn epoch(&self) -> ConnEpoch {
        self.session.epoch
    }

    /// The backend a command goes to: a task's owner for what is about a
    /// task, the displayed backend for the rest (oracle: the owner's
    /// session answers for its tasks). `BackendId::default()` is the first
    /// connection.
    pub fn backend_of(&self, command: &Command) -> crate::ids::BackendId {
        let agents = &self.screens.agents;
        match command {
            Command::Screen {
                command: ScreenCommand::Agents(command),
                ..
            } => agents::backend_of(agents, command),
            Command::StartTurn { thread, .. } => agents.owner_of(thread.as_str()),
            Command::RejectRequest { key } => agents.owner_of(key.thread.as_str()),
            Command::StartThread { backend, .. } => *backend,
            _ => agents.displayed,
        }
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

    /// The epoch of `backend`'s connection.
    pub fn epoch_of(&self, backend: BackendId) -> ConnEpoch {
        if backend == BackendId::default() {
            self.session.epoch
        } else {
            self.session
                .epochs
                .get(&backend)
                .copied()
                .unwrap_or_default()
        }
    }

    /// Attaches the next connection of the first backend: the epoch every
    /// event of it is stamped with (R1). Called once per connection, by the
    /// shell's start.
    pub fn attach(&mut self) -> ConnEpoch {
        self.attach_backend(BackendId::default())
    }

    /// Attaches the next connection of `backend`: its own epoch, so that a
    /// reconnect of one backend never drops the events of another.
    pub fn attach_backend(&mut self, backend: BackendId) -> ConnEpoch {
        let epoch = self.epoch_of(backend).next();
        let session = Rc::make_mut(&mut self.session);
        if backend == BackendId::default() {
            session.epoch = epoch;
        } else {
            session.epochs.insert(backend, epoch);
        }
        epoch
    }

    /// The ingress router: drops what an older connection of its backend
    /// sent (`None`, R1), applies the rest and returns the effects for the
    /// shell to run.
    pub fn ingest(&mut self, ingress: Ingress) -> Option<AppFx> {
        if ingress.epoch != self.epoch_of(ingress.backend) {
            return None;
        }
        let mut fx = AppFx::default();
        apply_app_from(self, ingress.backend, ingress.ev, &mut fx);
        Some(fx)
    }
}

/// Applies one event to the app-scoped model, thread events to their thread
/// model (`threads::apply`), and returns effects as data (R3, R10).
pub fn apply_app(app: &mut CodexApp, ev: AppEv, fx: &mut AppFx) {
    apply_app_from(app, BackendId::default(), ev, fx);
}

/// `apply_app` for an event of backend `origin` (notifications and requests
/// carry the backend they came from).
pub fn apply_app_from(app: &mut CodexApp, origin: BackendId, ev: AppEv, fx: &mut AppFx) {
    // Only events that can bring tasks of other directories into the center.
    let lists = matches!(ev, AppEv::Reply(..) | AppEv::Screen(_));
    apply_event(app, origin, ev, fx);
    refresh_cycle(app);
    if lists && agents::wants_dirs(&app.screens.agents) {
        let cwds = agents::wanted_dirs(&mut Rc::make_mut(&mut app.screens).agents);
        if !cwds.is_empty() {
            fx.commands.push(Command::Directories { cwds });
        }
    }
}

fn apply_event(app: &mut CodexApp, origin: BackendId, ev: AppEv, fx: &mut AppFx) {
    match ev {
        AppEv::Connected if !app.start_thread_on_connect => {
            Rc::make_mut(&mut app.session).status = Session::Ready;
        }
        AppEv::Connected => {
            let ticket = app.mint.next();
            app.outgoing
                .insert(ticket, AppCtx::StartThread { switch: None });
            fx.commands.push(Command::StartThread {
                ticket,
                backend: BackendId::default(),
            });
        }
        AppEv::StartFailed(message) => {
            fx.ends = true;
            set_status(app, Session::Failed(Failure::Message(message)));
        }
        AppEv::Disconnected => set_status(app, Session::Failed(Failure::Stopped)),
        AppEv::Backend(BackendEv::Notification(notification)) => {
            // A thread another backend owns is not this backend's to speak
            // of (oracle: `BackendRegistry::observe`); an unowned one is
            // claimed by the backend that did.
            if let Some(thread) = agents::notification_thread(&notification) {
                match app.screens.agents.owners.get(thread) {
                    Some(owner) if *owner != origin => return,
                    Some(_) => {}
                    None => {
                        Rc::make_mut(&mut app.screens)
                            .agents
                            .observe(origin, thread);
                    }
                }
            }
            // `serverRequest/resolved`: the server answered or withdrew a
            // request first; it leaves the queue and nothing is rejected.
            if let ServerNotification::ServerRequestResolved(resolved) = &*notification {
                let key = RequestKey {
                    epoch: app.epoch_of(origin),
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
            if agents::wants(&app.screens.agents, &notification) {
                let outs =
                    agents::observe(&mut Rc::make_mut(&mut app.screens).agents, &notification);
                send_agents(app, outs, fx);
            }
            if let Some((thread, ev)) = threads::project(&notification) {
                apply_thread(app, &thread, ev, fx);
            }
        }
        AppEv::Backend(BackendEv::Request(request)) => {
            if let Some(pending) = pending_request(app.epoch_of(origin), &request) {
                let queue = Rc::make_mut(&mut app.requests);
                if !queue.contains(&pending.key) {
                    queue.entries.push(pending);
                }
            }
        }
        AppEv::Backend(BackendEv::Lagged(skipped)) => {
            // Events were dropped: what the command center derived from them
            // is stale and its list is read again (oracle: `Lagged`).
            let screens = Rc::make_mut(&mut app.screens);
            agents::apply(&mut screens.agents, agents::Ev::Lagged, fx);
            let outs = agents::update(&mut screens.agents, agents::Intent::Open);
            send_agents(app, outs, fx);
            if let Some(id) = app.session.active.clone()
                && let Some(thread) = app.threads.get_mut(&id)
            {
                Rc::make_mut(thread).last_error = Some(Failure::Lagged(skipped));
            }
        }
        AppEv::Reply(ticket, reply) => apply_reply(app, ticket, reply, fx),
        AppEv::Intent(intent) => apply_intent(app, intent, fx),
        AppEv::Screen(ev) => {
            if let ScreenEv::Agents(agents::Ev::Started(thread)) = &ev {
                resume_into_chat(app, thread);
            }
            let reconnected = matches!(ev, ScreenEv::Agents(agents::Ev::Reconnected));
            screens::apply_ev(Rc::make_mut(&mut app.screens), ev, fx);
            if reconnected {
                // A new connection: the list is read again (oracle:
                // `finish_reconnect` leaves the overview to refresh).
                let outs = agents::update(
                    &mut Rc::make_mut(&mut app.screens).agents,
                    agents::Intent::Open,
                );
                send_agents(app, outs, fx);
            }
        }
    }
}

fn set_status(app: &mut CodexApp, status: Session) {
    if app.session.status != status {
        Rc::make_mut(&mut app.session).status = status;
    }
}

fn apply_reply(app: &mut CodexApp, ticket: Ticket, reply: Reply, fx: &mut AppFx) {
    if app.outgoing.contains(ticket) {
        let Some((ctx, answer)) = app.outgoing.resolve(ticket, reply) else {
            return;
        };
        match ctx {
            AppCtx::StartThread { switch } => {
                match (switch, answer) {
                    (Some(backend), Answer::ThreadStarted { thread }) => {
                        switch_to(app, backend, thread);
                    }
                    (Some(_), Answer::Failed(error)) => {
                        // The oracle shows the error and stays where it was.
                        if let Some(active) = app.session.active.clone()
                            && let Some(model) = app.threads.get_mut(&active)
                        {
                            Rc::make_mut(model).last_error = Some(Failure::from(error));
                        }
                    }
                    (Some(_), _) => {}
                    (None, answer) => started(app, answer, fx),
                }
            }
            AppCtx::Screen(ctx) => screen_answer(app, ctx, answer, fx),
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
        Intent::CycleBackend => {
            let switching = app
                .outgoing
                .any(|ctx| matches!(ctx, AppCtx::StartThread { switch: Some(_) }));
            if app.session.cycle.is_none() || switching {
                return;
            }
            let Some(backend) = app.screens.agents.next_backend() else {
                return;
            };
            let ticket = app.mint.next();
            app.outgoing.insert(
                ticket,
                AppCtx::StartThread {
                    switch: Some(backend),
                },
            );
            fx.commands.push(Command::StartThread { ticket, backend });
        }
        Intent::DismissRequest(key) => {
            let queue = Rc::make_mut(&mut app.requests);
            if queue.contains(&key) {
                queue.entries.retain(|entry| entry.key != key);
                fx.commands.push(Command::RejectRequest { key });
            }
        }
        Intent::Screen(ScreenIntent::Agents(intent)) => {
            let screens = Rc::make_mut(&mut app.screens);
            if matches!(intent, agents::Intent::Open) {
                // Opened over a chat: its thread is the one `esc` returns to
                // and the one the list starts on.
                screens.agents.current =
                    app.session.active.as_ref().map(|id| id.as_str().to_owned());
            }
            let outs = agents::update(&mut screens.agents, intent);
            send_agents(app, outs, fx);
        }
        Intent::Screen(intent) => screens::apply_intent(Rc::make_mut(&mut app.screens), intent, fx),
    }
}

/// Mints a ticket for each RPC the agents screen asks for and sends it. The
/// RPCs are app-scoped: the command carries no thread.
fn send_agents(app: &mut CodexApp, outs: Vec<agents::Out>, fx: &mut AppFx) {
    for out in outs {
        match out.command {
            agents::Command::NewWorktree { cwd } => {
                fx.commands.push(Command::Worktree { cwd });
                continue;
            }
            agents::Command::SaveGrouping { grouping } => {
                fx.commands.push(Command::SaveGrouping { grouping });
                continue;
            }
            _ => {}
        }
        // The shell drops the search before a new one (it never answers):
        // its ticket is released here (R5).
        if matches!(out.command, agents::Command::Search { .. }) {
            app.outgoing.retain(|ctx| {
                !matches!(
                    ctx,
                    AppCtx::Screen(ScreenCtx::Agents(agents::Ctx::Search { .. }))
                )
            });
        }
        let ticket = app.mint.next();
        app.outgoing
            .insert(ticket, AppCtx::Screen(ScreenCtx::Agents(out.ctx)));
        fx.commands.push(Command::Screen {
            ticket,
            thread: ThreadId::default(),
            command: ScreenCommand::Agents(out.command),
        });
    }
}

/// The reply of an app-scoped screen RPC.
fn screen_answer(app: &mut CodexApp, ctx: ScreenCtx, answer: Answer, fx: &mut AppFx) {
    let ScreenCtx::Agents(ctx) = ctx;
    let result = match answer {
        Answer::Screen(ScreenReply::Agents(reply)) => Ok(reply),
        Answer::Failed(error) => Err(error),
        _ => Err(RpcError::Mismatch),
    };
    if let (
        agents::Ctx::Resume { .. } | agents::Ctx::Fork | agents::Ctx::Start,
        Ok(
            agents::Reply::Resumed(thread)
            | agents::Reply::Forked(thread)
            | agents::Reply::Started(thread),
        ),
    ) = (&ctx, &result)
    {
        resume_into_chat(app, thread);
    }
    let outs = agents::apply_answer(&mut Rc::make_mut(&mut app.screens).agents, ctx, result);
    send_agents(app, outs, fx);
}

/// The first thread's answer.
fn started(app: &mut CodexApp, answer: Answer, fx: &mut AppFx) {
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
        Answer::Failed(error) => set_status(app, Session::Failed(Failure::from(error))),
        Answer::TurnStarted { .. } | Answer::Screen(_) => {}
    }
}

/// An empty conversation moved to `backend` (oracle:
/// `start_new_session_on_other_backend`): the new thread is the chat's, its
/// backend the displayed one; the composer's draft goes with it.
fn switch_to(app: &mut CodexApp, backend: BackendId, thread: ThreadId) {
    app.threads
        .insert(thread.clone(), Rc::new(ThreadModel::new(thread.clone())));
    let session = Rc::make_mut(&mut app.session);
    session.switched_from = session.active.replace(thread.clone());
    session.switches += 1;
    session.status = Session::Ready;
    let agents = &mut Rc::make_mut(&mut app.screens).agents;
    agents.observe(backend, thread.as_str());
    agents.display(backend);
    agents.current = Some(thread.as_str().to_owned());
    agents::reset_for_backend_switch(agents);
}

/// Sets `session.cycle`: the control is offered while the conversation is
/// empty and another backend is connected (oracle:
/// `can_cycle_conversation_backend` and `visible_new_backend_control`).
fn refresh_cycle(app: &mut CodexApp) {
    let agents = &app.screens.agents;
    let thread = app
        .session
        .active
        .as_ref()
        .and_then(|id| app.threads.get(id));
    let empty = thread.is_some_and(|thread| {
        thread.transcript.is_empty()
            && !thread.is_running()
            && thread.queue.is_empty()
            && thread.last_submit.is_none()
    });
    let cycle = (app.session.status == Session::Ready && empty && agents.next_backend().is_some())
        .then(|| agents.label());
    if app.session.cycle != cycle {
        Rc::make_mut(&mut app.session).cycle = cycle;
    }
}

/// The resumed thread becomes the chat's: its model is made and active.
fn resume_into_chat(app: &mut CodexApp, thread: &codex_app_server_protocol::Thread) {
    let id = ThreadId::new(&thread.id);
    app.threads
        .entry(id.clone())
        .or_insert_with(|| Rc::new(ThreadModel::new(id.clone())));
    let session = Rc::make_mut(&mut app.session);
    session.active = Some(id);
    session.status = Session::Ready;
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
