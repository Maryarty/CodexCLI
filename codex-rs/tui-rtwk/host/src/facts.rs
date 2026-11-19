//! What the status line and the footer need from the host: the loaded
//! config (model, reasoning effort, approval mode, `tui.status_line` and its
//! colours flag, the startup warnings) and the workspace (cwd as shown,
//! project root, git branch), read once at the start and handed to the app as
//! events. The texts are core's (`StatusFacts::items`); this only reads.
//! Oracle: `chatwidget/status_surfaces.rs` (`status_line_cwd`,
//! `status_line_project_root_name_for_cwd`, `format_directory_display`).

use std::path::Path;

use codex_core::config::Config;
use codex_tui_core::screens::chat::StatusFacts;
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
    StatusFacts {
        model: config.model.clone(),
        effort: config
            .model_reasoning_effort
            .as_ref()
            .map(|effort| effort.as_str().to_string()),
        approval_policy: Some(config.permissions.approval_policy.value().to_string()),
        auto_review: matches!(
            config.approvals_reviewer,
            codex_config::types::ApprovalsReviewer::AutoReview
        ),
        cwd: Some(directory_display(cwd, home.as_deref())),
        project_root: project_root_name(cwd, home.as_deref()),
        branch: codex_git_utils::current_branch_name(cwd).await,
        items: config.tui_status_line.clone(),
        colors: config.tui_status_line_use_colors,
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
        assert_eq!(directory_display(Path::new("/tmp/project"), Some(home)), "/tmp/project");
        assert_eq!(directory_display(Path::new("/tmp/project"), None), "/tmp/project");
    }

    #[test]
    fn project_root_is_the_git_roots_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("rtwk");
        std::fs::create_dir_all(root.join("src/deep")).expect("dirs");
        std::fs::create_dir_all(root.join(".git")).expect("git");
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").expect("head");
        assert_eq!(
            project_root_name(&root.join("src/deep"), None).as_deref(),
            Some("rtwk")
        );
        assert_eq!(project_root_name(dir.path(), None), None);
    }
}
