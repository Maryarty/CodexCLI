//! The machine the in-process server runs on, for the command center:
//! directory keys (canonical path, git repository) and new worktrees. It is
//! the host's `Workspace`; the shell's effects (`shell::workspace`) call it.
//! Oracle: `agents_overview_view.rs` (`directory_group_key`,
//! `AgentsOverviewProjectGroup::for_thread`), `agents_overview_new.rs`
//! (`new_agents_overview_worktree`, the bind in `start_agents_overview_session`).

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
use codex_core::config::load_config_toml_with_layer_stack;
use codex_tui_core::screens::agents::DirKeys;
use codex_tui_rtwk::shell::workspace::Boxed;
use codex_tui_rtwk::shell::workspace::Checkout;
use codex_tui_rtwk::shell::workspace::Workspace;
use codex_worktree::CreateWorktree;
use codex_worktree::WorktreeManager;
use codex_worktree::WorktreeSettings;

/// The local machine. `manager` is the one that made the last checkout: it
/// binds the task that starts in it.
pub struct Local {
    codex_home: PathBuf,
    manager: Arc<Mutex<Option<WorktreeManager>>>,
}

impl Local {
    pub fn new(codex_home: PathBuf) -> Self {
        Self {
            codex_home,
            manager: Arc::default(),
        }
    }
}

/// The keys of `cwd`: the project group is the repository (its common
/// directory and the directory within it, so linked worktrees merge into
/// their project) headed by the primary checkout's path, or the directory
/// itself; the directory group is its canonical path.
pub fn dir_keys(cwd: &str) -> DirKeys {
    let path = Path::new(cwd);
    let (project, heading) = match codex_git_utils::repository_identity(path) {
        Some(identity) => (
            (
                identity.common_dir.as_path().display().to_string(),
                identity.relative_cwd.display().to_string(),
            ),
            identity
                .primary_root
                .as_path()
                .join(&identity.relative_cwd)
                .display()
                .to_string(),
        ),
        None => ((cwd.to_owned(), String::new()), cwd.to_owned()),
    };
    let directory = dunce::canonicalize(path)
        .map(|canonical| canonical.display().to_string())
        .unwrap_or_else(|_| cwd.to_owned());
    DirKeys {
        project,
        heading,
        directory,
    }
}

impl Workspace for Local {
    fn dir_keys(&self, cwd: &str) -> Option<DirKeys> {
        Some(dir_keys(cwd))
    }

    fn create_worktree(&self, source_cwd: Option<String>) -> Boxed<Result<Checkout, String>> {
        let codex_home = self.codex_home.clone();
        let created = async move {
            let cwd = match source_cwd {
                Some(cwd) => PathBuf::from(cwd),
                None => std::env::current_dir().map_err(|error| error.to_string())?,
            };
            let config = ConfigBuilder::default()
                .harness_overrides(ConfigOverrides {
                    cwd: Some(cwd),
                    ..Default::default()
                })
                .build()
                .await
                .map_err(|error| error.to_string())?;
            if config.active_project.is_untrusted() {
                return Err("The source project is not trusted.".to_owned());
            }
            let host = load_config_toml_with_layer_stack(
                &codex_home,
                /*cwd*/ None,
                Vec::new(),
                codex_core::config::ConfigLoadOptions::default(),
            )
            .await
            .map_err(|error| error.to_string())?;
            let settings =
                WorktreeSettings::for_cli(&codex_home, host.config_toml.desktop.as_ref())
                    .map_err(|error| error.to_string())?;
            let manager = WorktreeManager::new(settings);
            let source_cwd = config.cwd.to_path_buf();
            let worker = manager.clone();
            let checkout = codex_tui_rtwk::shell::workspace::blocking(move || {
                codex_worktree::default_worktree_base(&source_cwd)
                    .and_then(|base| {
                        worker.create(&CreateWorktree {
                            source_cwd,
                            base: Some(base),
                        })
                    })
                    .map_err(|error| error.to_string())
            })
            .await
            .unwrap_or_else(|| Err("Worktree creation task failed.".to_owned()))?;
            Ok((manager, checkout))
        };
        let slot = self.manager.clone();
        Box::pin(async move {
            let (manager, checkout) = created.await?;
            if let Ok(mut held) = slot.lock() {
                *held = Some(manager);
            }
            Ok(Checkout {
                root: checkout.root.display().to_string(),
                cwd: checkout.cwd.display().to_string(),
            })
        })
    }

    fn bind_worktree(&self, checkout: &Checkout, thread: &str, cwd: &str) -> Result<(), String> {
        if !codex_utils_path::paths_match_after_normalization(cwd, &checkout.cwd) {
            return Err("The server did not apply the worktree directory.".to_owned());
        }
        let manager = self
            .manager
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "The worktree was not made by this session.".to_owned())?;
        manager
            .bind_thread(Path::new(&checkout.root), thread)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)] // the tests build a repository on disk
mod tests {
    use super::*;

    #[test]
    fn a_plain_directory_is_its_own_project_and_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dunce::canonicalize(dir.path()).expect("canonical");
        let keys = dir_keys(&dir.path().display().to_string());
        assert_eq!(keys.directory, canonical.display().to_string());
        assert_eq!(keys.project.1, "");
        assert_eq!(keys.heading, dir.path().display().to_string());
    }

    #[test]
    fn a_directory_of_a_missing_path_is_its_own_key() {
        let keys = dir_keys("/no/such/directory/for/codex");
        assert_eq!(keys.directory, "/no/such/directory/for/codex");
        assert_eq!(keys.heading, "/no/such/directory/for/codex");
    }

    #[test]
    fn a_linked_worktree_joins_the_project_of_its_primary_checkout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dunce::canonicalize(dir.path()).expect("canonical");
        let primary = root.join("repo");
        let git = primary.join(".git");
        std::fs::create_dir_all(git.join("worktrees").join("feature")).expect("git dir");
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("head");
        let linked = root.join("feature");
        std::fs::create_dir_all(&linked).expect("linked");
        let admin = git.join("worktrees").join("feature");
        std::fs::write(admin.join("commondir"), "../..\n").expect("commondir");
        std::fs::write(
            admin.join("gitdir"),
            format!("{}\n", linked.join(".git").display()),
        )
        .expect("gitdir");
        std::fs::write(
            linked.join(".git"),
            format!("gitdir: {}\n", admin.display()),
        )
        .expect("link");
        let in_primary = dir_keys(&primary.display().to_string());
        let in_linked = dir_keys(&linked.display().to_string());
        assert_eq!(in_primary.project, in_linked.project, "one project");
        // As the oracle joins it: `primary_root.join(relative_cwd)`.
        assert_eq!(in_linked.heading, primary.join("").display().to_string());
        assert_ne!(in_primary.directory, in_linked.directory, "two directories");
    }
}
