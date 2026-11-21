//! The provider wizards' host side (S03): what the GovCloud guidance at
//! launch needs from the machine. The guidance step itself (when it runs,
//! its view) is the startup page's; this reads the server and writes the
//! configuration. Oracle: `onboarding/bedrock.rs` (`check_gov_cloud`),
//! `lib.rs` (`check_gov_cloud_guidance`, the acknowledgment write).

use std::future::Future;
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_core::config::Config;
use codex_core::config::edit::ConfigEdit;
use codex_core::config::edit::ConfigEditsBuilder;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::onboarding::auth::provider::wire;
use codex_tui_rtwk::services::backend::rpc::Ids;

use crate::startup::Target;

/// How long the check waits for the server (`check_gov_cloud`).
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);

/// Whether the check runs at launch: the guidance was not acknowledged
/// before, and the provider is Amazon Bedrock or the server is not this
/// process's own (a connected server resolves its own provider, the local
/// config may differ).
pub fn gov_cloud_check_due(config: &Config, target: Target) -> bool {
    check_due(
        config.notices.hide_gov_cloud_guidance,
        config.model_provider.is_amazon_bedrock(),
        target,
    )
}

fn check_due(hidden: Option<bool>, bedrock: bool, target: Target) -> bool {
    hidden != Some(true) && (bedrock || target != Target::Embedded)
}

/// Whether the guidance shows: the server says the Bedrock authentication in
/// use resolves to GovCloud. A failed or late answer shows nothing.
pub async fn check_gov_cloud<F, Fut>(ids: &Ids, send: &F) -> bool
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let answer = tokio::time::timeout(CHECK_TIMEOUT, send(wire::gov_cloud_check(ids.next()))).await;
    match answer {
        Ok(result) => wire::is_gov_cloud(result.map_err(|error| error.message)),
        Err(_) => false,
    }
}

/// The guidance was acknowledged: it is not shown again
/// (`notice.hide_gov_cloud_guidance`). A failed write is only logged.
pub async fn acknowledge_gov_cloud(config: &Config) {
    let write = ConfigEditsBuilder::for_config(config)
        .with_edits([ConfigEdit::SetPath {
            segments: vec!["notice".to_owned(), "hide_gov_cloud_guidance".to_owned()],
            value: true.into(),
        }])
        .apply()
        .await;
    if let Err(error) = write {
        tracing::warn!(%error, "Failed to save GovCloud guidance acknowledgment");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // lib.rs: `check_gov_cloud_guidance`.
    #[test]
    fn the_check_runs_for_bedrock_or_a_connected_server_until_acknowledged() {
        assert!(check_due(None, true, Target::Embedded));
        assert!(!check_due(None, false, Target::Embedded));
        assert!(check_due(Some(false), false, Target::LocalDaemon));
        assert!(check_due(None, false, Target::Remote));
        assert!(!check_due(Some(true), true, Target::Remote));
    }

    #[tokio::test]
    async fn only_a_gov_cloud_answer_shows_the_guidance() {
        let answers = |value: Result<serde_json::Value, ServerError>| async move {
            check_gov_cloud(&Ids::default(), &move |_| {
                let value = value.clone();
                async move { value }
            })
            .await
        };
        assert!(answers(Ok(json!({"isGovCloud": true, "shouldWarn": true}))).await);
        assert!(!answers(Ok(json!({"isGovCloud": false, "shouldWarn": false}))).await);
        assert!(
            !answers(Err(ServerError {
                code: None,
                message: "denied".into(),
            }))
            .await
        );
    }
}
