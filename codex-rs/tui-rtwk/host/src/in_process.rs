//! The in-process app-server as a `Connector`. It lives in the host package
//! (`host/`, the `codex-rtwk` binary): the library links no server, so its
//! tests build and link fast. It is the shell's other half: one future the
//! shell runs, with no spawn and no lint allow of its own.

use std::sync::Arc;

use crate::workspace::Local;
use anyhow::Context as _;
use codex_app_server_client::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY;
use codex_app_server_client::EmbeddedNetworkPolicy;
use codex_app_server_client::EnvironmentManager;
use codex_app_server_client::ExecServerRuntimeOptions;
use codex_app_server_client::InProcessAppServerClient;
use codex_app_server_client::InProcessAppServerRequestHandle;
use codex_app_server_client::InProcessClientStartArgs;
use codex_app_server_client::InProcessServerEvent;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigWarningNotification;
use codex_arg0::Arg0DispatchPaths;
use codex_config::CloudConfigBundleLoader;
use codex_config::LoaderOverrides;
use codex_core::config::Config;
use codex_feedback::CodexFeedback;
use codex_protocol::protocol::SessionSource;
use codex_tui_core::app::AppEv;
use codex_tui_core::app::BackendEv;
use codex_tui_core::app::Command;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::wire;
use codex_tui_rtwk::shell::BackendTask;
use codex_tui_rtwk::shell::Connector;
use codex_tui_rtwk::shell::IngressTx;
use codex_tui_rtwk::shell::emit;
use codex_tui_rtwk::shell::rpc::Ids;
use codex_tui_rtwk::shell::rpc::answer;
use codex_tui_rtwk::shell::workspace::Boxed;
use codex_tui_rtwk::shell::workspace::Facts;
use codex_tui_rtwk::shell::workspace::Workspace;
use codex_tui_rtwk::shell::workspace::backend_ev;
use codex_tui_rtwk::shell::workspace::directories;
use codex_tui_rtwk::shell::workspace::worktree;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

/// Hands its value to `main` when dropped, however the future that holds it
/// ends. Quitting unmounts the app, and rtwk aborts the backend task: its
/// future is dropped at an `.await`, so no code after the loop can run.
/// `main` therefore owns the shutdown: it takes the client back through
/// this guard and awaits `client.shutdown()` itself.
pub struct HandBack<T> {
    value: Option<T>,
    to: Option<oneshot::Sender<T>>,
}

impl<T> HandBack<T> {
    pub fn new(value: T, to: oneshot::Sender<T>) -> Self {
        Self {
            value: Some(value),
            to: Some(to),
        }
    }
}

impl<T> HandBack<T> {
    /// The value, until the guard is dropped.
    pub fn get(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }
}

impl<T> Drop for HandBack<T> {
    fn drop(&mut self) {
        if let (Some(value), Some(to)) = (self.value.take(), self.to.take()) {
            let _ = to.send(value);
        }
    }
}

