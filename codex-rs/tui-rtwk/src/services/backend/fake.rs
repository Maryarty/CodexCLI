//! A scripted backend for screen tests: it answers our RPCs the way the
//! app-server does and plays the notifications the test scripts. It is one
//! future the shell runs on the tokio runtime and talks to the app only
//! through the ingress, as the real one does.

use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::Mutex;

use codex_app_server_protocol::ClientRequest;
use codex_tui_core::app::AppEv;
use codex_tui_core::app::BackendEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::Ingress;
use codex_tui_core::app::effects::{Done, HostDoors, ImageRead, ImageRevision, ImageSize, run_ask};
use codex_tui_core::app::worktree::Checkout as WorktreeCheckout;
use codex_tui_core::fixtures::builders::turn_started;
use codex_tui_core::fixtures::builders::user_item_with_receipt;
use codex_tui_core::ids::BackendId;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::ids::Ticket;
use codex_tui_core::ids::TurnId;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::rpc::{Address, Reply};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::DirKeys;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::import::Places;
use codex_tui_core::screens::usage::{
    AccountReply, Ask as UsageAsk, AskKind, Ev as UsageEv, LoadError as UsageError, ReportReply,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::oneshot;
use tokio::sync::watch;

use crate::services::agents::search::Gate;
use crate::services::agents::search::Latest;
use crate::services::agents::workspace::Boxed;
use crate::services::agents::workspace::Checkout;
use crate::services::agents::workspace::Facts;
use crate::services::agents::workspace::Workspace;
use crate::services::agents::workspace::account;
use crate::services::agents::workspace::backend_ev;
use crate::services::agents::workspace::directories;
use crate::services::agents::workspace::save_grouping;
use crate::services::agents::workspace::{bind_worktree, create_worktree};
use crate::services::backend::import;
use crate::services::backend::rpc::Ids;
use crate::services::backend::rpc::answer;
use crate::services::shell::Dispatch;
use crate::services::shell::IngressTx;

/// The id of the one thread the fake starts.
pub const FAKE_THREAD: &str = "thread-1";

#[derive(Default)]
pub(crate) struct Shared {
    commands: Vec<Command>,
    /// The events to play after each turn's `TurnStarted` reply, one entry
    /// per turn, in order.
    turns: VecDeque<Vec<BackendEv>>,
    started_turns: usize,
    ingress: Option<(IngressTx, ConnEpoch, BackendId)>,
    /// What the backend is, told to the app when it connects.
    facts: Option<Facts>,
    /// Answers our RPCs of the screens; without one they stay unanswered
    /// (a test replies with `inject`).
    responder: Option<Responder>,
    /// The wire requests that went out, as JSON, in order.
    requests: Vec<serde_json::Value>,
    /// History searches wait for this gate instead of the debounce's clock.
    search_gate: Option<Gate>,
    /// The first thread's start stays unanswered (a test replies with
    /// `inject`, the session is `Starting` until then).
    hold_thread_start: bool,
    /// A turn's start stays unanswered while its images are prepared (a test ends the wait with
    /// `inject` of the failure, the host's `prepare_images`).
    hold_turn_start: bool,
    /// The requests of these methods are answered only once their hold is released.
    held: BTreeMap<String, Hold>,
    /// The words before a turn's number in the id the fake mints (`mint_turns_as`); `turn-`
    /// without one.
    turn_prefix: Option<String>,
    /// What the `/usage` page's host answers (`FakeHandle::usage_accounts`, `usage_report`).
    usage: UsageHost,
    /// Files the test made available to the local image executor.
    image_paths: BTreeSet<String>,
}

/// The account lookups and reports the usage page asks of its host, answered from what a test
/// says: the lookups one per ask in order (the last stays for every ask after it, so a plan
/// that changes under an open page is two answers), the reports all the same. With none said
/// the asks stay recorded and unanswered (a test answers with `inject`).
#[derive(Default)]
struct UsageHost {
    accounts: VecDeque<Result<AccountReply, String>>,
    report: Option<Result<ReportReply, String>>,
}

impl UsageHost {
    fn account(&mut self) -> Option<Result<AccountReply, String>> {
        if self.accounts.len() > 1 {
            self.accounts.pop_front()
        } else {
            self.accounts.front().cloned()
        }
    }
}

/// A request the fake holds in flight (`FakeBackend::hold_request`): it is logged when the app
/// sends it, and answered as the responder says only after `release`. Cloned, every copy is the
/// same hold; once released it stays released.
#[derive(Clone, Debug)]
pub struct Hold(Arc<watch::Sender<bool>>);

impl Default for Hold {
    fn default() -> Self {
        Self::new()
    }
}

impl Hold {
    pub fn new() -> Self {
        Self(Arc::new(watch::channel(false).0))
    }

    /// Lets the held requests (and the later ones of the method) be answered.
    pub fn release(&self) {
        self.0.send_replace(true);
    }

    async fn released(&self) {
        let mut state = self.0.subscribe();
        // The sender is held by this hold, so the wait cannot fail.
        let _ = state.wait_for(|open| *open).await;
    }
}

/// What the scripted server answers to a wire request (as JSON): `None`
/// leaves it unanswered.
pub type Responder =
    Arc<dyn Fn(&serde_json::Value) -> Option<Result<serde_json::Value, ServerError>> + Send + Sync>;

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
    saved: Vec<String>,
    save: Option<Result<(), String>>,
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

    /// The groupings written to the config.
    pub fn saved(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|state| state.saved.clone())
            .unwrap_or_default()
    }

    /// Makes the next config writes fail with `error` (`None`: succeed).
    pub fn fail_save(&self, error: Option<&str>) {
        if let Ok(mut state) = self.state.lock() {
            state.save = Some(error.map_or(Ok(()), |error| Err(error.to_owned())));
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

    fn save_grouping(&self, grouping: &str) -> Boxed<Result<(), String>> {
        let result = self
            .state
            .lock()
            .map(|mut state| {
                state.saved.push(grouping.to_owned());
                state.save.clone().unwrap_or(Ok(()))
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
    barriers: UnboundedReceiver<oneshot::Sender<()>>,
}

/// The test's side: script, inspect, inject.
#[derive(Clone)]
pub struct FakeHandle {
    shared: Arc<Mutex<Shared>>,
    workspace: Arc<FakeWorkspace>,
    barriers: UnboundedSender<oneshot::Sender<()>>,
}

/// A barrier asked of the fake's command loop (`FakeHandle::barrier`).
pub struct Barrier(oneshot::Receiver<()>);

impl Barrier {
    /// Whether the loop has taken every command that was sent before the
    /// barrier was asked (a loop that ended counts: nothing more comes).
    pub fn passed(&mut self) -> bool {
        !matches!(self.0.try_recv(), Err(oneshot::error::TryRecvError::Empty))
    }
}

impl FakeServer {
    pub fn new() -> (FakeServer, FakeHandle) {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let workspace = Arc::new(FakeWorkspace::default());
        let (barrier_tx, barriers) = unbounded_channel();
        (
            FakeServer {
                shared: shared.clone(),
                workspace: workspace.clone(),
                barriers,
            },
            FakeHandle {
                shared,
                workspace,
                barriers: barrier_tx,
            },
        )
    }
}

impl FakeHandle {
    pub fn allow_image_snapshot(&self, path: String) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.image_paths.insert(path);
        }
    }

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
            if let Some((ingress, epoch, backend)) = &shared.ingress {
                let _ = ingress.send(Ingress::of(*backend, *epoch, backend_ev(facts)));
            }
        }
    }

    /// Answers the wire requests of the screens with `responder`.
    pub fn respond(
        &self,
        responder: impl Fn(&serde_json::Value) -> Option<Result<serde_json::Value, ServerError>>
        + Send
        + Sync
        + 'static,
    ) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.responder = Some(Arc::new(responder));
        }
    }

    /// Holds the first page of every history search at `gate` instead of
    /// the debounce's 180 ms: it goes out when the test opens the gate (no
    /// wall clock in the test). Set before the app starts.
    pub fn hold_searches(&self, gate: &Gate) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.search_gate = Some(gate.clone());
        }
    }

    /// Mints the ids of the turns it starts as `{prefix}{n}` (`fake-turn-1`) instead of `turn-{n}`:
    /// a script's own turns are `turn-1`.., and a turn the app started under the same id would be
    /// the script's turn, not another. Set before the first turn.
    pub fn mint_turns_as(&self, prefix: &str) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.turn_prefix = Some(prefix.to_string());
        }
    }

    /// Holds every request of wire `method` (`account/rateLimits/read`) in flight: the app has
    /// sent it (it is in `requests`) and gets its answer only after `hold.release()`, as the
    /// responder says it then. The state of a screen waiting on one request (`Refreshing...`,
    /// `Loading`) is then a state a test can look at while other requests are answered at once.
    pub fn hold_request(&self, method: &str, hold: &Hold) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.held.insert(method.to_string(), hold.clone());
        }
    }

    /// The answers to the usage page's account lookups, one per ask in order; the last one
    /// answers every ask after it. `Err` is a lookup that failed (the host's message). A plan
    /// that changes while a page is open is `[Ok(plus), Ok(enterprise)]`: the page's refresh
    /// gets the second.
    pub fn usage_accounts(&self, answers: Vec<Result<AccountReply, String>>) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.usage.accounts = answers.into();
        }
    }

    /// The answer to every report the usage page asks for (without one the report stays
    /// loading).
    pub fn usage_report(&self, answer: Result<ReportReply, String>) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.usage.report = Some(answer);
        }
    }

    /// Leaves every `StartThread` unanswered: the session stays `Starting`
    /// until the test injects the reply.
    pub fn hold_thread_start(&self) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.hold_thread_start = true;
        }
    }

    /// Leaves every `StartTurn` unanswered: the message is in flight (its images being
    /// prepared) until the test injects the reply.
    pub fn hold_turn_start(&self) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.hold_turn_start = true;
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

    /// A barrier on the command loop: it passes once the loop has taken and
    /// handled every command the app sent before this call (a search that a
    /// newer one drops is dropped by then). The app sends a command
    /// synchronously from its handler, so a test that has seen the screen
    /// show a key's effect has the key's commands in the queue. Without it a
    /// test measures whether the loop got to them yet.
    pub fn barrier(&self) -> Barrier {
        let (done, passed) = oneshot::channel();
        let _ = self.barriers.send(done);
        Barrier(passed)
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
            .and_then(|shared| shared.ingress.as_ref().map(|(_, epoch, _)| *epoch))
    }

    /// Delivers `ev` as if the backend had sent it, under `epoch` (a stale
    /// epoch must be dropped).
    pub fn inject(&self, epoch: ConnEpoch, ev: AppEv) {
        if let Ok(shared) = self.shared.lock()
            && let Some((ingress, _, backend)) = &shared.ingress
        {
            let _ = ingress.send(Ingress::of(*backend, epoch, ev));
        }
    }

    /// Delivers an envelope scripted with an explicit transport or failure ticket.
    pub fn inject_ingress(&self, event: Ingress) {
        if let Ok(shared) = self.shared.lock()
            && let Some((ingress, _, _)) = &shared.ingress
        {
            let _ = ingress.send(event);
        }
    }
}

