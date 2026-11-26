//! The shell: the only place that starts tasks, owns the backend's command
//! queue and runs effects (R1). The backend runs as one future the shell
//! hands to `spawn_tokio`; everything it learns enters the app as one
//! ordered ingress stream the shell `subscribe`s to, each event stamped with
//! the epoch its connection was attached under.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use rtwk::app::AppCx;
use rtwk_tokio::TokioApp;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch;
use tokio::time::Instant;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::AppFx;
use codex_tui_core::app::Command;
use codex_tui_core::app::Ingress;
use codex_tui_core::app::Now;
use codex_tui_core::app::Wake;
use codex_tui_core::ids::BackendId;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::rpc::Address;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::transcript::diff;

use crate::app::UiApp;

/// The id of the Cloud backend, the second connection of the app.
pub const CLOUD_BACKEND: BackendId = BackendId(1);

/// What the shell connects the app to.
pub enum Backend {
    /// Nothing: state tests that never start the shell.
    None,
    /// A real server, connected by the host (`codex-rtwk`: the in-process
    /// app-server, in `host/src/in_process.rs`).
    Connector(Box<dyn Connector>),
    /// A scripted server for screen tests.
    Fake(super::backend::fake::FakeServer),
    /// The first backend and others next to it (oracle:
    /// `InitialBackendSessions::register`), each under its own id. The
    /// others never send `Connected` or `StartFailed` (those are the
    /// first's); they speak through the same ingress, each under its own
    /// epoch.
    Several {
        first: Box<Backend>,
        others: Vec<(BackendId, Backend)>,
    },
}

/// Where a backend puts what it learns.
pub type IngressTx = UnboundedSender<Ingress>;

/// The backend's whole life as one future, run by the shell on the tokio
/// runtime.
pub type BackendTask = Pin<Box<dyn Future<Output = ()> + Send>>;

/// A queued command with the request handle it is allowed to use. Local work has no such stamp.
pub struct Dispatch {
    command: Command,
    address: Option<Address>,
}

impl Dispatch {
    pub fn new(command: Command, address: Option<Address>) -> Self {
        Self { command, address }
    }

    /// Open the envelope just before selecting a handle. Its reply retains that handle's epoch.
    pub fn on(
        self,
        backend: BackendId,
        transport_epoch: ConnEpoch,
    ) -> Option<(Command, Option<ConnEpoch>)> {
        if self
            .address
            .is_some_and(|address| address.backend != backend || address.epoch != transport_epoch)
        {
            return None;
        }
        Some((self.command, self.address.map(|address| address.epoch)))
    }
}

/// Take the next command usable on the selected request handle; discard queued stale RPCs.
pub async fn next_dispatch(
    commands: &mut UnboundedReceiver<Dispatch>,
    backend: BackendId,
    transport_epoch: ConnEpoch,
) -> Option<(Command, Option<ConnEpoch>)> {
    while let Some(dispatch) = commands.recv().await {
        if let Some(command) = dispatch.on(backend, transport_epoch) {
            return Some(command);
        }
    }
    None
}

/// A backend the host connects. It speaks to the app only through the
/// ingress and the commands, and it never spawns: its work is the future it
/// returns. The library itself links no server, so its tests build and link
/// fast.
pub trait Connector: Send {
    /// The backend under `epoch`: send `Connected` (or `StartFailed`) into
    /// `ingress`, answer `commands`, and finish when `commands` closes.
    fn run(
        self: Box<Self>,
        ingress: IngressTx,
        commands: UnboundedReceiver<Dispatch>,
        epoch: ConnEpoch,
    ) -> BackendTask;
}

/// What the shell reads the time from (ADR-012, "time as a declared wake"). The core never reads
/// a clock: the shell reads this one and tells the app (`AppEv::Clock`), at the start and when
/// the moment the app asked for (`AppFx::wake`) has come. The host's is the wall clock; a test
/// gives the link a [`FixedClock`] and says the time itself.
pub trait Clock: Send + Sync {
    fn now(&self) -> Now;
}

/// The wall clock, in the local time zone.
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> Now {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(0));
        Now::at_millis(millis).in_zone(chrono::Local::now().offset().local_minus_utc())
    }
}

/// A clock that stands where it was put (`set`): what a test hands a link, so no test waits for
/// a real minute or reads the real date.
pub struct FixedClock(Mutex<Now>);

