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
mod discovery;
mod notifications;
mod refresh;
mod sources;
mod update;
mod usage;
mod wire;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadUsage;
use codex_protocol::account::PlanType;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub use carried::AgentsOverviewGroup as Group;
pub use carried::format_tokens_compact;
pub use carried::preview_markdown;
pub use center::Center;
pub use center::CenterRow;
pub use center::GroupLabel;
pub use center::RowData;
pub use center::display_directory;
pub use details::DetailLine;
pub use details::Details;
pub use discovery::refill;
pub use notifications::observe;
pub use notifications::observes;
pub use notifications::wants;
pub use refresh::Out;
pub use refresh::account_ev;
pub use refresh::account_request;
pub use refresh::apply_answer;
pub use update::wants_dirs;
pub use sources::Fanout;
pub use sources::PinReader;
pub use sources::fanout;
pub use update::update;
pub use update::wanted_dirs;
pub use usage::UsageOutcome;
pub use usage::refresh_usage;
pub use usage::reset_for_backend_switch;
pub use usage::usage_wait;

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
    /// A worktree is being created for `w`.
    CreatingWorktree,
    LoadFailed,
    /// The server's own words about its version.
    ServerVersion(String),
    /// A failed action, as a message of the server (oracle:
    /// `add_agents_overview_error`).
    Error(String),
}

/// A removal the server refused (oracle: the `Could not archive task` /
/// `Could not delete task` selection view of `run_agents_overview_action`):
/// shown as a dialog with one row, `Back to agents`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub action: Action,
    /// The server's error, as `error_text` words it.
    pub error: String,
    /// The request was sent (the dialog warns that work may have stopped).
    pub attempted: bool,
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
}

/// What the latest reasoning summary of a task said (oracle:
/// `AgentsOverviewActivity`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Activity {
    pub latest_prompt: Option<String>,
    pub reasoning_header: Option<String>,
    pub last_message: Option<String>,
}

/// What is known of a task's usage (oracle: `AgentsOverviewUsage`): the
/// tokens its notifications reported, and the account's estimate with the
/// time (epoch seconds) it was fetched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// Input and output tokens.
    pub tokens: Option<(i64, i64)>,
    pub estimate: Option<ThreadUsage>,
    pub fetched_at: Option<i64>,
}

/// The keys the shell reads from the file system for one `cwd` (oracle:
/// `AgentsOverviewProjectGroup` and `directory_group_key`): the project is
/// its repository's common directory plus the path inside it, so linked
/// worktrees of one repository merge; the directory key is the canonical
/// path. Without an entry a `cwd` is its own key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirKeys {
    pub project: (String, String),
    /// The heading of the project group: the primary checkout's path.
    pub heading: String,
    pub directory: String,
}

/// A history search while a search line is open (oracle: `HistorySearch`);
/// its rows are `Model::search_matches`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    pub id: u64,
    /// How many matches the center wants under the active filter.
    pub target: usize,
    pub loading: bool,
    pub failed: bool,
    /// The paging cursor of the search's scan; `exhausted` once it ended.
    pub cursor: Option<String>,
    pub exhausted: bool,
    /// A page of this search was asked for (the shell debounces the first).
    pub started: bool,
    /// Matches the scan found beyond what was asked for.
    pub buffer: Vec<Thread>,
}

impl Eq for History {}
impl Eq for Paging {}

impl History {
    /// Oracle: `AgentsOverviewDiscovery::has_more`.
    pub fn has_more(&self) -> bool {
        !self.exhausted || !self.buffer.is_empty()
    }
}

/// Where the paged list stands (oracle: `loading`, `has_more`,
/// `refresh_failed` of the view state).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Paging {
    pub loading: bool,
    pub failed: bool,
    pub has_more: bool,
    /// The first page was merged: a refresh no longer moves the cursor.
    pub initialized: bool,
    /// The cursor the next page continues from.
    pub cursor: Option<String>,
    /// Rows fetched but not shown yet (oracle: the source's retained rows).
    pub buffer: Vec<Thread>,
    /// Vacancies left by removals, refilled without growing the window
    /// (oracle: `refill_count`).
    pub refill: usize,
    /// How many buffered rows the page in flight shows when it arrives.
    pub want: usize,
}

