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
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigWarningNotification;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use codex_arg0::Arg0DispatchPaths;
use codex_config::CloudConfigBundleLoader;
use codex_config::LoaderOverrides;
use codex_core::config::Config;
use codex_feedback::CodexFeedback;
use codex_protocol::protocol::SessionSource;
use codex_tui_rtwk::app::AppEv;
use codex_tui_rtwk::app::BackendEv;
use codex_tui_rtwk::app::Command;
use codex_tui_rtwk::ids::ConnEpoch;
use codex_tui_rtwk::ids::RequestKey;
use codex_tui_rtwk::ids::ThreadId;
use codex_tui_rtwk::ids::Ticket;
use codex_tui_rtwk::ids::TurnId;
use codex_tui_rtwk::rpc::Reply;
use codex_tui_rtwk::screens;
use codex_tui_rtwk::shell::BackendTask;
use codex_tui_rtwk::shell::Connector;
use codex_tui_rtwk::shell::IngressTx;
use codex_tui_rtwk::shell::emit;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

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
    /// Resolved when the client has shut down, so `main` can wait for it.
    pub done: Option<oneshot::Sender<()>>,
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
    let InProcess { args, done } = spec;
    let mut client = match InProcessAppServerClient::start(args).await {
        Ok(client) => client,
        Err(error) => {
            emit(&ingress, epoch, AppEv::StartFailed(error.to_string()));
            finish(done);
            return;
        }
    };
    emit(&ingress, epoch, AppEv::Connected);
    let handle = client.request_handle();
    let mut next_id = 0_i64;
    let mut inflight = FuturesUnordered::new();
    loop {
        tokio::select! {
            event = client.next_event() => match event {
                None => {
                    emit(&ingress, epoch, AppEv::Disconnected("the app-server stopped".to_string()));
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
                Some(Command::RejectRequest { key, .. }) => reject(&client, &key).await,
                Some(command) => {
                    next_id += 1;
                    inflight.push(request(command, handle.clone(), RequestId::Integer(next_id)));
                }
            },
            Some((ticket, reply)) = inflight.next(), if !inflight.is_empty() => {
                emit(&ingress, epoch, AppEv::Reply(ticket, reply));
            }
        }
    }
    let _ = client.shutdown().await;
    finish(done);
}

fn finish(done: Option<oneshot::Sender<()>>) {
    if let Some(done) = done {
        let _ = done.send(());
    }
}

/// One request of ours and the reply the app is waiting for.
async fn request(
    command: Command,
    handle: InProcessAppServerRequestHandle,
    request_id: RequestId,
) -> (Ticket, Reply) {
    match command {
        Command::StartThread { ticket, .. } => {
            let reply = match handle
                .request_typed::<ThreadStartResponse>(ClientRequest::ThreadStart {
                    request_id,
                    params: ThreadStartParams::default(),
                })
                .await
            {
                Ok(response) => Reply::thread_started(ThreadId::new(response.thread.id)),
                Err(error) => Reply::failed(format!("thread/start: {error}")),
            };
            (ticket, reply)
        }
        Command::StartTurn {
            ticket,
            thread,
            text,
            ..
        } => {
            let reply = match handle
                .request_typed::<TurnStartResponse>(ClientRequest::TurnStart {
                    request_id,
                    params: TurnStartParams {
                        thread_id: thread.to_string(),
                        input: vec![UserInput::Text {
                            text,
                            text_elements: Vec::new(),
                        }],
                        ..TurnStartParams::default()
                    },
                })
                .await
            {
                Ok(response) => Reply::turn_started(thread, TurnId::new(response.turn.id)),
                Err(error) => Reply::failed(format!("turn/start: {error}")),
            };
            (ticket, reply)
        }
        Command::Screen {
            ticket,
            thread,
            command,
            ..
        } => {
            // A slot's RPC: the slot's `wire` makes the request and its
            // `decode` reads the result.
            let reply = match handle
                .request_typed::<serde_json::Value>(screens::wire(&command, &thread, request_id))
                .await
            {
                Ok(result) => match screens::decode(&command, result) {
                    Ok(reply) => Reply::screen(reply),
                    Err(error) => Reply::failed(error),
                },
                Err(error) => Reply::failed(error.to_string()),
            };
            (ticket, reply)
        }
        // Handled by the pump, which owns the client.
        Command::RejectRequest { .. } => std::future::pending().await,
    }
}

async fn reject(client: &InProcessAppServerClient, key: &RequestKey) {
    let Ok(request_id) = serde_json::from_str::<RequestId>(key.id.as_str()) else {
        return;
    };
    let _ = client
        .reject_server_request(
            request_id,
            JSONRPCErrorError {
                code: -32000,
                data: None,
                message: "dismissed by the user".to_string(),
            },
        )
        .await;
}