pub(crate) async fn run(
    server: FakeServer,
    ingress: IngressTx,
    commands: UnboundedReceiver<Dispatch>,
    epoch: ConnEpoch,
) {
    run_as(server, None, ingress, commands, epoch).await;
}

/// A backend next to the first: it does not connect the app (no
/// `Connected`, no account), and says only that it is the cloud when its
/// facts say so.
pub(crate) async fn run_other(
    server: FakeServer,
    id: BackendId,
    ingress: IngressTx,
    commands: UnboundedReceiver<Dispatch>,
    epoch: ConnEpoch,
) {
    run_as(server, Some(id), ingress, commands, epoch).await;
}

async fn run_as(
    server: FakeServer,
    other: Option<BackendId>,
    ingress: IngressTx,
    mut commands: UnboundedReceiver<Dispatch>,
    epoch: ConnEpoch,
) {
    let shared = server.shared;
    let mut barriers = server.barriers;
    let backend = other.unwrap_or_default();
    if let Ok(mut guard) = shared.lock() {
        guard.ingress = Some((ingress.clone(), epoch, backend));
    }
    let emit = |ev: AppEv| {
        let _ = ingress.send(Ingress::of(backend, epoch, ev));
    };
    let facts = shared.lock().ok().and_then(|guard| guard.facts);
    match other {
        None => {
            emit(AppEv::Connected);
            if let Some(facts) = facts {
                emit(backend_ev(facts));
            }
        }
        Some(id) => {
            if facts.is_some_and(|facts| facts.cloud) {
                emit(AppEv::Screen(ScreenEv::Agents(AgentsEv::CloudBackend(id))));
            }
        }
    }
    let ids = Ids::default();
    let gate = shared
        .lock()
        .ok()
        .and_then(|guard| guard.search_gate.clone());
    let mut latest = gate
        .as_ref()
        .map_or_else(Latest::default, Latest::paused_by);
    // As the host does once connected: the account is read, for the usage.
    if other.is_none()
        && let Some(responder) = shared.lock().ok().and_then(|guard| guard.responder.clone())
    {
        let (ingress, ids, log) = (ingress.clone(), ids.clone(), shared.clone());
        tokio::spawn(async move {
            let send = request_sender(log, responder);
            if let Some(ev) = account(ids, send).await {
                let _ = ingress.send(Ingress::of(backend, epoch, ev));
            }
        });
    }
    loop {
        // Commands first: a barrier is seen only when none is queued.
        let dispatch = tokio::select! {
            biased;
            command = commands.recv() => match command {
                Some(command) => command,
                None => return,
            },
            Some(done) = barriers.recv() => {
                let _ = done.send(());
                continue;
            }
        };
        let Some((command, stamp)) = dispatch.on(backend, epoch) else {
            continue;
        };
        let scripted = {
            let Ok(mut guard) = shared.lock() else {
                return;
            };
            guard.commands.push(command.clone());
            match &command {
                Command::StartTurn { .. } => {
                    guard.started_turns += 1;
                    Some((
                        guard.started_turns,
                        guard.turns.pop_front(),
                        guard.turn_prefix.clone(),
                    ))
                }
                _ => None,
            }
        };
        match command {
            Command::StartThread { .. }
                if shared.lock().is_ok_and(|guard| guard.hold_thread_start) => {}
            // A responder that answers `thread/start` is the server: the reply is the production
            // decode of what it says (the instruction sources come with it).
            Command::StartThread { .. } if scripts_thread_start(&shared, &command, &ids) => {
                let facts = facts.unwrap_or(Facts {
                    cloud: false,
                    worktrees: false,
                });
                let responder = shared.lock().ok().and_then(|guard| guard.responder.clone());
                let Some(responder) = responder else { continue };
                let (ingress, ids, log) = (ingress.clone(), ids.clone(), shared.clone());
                tokio::spawn(async move {
                    let send = request_sender(log, responder);
                    if let Some((ticket, reply)) = answer(command, facts, ids, None, send).await {
                        let _ =
                            ingress.send(Ingress::of(backend, epoch, AppEv::Reply(ticket, reply)));
                    }
                });
            }
            Command::StartThread { ticket, .. } => emit(AppEv::Reply(
                ticket,
                // A further backend's thread has an id of its own.
                Reply::thread_started(ThreadId::new(&match other {
                    None => FAKE_THREAD.to_owned(),
                    Some(id) => format!("{FAKE_THREAD}-{}", id.0),
                })),
            )),
            Command::StartTurn { .. } if shared.lock().is_ok_and(|guard| guard.hold_turn_start) => {
            }
            Command::StartTurn {
                ticket,
                thread,
                message,
                client_id,
                ..
            } => {
                let (n, events, prefix) = scripted.unwrap_or((0, None, None));
                let turn_id = format!("{}{n}", prefix.as_deref().unwrap_or("turn-"));
                emit(AppEv::Reply(
                    ticket,
                    Reply::turn_started(thread.clone(), TurnId::new(&turn_id)),
                ));
                // As the server does: the turn starts, the user message
                // is echoed back as an item.
                emit(AppEv::Backend(turn_started(thread.as_str(), &turn_id)));
                emit(AppEv::Backend(user_item_with_receipt(
                    thread.as_str(),
                    &turn_id,
                    &format!("user-{n}"),
                    &message.text,
                    Some(client_id),
                )));
                for event in events.unwrap_or_default() {
                    emit(AppEv::Backend(event));
                }
            }
            Command::RejectRequest { .. }
            | Command::ResolveRequest { .. }
            | Command::InterruptTurn { .. } => {}
            Command::Effect { request, ask, .. } => {
                let doors = FakeDoors {
                    request,
                    shared: &shared,
                    ids: &ids,
                    workspace: &server.workspace,
                };
                match run_ask(&doors, ask).await {
                    Some(FakeWork::Ready(event)) => emit(*event),
                    Some(FakeWork::Spawn(job)) => {
                        let ingress = ingress.clone();
                        tokio::spawn(async move {
                            if let Some(event) = job.await {
                                let reply = match stamp {
                                    Some(stamp) => {
                                        Ingress::transport_event(backend, epoch, stamp, event)
                                    }
                                    None => Ingress::of(backend, epoch, event),
                                };
                                let _ = ingress.send(reply);
                            }
                        });
                    }
                    None => {}
                }
            }
            other @ (Command::Screen { .. }
            | Command::Feature { .. }
            | Command::ReadRateLimits { .. }
            | Command::SendAddCreditsNudge { .. }) => {
                let (responder, facts) = shared
                    .lock()
                    .map(|guard| (guard.responder.clone(), guard.facts))
                    .unwrap_or((None, None));
                let Some(responder) = responder else { continue };
                let facts = facts.unwrap_or(Facts {
                    cloud: false,
                    worktrees: false,
                });
                let ingress = ingress.clone();
                let ids = ids.clone();
                let log = shared.clone();
                let held = other.clone();
                let work: Boxed<Option<()>> = Box::pin(async move {
                    let send = request_sender(log, responder);
                    if let Some((ticket, reply)) = answer(other, facts, ids, None, send).await {
                        let _ =
                            ingress.send(Ingress::of(backend, epoch, AppEv::Reply(ticket, reply)));
                    }
                    Some(())
                });
                tokio::spawn(latest.wrap(&held, work));
            }
        }
    }
}

