//! The shell: the only place that starts tasks, owns the backend's command
//! queue and runs effects (R1). The backend runs as one future the shell
//! hands to `spawn_tokio`; everything it learns enters the app as one
//! ordered ingress stream the shell `subscribe`s to, each event stamped with
//! the epoch its connection was attached under.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use rtwk::app::AppCx;
use rtwk_tokio::TokioApp;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::AppFx;
use codex_tui_core::app::Command;
use codex_tui_core::app::Ingress;
use codex_tui_core::ids::BackendId;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::status::Ev as StatusEv;
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
        commands: UnboundedReceiver<Command>,
        epoch: ConnEpoch,
    ) -> BackendTask;
}

/// The app's end of the shell: set up at start, then only used to send.
pub struct Link {
    pub(crate) backend: Option<Backend>,
    /// The other backends the app connects next to the first (oracle:
    /// `InitialBackendSessions::register`), each under its own id.
    pub(crate) others: Vec<(BackendId, Backend)>,
    pub(crate) ingress: Option<IngressTx>,
    /// The first backend's commands (`BackendId::default()`).
    pub(crate) commands: Option<UnboundedSender<Command>>,
    /// The other backends' commands.
    pub(crate) other_commands: BTreeMap<BackendId, UnboundedSender<Command>>,
}

impl Link {
    pub fn new(backend: Backend) -> Self {
        Self {
            backend: Some(backend),
            others: Vec::new(),
            ingress: None,
            commands: None,
            other_commands: BTreeMap::new(),
        }
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
    cx.spawn_tokio(ambient(ingress_tx.clone(), epoch), |_, (), _| {});
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

/// How often the clock the screens show ages by is moved on.
const CLOCK_EVERY: std::time::Duration = std::time::Duration::from_secs(20);

/// The environment's facts the core cannot read (R12): the home directory
/// once, then the clock at every `CLOCK_EVERY`. The command center shows
/// `~` for the home and ages from the clock.
async fn ambient(ingress: IngressTx, epoch: ConnEpoch) {
    let home = std::env::var_os("HOME").map(|home| home.to_string_lossy().into_owned());
    emit(
        &ingress,
        epoch,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::Home(home))),
    );
    loop {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
        emit(
            &ingress,
            epoch,
            AppEv::Screen(ScreenEv::Agents(AgentsEv::Clock(now))),
        );
        // A `/status` card is made at this clock, in the local time zone.
        emit(
            &ingress,
            epoch,
            AppEv::Screen(ScreenEv::Status(StatusEv::Clock {
                now,
                utc_offset_secs: chrono::Local::now().offset().local_minus_utc(),
            })),
        );
        tokio::time::sleep(CLOCK_EVERY).await;
    }
}

/// Runs the effects of one `apply_app` call.
pub fn run_effects(app: &UiApp, fx: AppFx) {
    let Some(commands) = &app.link.commands else {
        return;
    };
    for command in fx.into_commands() {
        let backend = app.core.backend_of(&command);
        if let Some(target) = app.link.other_commands.get(&backend) {
            let _ = target.send(command);
        } else if backend == BackendId::default() {
            let _ = commands.send(command);
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
