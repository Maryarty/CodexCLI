//! The host's side of the Windows sandbox prompts (oracle `windows_sandbox.rs`
//! `WindowsSandboxConfig::read`, `windows_sandbox_ready`): the sandbox configuration and managed
//! requirements read from the server, whether the elevated setup is complete, and where the
//! executors are. The feature gets them as a fact (`features::windows_sandbox`). Only Windows
//! has a sandbox to set up: elsewhere nothing is read and the feature stays quiet.

use std::time::Duration;

use codex_app_server_protocol::{
    ClientRequest, ConfigReadParams, ConfigReadResponse, ConfigRequirementsReadResponse,
    WindowsSandboxReadiness, WindowsSandboxReadinessResponse,
};
use codex_tui_core::app::AppEv;
use codex_tui_core::features::windows_sandbox::{Config, Fact, Host};
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;

/// How long the server may take (the oracle's budget).
const WITHIN: Duration = Duration::from_secs(5);

/// The fact for the sandbox as `cwd` sees it, or none when the server does not answer (the
/// oracle says an error line then; the feature has nothing to show without it).
pub(crate) async fn fetch<F, Fut>(
    cwd: String,
    remote_workspace: bool,
    ids: &Ids,
    send: F,
) -> Option<AppEv>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, ServerError>>,
{
    if !cfg!(windows) {
        return None;
    }
    let read = async {
        let config: ConfigReadResponse = serde_json::from_value(
            send(ClientRequest::ConfigRead {
                request_id: ids.next(),
                params: ConfigReadParams {
                    include_layers: false,
                    cwd: Some(cwd),
                },
            })
            .await
            .ok()?,
        )
        .ok()?;
        let requirements: ConfigRequirementsReadResponse = serde_json::from_value(
            send(ClientRequest::ConfigRequirementsRead {
                request_id: ids.next(),
                params: None,
            })
            .await
            .ok()?,
        )
        .ok()?;
        let ready = send(ClientRequest::WindowsSandboxReadiness {
            request_id: ids.next(),
            params: None,
        })
        .await
        .ok()
        .and_then(|value| serde_json::from_value::<WindowsSandboxReadinessResponse>(value).ok())
        .is_some_and(|response| response.status == WindowsSandboxReadiness::Ready);
        Some((Config::from_responses(&config, requirements), ready))
    };
    let (config, elevated_ready) = tokio::time::timeout(WITHIN, read).await.ok().flatten()?;
    Some(AppEv::Feature(
        Fact::Environment {
            config,
            host: if remote_workspace {
                Host::Remote
            } else {
                Host::Local
            },
            local_server: !remote_workspace,
            elevated_ready,
        }
        .into(),
    ))
}

/// `windowsSandbox/setupCompleted` as the fact.
pub(crate) fn completed(
    notification: &codex_app_server_protocol::WindowsSandboxSetupCompletedNotification,
) -> AppEv {
    AppEv::Feature(
        Fact::Completed {
            mode: notification.mode,
            success: notification.success,
            error: notification.error.clone(),
        }
        .into(),
    )
}
