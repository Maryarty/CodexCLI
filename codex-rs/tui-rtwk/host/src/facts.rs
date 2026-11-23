//! What the status line and the footer need from the host: the loaded
//! config (model, reasoning effort, approval mode, `tui.status_line` and its
//! colours flag, the startup warnings) and the workspace (cwd as shown,
//! project root, git branch), read once at the start and handed to the app as
//! events. The texts are core's (`StatusFacts::items`); this only reads.
//! Oracle: `chatwidget/status_surfaces.rs` (`status_line_cwd`,
//! `status_line_project_root_name_for_cwd`, `format_directory_display`).

use std::future::Future;
use std::path::Path;

use codex_app_server_protocol::{
    Account, ClientRequest, GetAccountParams, GetAccountResponse, Model, ModelListParams,
    ModelListResponse,
};
use codex_config::ConfigLayerSource;
use codex_config::types::{NotificationCondition, Notifications, TuiKeymap};
use codex_core::config::Config;
use codex_tui_core::app::AppEv;
use codex_tui_core::keymap::{Keymap, KeymapSpec};
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::chat::{NotificationPolicy, StatusFacts};
use codex_tui_core::screens::dialogs::Warning;
use codex_tui_core::screens::select::permissions::{Facts, Limit, Limits};
use codex_tui_core::threads::daybreak::{Access, ModelAccess, Programs};
use codex_tui_rtwk::services::backend::rpc::Ids;

/// `format_directory_display` without a width: `~` for the home, `~/x` below
/// it, the path as it is otherwise.
pub fn directory_display(directory: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| directory.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        None => directory.display().to_string(),
    }
}

/// The name of the project root (`status_line_project_root_name_for_cwd`): the
/// git root's directory name, or its display when it has none.
pub fn project_root_name(cwd: &Path, home: Option<&Path>) -> Option<String> {
    let root = codex_git_utils::get_git_repo_root(cwd)?;
    Some(match root.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => directory_display(&root, home),
    })
}

/// The granular approval policy as JSON (`/presets` recalls it), none for the plain ones.
fn granular_policy(config: &Config) -> Option<String> {
    use codex_app_server_protocol::AskForApproval;
    match AskForApproval::from(config.permissions.approval_policy.value()) {
        granular @ AskForApproval::Granular { .. } => serde_json::to_string(&granular).ok(),
        _ => None,
    }
}

