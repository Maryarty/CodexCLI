//! `agents` (S21, the Agent command center): the core half. The model is the
//! oracle's `AgentsOverviewState` + `AgentsOverviewViewState` with the
//! `TextArea` and completion left out; the rows are the protocol's `Thread`.
//! Everything the view shows is derived from it by `Model::center` (R7: never
//! stored). Intents change view state and ask the backend through `Out`; the
//! app (`app.rs`) mints a ticket for each `Out` and sends it. No rtwk, no IO.
//! Oracle: `app/agents_overview*.rs`, `app/agent_center/`.
//!
//! API note (stable; the tests port targets it):
//! - State: `Model` (`CodexApp.screens.agents`); `Model::default()` is the
//!   blank screen. Derived, never stored: `Model::center() -> Center`
//!   (`rows`, `entries: Vec<CenterRow>`, `tab_counts`, `selected_row()`,
//!   `show_more_selected`), `Model::details(&RowData)`.
//! - Events (`Ev`, through `apply`/`AppEv::Screen(ScreenEv::Agents(..))`):
//!   `Listed(Page)`, `Clock(i64)` (now, epoch seconds), `Current(Option<id>)`,
//!   `Pinned(Option<Vec<id>>)`, `ListFailed`, `Notice`, `Request`. A script's
//!   `screen` step builds them through `seed` (`{"listed":{"threads":[..]}}`).
//! - Intents (`Intent`, through `AppEv::Intent(Intent::Screen(ScreenIntent::
//!   Agents(..)))`): `update(&mut Model, Intent) -> Vec<Out>` is the pure
//!   form; the app mints a ticket per `Out` and sends `Command::Screen`.
//! - RPCs: `Command` -> `wire`/`decode` -> `Reply`; `apply_answer(model, ctx,
//!   result)`; notifications through `observe` (`observes` says which).

mod carried;
mod center;
mod details;
mod update;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub use carried::AgentsOverviewGroup as Group;
pub use center::Center;
pub use center::CenterRow;
pub use center::GroupLabel;
pub use center::RowData;
pub use details::DetailLine;
pub use details::Details;
pub use carried::format_tokens_compact;
pub use carried::preview_markdown;
pub use center::display_directory;
pub use update::Out;
pub use update::apply_answer;
pub use update::observe;
pub use update::observes;
pub use update::update;

pub const NAME: &str = "agents";
pub const ID: &str = "S21";
pub const KIND: Kind = Kind::Route;

/// How many matches / rows one page of the list asks for (oracle: `PAGE_SIZE`).
pub const PAGE_SIZE: u32 = 10;

/// Command center grouping (oracle: `AgentsOverviewGrouping`).
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum Grouping {
    #[default]
    Project,
    Directory,
    Status,
    Model,
}

impl Grouping {
    /// The next one of the `g` cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Project => Self::Directory,
            Self::Directory => Self::Status,
            Self::Status => Self::Model,
            Self::Model => Self::Project,
        }
    }

    /// The value written to `tui.agents_overview_grouping`; `Directory` is
    /// view-local (oracle: `AgentsOverviewGrouping::persisted`).
    pub fn persisted(self) -> Option<&'static str> {
        match self {
            Self::Project => Some("project"),
            Self::Directory => None,
            Self::Status => Some("status"),
            Self::Model => Some("model"),
        }
    }

    /// The configured value (`tui.agents_overview_grouping`).
    pub fn from_config(value: &str) -> Option<Self> {
        match value {
            "project" => Some(Self::Project),
            "status" => Some(Self::Status),
            "model" => Some(Self::Model),
            _ => None,
        }
    }
}

/// What the highlight is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sel {
    Task(String),
    /// The "Show more" row after the last task.
    ShowMore,
}

/// A lifecycle action that holds the server's lock (oracle:
/// `AgentsOverviewAction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Archive,
    Delete,
}

/// The notice that replaces the footer hints (oracle: `connection_notice`,
/// `creating_worktree`, `removal`, `refresh_failed`, `server_version_notice`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    Reconnecting,
    ReconnectFailed,
    Removing(Action),
    LoadFailed,
    /// The server's own words about its version.
    ServerVersion(String),
    /// A failed action, as a message of the server (oracle:
    /// `add_agents_overview_error`).
    Error(String),
}

/// A confirmation waiting for the user (oracle: `confirm_agents_overview_action`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirm {
    pub thread: String,
    pub action: Action,
}

/// The rename editor (oracle: `rename_target` and the view's `input`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rename {
    pub target: String,
    pub text: String,
}

/// What the latest reasoning summary of a task said (oracle:
/// `AgentsOverviewActivity`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Activity {
    pub latest_prompt: Option<String>,
    pub reasoning_header: Option<String>,
    pub last_message: Option<String>,
}

/// Tokens of a task (oracle: `AgentsOverviewUsage.tokens`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: i64,
    pub output: i64,
}

/// Where the paged list stands (oracle: `loading`, `has_more`,
/// `refresh_failed` of the view state).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Paging {
    pub loading: bool,
    pub failed: bool,
    pub has_more: bool,
    /// The cursor the next page continues from.
    pub cursor: Option<String>,
}