impl FixedClock {
    pub fn at(now: Now) -> Arc<Self> {
        Arc::new(Self(Mutex::new(now)))
    }

    pub fn set(&self, now: Now) {
        if let Ok(mut held) = self.0.lock() {
            *held = now;
        }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Now {
        self.0.lock().map(|held| *held).unwrap_or_default()
    }
}

/// The app's end of the shell: set up at start, then only used to send.
pub struct Link {
    pub(crate) backend: Option<Backend>,
    /// The other backends the app connects next to the first (oracle:
    /// `InitialBackendSessions::register`), each under its own id.
    pub(crate) others: Vec<(BackendId, Backend)>,
    pub(crate) ingress: Option<IngressTx>,
    /// The first backend's commands (`BackendId::default()`).
    pub(crate) commands: Option<UnboundedSender<Dispatch>>,
    /// The other backends' commands.
    pub(crate) other_commands: BTreeMap<BackendId, UnboundedSender<Dispatch>>,
    /// Where the time is read (`Clock`).
    pub(crate) clock: Arc<dyn Clock>,
    /// The nearest moment the app wants the clock told, as of its last step, for the timer.
    pub(crate) wake: Option<watch::Sender<Option<Wake>>>,
}

impl Link {
    pub fn new(backend: Backend) -> Self {
        Self {
            backend: Some(backend),
            others: Vec::new(),
            ingress: None,
            commands: None,
            other_commands: BTreeMap::new(),
            clock: Arc::new(WallClock),
            wake: None,
        }
    }

    /// Reads the time from `clock` instead of the wall clock.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

/// Starts the connection: subscribes to the ingress, attaches the next
/// epoch and runs the backend on the tokio runtime. Called once, from the
/// root's `on_start`.
pub fn start(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let Some(mut backend) = app.link.backend.take() else {
        return;
    };
    if let Backend::Several { first, others } = backend {
        app.link.others = others;
        backend = *first;
    }
    // Nothing to connect: state tests and seeded models never start the shell.
    if matches!(backend, Backend::None) {
        return;
    }
    let (ingress_tx, mut ingress_rx) = tokio::sync::mpsc::unbounded_channel::<Ingress>();
    cx.subscribe(
        futures::stream::poll_fn(move |task| ingress_rx.poll_recv(task)),
        |app: &mut UiApp, ingress, cx| app.ingest(ingress, cx),
        |_, _| {},
    );
    let epoch = app.core.attach();
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    app.link.ingress = Some(ingress_tx.clone());
    app.link.commands = Some(command_tx);
    let (wake_tx, wake_rx) = watch::channel(None);
    app.link.wake = Some(wake_tx);
    cx.spawn_tokio(
        ambient(ingress_tx.clone(), epoch, app.link.clock.clone(), wake_rx),
        |_, (), _| {},
    );
    let task: BackendTask = match backend {
        Backend::None | Backend::Several { .. } => return,
        Backend::Connector(connector) => connector.run(ingress_tx.clone(), command_rx, epoch),
        Backend::Fake(server) => Box::pin(super::backend::fake::run(
            server,
            ingress_tx.clone(),
            command_rx,
            epoch,
        )),
    };
    cx.spawn_tokio(task, |_, (), _| {});
    for (id, backend) in std::mem::take(&mut app.link.others) {
        // Its own epoch: a reconnect of one backend never drops the other's.
        let epoch = app.core.attach_backend(id);
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let task: BackendTask = match backend {
            Backend::None | Backend::Several { .. } => continue,
            Backend::Connector(connector) => connector.run(ingress_tx.clone(), command_rx, epoch),
            Backend::Fake(server) => Box::pin(super::backend::fake::run_other(
                server,
                id,
                ingress_tx.clone(),
                command_rx,
                epoch,
            )),
        };
        app.link.other_commands.insert(id, command_tx);
        cx.spawn_tokio(task, |_, (), _| {});
    }
}

/// The environment's facts the core cannot read (R12), and the one timer of the app: the home
/// directory once, the time at the start, then the time again whenever the moment the app asked
/// for has come (`AppFx::wake`, kept up to date by `run_effects`). Nothing here ticks on a
/// schedule: with no wake asked for there is no timer, and what is woken (an age in minutes, a
/// usage window) says when. The timer is armed when the app names a moment other than the one
/// it waits for, so a stream of keys that ask the same moment does not push it back.
async fn ambient(
    ingress: IngressTx,
    epoch: ConnEpoch,
    clock: Arc<dyn Clock>,
    mut wake: watch::Receiver<Option<Wake>>,
) {
    let home = std::env::var_os("HOME").map(|home| home.to_string_lossy().into_owned());
    emit(
        &ingress,
        epoch,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::Home(home))),
    );
    emit(&ingress, epoch, AppEv::Clock(clock.now()));
    // The moment waited for, and when on the runtime's clock it is.
    let mut armed: Option<(Now, Instant)> = None;
    loop {
        let changed = match armed {
            None => wake.changed().await.map(|()| false).map_err(|_| ()),
            Some((_, deadline)) => match tokio::time::timeout_at(deadline, wake.changed()).await {
                Ok(changed) => changed.map(|()| false).map_err(|_| ()),
                Err(_elapsed) => Ok(true),
            },
        };
        match changed {
            Err(()) => return,
            // The moment has come: tell the app the time; its next step names the next one.
            Ok(true) => {
                armed = None;
                emit(&ingress, epoch, AppEv::Clock(clock.now()));
            }
            Ok(false) => {
                let asked = *wake.borrow_and_update();
                armed = match (asked, armed) {
                    (None, _) => None,
                    (Some(next), Some((at, deadline))) if next.at() == at => Some((at, deadline)),
                    (Some(next), _) => Some((next.at(), Instant::now() + next.after())),
                };
            }
        }
    }
}

