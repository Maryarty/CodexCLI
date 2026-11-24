//! The host's side of the Windows sandbox prompts (oracle `windows_sandbox.rs`
//! `WindowsSandboxConfig::read`, `windows_sandbox_ready`): the sandbox configuration and managed
//! requirements read from the server and whether the elevated setup is complete. Only Windows
//! has a sandbox to set up: elsewhere nothing is read and the feature stays quiet. The pump
//! (`in_process.rs`) turns what is read into the feature's fact.

use std::time::Duration;

use codex_app_server_protocol::{
    ClientRequest, ConfigReadParams, ConfigReadResponse, ConfigRequirementsReadResponse,
    WindowsSandboxReadiness, WindowsSandboxReadinessResponse,
};
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;

/// How long the server may take (the oracle's budget).
const WITHIN: Duration = Duration::from_secs(5);

/// What the server says of the sandbox as `cwd` sees it, and whether the elevated setup is
/// ready; none when it does not answer (the feature has nothing to show without it).
pub(crate) async fn fetch<F, Fut>(
    cwd: String,
    ids: &Ids,
    send: F,
) -> Option<(ConfigReadResponse, ConfigRequirementsReadResponse, bool)>
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
        Some((config, requirements, ready))
    };
    tokio::time::timeout(WITHIN, read).await.ok().flatten()
}
