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

use codex_app_server_client::AppServerRequestHandle;
use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
use codex_core::config::edit::ConfigEdit;
use codex_core::config::edit::ConfigEditsBuilder;
use codex_core::config::load_config_toml_with_layer_stack;
use codex_tui_core::app::effects::Done;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::agents::DirKeys;
use codex_tui_core::screens::select::worktree::{Entry, Owner, Thread};
use codex_tui_rtwk::services::agents::workspace::Boxed;
use codex_tui_rtwk::services::agents::workspace::Checkout;
use codex_tui_rtwk::services::agents::workspace::Workspace;
use codex_tui_rtwk::services::backend::rpc::Ids;
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

/// `Ask::Worktrees` (oracle `worktree_browser::fetch`): the managed worktrees of the repository
/// `cwd` is in, each with its owner thread. The pool is the worktree manager's; the owner is read
/// from the app server (`read_thread`, one `thread/read` answered as JSON) and is archived when
/// its rollout lies in the archive, unavailable when the thread cannot be read.
pub(crate) async fn list_worktrees<F, Fut>(
    codex_home: PathBuf,
    cwd: String,
    read_thread: F,
) -> Result<Vec<Entry>, String>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, String>>,
{
    let mut entries = pool(codex_home.clone(), PathBuf::from(cwd)).await?;
    let archive = codex_home.join(ARCHIVED_SESSIONS_SUBDIR);
    for entry in &mut entries {
        let Owner::Unavailable(id) = &entry.owner else {
            continue;
        };
        let id = id.clone();
        let Ok(value) = read_thread(id.to_string()).await else {
            continue;
        };
        if let Some(owner) = owner_of(id, &value, &archive) {
            entry.owner = owner;
        }
    }
    Ok(entries)
}

/// The managed worktrees of the repository `cwd` is in (`Ask::Worktrees`): the owners are read
/// through the app server of `handle`.
pub(crate) async fn list(
    codex_home: PathBuf,
    cwd: String,
    ids: &Ids,
    handle: &AppServerRequestHandle,
) -> Done {
    let read = |thread: String| {
        let send = crate::in_process::sender(handle.clone());
        let id = ids.next();
        async move {
            send(codex_tui_core::screens::select::worktree::owner_request(
                &thread, id,
            ))
            .await
            .map_err(|error| error.message)
        }
    };
    Done::Worktrees(list_worktrees(codex_home, cwd, read).await)
}

/// The directory of the archived rollouts (`codex_rollout::ARCHIVED_SESSIONS_SUBDIR`).
const ARCHIVED_SESSIONS_SUBDIR: &str = "archived_sessions";

/// The title a worktree row shows is cut at this many characters.
const TITLE_LIMIT: usize = 80;

/// The owner a `thread/read` answer describes: none when it names no rollout that still exists.
fn owner_of(id: ThreadId, value: &serde_json::Value, archive: &Path) -> Option<Owner> {
    let thread = value.get("thread")?;
    let path = PathBuf::from(thread.get("path")?.as_str()?);
    if !path.exists() {
        return None;
    }
    let named = thread
        .get("name")
        .and_then(|name| name.as_str())
        .filter(|name| !name.trim().is_empty());
    let preview = thread
        .get("preview")
        .and_then(|preview| preview.as_str())
        .and_then(|preview| preview.lines().next())
        .filter(|line| !line.trim().is_empty());
    let title = named.or(preview).unwrap_or("Untitled conversation");
    let title = if title.chars().count() > TITLE_LIMIT {
        format!(
            "{}…",
            title.chars().take(TITLE_LIMIT - 1).collect::<String>()
        )
    } else {
        title.to_owned()
    };
    let summary = Thread {
        id,
        title,
        updated_at: thread
            .get("updatedAt")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_default(),
    };
    Some(if path.starts_with(archive) {
        Owner::Archived(summary)
    } else {
        Owner::Resumable(summary)
    })
}

/// The worktrees of the pool of `cwd`'s repository, owners recorded but not yet read.
async fn pool(codex_home: PathBuf, cwd: PathBuf) -> Result<Vec<Entry>, String> {
    let settings = worktree_settings(&codex_home).await?;
    codex_tui_rtwk::services::agents::workspace::blocking(move || {
        let cwd = codex_git_utils::get_git_repo_root(&cwd).unwrap_or(cwd);
        let manager = WorktreeManager::new(settings);
        let listed = manager.list(&cwd).map_err(|error| error.to_string())?;
        Ok(listed
            .into_iter()
            .map(|checkout| Entry {
                owner: manager
                    .owner(&checkout.root)
                    .ok()
                    .flatten()
                    .map(|owner| ThreadId::new(&owner))
                    .map_or(Owner::None, Owner::Unavailable),
                root: checkout.root.display().to_string(),
                cwd: checkout.cwd.display().to_string(),
            })
            .collect())
    })
    .await
    .unwrap_or_else(|| Err("Worktree listing task failed.".to_owned()))
}

/// `Ask::WorktreeRemove` (oracle `worktree_browser::remove`): `root` removed from the pool of
/// the repository `cwd` is in.
pub(crate) async fn remove_worktree(
    codex_home: PathBuf,
    cwd: String,
    root: String,
) -> Result<(), String> {
    let settings = worktree_settings(&codex_home).await?;
    codex_tui_rtwk::services::agents::workspace::blocking(move || {
        WorktreeManager::new(settings)
            .remove(Path::new(&cwd), Path::new(&root))
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap_or_else(|| Err("Worktree removal task failed.".to_owned()))
}

/// The settings of the worktree pool: the config's desktop table over the defaults.
async fn worktree_settings(codex_home: &Path) -> Result<WorktreeSettings, String> {
    let host = load_config_toml_with_layer_stack(
        codex_home,
        /*cwd*/ None,
        Vec::new(),
        codex_core::config::ConfigLoadOptions::default(),
    )
    .await
    .map_err(|error| error.to_string())?;
    WorktreeSettings::for_cli(codex_home, host.config_toml.desktop.as_ref())
        .map_err(|error| error.to_string())
}

impl Workspace for Local {
    fn dir_keys(&self, cwd: &str) -> Option<DirKeys> {
        Some(dir_keys(cwd))
    }

    fn save_grouping(&self, grouping: &str) -> Boxed<Result<(), String>> {
        let codex_home = self.codex_home.clone();
        let grouping = grouping.to_owned();
        Box::pin(async move {
            ConfigEditsBuilder::new(&codex_home)
                .with_edits([ConfigEdit::SetPath {
                    segments: vec!["tui".to_owned(), "agents_overview_grouping".to_owned()],
                    value: grouping.as_str().into(),
                }])
                .apply()
                .await
                .map_err(|error| error.to_string())
        })
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
            let checkout = codex_tui_rtwk::services::agents::workspace::blocking(move || {
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
mod tests {
    use super::*;

    /// A file with `text` (`std::fs::write` is the shell's: R12).
    fn put(path: impl AsRef<std::path::Path>, text: impl AsRef<str>) -> std::io::Result<()> {
        use std::io::Write as _;
        std::fs::File::create(path)?.write_all(text.as_ref().as_bytes())
    }

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
        put(git.join("HEAD"), "ref: refs/heads/main\n").expect("head");
        let linked = root.join("feature");
        std::fs::create_dir_all(&linked).expect("linked");
        let admin = git.join("worktrees").join("feature");
        put(admin.join("commondir"), "../..\n").expect("commondir");
        put(
            admin.join("gitdir"),
            format!("{}\n", linked.join(".git").display()),
        )
        .expect("gitdir");
        put(
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
