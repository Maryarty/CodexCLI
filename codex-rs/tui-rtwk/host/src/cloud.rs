//! The Cloud backend next to the first connection (oracle: `lib.rs`
//! `run_main_inner` registers `cloud_backend::connect` with the
//! `InitialBackendSessions`). One future the shell runs, with its own pump:
//! it answers the commands routed to it and forwards what the cloud tells
//! under its own epoch and backend id. It never sends `Connected`.

use std::sync::Arc;

use crate::in_process::reject;
use crate::in_process::resolve;
use crate::in_process::work;
use crate::target::Mode;
use crate::workspace::Local;
use codex_app_server_client::AppServerClient;
use codex_app_server_client::AppServerEvent;
use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_client::CloudUserMessageReceipt;
use codex_app_server_client::EmbeddedNetworkPolicy;
use codex_app_server_client::TypedRequestError;
use codex_app_server_client::cloud_backend;
use codex_app_server_protocol::ServerNotification;
use codex_config::LoaderOverrides;
use codex_core::config::Config;
use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
use codex_features::Feature;
use codex_tui_core::app::AppEv;
use codex_tui_core::app::BackendEv;
use codex_tui_core::app::Command;
use codex_tui_core::ids::BackendId;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::ids::Ticket;
use codex_tui_core::ids::TurnId;
use codex_tui_core::rpc::CloudSubmission;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::dialogs::Ev as DialogsEv;
use codex_tui_core::screens::dialogs::Warning;
use codex_tui_rtwk::services::agents::search::Latest;
use codex_tui_rtwk::services::agents::workspace::Boxed;
use codex_tui_rtwk::services::agents::workspace::Facts;
use codex_tui_rtwk::services::agents::workspace::Workspace;
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_tui_rtwk::services::shell::BackendTask;
use codex_tui_rtwk::services::shell::Connector;
use codex_tui_rtwk::services::shell::Dispatch;
use codex_tui_rtwk::services::shell::IngressTx;
use codex_tui_rtwk::services::shell::emit_from;
use codex_tui_rtwk::services::shell::next_dispatch;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::sync::mpsc::UnboundedReceiver;

/// What the Cloud connection is started with: what the first connection's
/// config was built from.
pub struct Cloud {
    pub id: BackendId,
    /// The first connection's server: the cloud preview borrows the
    /// embedded network policy, so it is only offered where that exists
    /// (oracle: `AppServerTarget::uses_embedded_network_policy`).
    pub mode: Mode,
    pub model: Option<String>,
    pub cwd: Option<std::path::PathBuf>,
    pub config_overrides: Vec<String>,
}

/// Whether the first connection's server leaves the cloud preview on
/// (oracle: `uses_embedded_network_policy`: the embedded server, or the
/// local daemon with the embedded fallback).
pub fn preview_available(mode: &Mode) -> bool {
    matches!(mode, Mode::Embedded | Mode::Daemon)
}

/// The oracle's gate on the config (`lib.rs`): macOS or Linux, the
/// multi-backend feature, no workload identity, no config server.
pub fn gate(config: &Config, workload_identity: bool) -> bool {
    cfg!(any(target_os = "macos", target_os = "linux"))
        && config.features.enabled(Feature::TuiMultiBackend)
        && !workload_identity
        && config
            .config_layer_stack
            .requirements_toml()
            .config_server
            .is_none()
}

impl Connector for Cloud {
    fn run(
        self: Box<Self>,
        ingress: IngressTx,
        commands: UnboundedReceiver<Dispatch>,
        epoch: ConnEpoch,
    ) -> BackendTask {
        Box::pin(run(*self, ingress, commands, epoch))
    }
}