/// The responder answers `thread/start` (asked without logging it).
/// Scripts can answer immediately or do the same asynchronous work as the
/// host. Unscripted capabilities remain recorded for the test to answer.
struct FakeDoors<'a> {
    request: u64,
    shared: &'a Arc<Mutex<Shared>>,
    ids: &'a Ids,
    workspace: &'a Arc<FakeWorkspace>,
}

enum FakeWork {
    Ready(Box<AppEv>),
    Spawn(Boxed<Option<AppEv>>),
}

impl FakeDoors<'_> {
    fn responder(&self) -> Option<Responder> {
        self.shared.lock().ok()?.responder.clone()
    }
}

impl HostDoors for FakeDoors<'_> {
    type Out = FakeWork;

    async fn image_snapshot(
        &self,
        _thread: ThreadId,
        path: String,
        preview: Option<ImageSize>,
        previous: Option<ImageRevision>,
    ) -> Option<FakeWork> {
        let allowed = self.shared.lock().ok()?.image_paths.contains(&path);
        let snapshot = if allowed {
            crate::services::images::local_snapshot(std::path::Path::new(&path), preview, previous)
        } else {
            ImageRead::Unavailable
        };
        let request = self.request;
        Some(FakeWork::Spawn(Box::pin(async move {
            Some(AppEv::Effect {
                request,
                done: Done::ImageSnapshot(snapshot),
            })
        })))
    }

    async fn import(&self, ask: codex_tui_core::screens::import::Request) -> Option<FakeWork> {
        let responder = self.responder()?;
        let (log, ids) = (self.shared.clone(), self.ids.clone());
        Some(FakeWork::Spawn(Box::pin(async move {
            let site = import::Site {
                cwd: None,
                places: Box::new(|_| Places::default()),
            };
            let send = request_sender(log, responder);
            import::run(ask, site, ids, send).await
        })))
    }

    async fn git_summary(
        &self,
        _thread: ThreadId,
        cwd: String,
        _address: Address,
    ) -> Option<FakeWork> {
        let responder = self.responder()?;
        let (log, ids, request) = (self.shared.clone(), self.ids.clone(), self.request);
        Some(FakeWork::Spawn(Box::pin(async move {
            let exec =
                crate::services::exec::AppServerExec::new(request_sender(log, responder), ids);
            let summary =
                crate::services::git_summary::read(&exec, std::path::Path::new(&cwd)).await;
            Some(AppEv::Effect {
                request,
                done: Done::GitSummary(summary),
            })
        })))
    }

    async fn directories(&self, cwds: Vec<String>) -> Option<FakeWork> {
        let workspace = self.workspace.clone();
        Some(FakeWork::Spawn(Box::pin(async move {
            Some(directories(workspace, cwds).await)
        })))
    }

    async fn save_grouping(&self, grouping: String) -> Option<FakeWork> {
        let workspace = self.workspace.clone();
        Some(FakeWork::Spawn(Box::pin(async move {
            save_grouping(workspace, grouping).await
        })))
    }

    async fn create_worktree(&self, ticket: Ticket, cwd: Option<String>) -> Option<FakeWork> {
        let workspace = self.workspace.clone();
        Some(FakeWork::Spawn(Box::pin(async move {
            Some(create_worktree(workspace, ticket, cwd).await)
        })))
    }

    async fn bind_worktree(
        &self,
        ticket: Ticket,
        checkout: WorktreeCheckout,
        thread: ThreadId,
        cwd: String,
    ) -> Option<FakeWork> {
        let workspace = self.workspace.clone();
        Some(FakeWork::Spawn(Box::pin(async move {
            Some(bind_worktree(workspace, ticket, checkout, thread, cwd).await)
        })))
    }

    async fn usage(&self, ask: UsageAsk) -> Option<FakeWork> {
        let UsageAsk {
            session,
            stamp,
            what,
        } = ask;
        let event = match what {
            AskKind::Account => {
                let result = self
                    .shared
                    .lock()
                    .ok()?
                    .usage
                    .account()?
                    .map_err(UsageError::from);
                UsageEv::Account {
                    session,
                    stamp,
                    result,
                }
            }
            AskKind::Report { section, .. } => {
                let result = self
                    .shared
                    .lock()
                    .ok()?
                    .usage
                    .report
                    .clone()?
                    .map_err(UsageError::from);
                UsageEv::Report {
                    session,
                    stamp,
                    section,
                    result,
                }
            }
            _ => return None,
        };
        Some(FakeWork::Ready(Box::new(AppEv::Screen(ScreenEv::Usage(
            event,
        )))))
    }
}

