//! The shell: the only place that starts tasks, owns the backend's command
//! queue and runs effects (R1). The backend runs as one future the shell
//! hands to `spawn_tokio`; everything it learns enters the app as one
//! ordered ingress stream the shell `subscribe`s to, each event stamped with
//! the epoch its connection was attached under.
//!
//! The clippy `disallowed_methods` allow below is the only one in the
//! library; `tests/rules.rs` rejects any other.
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

pub mod fake;

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use rtwk::app::AppCx;
use rtwk_tokio::TokioApp;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppEv;
use crate::app::AppFx;
use crate::app::CodexApp;
use crate::app::Command;
use crate::app::Ingress;
use crate::ids::ConnEpoch;

/// What the shell connects the app to.
pub enum Backend {
    /// Nothing: state tests that never start the shell.
    None,
    /// A real server, connected by the host (`codex-rtwk`: the in-process
    /// app-server, in `host/src/in_process.rs`).
    Connector(Box<dyn Connector>),
    /// A scripted server for screen tests.
    Fake(fake::FakeServer),
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
    pub(crate) ingress: Option<IngressTx>,
    pub(crate) commands: Option<UnboundedSender<Command>>,
}

impl Link {
    pub fn new(backend: Backend) -> Self {
        Self {
            backend: Some(backend),
            ingress: None,
            commands: None,
        }
    }
}

/// Starts the connection: subscribes to the ingress, attaches the next
/// epoch and runs the backend on the tokio runtime. Called once, from the
/// root's `on_start`.
pub fn start(app: &mut CodexApp, cx: &mut AppCx<'_, '_, CodexApp>) {
    let Some(backend) = app.link.backend.take() else {
        return;
    };
    let (ingress_tx, mut ingress_rx) = tokio::sync::mpsc::unbounded_channel::<Ingress>();
    cx.subscribe(
        futures::stream::poll_fn(move |task| ingress_rx.poll_recv(task)),
        |app: &mut CodexApp, ingress, cx| app.ingest(ingress, cx),
        |_, _| {},
    );
    let epoch = app.session.epoch.next();
    Rc::make_mut(&mut app.session).epoch = epoch;
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    app.link.ingress = Some(ingress_tx.clone());
    app.link.commands = Some(command_tx);
    let task: BackendTask = match backend {
        Backend::None => return,
        Backend::Connector(connector) => connector.run(ingress_tx, command_rx, epoch),
        Backend::Fake(server) => Box::pin(fake::run(server, ingress_tx, command_rx, epoch)),
    };
    cx.spawn_tokio(task, |_, (), _| {});
}

/// Runs the effects of one `apply_app` call.
pub fn run_effects(app: &CodexApp, fx: AppFx) {
    let Some(commands) = &app.link.commands else {
        return;
    };
    for command in fx.commands {
        let _ = commands.send(command);
    }
}

/// Puts an event into the ingress under `ingress.epoch`.
pub fn send(app: &CodexApp, ingress: Ingress) {
    if let Some(sender) = &app.link.ingress {
        let _ = sender.send(ingress);
    }
}

/// Awaits the result of a dialog and delivers `msg` to the widget when it
/// closes: the one place a widget's future is spawned (R1).
pub fn on_dialog_closed<S, T>(
    cx: &mut rtwk::Cx<'_, S>,
    result: rtwk::overlay::DialogResult<T>,
    msg: S::Msg,
) where
    S: rtwk::State,
    S::Msg: 'static,
    T: 'static,
{
    cx.spawn(async move {
        let _ = result.await;
        Some(msg)
    });
}

/// Puts `ev` into `ingress` under `epoch` (for connectors).
pub fn emit(ingress: &IngressTx, epoch: ConnEpoch, ev: AppEv) {
    let _ = ingress.send(Ingress { epoch, ev });
}