impl Paging {
    /// Oracle: `AgentsOverviewDiscovery::has_more`.
    pub fn more(&self) -> bool {
        self.cursor.is_some() || !self.buffer.is_empty()
    }
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
    /// Removed by a lifecycle action; a later page never brings them back
    /// (oracle: `removed_threads`).
    pub removed: BTreeSet<String>,
    pub last_messages: BTreeMap<String, String>,
    pub activity: BTreeMap<String, Activity>,
    /// The first pending request of a thread, previewed (oracle:
    /// `agents_overview_request_preview`).
    pub requests: BTreeMap<String, String>,
    pub usage: BTreeMap<String, Usage>,
    /// The account has no thread usage (oracle: `usage_disabled`).
    pub usage_disabled: bool,
    /// The usage request in flight: the thread and its ticket number.
    pub pending_usage: Option<(String, u64)>,
    pub usage_seq: u64,
    /// The account's plan and whether it has Codex backend auth (usage is
    /// fetched for the plans that show it).
    pub plan: Option<PlanType>,
    pub backend_auth: bool,
    /// The pinned threads in rank order; `None`: the server has no pins.
    pub pinned: Option<Vec<String>>,
    /// The thread the chat shows (oracle: `is_current`).
    pub current: Option<String>,
    /// The clock, epoch seconds (R11).
    pub now: i64,
    /// The home directory (the shell reads it): `~` in displayed paths.
    pub home: Option<String>,
    /// How many resumes were answered (the view pops itself on a new one).
    pub resumes: u32,
    pub paging: Paging,
    /// The backend is the cloud: a cloud archive removes only the task.
    pub cloud: bool,
    /// Worktrees are enabled (`w`, project grouping by repository).
    pub worktrees: bool,
    pub dirs: BTreeMap<String, DirKeys>,
    /// The `cwd`s the shell was asked the keys of (oracle: none, the view
    /// read them while building rows): asked once each.
    pub dirs_asked: BTreeSet<String>,
    /// A pin change is in flight (oracle: `pin_action_pending`).
    pub pin_pending: bool,
    pub history: Option<History>,
    /// The rows a history search found beyond the browsed list.
    pub search_matches: Vec<Thread>,
    pub search_seq: u64,
    pub notice: Option<Notice>,
    pub confirm: Option<Confirm>,
    /// A refused removal, until the user goes back (it holds the screen).
    pub failure: Option<Failure>,
    /// The thread that owns the voice session (oracle: `voice_owner_thread_id`).
    pub voice: Option<String>,
    /// Tasks go without their own colour (`tui.status_line_use_colors` is
    /// off); the default, colours on, is `false` here.
    pub thread_colors_off: bool,
    pub filter: usize,
    pub grouping: Grouping,
    pub search: String,
    pub searching: bool,
    pub rename: Option<Rename>,
    pub help: bool,
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
    /// The backend and what it enables.
    Backend {
        cloud: bool,
        worktrees: bool,
    },
    /// The directory keys of `cwd`s, read by the shell.
    Directories(Vec<(String, DirKeys)>),
    /// The account's plan and Codex backend auth.
    Account {
        plan: Option<PlanType>,
        backend_auth: bool,
    },
    /// The thread the chat shows.
    Current(Option<String>),
    Pinned(Option<Vec<String>>),
    /// The list failed to load.
    ListFailed,
    /// A notice, or none.
    Notice(Option<Notice>),
    /// A request waits on a thread (preview text), or was answered.
    Request(String, Option<String>),
    /// The shell started a task by itself (a worktree's) and it opens in the
    /// chat; the busy notice ends.
    Started(Box<Thread>),
    /// The thread that owns the voice session, if any.
    Voice(Option<String>),
    /// `tui.status_line_use_colors`: whether tasks get their own colour.
    ThreadColors(bool),
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
    /// The pinned tasks, read with the first page of the list.
    pub pins: Option<Pins>,
}

