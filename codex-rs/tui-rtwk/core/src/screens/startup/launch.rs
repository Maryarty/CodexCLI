//! The startup prompts of a launch: which are due and in what order, and
//! their protocol wiring. Oracle: `lib.rs` `run_ratatui_app` (the hooks
//! review before the app runs), `startup_hooks_review.rs`
//! (`review_is_needed`), `app/startup_prompts.rs`
//! (`handle_model_migration_prompt_if_needed`,
//! `should_show_model_migration_prompt`, `model_upgrade_for_migration`),
//! `hooks_rpc.rs` (`fetch_hooks_list`, `write_hook_trusts`).
//!
//! The host reads the facts (hooks/list, model/list, the config's notices)
//! and asks these functions which prompts are due; the answers come back as
//! config writes built here.

use std::collections::BTreeMap;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigEdit;
use codex_app_server_protocol::HookTrustStatus;
use codex_app_server_protocol::HooksListEntry;
use codex_app_server_protocol::HooksListParams;
use codex_app_server_protocol::HooksListResponse;
use codex_app_server_protocol::MergeStrategy;
use codex_app_server_protocol::Model;
use codex_app_server_protocol::ModelListParams;
use codex_app_server_protocol::ModelListResponse;
use codex_app_server_protocol::RequestId;
use serde_json::Value;

use super::{HookTrust, HooksPrompt, MigrationPrompt, Prompt, Trust};

/// `hooks/list` for the launch's folder (`HooksListTarget::Cwd`).
pub fn hooks_list(cwd: &str, request_id: RequestId) -> ClientRequest {
    ClientRequest::HooksList {
        request_id,
        params: HooksListParams {
            cwds: vec![cwd.into()],
            ..Default::default()
        },
    }
}

/// `hook_needs_review`: a hook that is new or changed since it was trusted.
fn needs_review(status: HookTrustStatus) -> bool {
    matches!(
        status,
        HookTrustStatus::Untrusted | HookTrustStatus::Modified
    )
}

/// `review_is_needed`: hooks to review, and the launch does not bypass
/// their trust (`--dangerously-bypass-hook-trust`).
pub fn review_is_needed(bypass_hook_trust: bool, entry: &HooksListEntry) -> bool {
    !bypass_hook_trust
        && entry
            .hooks
            .iter()
            .any(|hook| needs_review(hook.trust_status))
}

/// The hooks review due for `entry`, if any.
pub fn hooks_prompt(bypass_hook_trust: bool, entry: &HooksListEntry) -> Option<HooksPrompt> {
    if !review_is_needed(bypass_hook_trust, entry) {
        return None;
    }
    let updates: Vec<HookTrust> = entry
        .hooks
        .iter()
        .filter(|hook| needs_review(hook.trust_status))
        .map(|hook| HookTrust {
            key: hook.key.clone(),
            hash: hook.current_hash.clone(),
        })
        .collect();
    Some(HooksPrompt {
        count: updates.len(),
        trust: Trust::Idle,
        updates,
    })
}

/// The first inventory of a `hooks/list` answer. A failed or empty read is
/// an empty inventory: the oracle logs it and reviews nothing
/// (`load_startup_hooks_review_entry`).
pub fn hooks_entry(result: Value) -> Option<HooksListEntry> {
    serde_json::from_value::<HooksListResponse>(result)
        .ok()?
        .data
        .into_iter()
        .next()
}

/// `write_hook_trusts`: every hook of the review trusted at its hash.
pub fn hooks_trust_write(updates: &[HookTrust], request_id: RequestId) -> ClientRequest {
    let value = Value::Object(
        updates
            .iter()
            .map(|u| {
                (
                    u.key.clone(),
                    serde_json::json!({ "trusted_hash": u.hash }),
                )
            })
            .collect(),
    );
    ClientRequest::ConfigBatchWrite {
        request_id,
        params: ConfigBatchWriteParams {
            edits: vec![ConfigEdit {
                key_path: "hooks.state".to_owned(),
                value,
                merge_strategy: MergeStrategy::Upsert,
            }],
            file_path: None,
            expected_version: None,
            reload_user_config: true,
        },
    }
}

/// `model/list` with the hidden models: a migration may target one.
pub fn model_list(request_id: RequestId) -> ClientRequest {
    ClientRequest::ModelList {
        request_id,
        params: ModelListParams {
            include_hidden: Some(true),
            ..Default::default()
        },
    }
}

/// The models of a `model/list` answer; none when it does not read.
pub fn models(result: Value) -> Vec<Model> {
    serde_json::from_value::<ModelListResponse>(result)
        .map(|r| r.data)
        .unwrap_or_default()
}

