//! The launch's startup prompts (S05) read from the server, and the writes
//! of their answers. Oracle: `lib.rs` `run_ratatui_app` (the hooks review
//! after the trust and the reload), `app/startup_prompts.rs`
//! (`handle_model_migration_prompt_if_needed`), `hooks_rpc.rs`; the
//! decisions are core's `screens::startup::launch`.

use std::future::Future;
use std::sync::Arc;

use codex_app_server_protocol::ClientRequest;
use codex_core::config::Config;
use codex_core::config::edit::ConfigEditsBuilder;
use codex_protocol::openai_models::ReasoningEffort;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::startup::launch::{self, MigrationFacts};
use codex_tui_core::screens::startup::{HookTrust, Prompt};
use codex_tui_rtwk::services::backend::rpc::Ids;

/// The session's config as it stands: the launch's, replaced by the reload
/// after the trust or the sign-in (`load_config_or_exit_with_fallback_cwd`).
/// A reload holds the write lock while it loads, so what reads it after
/// the reload was asked sees the new config.
pub type ConfigSlot = Arc<tokio::sync::RwLock<Arc<Config>>>;

/// The startup prompts due now, in the oracle's order.
pub async fn prompts<F, Fut>(
    config: &Config,
    cwd: String,
    bypass_hook_trust: bool,
    ids: &Ids,
    send: &F,
) -> Vec<Prompt>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    // `load_startup_hooks_review_entry`: a failed read reviews nothing.
    let hooks = match send(launch::hooks_list(&cwd, ids.next())).await {
        Ok(result) => launch::hooks_entry(result),
        Err(error) => {
            tracing::warn!(
                "failed to load startup hook review state: {}",
                error.message
            );
            None
        }
    }
    .and_then(|entry| launch::hooks_prompt(bypass_hook_trust, &entry));
    let models = send(launch::model_list(ids.next()))
        .await
        .map(launch::models)
        .unwrap_or_default();
    // The session's model: the config's, else the catalog's default.
    let model = config.model.clone().or_else(|| {
        models
            .iter()
            .find(|m| m.is_default)
            .map(|m| m.model.clone())
    });
    let migration = model.and_then(|model| {
        let notices = &config.notices;
        launch::migration_prompt(
            &MigrationFacts {
                provider: config.model_provider_id.clone(),
                model,
                seen: notices.model_migrations.clone(),
                hide_gpt5_1: notices.hide_gpt5_1_migration_prompt.unwrap_or(false),
                hide_gpt_5_1_codex_max: notices
                    .hide_gpt_5_1_codex_max_migration_prompt
                    .unwrap_or(false),
            },
            &models,
        )
    });
    launch::due(hooks, migration)
}

/// `write_hook_trusts`: the failure's text, if any.
pub async fn trust_hooks<F, Fut>(updates: &[HookTrust], ids: &Ids, send: &F) -> Option<String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    send(launch::hooks_trust_write(updates, ids.next()))
        .await
        .err()
        .map(|error| format!("Failed to trust hooks: {}", error.message))
}

/// `PersistModelMigrationPromptAcknowledged` and, accepted,
/// `PersistModelSelection`: written to the user's config; a failure is
/// logged.
pub async fn save_migration(
    config: &Config,
    from: &str,
    to: &str,
    accepted: bool,
    effort: Option<String>,
) {
    let mut edits = ConfigEditsBuilder::for_config(config).record_model_migration_seen(from, to);
    if accepted {
        let effort = effort.and_then(|e| {
            serde_json::from_value::<ReasoningEffort>(serde_json::Value::String(e)).ok()
        });
        edits = edits.set_model(Some(to), effort);
    }
    if let Err(error) = edits.apply().await {
        tracing::error!(
            error = %error,
            "failed to persist model migration prompt acknowledgement"
        );
    }
}