/// The pinned tasks as the server lists them (oracle: `list_pinned_threads`,
/// whose `Ok(None)` is a server without pins).
#[derive(Clone, Debug, PartialEq)]
pub struct Pins {
    /// Their ids in rank order; `None`: the server has no pins.
    pub ids: Option<Vec<String>>,
    /// The tasks themselves: a pinned task need not be among the recent ones.
    pub threads: Vec<Thread>,
}

/// What the user asks for; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The screen opened: list the threads.
    Open,
    Refresh,
    /// Tab (`back`: shift-tab).
    CycleFilter {
        back: bool,
    },
    SetFilter(usize),
    CycleGrouping,
    SetGrouping(Grouping),
    ToggleHelp,
    CloseHelp,
    BeginSearch,
    /// The search line's text changed.
    SetSearch(String),
    /// Esc / ctrl-c while editing: leave search or rename.
    CancelEditing,
    BeginRename(String),
    /// Enter on the list's cursor row: open the task or the "Show more" row.
    Accept(Sel),
    /// Right: as `Accept`, but not while a line is edited.
    Right(Sel),
    /// Enter in the rename line, with its text.
    Commit(String),
    Stop(String),
    Archive(String),
    Delete(String),
    /// The confirmation's answer.
    Confirmed(bool),
    /// `Back to agents` on the failure dialog.
    DismissFailure,
    Hide(String),
    TogglePin(String),
    /// `n`: a new task in the directory of the cursor's task, if any.
    New(Option<String>),
    /// `w`: a new task in a worktree of the cursor task's directory.
    NewWorktree(Option<String>),
    /// Fork the task and open the copy.
    Fork(String),
    /// The usage of the cursor's task is wanted (the clock ticks).
    NeedUsage(Option<String>),
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
    List {
        cursor: Option<String>,
    },
    Rename {
        thread: String,
        name: String,
    },
    Archive {
        thread: String,
    },
    Delete {
        thread: String,
    },
    /// The turn in progress of a thread (`thread/read` with turns), the first
    /// half of stopping it.
    ActiveTurn {
        thread: String,
    },
    Interrupt {
        thread: String,
        turn: String,
    },
    Pin {
        thread: String,
        pinned: bool,
    },
    /// Resume a thread into the chat (`thread/resume`).
    Resume {
        thread: String,
    },
    /// A new task in the same directory (`thread/start`).
    Start {
        cwd: Option<String>,
    },
    /// Fork a task (`thread/fork`).
    Fork {
        thread: String,
    },
    /// A new worktree in `cwd`'s repository with a task in it: git work,
    /// done by the shell (no RPC).
    NewWorktree {
        cwd: Option<String>,
    },
    /// A page of the history search's scan (`thread/list`, 100 rows, no
    /// server-side term: the core filters).
    Search {
        cursor: Option<String>,
        first: bool,
    },
    /// The usage of a task (`account/usage/read`).
    Usage {
        thread: String,
    },
}

/// The decoded replies.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    Listed(Page),
    Done,
    /// The id of the thread's turn in progress, when it has one.
    ActiveTurn(Option<String>),
    Resumed(Box<Thread>),
    Forked(Box<Thread>),
    /// The task `thread/start` made.
    Started(Box<Thread>),
    Usage(UsageOutcome),
}

impl Eq for Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    List {
        append: bool,
    },
    Rename {
        thread: String,
    },
    Action {
        thread: String,
        action: Option<Action>,
    },
    Pin {
        thread: String,
        pinned: bool,
    },
    Resume {
        thread: String,
    },
    /// The read of the turn to stop.
    Stop {
        thread: String,
    },
    Interrupt,
    Start,
    Fork,
    NewWorktree,
    /// A page of the search `id`, asked for `limit` matches.
    Search {
        id: u64,
        limit: usize,
    },
    Usage {
        thread: String,
        id: u64,
    },
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    refresh::apply_ev(model, ev);
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
    wire::wire_request(command, id)
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    wire::decode_reply(command, result)
}

/// A script's `screen` step for this directory. The forms: `{"listed":
/// {"threads":[<protocol Thread>..],"unread":[..],"next_cursor":..}}`,
/// `{"clock":n}`, `{"current":"id"}`, `{"pinned":["id"..]}`, `{"request":["id","text"]}`.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    wire::seed_ev(value).map_err(crate::screens::SeedError::Bad)
}
