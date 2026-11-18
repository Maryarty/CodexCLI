//! What the shell knows of the machine the command center runs on, and the
//! effects that read it: the directory keys of the tasks' cwds, and a new
//! task in a new worktree. The host implements `Workspace` (git, the file
//! system); the effects here are shared by the host's backend and the fake.
//! Oracle: `agents_overview_view.rs` (`directory_group_key`,
//! `AgentsOverviewProjectGroup`), `agents_overview_new.rs`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use codex_app_server_protocol::ClientRequest;
use codex_tui_core::app::AppEv;
use codex_tui_core::screens::agents::DirKeys;

use super::rpc::Ids;

/// What the backend is, as the oracle derives it: `cloud` is the cloud task
/// list (`AppServerRequestHandle::Cloud`), `worktrees` is a local workspace
/// (`!uses_remote_workspace_or_environment`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub cloud: bool,
    pub worktrees: bool,
}

/// The event that tells the center what the backend is.
pub fn backend_ev(facts: Facts) -> AppEv {
    let _ = facts;
    todo!("backend_ev")
}

/// A checkout made for a new task: its root, and the directory the task
/// starts in (oracle: `ManagedWorktree::{root, cwd}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkout {
    pub root: String,
    pub cwd: String,
}

/// A boxed future, as the trait needs it.
pub type Boxed<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// The machine: blocking reads, called by the shell's effects off the async
/// threads (`blocking`), except where a future is returned.
pub trait Workspace: Send + Sync + 'static {
    /// The canonical directory and repository keys of `cwd`.
    fn dir_keys(&self, cwd: &str) -> Option<DirKeys>;
    /// A new checkout of `source_cwd`'s repository (the default branch); the
    /// message of why not, as the oracle words it.
    fn create_worktree(&self, source_cwd: Option<String>) -> Boxed<Result<Checkout, String>>;
    /// Binds the started task to its checkout; `cwd` is the directory the
    /// server started the task in, which must be the checkout's.
    fn bind_worktree(&self, checkout: &Checkout, thread: &str, cwd: &str) -> Result<(), String>;
}

/// Runs `work` on the blocking pool; `None` if it panicked.
pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let _ = work;
    todo!("blocking")
}

/// `Ev::Directories` for `cwds`: the keys the workspace reads; a `cwd` it
/// cannot read has none (it is its own key).
pub async fn directories(workspace: Arc<dyn Workspace>, cwds: Vec<String>) -> AppEv {
    let _ = (workspace, cwds);
    todo!("directories")
}

/// A new task in a new worktree of `cwd`'s repository, as the oracle does
/// it: create the checkout, start the task there (`send` is the client),
/// bind it. The event is `Ev::Started` or the error as the center's notice;
/// a checkout left behind by a later failure is named in the message.
pub async fn worktree<F, Fut>(
    workspace: Arc<dyn Workspace>,
    cwd: Option<String>,
    ids: Ids,
    send: F,
) -> AppEv
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, String>>,
{
    let _ = (workspace, cwd, ids, send);
    todo!("worktree")
}