/// The app-scoped model of the screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    /// Every thread the center knows, children included, in the order the
    /// backend listed them (oracle: `threads`).
    pub threads: Vec<Thread>,
    pub unread: BTreeSet<String>,
    /// Local visibility only (oracle: `hidden_threads`).
    pub hidden: BTreeSet<String>,
    pub last_messages: BTreeMap<String, String>,
    pub activity: BTreeMap<String, Activity>,
    /// The first pending request of a thread, previewed (oracle:
    /// `agents_overview_request_preview`).
    pub requests: BTreeMap<String, String>,
    pub usage: BTreeMap<String, Usage>,
    /// The pinned threads in rank order; `None`: the server has no pins.
    pub pinned: Option<Vec<String>>,
    /// The thread the chat shows (oracle: `is_current`).
    pub current: Option<String>,
    /// The clock, epoch seconds (R11).
    pub now: i64,
    /// The home directory (the shell reads it): `~` in displayed paths.
    pub home: Option<String>,
    pub paging: Paging,
    pub notice: Option<Notice>,
    pub confirm: Option<Confirm>,
    pub filter: usize,
    pub grouping: Grouping,
    pub search: String,
    pub searching: bool,
    pub rename: Option<Rename>,
    pub selected: Option<Sel>,
    pub help: bool,
    /// The selection before a search began (oracle: `browse_selection`).
    pub browse: Option<Sel>,
}

impl Eq for Model {}

/// The events of this screen; a variant of the root `ScreenEv`. The seed
/// form of a script is the serde form of `Ev`.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// The first page of the list, or a refresh of it.
    Listed(Page),
    /// The clock, epoch seconds.
    Clock(i64),
    /// The home directory, for `~`.
    Home(Option<String>),
    /// The thread the chat shows.
    Current(Option<String>),
    Pinned(Option<Vec<String>>),
    /// The list failed to load.
    ListFailed,
    /// A notice, or none.
    Notice(Option<Notice>),
    /// A request waits on a thread (preview text), or was answered.
    Request(String, Option<String>),
}

impl Eq for Ev {}

/// One page of `thread/list`.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub threads: Vec<Thread>,
    pub unread: Vec<String>,
    pub next_cursor: Option<String>,
    /// The page continues the list instead of replacing it.
    pub append: bool,
}

/// What the user asks for; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The screen opened: list the threads.
    Open,
    Refresh,
    Move { forward: bool },
    Jump { top: bool },
    /// A page of the list; `height` is the viewport's rows.
    Page { forward: bool, height: usize },
    Select(Sel),
    /// Tab (`back`: shift-tab).
    CycleFilter { back: bool },
    SetFilter(usize),
    CycleGrouping,
    SetGrouping(Grouping),
    ToggleHelp,
    CloseHelp,
    BeginSearch,
    /// Text typed or pasted into the search or rename line.
    Type(String),
    Backspace,
    /// Esc / ctrl-c while editing: leave search or rename.
    CancelEditing,
    BeginRename,
    /// Enter: open the task, the "Show more" row, or commit the rename.
    Accept,
    Stop,
    Archive,
    Delete,
    /// The confirmation's answer.
    Confirmed(bool),
    Hide,
    TogglePin,
    New,
}

/// The per-thread part of the screen: none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen. They are app-scoped: the app mints the ticket
/// (`update::Out`) and the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    List { cursor: Option<String> },
    Rename { thread: String, name: String },
    Archive { thread: String },
    Delete { thread: String },
    /// The turn in progress of a thread (`thread/read` with turns), the first
    /// half of stopping it.
    ActiveTurn { thread: String },
    Interrupt { thread: String, turn: String },
    Pin { thread: String, pinned: bool },
    /// Resume a thread into the chat (`thread/resume`).
    Resume { thread: String },
    /// A new task in the same directory (`thread/start`).
    Start { cwd: Option<String> },
}

/// The decoded replies.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    Listed(Page),
    Done,
    /// The id of the thread's turn in progress, when it has one.
    ActiveTurn(Option<String>),
    Resumed(Box<Thread>),
}

impl Eq for Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    List { append: bool },
    Rename { thread: String },
    Action { thread: String, action: Option<Action> },
    Pin { thread: String },
    Resume { thread: String },
    /// The read of the turn to stop.
    Stop { thread: String },
    Interrupt,
    Start,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    update::apply_ev(model, ev);
}

/// The screen-slot entry; the app routes `Intent`s through `update` so that
/// it can mint tickets, and this path (a script's intent) drops the RPCs.
pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    let _ = update::update(model, intent);
}

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
    let _ = (part, ctx, result, fx);
}

/// The wire request of `command` (the slot's per-thread form; the thread is
/// not used: these RPCs are app-scoped).
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = thread;
    update::wire_request(command, id)
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    update::decode_reply(command, result)
}

/// A script's `screen` step for this directory. The forms: `{"listed":
/// {"threads":[<protocol Thread>..],"unread":[..],"next_cursor":..}}`,
/// `{"clock":n}`, `{"current":"id"}`, `{"pinned":["id"..]}`, `{"request":["id","text"]}`.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    update::seed_ev(value).map_err(crate::screens::SeedError::Bad)
}
