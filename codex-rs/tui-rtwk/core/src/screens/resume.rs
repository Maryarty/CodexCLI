//! `resume` (S18, resume and fork picker): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/resume/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "resume";
pub const ID: &str = "S18";
pub const KIND: Kind = Kind::Route;

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The picker's rows, newest first as the backend listed them.
    pub rows: Vec<Row>,
    /// The reference time of the relative ages, epoch seconds (R11: the
    /// clock is a model input).
    pub clock: i64,
    pub load: Load,
    /// The thread the user asked to resume (the app asks the backend).
    pub resuming: Option<ThreadId>,
    /// The picker was opened on the live backend: its rows are the
    /// backend's sessions (`sync`), not the ones a script seeded.
    pub live: bool,
}

/// Where the list stands (oracle: `PaginationState` and `inline_error`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Load {
    #[default]
    Idle,
    /// The first page is loading (`Loading sessions…`).
    Loading,
    /// A page failed; the message shows on the search line.
    Failed(String),
}

/// One saved thread in the picker's list (oracle: `resume_picker::Row`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: ThreadId,
    pub preview: String,
    pub model_provider: String,
    /// RFC 3339, when the thread started (the fake rollouts' form).
    pub started: String,
    pub thread_name: Option<String>,
    /// Epoch seconds.
    pub created: Option<i64>,
    pub updated: Option<i64>,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    /// The thread lives on a cloud backend (`☁︎`).
    pub cloud: bool,
    /// The thread has unread output (`●`).
    pub unread: bool,
}

impl Row {
    /// Oracle: `Row::display_preview`: the thread's name when it has one.
    pub fn display_preview(&self) -> &str {
        self.thread_name.as_deref().unwrap_or(&self.preview)
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// The backend listed the saved threads.
    Listed(Vec<Row>),
    /// The reference time of the ages.
    Clock(i64),
    /// A listing started.
    Loading,
    /// A listing failed with this message.
    Failed(String),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The picker opened on the live backend: the sessions are listed.
    Open,
    /// Enter on a row: resume (or fork) this thread.
    Resume(ThreadId),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Listed(rows) => {
            model.rows = rows;
            model.load = Load::Idle;
        }
        Ev::Clock(secs) => model.clock = secs,
        Ev::Loading => model.load = Load::Loading,
        Ev::Failed(message) => model.load = Load::Failed(message),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Open => model.live = true,
        Intent::Resume(id) => model.resuming = Some(id),
    }
}

/// The picker's rows out of the sessions the app knows (the command center's
/// list, which the backend's `thread/list` fills): the root threads, newest
/// first. Hidden tasks are in: the picker is how they come back.
pub fn listing(agents: &crate::screens::agents::Model) -> (Vec<Row>, Load) {
    let mut threads: Vec<_> = agents
        .threads
        .iter()
        .filter(|thread| {
            thread.parent_thread_id.is_none()
                && !thread.ephemeral
                && !agents.removed.contains(&thread.id)
        })
        .collect();
    threads.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    let rows: Vec<Row> = threads
        .into_iter()
        .map(|thread| Row {
            id: ThreadId::new(&thread.id),
            preview: thread.preview.clone(),
            model_provider: thread.model_provider.clone(),
            started: String::new(),
            thread_name: thread.name.clone(),
            created: Some(thread.created_at),
            updated: Some(thread.updated_at),
            cwd: Some(thread.cwd.display().to_string()),
            branch: thread.git_info.as_ref().and_then(|git| git.branch.clone()),
            cloud: false,
            unread: agents.unread.contains(&thread.id),
        })
        .collect();
    let load = if agents.paging.failed {
        Load::Failed("Could not load more sessions".to_owned())
    } else if agents.paging.loading && rows.is_empty() {
        Load::Loading
    } else {
        Load::Idle
    };
    (rows, load)
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// The seed form of this screen's events (`{"listed":{"rows":[..]}}`,
/// `{"clock":..}`, `{"loading":null}`, `{"failed":".."}`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    /// The backend listed the saved threads: the fields of the app-server
    /// tests' `create_fake_rollout`, and the picker's own.
    Listed { rows: Vec<RolloutSeed> },
    /// The reference time, epoch seconds.
    Clock(i64),
    Loading(()),
    Failed(String),
}

#[derive(Clone, Debug, serde::Deserialize)]
struct RolloutSeed {
    id: String,
    preview: String,
    #[serde(default)]
    model_provider: String,
    /// RFC 3339, the rollout's start.
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    thread_name: Option<String>,
    #[serde(default)]
    created: Option<i64>,
    #[serde(default)]
    updated: Option<i64>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    cloud: bool,
    #[serde(default)]
    unread: bool,
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Listed { rows } => {
            Ev::Listed(
                rows.into_iter()
                    .map(|row| Row {
                        id: ThreadId::new(row.id),
                        preview: row.preview,
                        model_provider: row.model_provider,
                        started: row.timestamp,
                        thread_name: row.thread_name,
                        created: row.created,
                        updated: row.updated,
                        cwd: row.cwd,
                        branch: row.branch,
                        cloud: row.cloud,
                        unread: row.unread,
                    })
                    .collect(),
            )
        }
        SeedEv::Clock(secs) => Ev::Clock(secs),
        SeedEv::Loading(()) => Ev::Loading,
        SeedEv::Failed(message) => Ev::Failed(message),
    })
}