/// What the migration decision reads of the config.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MigrationFacts {
    pub provider: String,
    /// The session's model.
    pub model: String,
    /// `notices.model_migrations`: the migrations already answered.
    pub seen: BTreeMap<String, String>,
    /// `notices.hide_gpt5_1_migration_prompt`.
    pub hide_gpt5_1: bool,
    /// `notices.hide_gpt-5.1-codex-max_migration_prompt`.
    pub hide_gpt_5_1_codex_max: bool,
}

/// `ModelUpgrade`, as `model_preset_from_api_model` reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Upgrade {
    id: String,
    config_key: String,
    link: Option<String>,
    copy: Option<String>,
    markdown: Option<String>,
}

fn upgrade_of(model: &Model) -> Option<Upgrade> {
    let id = model.upgrade.clone()?;
    let info = model.upgrade_info.as_ref();
    Some(Upgrade {
        id,
        config_key: model.model.clone(),
        link: info.and_then(|i| i.model_link.clone()),
        copy: info.and_then(|i| i.upgrade_copy.clone()),
        markdown: info.and_then(|i| i.migration_markdown.clone()),
    })
}

const OPENAI_PROVIDER_ID: &str = "openai";
const AMAZON_BEDROCK_PROVIDER_ID: &str = "amazon-bedrock";
const HIDE_GPT5_1_MIGRATION_PROMPT_CONFIG: &str = "hide_gpt5_1_migration_prompt";
const HIDE_GPT_5_1_CODEX_MAX_MIGRATION_PROMPT_CONFIG: &str =
    "hide_gpt-5.1-codex-max_migration_prompt";

/// `model_upgrade_for_migration`: the catalog's upgrade for `model`, else
/// the fallback for a saved selection that outlived its catalog entry.
fn upgrade_for(provider: &str, model: &str, models: &[Model]) -> Option<Upgrade> {
    if let Some(preset) = models.iter().find(|m| m.model == model) {
        return upgrade_of(preset);
    }
    let (target, current_name, target_name) = match (provider, model) {
        (OPENAI_PROVIDER_ID, "gpt-5.4") => ("gpt-6-sol", "GPT-5.4", "GPT-6 Sol"),
        (AMAZON_BEDROCK_PROVIDER_ID, "openai.gpt-5.4") => (
            "openai.gpt-6-sol",
            "GPT-5.4 on Amazon Bedrock",
            "GPT-6 Sol on Amazon Bedrock",
        ),
        (OPENAI_PROVIDER_ID, "gpt-5.4-mini") => ("gpt-6-luna", "GPT-5.4 Mini", "GPT-6 Luna"),
        _ => return None,
    };
    let availability = if model == "openai.gpt-5.4" {
        "no longer offered in Codex"
    } else {
        "no longer available"
    };
    Some(Upgrade {
        id: target.to_owned(),
        config_key: model.to_owned(),
        link: None,
        copy: None,
        markdown: Some(format!(
            "{current_name} is {availability}\n\nCodex now uses {target_name} in place of {current_name}. Switch to {target_name} to continue.\n"
        )),
    })
}

/// `should_show_model_migration_prompt`.
pub fn should_show_model_migration_prompt(
    provider: &str,
    current_model: &str,
    target_model: &str,
    seen: &BTreeMap<String, String>,
    models: &[Model],
) -> bool {
    if target_model == current_model {
        return false;
    }
    if seen.get(current_model).is_some_and(|t| t == target_model) {
        return false;
    }
    if !models.iter().any(|m| m.model == target_model && !m.hidden) {
        return false;
    }
    if upgrade_for(provider, current_model, models).is_some_and(|u| u.id == target_model) {
        return true;
    }
    models.iter().any(|m| m.upgrade.as_deref() == Some(target_model))
}

/// The model migration due at launch, if any
/// (`handle_model_migration_prompt_if_needed` up to the prompt).
pub fn migration_prompt(facts: &MigrationFacts, models: &[Model]) -> Option<MigrationPrompt> {
    let upgrade = upgrade_for(&facts.provider, &facts.model, models)?;
    let hidden = match upgrade.config_key.as_str() {
        HIDE_GPT_5_1_CODEX_MAX_MIGRATION_PROMPT_CONFIG => facts.hide_gpt_5_1_codex_max,
        HIDE_GPT5_1_MIGRATION_PROMPT_CONFIG => facts.hide_gpt5_1,
        _ => false,
    };
    if hidden
        || !should_show_model_migration_prompt(
            &facts.provider,
            &facts.model,
            &upgrade.id,
            &facts.seen,
            models,
        )
    {
        return None;
    }
    // `target_preset_for_upgrade`.
    let target = models.iter().find(|m| m.model == upgrade.id)?;
    let display_name = if target.display_name == facts.model {
        upgrade.id.clone()
    } else {
        target.display_name.clone()
    };
    Some(MigrationPrompt {
        current_model: facts.model.clone(),
        target_model: upgrade.id.clone(),
        target_display_name: display_name,
        target_description: (!target.description.is_empty()).then(|| target.description.clone()),
        copy: upgrade.copy,
        link: upgrade.link,
        markdown: upgrade.markdown,
        can_opt_out: models.iter().any(|m| m.model == facts.model),
        effort: serde_json::to_value(&target.default_reasoning_effort)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned)),
    })
}

