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
use codex_app_server_protocol::ThreadUnsubscribeParams;
use codex_tui_core::app::AppEv;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Command as AgentsCommand;
use codex_tui_core::screens::agents::DirKeys;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::agents::Notice;
use codex_tui_core::screens::agents::Reply;
use codex_tui_core::screens::agents::account_ev;
use codex_tui_core::screens::agents::account_request;
use codex_tui_core::screens::agents::decode;
use codex_tui_core::screens::agents::wire as agents_wire;

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
    AppEv::Screen(ScreenEv::Agents(AgentsEv::Backend {
        cloud: facts.cloud,
        worktrees: facts.worktrees,
    }))
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
    /// Writes `tui.agents_overview_grouping` to the user's config; the
    /// message of why not.
    fn save_grouping(&self, grouping: &str) -> Boxed<Result<(), String>>;
}

/// Runs `work` on the blocking pool; `None` if it panicked.
pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    tokio::task::spawn_blocking(work).await.ok()
}

/// `Ev::Directories` for `cwds`: the keys the workspace reads; a `cwd` it
/// cannot read has none (it is its own key).
pub async fn directories(workspace: Arc<dyn Workspace>, cwds: Vec<String>) -> AppEv {
    let keys = blocking(move || {
        cwds.into_iter()
            .filter_map(|cwd| workspace.dir_keys(&cwd).map(|keys| (cwd, keys)))
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();
    AppEv::Screen(ScreenEv::Agents(AgentsEv::Directories(keys)))
}

/// The config edit of a grouping change; a failure is the center's error
/// notice (oracle: `persist_agents_overview_grouping`).
pub async fn save_grouping(workspace: Arc<dyn Workspace>, grouping: String) -> Option<AppEv> {
    let error = workspace.save_grouping(&grouping).await.err()?;
    Some(AppEv::Screen(ScreenEv::Agents(AgentsEv::Notice(Some(
        Notice::Error(format!("Failed to save Command Center grouping: {error}")),
    )))))
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
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let notice = |message: String| {
        AppEv::Screen(ScreenEv::Agents(AgentsEv::Notice(Some(Notice::Error(
            message,
        )))))
    };
    let checkout = match workspace.create_worktree(cwd).await {
        Ok(checkout) => checkout,
        Err(error) => return notice(error),
    };
    // Oracle: `agents_overview_retained_worktree_error`.
    let retained = |reason: String| {
        notice(format!(
            "{reason} A checkout was retained at {}; remove it with `git worktree remove <checkout-path>` from the source repository if it is no longer needed.",
            checkout.root
        ))
    };
    let start = AgentsCommand::Start {
        cwd: Some(checkout.cwd.clone()),
    };
    let wire = agents_wire(&start, &ThreadId::default(), ids.next());
    let started = match send(wire).await {
        Ok(result) => match decode(&start, result) {
            Ok(Reply::Started(thread)) => *thread,
            Ok(_) => return retained("Failed to start session: no task started".to_owned()),
            Err(error) => return retained(format!("Failed to start session: {error:?}")),
        },
        Err(error) => return retained(format!("Failed to start session: {error}")),
    };
    // Oracle: a task that cannot be bound to its checkout is unsubscribed
    // (`thread_unsubscribe`) before the error shows; a failure of that is
    // the server's own, not shown.
    let unsubscribe = || async {
        let wire = ClientRequest::ThreadUnsubscribe {
            request_id: ids.next(),
            params: ThreadUnsubscribeParams {
                thread_id: started.id.clone(),
            },
        };
        let _ = send(wire).await;
    };
    let bound = {
        let workspace = workspace.clone();
        let checkout = checkout.clone();
        let thread = started.id.clone();
        let cwd = started.cwd.display().to_string();
        blocking(move || workspace.bind_worktree(&checkout, &thread, &cwd)).await
    };
    match bound {
        Some(Ok(())) => AppEv::Screen(ScreenEv::Agents(AgentsEv::Started(Box::new(started)))),
        Some(Err(error)) => {
            unsubscribe().await;
            retained(error)
        }
        None => {
            unsubscribe().await;
            retained("The worktree could not be bound.".to_owned())
        }
    }
}

/// The account the server is signed in to (oracle: `read_account` at
/// bootstrap, `SessionAccount::from`): the plan and the Codex backend auth
/// the center's usage fetch needs. `None` when the read fails or the reply
/// is not an account: the center then keeps usage off, as the oracle's
/// bootstrap would not have started either.
pub async fn account<F, Fut>(ids: Ids, send: F) -> Option<AppEv>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let result = send(account_request(ids.next())).await.ok()?;
    account_ev(result)
        .ok()
        .map(|ev| AppEv::Screen(ScreenEv::Agents(ev)))
}
