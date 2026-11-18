//! The in-process app-server as a `Connector`. It lives in the host package
//! (`host/`, the `codex-rtwk` binary): the library links no server, so its
//! tests build and link fast. It is the shell's other half: one future the
//! shell runs, with no spawn and no lint allow of its own.

use std::sync::Arc;

use anyhow::Context as _;
use codex_app_server_client::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY;
use codex_app_server_client::EmbeddedNetworkPolicy;
use codex_app_server_client::EnvironmentManager;
use codex_app_server_client::ExecServerRuntimeOptions;
use codex_app_server_client::InProcessAppServerClient;
use codex_app_server_client::InProcessAppServerRequestHandle;
use codex_app_server_client::InProcessClientStartArgs;
use codex_app_server_client::InProcessServerEvent;
use codex_app_server_protocol::ConfigWarningNotification;
use codex_app_server_protocol::RequestId;
use codex_arg0::Arg0DispatchPaths;
use codex_config::CloudConfigBundleLoader;
use codex_config::LoaderOverrides;
use codex_core::config::Config;
use codex_feedback::CodexFeedback;
use codex_protocol::protocol::SessionSource;
use codex_tui_rtwk::shell::BackendTask;
use codex_tui_rtwk::shell::Connector;
use codex_tui_rtwk::shell::IngressTx;
use codex_tui_rtwk::shell::emit;
use codex_tui_rtwk_core::app::AppEv;
use codex_tui_rtwk_core::app::BackendEv;
use codex_tui_rtwk_core::app::Command;
use codex_tui_rtwk_core::ids::ConnEpoch;
use codex_tui_rtwk_core::ids::RequestKey;
use codex_tui_rtwk_core::ids::Ticket;
use codex_tui_rtwk_core::rpc::Reply;
use codex_tui_rtwk_core::wire;
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
    let started = match InProcessAppServerClient::start(args).await {
        Ok(client) => client,
        Err(error) => {
            emit(&ingress, epoch, AppEv::StartFailed(error.to_string()));
            return;
        }
    };
    let mut held = HandBack::new(started, done);
    emit(&ingress, epoch, AppEv::Connected);
    let Some(handle) = held.get().map(|client| client.request_handle()) else {
        return;
    };
    let mut next_id = 0_i64;
    let mut inflight = FuturesUnordered::new();
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
                Some(command) => {
                    next_id += 1;
                    inflight.push(request(
                        command,
                        handle.clone(),
                        RequestId::Integer(next_id),
                        model.clone(),
                    ));
                }
            },
            Some((ticket, reply)) = inflight.next(), if !inflight.is_empty() => {
                emit(&ingress, epoch, AppEv::Reply(ticket, reply));
            }
        }
    }
    // Leaving the loop drops `client`: the guard hands it to `main`.
}

/// One request of ours and the reply the app is waiting for: core's `wire`
/// makes the request and reads the result; this only sends it.
async fn request(
    command: Command,
    handle: InProcessAppServerRequestHandle,
    request_id: RequestId,
    model: Option<String>,
) -> (Ticket, Reply) {
    // `RejectRequest` is handled by the pump, which owns the client.
    let Some(wire_request) = wire::client_request(&command, request_id, model.as_deref()) else {
        return std::future::pending().await;
    };
    let result = handle
        .request_typed::<serde_json::Value>(wire_request)
        .await
        .map_err(|error| error.to_string());
    match wire::reply(&command, result) {
        Some(answer) => answer,
        None => std::future::pending().await,
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
