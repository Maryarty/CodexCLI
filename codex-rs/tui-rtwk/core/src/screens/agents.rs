//! `agents` (S21, agents command center): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/agents/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "agents";
pub const ID: &str = "S21";
pub const KIND: Kind = Kind::Route;

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The root sessions of the command center, in the order the backend
    /// listed them (oracle: `AgentsOverviewView::rows`).
    pub tasks: Vec<Task>,
    /// The highlighted task (the first when none is).
    pub selected: Option<ThreadId>,
    pub filter: Filter,
}

/// One root session (oracle: `agents_overview_threads`, the rows of
/// `shared_overview_shows_only_root_sessions`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub model: Option<String>,
    pub reasoning: Option<String>,
    pub cwd: String,
    pub prompt: String,
    /// The subagent that needs attention, when one does.
    pub child: Option<Child>,
}

/// The backend's thread status (oracle: `ThreadStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Idle,
    Active,
    NotLoaded,
}

/// A subagent of a task and what it waits for.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Child {
    pub name: String,
    #[serde(default)]
    pub last_message: Option<String>,
    #[serde(default)]
    pub waits: Vec<Wait>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wait {
    Approval,
    Response,
}

/// The tab row's filters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    NeedsYou,
    Working,
    Ready,
    Inactive,
}

/// What a task needs of the user (oracle: the overview's attention groups).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attention {
    NeedsYou,
    Working,
    Ready,
    Inactive,
}

impl Task {
    /// A waiting subagent makes the task need the user; otherwise the
    /// thread's status decides.
    pub fn attention(&self) -> Attention {
        if self.child.as_ref().is_some_and(|child| !child.waits.is_empty()) {
            return Attention::NeedsYou;
        }
        match self.status {
            Status::Active => Attention::Working,
            Status::Idle => Attention::Ready,
            Status::NotLoaded => Attention::Inactive,
        }
    }
}

impl Model {
    /// How many tasks need `attention`.
    pub fn count(&self, attention: Attention) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.attention() == attention)
            .count()
    }

    /// The tasks the filter keeps.
    pub fn visible(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| match self.filter {
                Filter::All => true,
                Filter::NeedsYou => task.attention() == Attention::NeedsYou,
                Filter::Working => task.attention() == Attention::Working,
                Filter::Ready => task.attention() == Attention::Ready,
                Filter::Inactive => task.attention() == Attention::Inactive,
            })
            .collect()
    }

    /// The highlighted task: the selected one, else the first visible.
    pub fn highlighted(&self) -> Option<&Task> {
        let visible = self.visible();
        self.selected
            .as_ref()
            .and_then(|id| visible.iter().copied().find(|task| task.id == id.as_str()))
            .or_else(|| visible.first().copied())
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`. The seed form is `{"listed":[..]}`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ev {
    /// The backend listed the root sessions.
    Listed(Vec<Task>),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Move the highlight to this task.
    Select(ThreadId),
    SetFilter(Filter),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Listed(tasks) => {
            model.tasks = tasks;
            let known = model
                .selected
                .as_ref()
                .is_some_and(|id| model.tasks.iter().any(|task| task.id == id.as_str()));
            if !known {
                model.selected = None;
            }
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Select(id) => {
            if model.tasks.iter().any(|task| task.id == id.as_str()) {
                model.selected = Some(id);
            }
        }
        Intent::SetFilter(filter) => model.filter = filter,
    }
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

/// A script's `screen` step for this directory: the event in its serde form.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))
}
