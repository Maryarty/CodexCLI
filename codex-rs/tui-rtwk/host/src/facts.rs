//! What the status line and the footer need from the host: the loaded
//! config (model, reasoning effort, approval mode, `tui.status_line` and its
//! colours flag, the startup warnings) and the workspace (cwd as shown,
//! project root, git branch), read once at the start and handed to the app as
//! events. The texts are core's (`StatusFacts::items`); this only reads.
//! Oracle: `chatwidget/status_surfaces.rs` (`status_line_cwd`,
//! `status_line_project_root_name_for_cwd`, `format_directory_display`).

use std::path::Path;

use codex_config::ConfigLayerSource;
use codex_config::types::{NotificationCondition, Notifications};
use codex_core::config::Config;
use codex_tui_core::screens::chat::{NotificationPolicy, StatusFacts};
use codex_tui_core::screens::dialogs::Warning;

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
        account_usage: config.model_provider.requires_openai_auth,
        empty_state: config.tui_effects.welcome,
        notification_policy: NotificationPolicy {
            enabled,
            always: notifications.condition == NotificationCondition::Always,
            kinds,
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

#[cfg(test)]
mod tests {
    use super::*;

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