fn scripts_thread_start(shared: &Arc<Mutex<Shared>>, command: &Command, ids: &Ids) -> bool {
    let responder = shared.lock().ok().and_then(|guard| guard.responder.clone());
    let Some(responder) = responder else {
        return false;
    };
    let Some(request) = codex_tui_core::protocol::client_request(command, ids.next(), None) else {
        return false;
    };
    responder(&serde_json::to_value(&request).unwrap_or_default()).is_some()
}

/// The client of the fake: it logs the request and answers it as the
/// responder says (unanswered stays pending).
fn request_sender(
    log: Arc<Mutex<Shared>>,
    responder: Responder,
) -> impl Fn(ClientRequest) -> Boxed<Result<serde_json::Value, ServerError>> {
    move |request: ClientRequest| {
        let json = serde_json::to_value(&request).unwrap_or_default();
        if let Ok(mut guard) = log.lock() {
            guard.requests.push(json.clone());
        }
        let held = json["method"]
            .as_str()
            .and_then(|method| log.lock().ok()?.held.get(method).cloned());
        if let Some(hold) = held {
            let (log, responder) = (log.clone(), responder.clone());
            return Box::pin(async move {
                hold.released().await;
                // The table as it is by then: a test may have changed an answer while it held.
                let now = log.lock().ok().and_then(|guard| guard.responder.clone());
                match now.unwrap_or(responder)(&json) {
                    Some(result) => result,
                    None => std::future::pending().await,
                }
            });
        }
        let answered = responder(&json);
        Box::pin(async move {
            match answered {
                Some(result) => result,
                None => std::future::pending().await,
            }
        })
    }
}