/// What the in-process client is started with, for `config`.
pub async fn start_args(
    config: Config,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<InProcessClientStartArgs> {
    let loader_overrides = LoaderOverrides::default();
    let embedded_network_policy = EmbeddedNetworkPolicy::load(&loader_overrides).await;
    let state_db = codex_core::init_state_db(&config).await;
    let runtime_paths = ExecServerRuntimeOptions::from_optional_paths(
        arg0_paths.codex_self_exe.clone(),
        arg0_paths.codex_linux_sandbox_exe.clone(),
    )
    .context("exec-server runtime paths")?;
    let environment_manager = EnvironmentManager::from_codex_home(
        config.codex_home.clone(),
        Some(runtime_paths),
        embedded_network_policy.bind(config.http_client_factory()),
    )
    .await
    .context("environment manager")?;
    let config_warnings = config
        .startup_warnings
        .iter()
        .map(|warning| ConfigWarningNotification {
            summary: warning.clone(),
            details: None,
            path: None,
            range: None,
        })
        .collect();
    Ok(InProcessClientStartArgs {
        required_human_approval: false,
        arg0_paths,
        config: Arc::new(config),
        cli_overrides: Vec::new(),
        loader_overrides,
        strict_config: false,
        cloud_config_bundle: CloudConfigBundleLoader::default(),
        embedded_network_policy,
        feedback: CodexFeedback::new(),
        log_db: None,
        state_db,
        environment_manager: Arc::new(environment_manager),
        config_warnings,
        session_source: SessionSource::Cli,
        enable_codex_api_key_env: true,
        client_name: "codex-rtwk".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        mcp_server_openai_form_elicitation: false,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
}

/// The in-process app-server, started when the app attaches it.
pub struct InProcess {
    pub args: InProcessClientStartArgs,
    /// Receives the client when the backend future ends or is aborted, so
    /// that `main` shuts it down (see `HandBack`).
    pub done: oneshot::Sender<InProcessAppServerClient>,
    /// `--model`: the model of the thread, over the config's.
    pub model: Option<String>,
}

impl Connector for InProcess {
    fn run(
        self: Box<Self>,
        ingress: IngressTx,
        commands: UnboundedReceiver<Command>,
        epoch: ConnEpoch,
    ) -> BackendTask {
        Box::pin(run(*self, ingress, commands, epoch))
    }
}

/// The in-process server's side: the pump and the requests it makes, all in
/// this one future (nothing is spawned; the shell runs the future).
async fn run(
    spec: InProcess,
    ingress: IngressTx,
    mut commands: UnboundedReceiver<Command>,
    epoch: ConnEpoch,
) {
    let InProcess { args, done, model } = spec;
    // Oracle: `worktrees_enabled` is a local workspace, and no cloud list.
    let local = !args
        .environment_manager
        .default_environment()
        .is_some_and(|environment| environment.is_remote());
    let facts = Facts {
        cloud: false,
        worktrees: local,
    };
    let workspace: Arc<dyn Workspace> = Arc::new(Local::new(args.config.codex_home.to_path_buf()));
    // Boxed: the start future is large, and the pump should not carry it inline.
    let started = match Box::pin(InProcessAppServerClient::start(args)).await {
        Ok(client) => client,
        Err(error) => {
            emit(&ingress, epoch, AppEv::StartFailed(error.to_string()));
            return;
        }
    };
    let mut held = HandBack::new(started, done);
    emit(&ingress, epoch, AppEv::Connected);
    emit(&ingress, epoch, backend_ev(facts));
    let Some(handle) = held.get().map(|client| client.request_handle()) else {
        return;
    };
    let ids = Ids::default();
    let mut inflight: FuturesUnordered<Boxed<Option<AppEv>>> = FuturesUnordered::new();
    while let Some(client) = held.get() {
        tokio::select! {
            event = client.next_event() => match event {
                None => {
                    emit(&ingress, epoch, AppEv::Disconnected);
                    break;
                }
                Some(InProcessServerEvent::Lagged { skipped }) => {
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Lagged(skipped)));
                }
                Some(InProcessServerEvent::ServerNotification(notification)) => {
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Notification(notification)));
                }
                Some(InProcessServerEvent::ServerRequest(request)) => {
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Request(request)));
                }
            },
            command = commands.recv() => match command {
                None => break,
                Some(Command::RejectRequest { key, .. }) => reject(client, &key).await,
                Some(command) => inflight.push(work(
                    command,
                    handle.clone(),
                    ids.clone(),
                    facts,
                    workspace.clone(),
                    model.clone(),
                )),
            },
            Some(done) = inflight.next(), if !inflight.is_empty() => {
                if let Some(ev) = done {
                    emit(&ingress, epoch, ev);
                }
            }
        }
    }
    // Leaving the loop drops `client`: the guard hands it to `main`.
}

/// The work a command of ours asks, as one boxed future (the pump holds many
/// of them; boxing keeps its type, and its stack, small) and the event the
/// app is waiting for, if any. Core's `wire` makes the requests and reads the
/// results; the shell's `answer`, `directories` and `worktree` run them; this
/// only sends.
fn work(
    command: Command,
    handle: InProcessAppServerRequestHandle,
    ids: Ids,
    facts: Facts,
    workspace: Arc<dyn Workspace>,
    model: Option<String>,
) -> Boxed<Option<AppEv>> {
    let send = move |request: ClientRequest| -> Boxed<Result<serde_json::Value, String>> {
        let handle = handle.clone();
        Box::pin(async move {
            handle
                .request_typed::<serde_json::Value>(request)
                .await
                .map_err(|error| error.to_string())
        })
    };
    match command {
        Command::Directories { cwds, .. } => {
            Box::pin(async move { Some(directories(workspace, cwds).await) })
        }
        Command::Worktree { cwd, .. } => {
            Box::pin(async move { Some(worktree(workspace, cwd, ids, send).await) })
        }
        // `RejectRequest` is handled by the pump, which owns the client.
        command => Box::pin(async move {
            answer(command, facts, ids, model, send)
                .await
                .map(|(ticket, reply)| AppEv::Reply(ticket, reply))
        }),
    }
}

async fn reject(client: &InProcessAppServerClient, key: &RequestKey) {
    let Some(request_id) = wire::reject_target(key) else {
        return;
    };
    let _ = client
        .reject_server_request(request_id, wire::dismissal())
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_guard_hands_its_value_back() {
        let (tx, mut rx) = oneshot::channel();
        drop(HandBack::new(7_u8, tx));
        assert_eq!(rx.try_recv().ok(), Some(7));
    }

    /// The case that matters: the future is dropped at an `.await`, as
    /// rtwk's abort does on quit.
    #[test]
    fn a_future_dropped_at_an_await_hands_its_value_back() {
        use futures::FutureExt as _;
        let (tx, mut rx) = oneshot::channel();
        let mut future = Box::pin(async move {
            let _held = HandBack::new(7_u8, tx);
            std::future::pending::<()>().await;
        });
        assert!(
            (&mut future).now_or_never().is_none(),
            "parked at the await"
        );
        assert!(rx.try_recv().is_err(), "still held");
        drop(future);
        assert_eq!(rx.try_recv().ok(), Some(7));
    }
}