/// The status facts of `config` and its cwd.
pub async fn status(config: &Config) -> StatusFacts {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let cwd = config.cwd.as_path();
    let notifications = &config.tui_notifications;
    let (enabled, kinds) = match &notifications.notifications {
        Notifications::Enabled(enabled) => (*enabled, None),
        Notifications::Custom(kinds) => (true, Some(kinds.clone())),
    };
    let profile_name = config
        .config_layer_stack
        .get_active_user_layer()
        .and_then(|layer| match &layer.name {
            ConfigLayerSource::User { profile, .. } => profile.clone(),
            _ => None,
        });
    StatusFacts {
        model: config.model.clone(),
        effort: config
            .model_reasoning_effort
            .as_ref()
            .map(|effort| effort.as_str().to_string()),
        approval_policy: Some(config.permissions.approval_policy.value().to_string()),
        approval_granular: granular_policy(config),
        permission_profile: config
            .permissions
            .active_permission_profile()
            .map(|profile| profile.id),
        profile: match config.permissions.legacy_sandbox_policy(cwd) {
            codex_protocol::protocol::SandboxPolicy::ReadOnly { .. } => {
                codex_tui_core::screens::status::Profile::ReadOnly
            }
            codex_protocol::protocol::SandboxPolicy::WorkspaceWrite { .. } => {
                codex_tui_core::screens::status::Profile::Workspace
            }
            _ => codex_tui_core::screens::status::Profile::Full,
        },
        auto_review: matches!(
            config.approvals_reviewer,
            codex_config::types::ApprovalsReviewer::AutoReview
        ),
        cwd: Some(directory_display(cwd, home.as_deref())),
        home_directory: home.as_deref().map(|path| path.display().to_string()),
        service_tier: config.service_tier.clone(),
        fast_mode: config.features.enabled(codex_features::Feature::FastMode),
        ultrafast_mode: config
            .features
            .enabled(codex_features::Feature::UltrafastMode),
        project_root: project_root_name(cwd, home.as_deref()),
        branch: codex_git_utils::current_branch_name(cwd).await,
        items: config.tui_status_line.clone(),
        colors: config.tui_status_line_use_colors,
        terminal_title: config.tui_terminal_title.clone(),
        hostname: codex_config::os_host_name(),
        profile_name,
        provider: Some(config.model_provider_id.clone()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        animations: config.animations,
        progress_effect: config.tui_effects.progress,
        shimmer_effect: config.tui_effects.shimmer,
        starfield_effect: config.tui_effects.starfield,
        account_usage: config.model_provider.requires_openai_auth,
        empty_state: config.tui_effects.welcome,
        notification_policy: NotificationPolicy {
            enabled,
            always: notifications.condition == NotificationCondition::Always,
            kinds,
        },
        permissions: permissions(config),
        agent_threads: Some(config.multi_agent_v2.max_concurrent_threads_per_session),
    }
}

/// What `/permissions` reads of the config: the guardian feature, what the
/// requirements forbid (`can_set` errors as the oracle shows them) and whether
/// auto-review is mandatory or available.
/// Oracle: `permission_popups.rs` (`open_legacy_permissions_popup`),
/// `permissions_menu.rs` (`cyber_model_approval_reviewer`).
pub fn permissions(config: &Config) -> Facts {
    use codex_app_server_protocol::AskForApproval;
    use codex_config::types::ApprovalsReviewer;
    use codex_features::Feature;
    use codex_protocol::protocol::AskForApproval as CoreApproval;

    let guardian = config.features.enabled(Feature::GuardianApproval);
    let guardian_disabled_reason = |enabled: bool| {
        let mut next = config.features.get().clone();
        next.set_enabled(Feature::GuardianApproval, enabled);
        config
            .features
            .can_set(&next)
            .err()
            .map(|error| error.to_string())
    };
    let approvals = [CoreApproval::OnRequest, CoreApproval::Never]
        .into_iter()
        .filter_map(|approval| {
            config
                .permissions
                .approval_policy
                .can_set(&approval)
                .err()
                .map(|error| Limit {
                    approval: AskForApproval::from(approval),
                    reason: error.to_string(),
                })
        })
        .collect();
    let requirements = config.config_layer_stack.requirements();
    let auto_available = requirements
        .approval_policy
        .can_set(&CoreApproval::OnRequest)
        .is_ok()
        && guardian
        && requirements
            .approvals_reviewer
            .can_set(&ApprovalsReviewer::AutoReview)
            .is_ok();
    Facts {
        guardian,
        auto_required: config
            .model
            .as_deref()
            .is_some_and(|model| requirements.auto_review_required_for_model(model)),
        auto_available,
        limits: Limits {
            approvals,
            reviewer_user: guardian_disabled_reason(false),
            reviewer_auto: guardian_disabled_reason(true),
        },
    }
}

/// The startup warnings the config collected, as the warnings viewer shows
/// them (`source`, `details`).
pub fn warnings(config: &Config) -> Vec<Warning> {
    config
        .startup_warnings
        .iter()
        .map(|text| Warning {
            source: "Startup".to_string(),
            details: text.clone(),
        })
        .collect()
}

/// `tui.keymap.*`: the bindings the user set, resolved over the built-in
/// defaults (`RuntimeKeymap::from_config`). An error is the oracle's text,
/// naming the config path; the app then keeps the defaults.
pub fn keymap(config: &Config) -> Result<Keymap, String> {
    keymap_of(&config.tui_keymap)
}

fn keymap_of(config: &TuiKeymap) -> Result<Keymap, String> {
    let value = serde_json::to_value(config).map_err(|error| error.to_string())?;
    Keymap::resolve(&KeymapSpec::from_json(&value)).map_err(|error| error.to_string())
}

/// What the config says of cyber access programs (oracle `daybreak_feature_enabled` and the
/// provider's support); the account decides the rest.
#[derive(Clone, Copy, Debug)]
pub struct CyberPolicy {
    /// `Feature::CliDaybreak`.
    feature: bool,
    /// `Feature::ApiKeyCyberAccessPrograms`: an API-key account may use the programs.
    api_key: bool,
    /// The configured provider supports cyber access programs.
    provider: bool,
}

impl CyberPolicy {
    pub fn of(config: &Config) -> Self {
        Self {
            feature: config
                .features
                .enabled(codex_features::Feature::CliDaybreak),
            api_key: config
                .features
                .enabled(codex_features::Feature::ApiKeyCyberAccessPrograms),
            provider: config
                .model_providers
                .get(&config.model_provider_id)
                .is_some_and(|provider| {
                    provider.supports_cyber_access_programs(&config.model_provider_id)
                }),
        }
    }

    /// Oracle `daybreak_account_eligible`: the feature is on (for an API key only with its own
    /// flag too), the provider supports the programs, and the account is a ChatGPT sign-in or an
    /// API key.
    fn eligible(self, account: Option<&Account>) -> bool {
        let chatgpt = matches!(account, Some(Account::Chatgpt { .. }));
        let api_key = matches!(account, Some(Account::ApiKey {}));
        self.feature
            && (chatgpt || !api_key || self.api_key)
            && self.provider
            && (chatgpt || api_key)
    }
}

/// The cyber programs each model of the catalog advertises (`available_access_programs`: none
/// for a model whose metadata says nothing, which an empty list is not).
fn model_access(models: &[Model]) -> Vec<ModelAccess> {
    models
        .iter()
        .map(|model| ModelAccess {
            model: model.model.clone(),
            programs: model.available_access_programs.as_ref().map(|programs| {
                let programs =
                    codex_protocol::openai_models::ModelAccessPrograms::from(programs.clone());
                Programs {
                    standard: programs.standard().is_some(),
                    daybreak: programs.daybreak().is_some(),
                }
            }),
        })
        .collect()
}

/// The Daybreak facts that word a refused cybersecurity request (`AppEv::Daybreak`): the
/// account is read, and for an eligible one the model catalog. A failed read leaves the default
/// ("not eligible, no catalog": pending or failed discovery never implies access).
pub async fn daybreak<F, Fut>(policy: CyberPolicy, ids: Ids, send: F) -> Option<AppEv>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let account = send(ClientRequest::GetAccount {
        request_id: ids.next(),
        params: GetAccountParams {
            refresh_token: false,
        },
    })
    .await
    .ok()?;
    let account = serde_json::from_value::<GetAccountResponse>(account)
        .ok()?
        .account;
    if !policy.eligible(account.as_ref()) {
        return Some(AppEv::Daybreak(Access::default()));
    }
    let mut models = Vec::new();
    let mut cursor = None;
    loop {
        let page = send(ClientRequest::ModelList {
            request_id: ids.next(),
            params: ModelListParams {
                cursor: cursor.take(),
                include_hidden: Some(true),
                ..Default::default()
            },
        })
        .await
        .ok()
        .and_then(|page| serde_json::from_value::<ModelListResponse>(page).ok());
        let Some(page) = page else { break };
        models.extend(page.data);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    Some(AppEv::Daybreak(Access {
        eligible: true,
        models: model_access(&models),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keymap_config_is_resolved_over_the_defaults() {
        use codex_tui_core::keymap::Stroke;
        use codex_tui_core::keymap::actions::{LIST_ACCEPT, LIST_CANCEL};
        let config: TuiKeymap = serde_json::from_value(serde_json::json!({
            "list": { "accept": "ctrl-y", "cancel": [] },
        }))
        .expect("a keymap table");
        let keymap = keymap_of(&config).expect("a valid keymap");
        assert_eq!(
            keymap.singles(LIST_ACCEPT),
            [Stroke::parse("ctrl-y").expect("a stroke")]
        );
        assert!(keymap.singles(LIST_CANCEL).is_empty(), "[] unbinds");
        assert_eq!(keymap_of(&TuiKeymap::default()), Ok(Keymap::defaults()));
    }

    #[test]
    fn a_keymap_the_oracle_refuses_is_an_error_with_its_text() {
        let config: TuiKeymap = serde_json::from_value(serde_json::json!({
            "list": { "move_up": "down" },
        }))
        .expect("a keymap table");
        let message = keymap_of(&config).expect_err("a conflict");
        assert!(
            message.starts_with("Ambiguous `tui.keymap.list` bindings:"),
            "{message}"
        );
    }

    #[test]
    fn directory_display_abbreviates_the_home() {
        let home = Path::new("/home/ann");
        assert_eq!(directory_display(Path::new("/home/ann"), Some(home)), "~");
        assert_eq!(
            directory_display(Path::new("/home/ann/dev/app"), Some(home)),
            "~/dev/app"
        );
        assert_eq!(
            directory_display(Path::new("/tmp/project"), Some(home)),
            "/tmp/project"
        );
        assert_eq!(
            directory_display(Path::new("/tmp/project"), None),
            "/tmp/project"
        );
    }

    #[test]
    fn project_root_is_the_git_roots_name() {
        // This checkout is a git work tree: the name is its root directory's.
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = codex_git_utils::get_git_repo_root(here).expect("a git checkout");
        let want = root.file_name().map(|n| n.to_string_lossy().to_string());
        assert_eq!(project_root_name(here, None), want);
        // Outside a repository there is none.
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(project_root_name(dir.path(), None), None);
    }
    fn policy(feature: bool, api_key: bool, provider: bool) -> CyberPolicy {
        CyberPolicy {
            feature,
            api_key,
            provider,
        }
    }

    fn chatgpt() -> Account {
        serde_json::from_value(serde_json::json!({
            "type": "chatgpt", "email": null, "planType": "plus"
        }))
        .expect("a ChatGPT account")
    }

    #[test]
    fn cyber_access_needs_the_feature_the_provider_and_a_signed_in_account() {
        let key = Account::ApiKey {};
        let chatgpt = chatgpt();
        assert!(policy(true, false, true).eligible(Some(&chatgpt)));
        assert!(
            !policy(false, false, true).eligible(Some(&chatgpt)),
            "feature off"
        );
        assert!(
            !policy(true, false, false).eligible(Some(&chatgpt)),
            "provider"
        );
        assert!(!policy(true, false, true).eligible(None), "no account");
        // an API key needs its own flag too
        assert!(!policy(true, false, true).eligible(Some(&key)));
        assert!(policy(true, true, true).eligible(Some(&key)));
    }

    /// A server whose `account/read` says `account` and whose `model/list` is two pages.
    fn server(
        account: serde_json::Value,
        asked: &std::sync::Mutex<Vec<String>>,
    ) -> impl Fn(ClientRequest) -> std::future::Ready<Result<serde_json::Value, ServerError>> + '_
    {
        let model = |name: &str, programs: serde_json::Value| {
            serde_json::json!({
                "id": name, "model": name, "upgrade": null, "upgradeInfo": null,
                "availabilityNux": null, "displayName": name, "description": "", "hidden": false,
                "supportedReasoningEfforts": [], "defaultReasoningEffort": "high",
                "multiAgentVersion": null, "availableAccessPrograms": programs,
                "isDefault": false
            })
        };
        move |request| {
            asked.lock().unwrap().push(request.method_name().to_owned());
            std::future::ready(Ok(match request {
                ClientRequest::GetAccount { .. } => account.clone(),
                ClientRequest::ModelList { params, .. } if params.cursor.is_none() => {
                    serde_json::json!({
                        "data": [
                            model("gpt-6-astra", serde_json::json!({"cyber": ["standard"]})),
                            model("gpt-5.6-sol", serde_json::json!({"cyber": ["standard", "daybreakBlue"]})),
                        ],
                        "nextCursor": "page-2",
                    })
                }
                ClientRequest::ModelList { .. } => serde_json::json!({
                    "data": [model("gpt-5", serde_json::Value::Null)],
                    "nextCursor": null,
                }),
                other => panic!("unexpected request {other:?}"),
            }))
        }
    }

    #[tokio::test]
    async fn an_eligible_account_reads_the_whole_catalog_with_its_programs() {
        let asked = std::sync::Mutex::new(Vec::new());
        let account = serde_json::json!({
            "account": {"type": "chatgpt", "email": null, "planType": "plus"},
            "requiresOpenaiAuth": true,
        });
        let ev = daybreak(
            policy(true, false, true),
            Ids::default(),
            server(account, &asked),
        )
        .await;
        let Some(AppEv::Daybreak(access)) = ev else {
            panic!("no Daybreak event");
        };
        let programs = |standard, daybreak| Some(Programs { standard, daybreak });
        assert_eq!(
            access,
            Access {
                eligible: true,
                models: vec![
                    ModelAccess {
                        model: "gpt-6-astra".into(),
                        programs: programs(true, false)
                    },
                    ModelAccess {
                        model: "gpt-5.6-sol".into(),
                        programs: programs(true, true)
                    },
                    // missing metadata is not an empty list
                    ModelAccess {
                        model: "gpt-5".into(),
                        programs: None
                    },
                ],
            }
        );
        assert_eq!(
            *asked.lock().unwrap(),
            ["account/read", "model/list", "model/list"]
        );
    }

    #[tokio::test]
    async fn an_account_without_access_is_not_asked_for_the_catalog() {
        let asked = std::sync::Mutex::new(Vec::new());
        let account = serde_json::json!({"account": null, "requiresOpenaiAuth": true});
        let ev = daybreak(
            policy(true, false, true),
            Ids::default(),
            server(account, &asked),
        )
        .await;
        let Some(AppEv::Daybreak(access)) = ev else {
            panic!("no Daybreak event");
        };
        assert_eq!(access, Access::default());
        assert_eq!(*asked.lock().unwrap(), ["account/read"]);
    }
}