/// Runs the effects of one `apply_app` call.
pub fn run_effects(app: &UiApp, fx: AppFx) {
    if let Some(timer) = &app.link.wake {
        timer.send_replace(fx.wake());
    }
    let Some(commands) = &app.link.commands else {
        return;
    };
    for (command, tell) in fx.into_dispatch() {
        let Some((backend, address)) = app.core.dispatch_route(&command, tell) else {
            continue;
        };
        let dispatch = Dispatch::new(command, address);
        if let Some(target) = app.link.other_commands.get(&backend) {
            let _ = target.send(dispatch);
        } else if backend == BackendId::default() {
            let _ = commands.send(dispatch);
        }
    }
}

/// Puts an event into the ingress under `ingress.epoch`.
pub fn send(app: &UiApp, ingress: Ingress) {
    if let Some(sender) = &app.link.ingress {
        let _ = sender.send(ingress);
    }
}

/// Puts `ev` of the first backend into `ingress` under `epoch` (for
/// connectors).
pub fn emit(ingress: &IngressTx, epoch: ConnEpoch, ev: AppEv) {
    emit_from(ingress, BackendId::default(), epoch, ev);
}

/// Puts `ev` of `backend` into `ingress` under that backend's `epoch`.
pub fn emit_from(ingress: &IngressTx, backend: BackendId, epoch: ConnEpoch, ev: AppEv) {
    let _ = ingress.send(Ingress::of(backend, epoch, ev));
}

/// Where `/diff` runs its commands (oracle `WorkspaceCommandExecutor`): in the
/// workspace of the active session, which may be another machine. The host
/// implements it over the app server's one-off `command/exec` (argv, cwd, env,
/// timeout, output cap from the call's limits); a test implements it with a
/// script. An `Err` is the command not running at all, not a non-zero exit.
pub trait WorkspaceExec: Send + Sync {
    fn run(
        &self,
        call: diff::Call,
        cwd: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<diff::Output, String>> + Send + '_>>;
}

/// The diff of `cwd` (oracle `get_git_diff`): the core's step machine driven
/// to its end over `exec`, each batch of commands in order.
pub async fn git_diff(exec: &dyn WorkspaceExec, cwd: &Path) -> Result<diff::Report, diff::Failure> {
    let (mut machine, mut step) = diff::Diff::start();
    loop {
        match step {
            diff::DiffStep::Done(result) => return result,
            diff::DiffStep::Run(calls) => {
                let mut outputs = Vec::with_capacity(calls.len());
                for call in calls {
                    outputs.push(exec.run(call, cwd).await);
                }
                step = machine.feed(outputs);
            }
        }
    }
}