/// The prompts due at launch, in the oracle's order: the hooks review
/// (before the app runs), then the model migration (as the app starts).
pub fn due(hooks: Option<HooksPrompt>, migration: Option<MigrationPrompt>) -> Vec<Prompt> {
    hooks
        .map(Prompt::Hooks)
        .into_iter()
        .chain(migration.map(Prompt::Migration))
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use codex_app_server_protocol::{HookEventName, HookHandlerMetadata, HookMetadata, HookSource};
    use serde_json::json;

    // startup_hooks_review.rs `hook`.
    fn hook(key: &str, trust_status: HookTrustStatus) -> HookMetadata {
        HookMetadata {
            key: key.to_string(),
            event_name: HookEventName::PreToolUse,
            handler: HookHandlerMetadata::Command {
                command: "/tmp/hook.sh".to_string(),
                r#async: false,
            },
            is_managed: false,
            matcher: Some("Bash".to_string()),
            timeout_sec: 30,
            status_message: None,
            additional_context_limit: None,
            source_path: serde_json::from_value(json!("/tmp/hooks.json")).unwrap(),
            source: HookSource::User,
            plugin_id: None,
            environment_id: None,
            display_order: 0,
            enabled: false,
            current_hash: ["sha256:", key].concat(),
            trust_status,
        }
    }

    // startup_hooks_review.rs `entry`.
    fn entry() -> HooksListEntry {
        HooksListEntry {
            cwd: "/tmp".into(),
            hooks: vec![
                hook("path:new", HookTrustStatus::Untrusted),
                hook("path:changed", HookTrustStatus::Modified),
            ],
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    #[test]
    fn bypass_hook_trust_suppresses_startup_review() {
        assert!(!review_is_needed(/*bypass_hook_trust*/ true, &entry()));
        assert_eq!(hooks_prompt(true, &entry()), None);
    }

    #[test]
    fn untrusted_hooks_need_review_without_bypass() {
        assert!(review_is_needed(/*bypass_hook_trust*/ false, &entry()));
        let prompt = hooks_prompt(false, &entry()).expect("a review");
        assert_eq!(prompt.count, 2);
        assert_eq!(
            prompt.updates,
            [
                HookTrust {
                    key: "path:new".into(),
                    hash: "sha256:path:new".into()
                },
                HookTrust {
                    key: "path:changed".into(),
                    hash: "sha256:path:changed".into()
                },
            ]
        );
    }

    #[test]
    fn trusted_hooks_need_no_review() {
        let mut entry = entry();
        entry.hooks = vec![hook("path:ok", HookTrustStatus::Trusted)];
        assert!(!review_is_needed(false, &entry));
    }

    // hooks_rpc.rs `write_hook_trusts`: one upsert of `hooks.state`, the
    // user config reloaded.
    #[test]
    fn the_hooks_trust_write_upserts_the_hashes() {
        let updates = [HookTrust {
            key: "path:new".into(),
            hash: "sha256:x".into(),
        }];
        let ClientRequest::ConfigBatchWrite { params, .. } =
            hooks_trust_write(&updates, RequestId::Integer(1))
        else {
            panic!("a batch write");
        };
        assert!(params.reload_user_config);
        assert_eq!(params.edits.len(), 1);
        assert_eq!(params.edits[0].key_path, "hooks.state");
        assert_eq!(
            params.edits[0].value,
            json!({"path:new": {"trusted_hash": "sha256:x"}})
        );
    }

    /// A catalog entry: `upgrade` names the model it migrates to.
    fn model(name: &str, upgrade: Option<&str>, hidden: bool) -> Value {
        json!({
            "id": name, "model": name, "upgrade": upgrade,
            "upgradeInfo": upgrade.map(|u| json!({
                "model": u, "upgradeCopy": null, "modelLink": null,
                "migrationMarkdown": "{model_from} to {model_to}"
            })),
            "availabilityNux": null, "displayName": name.to_uppercase(),
            "description": "", "hidden": hidden,
            "supportedReasoningEfforts": [], "defaultReasoningEffort": "high",
            "isDefault": false
        })
    }

    fn catalog(entries: Vec<Value>) -> Vec<Model> {
        let models = models(json!({ "data": entries }));
        assert!(!models.is_empty(), "the catalog reads");
        models
    }

    fn test_catalog() -> Vec<Model> {
        catalog(vec![
            model("gpt-5.2", Some("gpt-5.5"), false),
            model("gpt-5.5", None, false),
            model("gpt-6-sol", None, false),
            model("gpt-6-luna", None, false),
        ])
    }

    #[test]
    fn model_migration_prompt_only_shows_for_deprecated_models() {
        let seen = BTreeMap::new();
        let presets = test_catalog();
        assert!(should_show_model_migration_prompt(
            "openai", "gpt-5.2", "gpt-5.5", &seen, &presets
        ));
        assert!(should_show_model_migration_prompt(
            "openai",
            "gpt-5.4",
            "gpt-6-sol",
            &seen,
            &presets
        ));
        assert!(!should_show_model_migration_prompt(
            "openai", "gpt-5.4", "gpt-5.4", &seen, &presets
        ));
    }

    #[test]
    fn retired_model_migration_respects_catalog_metadata() {
        for name in ["gpt-5.4-mini", "gpt-5.4", "openai.gpt-5.4"] {
            let presets = catalog(vec![model(name, Some("gpt-5.5"), false)]);
            assert_eq!(
                upgrade_for("openai", name, &presets).map(|u| u.id),
                Some("gpt-5.5".to_owned())
            );
            let presets = catalog(vec![model(name, None, false)]);
            assert_eq!(upgrade_for("openai", name, &presets), None);
        }
    }

    #[test]
    fn model_migration_prompt_respects_hide_flag_and_self_target() {
        let presets = test_catalog();
        let mut seen = BTreeMap::from([("gpt-5.4".to_owned(), "gpt-5.6-terra".to_owned())]);
        assert!(should_show_model_migration_prompt(
            "openai",
            "gpt-5.4",
            "gpt-6-sol",
            &seen,
            &presets
        ));
        seen.insert("gpt-5.4".into(), "gpt-6-sol".into());
        assert!(!should_show_model_migration_prompt(
            "openai",
            "gpt-5.4",
            "gpt-6-sol",
            &seen,
            &presets
        ));
        assert!(!should_show_model_migration_prompt(
            "openai",
            "gpt-6-sol",
            "gpt-6-sol",
            &seen,
            &presets
        ));
        // `migration_prompt_hidden`: the notice that hides the prompt.
        let presets = catalog(vec![
            model("hide_gpt5_1_migration_prompt", Some("gpt-5.5"), false),
            model("gpt-5.5", None, false),
        ]);
        let mut facts = MigrationFacts {
            provider: "openai".into(),
            model: "hide_gpt5_1_migration_prompt".into(),
            ..MigrationFacts::default()
        };
        assert!(migration_prompt(&facts, &presets).is_some());
        facts.hide_gpt5_1 = true;
        assert_eq!(migration_prompt(&facts, &presets), None);
    }

    #[test]
    fn model_migration_prompt_skips_when_target_missing_or_hidden() {
        let hidden = catalog(vec![model("gpt-6-sol", None, true)]);
        assert!(!should_show_model_migration_prompt(
            "openai",
            "gpt-5.4",
            "gpt-6-sol",
            &BTreeMap::new(),
            &hidden
        ));
        let missing = catalog(vec![model("gpt-5.5", None, false)]);
        assert!(!should_show_model_migration_prompt(
            "openai",
            "gpt-5.4",
            "gpt-6-sol",
            &BTreeMap::new(),
            &missing
        ));
    }

    #[test]
    fn model_migration_prompt_shows_for_hidden_model() {
        let presets = catalog(vec![
            model("gpt-5.2", Some("gpt-5.5"), true),
            model("gpt-5.5", None, false),
        ]);
        let facts = MigrationFacts {
            provider: "openai".into(),
            model: "gpt-5.2".into(),
            ..MigrationFacts::default()
        };
        let prompt = migration_prompt(&facts, &presets).expect("a migration");
        assert_eq!(prompt.target_model, "gpt-5.5");
        assert_eq!(prompt.target_display_name, "GPT-5.5");
        assert_eq!(prompt.effort.as_deref(), Some("high"));
        assert!(prompt.can_opt_out);
    }

    // run_ratatui_app: the hooks review comes before the migration.
    #[test]
    fn the_hooks_review_is_asked_before_the_migration() {
        let hooks = hooks_prompt(false, &entry());
        let facts = MigrationFacts {
            provider: "openai".into(),
            model: "gpt-5.2".into(),
            ..MigrationFacts::default()
        };
        let migration = migration_prompt(&facts, &test_catalog());
        let due = due(hooks, migration);
        assert!(matches!(
            due.as_slice(),
            [Prompt::Hooks(_), Prompt::Migration(_)]
        ));
    }
}