async fn run(
    spec: Cloud,
    ingress: IngressTx,
    mut commands: UnboundedReceiver<Dispatch>,
    epoch: ConnEpoch,
) {
    // Without the cloud the commands routed here are never sent: nothing
    // lists it, so no task is owned by it.
    let Some((client, home)) = connect(&spec, &ingress, epoch).await else {
        while commands.recv().await.is_some() {}
        return;
    };
    let workspace: Arc<dyn Workspace> = Arc::new(Local::new(home));
    let facts = Facts {
        cloud: true,
        worktrees: false,
    };
    emit_from(
        &ingress,
        spec.id,
        epoch,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::CloudBackend(spec.id))),
    );
    emit_from(&ingress, spec.id, epoch, AppEv::Connected);
    let handle = client.request_handle();
    let ids = Ids::default();
    let mut inflight: FuturesUnordered<Boxed<Option<AppEv>>> = FuturesUnordered::new();
    let mut latest = Latest::default();
    let mut client = client;
    loop {
        tokio::select! {
            event = client.next_event() => match event {
                None | Some(AppServerEvent::Disconnected { .. }) => break,
                Some(AppServerEvent::Lagged { skipped }) => {
                    emit_from(&ingress, spec.id, epoch, AppEv::Backend(BackendEv::Lagged(skipped)));
                }
                Some(AppServerEvent::ServerNotification(notification)) => {
                    emit_from(&ingress, spec.id, epoch, AppEv::Backend(BackendEv::Notification(notification)));
                }
                Some(AppServerEvent::ServerRequest(request)) => {
                    emit_from(&ingress, spec.id, epoch, AppEv::Backend(BackendEv::Request(request)));
                }
                // The stream of one thread ended, or is back (oracle `app_server_events.rs`): the
                // app decides what the user sees and sends; the owner check is its.
                Some(AppServerEvent::ThreadStreamDisconnected(error)) => {
                    let thread = ThreadId::new(&error.thread_id);
                    emit_from(&ingress, spec.id, epoch, AppEv::Backend(BackendEv::StreamLost { thread, error }));
                }
                Some(AppServerEvent::ThreadStreamReconnected(thread)) => {
                    emit_from(&ingress, spec.id, epoch, AppEv::Backend(BackendEv::StreamRestored(ThreadId::new(&thread))));
                }
            },
            command = next_dispatch(&mut commands, spec.id, epoch) => match command {
                None => break,
                Some((Command::RejectRequest { key, .. }, _)) => {
                    if key.epoch == epoch {
                        reject(&client, &key).await;
                    }
                }
                Some((Command::ResolveRequest { key, result, .. }, _)) => {
                    if key.epoch == epoch && let Some(ev) = resolve(&client, key, result).await {
                        emit_from(&ingress, spec.id, epoch, ev);
                    }
                }
                // The idle displayed thread lost its stream: the backend attaches it again quietly
                // (`reconnect_idle_thread`); when it does not take it, the error is the thread's
                // line, as the oracle's handler falls through to it.
                Some((Command::ReconnectIdleThread { thread, error, .. }, _)) => {
                    let accepted = matches!(
                        &handle,
                        AppServerRequestHandle::Cloud(cloud) if cloud.reconnect_idle_thread(thread.as_str())
                    );
                    if !accepted {
                        emit_from(
                            &ingress,
                            spec.id,
                            epoch,
                            AppEv::Backend(BackendEv::Notification(Box::new(
                                ServerNotification::Error(*error),
                            ))),
                        );
                    }
                }
                Some((command, _)) => {
                    let job = work(
                        command.clone(),
                        handle.clone(),
                        ids.clone(),
                        facts,
                        workspace.clone(),
                        spec.model.clone(),
                        None,
                    );
                    inflight.push(latest.wrap(&command, job));
                }
            },
            Some(done) = inflight.next(), if !inflight.is_empty() => {
                if let Some(ev) = done {
                    emit_from(&ingress, spec.id, epoch, ev);
                }
            }
        }
    }
    emit_from(&ingress, spec.id, epoch, AppEv::Disconnected);
}

pub(crate) fn submission_reply(
    ticket: Ticket,
    admission: Result<CloudUserMessageReceipt, TypedRequestError>,
) -> AppEv {
    let reply = match admission {
        Ok(CloudUserMessageReceipt::Started(turn)) => {
            Reply::cloud_submission(CloudSubmission::Started(TurnId::new(&turn.id)))
        }
        Ok(CloudUserMessageReceipt::Steered(turn)) => {
            Reply::cloud_submission(CloudSubmission::Steered(TurnId::new(&turn.id)))
        }
        Ok(CloudUserMessageReceipt::Duplicate { .. }) => {
            Reply::cloud_submission(CloudSubmission::Duplicate)
        }
        // The Cloud worker reserves this code for failures before AddUserMessage.
        Err(TypedRequestError::Server { source, .. }) if source.code == -32602 => {
            Reply::cloud_submission(CloudSubmission::Preflight(source.message))
        }
        Err(error) => Reply::cloud_submission(CloudSubmission::Unconfirmed(error.to_string())),
    };
    AppEv::Reply(ticket, reply)
}

/// The cloud client and the codex home of its config, when the gate lets it
/// be tried; a failed attempt is told among the startup warnings (the oracle
/// inserts a history line).
async fn connect(
    spec: &Cloud,
    ingress: &IngressTx,
    epoch: ConnEpoch,
) -> Option<(AppServerClient, std::path::PathBuf)> {
    if !preview_available(&spec.mode) {
        return None;
    }
    let cli_overrides = codex_utils_cli::CliConfigOverrides {
        raw_overrides: spec.config_overrides.clone(),
    }
    .parse_overrides()
    .ok()?;
    let config = ConfigBuilder::default()
        .cli_overrides(cli_overrides)
        .harness_overrides(ConfigOverrides {
            cwd: spec.cwd.clone(),
            ..Default::default()
        })
        .build()
        .await
        .ok()?;
    if !gate(&config, codex_login::is_workload_identity_selected()) {
        return None;
    }
    // A daemon-backed resume can rebuild Config without the local
    // transport handle: the policy is bound to the config again.
    let mut cloud_config = config.clone();
    EmbeddedNetworkPolicy::load(&LoaderOverrides::default())
        .await
        .bind_config(&mut cloud_config);
    match Box::pin(cloud_backend::connect(&cloud_config)).await {
        Ok(client) => Some((client, cloud_config.codex_home.to_path_buf())),
        Err(error) => {
            emit_from(
                ingress,
                spec.id,
                epoch,
                AppEv::Screen(ScreenEv::Dialogs(DialogsEv::Retained(vec![Warning {
                    source: "Cloud".to_string(),
                    details: format!("Could not add the Cloud connection. {error}"),
                }]))),
            );
            None
        }
    }
}

#[cfg(test)]
mod tests;
