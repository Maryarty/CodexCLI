//! What the status line and the footer need from the host: the loaded
//! config (model, reasoning effort, approval mode, `tui.status_line` and its
//! colours flag, the startup warnings) and the workspace (cwd as shown,
//! project root, git branch), read once at the start and handed to the app as
//! events. The texts are core's (`StatusFacts::items`); this only reads.
//! Oracle: `chatwidget/status_surfaces.rs` (`status_line_cwd`,
//! `status_line_project_root_name_for_cwd`, `format_directory_display`).

use std::path::Path;

use codex_config::ConfigLayerSource;
use codex_config::types::{NotificationCondition, Notifications, TuiKeymap};
use codex_core::config::Config;
use codex_tui_core::keymap::{Keymap, KeymapSpec};
use codex_tui_core::screens::chat::{NotificationPolicy, StatusFacts};
use codex_tui_core::screens::dialogs::Warning;
use codex_tui_core::screens::select::permissions::{Facts, Limit, Limits};

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
}
