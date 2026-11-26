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
//!   `Listed(Page)`, `Current(Option<id>)`,
//!   `Pinned(Option<Vec<id>>)`, `ListFailed`, `Notice`, `Request`. The clock is the app's (`AppEv::Clock`, `Model::now`). A script's
//!   `screen` step builds them through `seed` (`{"listed":{"threads":[..]}}`).
//! - Intents (`Intent`, through `AppEv::Intent(Intent::Screen(ScreenIntent::
//!   Agents(..)))`): `update(&mut Model, Intent) -> Vec<Out>` is the pure
//!   form; the app mints a ticket per `Out` and sends `Command::Screen`.
//! - RPCs: `Command` -> `wire`/`decode` -> `Reply`; `apply_answer(model, ctx,
//!   result)`; notifications through `observe` (`observes` says which).

pub(crate) mod carried;
mod center;
mod details;
mod discovery;
mod notifications;
mod reads;
mod refresh;
mod sources;
mod update;
mod usage;

use crate::threads::ThreadAsk;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::ThreadUsage;
use codex_protocol::account::PlanType;

use crate::app::AppFx;
use crate::ids::BackendId;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub use crate::protocol::agents::account_ev;
pub use crate::protocol::agents::account_request;
pub use carried::AgentsOverviewGroup as Group;
pub use carried::preview_markdown;
pub(crate) use carried::{PREVIEW_CHARS, preview_text, request_preview};
pub use center::Center;
pub use center::CenterRow;
pub use center::CenterThreads;
pub use center::GroupLabel;
pub use center::Lens;
pub use center::RowData;
pub use center::display_directory;
pub use details::DetailLine;
pub use details::Details;
pub use discovery::refill;
pub use notifications::observe;
pub use notifications::observes;
pub use refresh::Out;
pub use refresh::apply_answer;
pub use refresh::attach_loaded;
pub(crate) use refresh::finish_start;
pub use refresh::is_active_writer;
pub use sources::Fanout;
pub use sources::PinReader;
pub use sources::fanout;
pub use update::update;
pub use update::wake_at;
pub use update::wanted_dirs;
pub use update::wants_dirs;
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
/// `creating_worktree`, `removal`, `refresh_failed`; the server version
/// notice is `Model::server_version`, which yields to all of them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    Reconnecting,
    ReconnectFailed,
    Removing(Action),
    /// A worktree is being created for `w`.
    CreatingWorktree,
    WorktreeRetained {
        root: String,
        reason: crate::app::worktree::RetainedReason,
    },
    LoadFailed,
    /// The host could not look up the destination folder’s trust.
    TrustCheckFailed(String),
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

/// What is known of a task's usage (oracle: `AgentsOverviewUsage`): the
/// tokens its notifications reported, and the account's estimate with the
/// time (epoch seconds) it was fetched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// Input and output tokens.
    pub tokens: Option<(i64, i64)>,
    /// Core's cache of the answer; views read the figures through the
    /// methods below (a layers rule bans `.estimate` in view sources).
    pub estimate: Option<ThreadUsage>,
    pub fetched_at: Option<i64>,
}

impl Usage {
    /// The estimate's credits, in micros; `None` before it was fetched.
    pub fn credits_micros(&self) -> Option<i64> {
        self.estimate
            .as_ref()
            .map(|estimate| estimate.estimated_usage_credits_micros)
    }

    /// The estimate in dollars, in micros.
    pub fn usd_micros(&self) -> Option<i64> {
        self.estimate.as_ref()?.estimated_usage_usd_micros
    }

    /// The native cost in dollars, in micros (a plan billed in dollars).
    pub fn native_usd_micros(&self) -> Option<i64> {
        self.estimate.as_ref()?.native_usage_usd_micros
    }

    /// Whether an estimate is known.
    pub fn has_estimate(&self) -> bool {
        self.estimate.is_some()
    }
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

/// Phase of a catalog read. A revert or lag invalidates only a turns reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailRead {
    Thread,
    Turns,
    StaleTurns,
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
    /// The details read in progress for each task, and reruns requested
    /// while it is in flight.
    pub reading: BTreeMap<ThreadId, DetailRead>,
    pub reread: BTreeSet<ThreadId>,
    /// What notifications set on a task while its read was in flight: they
    /// are newer than the read and are replayed over it (oracle:
    /// `refresh_notifications`).
    pub overlay: BTreeMap<String, Overlay>,
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
    /// The backend the chat shows; its rows are the ones the list pages of
    /// `Ev::Listed` belong to (oracle: `app_server_backend_id`).
    pub displayed: BackendId,
    /// The backend that owns each thread: the first that listed it (oracle:
    /// `BackendRegistry::observe`). A later backend's row of the same id is
    /// dropped.
    pub owners: BTreeMap<ThreadId, BackendId>,
    /// Each backend's pins in its own rank order; `None`: it has none
    /// (oracle: `pins_by_backend`).
    pub pins_by_backend: BTreeMap<BackendId, Option<Vec<ThreadId>>>,
    /// The clock, epoch seconds (R11).
    pub now: i64,
    /// The home directory (the shell reads it): `~` in displayed paths.
    pub home: Option<String>,
    /// How many resumes were answered (the view pops itself on a new one).
    pub resumes: u32,
    /// The task being opened from the center: its resume is out and the
    /// session page shows its loading state until the reply (success or
    /// failure) clears it (open-instant: the page change does not wait for
    /// the backend).
    pub opening: Option<ThreadId>,
    /// The displayed backend's paging; the others' are `parked` (oracle:
    /// `initialized` and `discovery` per backend).
    pub paging: Paging,
    pub parked: BTreeMap<BackendId, Paging>,
    /// The displayed backend is the cloud: a cloud archive removes only the
    /// task.
    pub cloud: bool,
    /// The backends that are the cloud (their tasks archive alone).
    pub clouds: BTreeSet<BackendId>,
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
    /// The server's own words about its version (oracle:
    /// `server_version_notice`): the footer shows them when no notice does.
    pub server_version: Option<String>,
    /// A refused removal, until the user goes back (it holds the screen).
    pub failure: Option<Failure>,
    /// The thread that owns the voice session (oracle: `voice_owner_thread_id`).
    pub voice: Option<String>,
    /// Tasks go without their own colour (`tui.status_line_use_colors` is
    /// off); the default, colours on, is `false` here.
    pub thread_colors_off: bool,
    pub grouping: Grouping,
    pub search: String,
}

impl Eq for Model {}

/// The events of this screen; a variant of the root `ScreenEv`. The seed
/// form of a script is the serde form of `Ev`.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// The first page of the list, or a refresh of it.
    Listed(Page),
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
    Current(Option<ThreadId>),
    Pinned(Option<Vec<String>>),
    /// The list failed to load.
    ListFailed,
    /// A notice, or none.
    Notice(Option<Notice>),
    /// The server is older than (or differs from) the client: its words, or
    /// none (oracle: `update_server_version_overview_notice`).
    ServerVersion(Option<String>),
    /// The connection to the server was lost and is being restored: the
    /// stale reads are dropped (oracle: `begin_reconnect`) and the footer
    /// says `Reconnecting`.
    Reconnecting,
    /// A new connection is up (oracle: `finish_reconnect`): the notice ends
    /// and the list is read again.
    Reconnected,
    /// A request waits on a thread (preview text), or was answered.
    Request(ThreadId, Option<String>),
    /// The thread that owns the voice session, if any.
    Voice(Option<String>),
    /// `tui.status_line_use_colors`: whether tasks get their own colour.
    ThreadColors(bool),
    /// `tui.agents_overview_grouping`: the grouping the center opens with.
    Grouping(Grouping),
    /// A page of one backend's list (`Listed` is the displayed backend's).
    ListedBy(BackendId, Page),
    /// The chat now shows this backend's session (oracle:
    /// `app_server.backend_id()` after a backend switch): its paging is the
    /// displayed one, the previous one is kept.
    Displayed(BackendId),
    /// This backend is the cloud.
    CloudBackend(BackendId),
    /// The event stream dropped events (oracle: `AppServerEvent::Lagged`):
    /// what was derived from it is dropped.
    Lagged,
}

impl Eq for Ev {}

/// What a notification set on a task while it was being read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    pub name: Option<Option<String>>,
    pub status: Option<ThreadStatus>,
    /// The directory, model and provider of a `ThreadSettingsUpdated`.
    pub settings: Option<codex_app_server_protocol::ThreadSettings>,
}

impl Model {
    /// Claims `id` for `owner` unless another backend holds it; whether
    /// `owner` owns it (oracle: `BackendRegistry::observe`).
    pub fn observe(&mut self, owner: BackendId, id: &str) -> bool {
        *self.owners.entry(ThreadId::new(id)).or_insert(owner) == owner
    }

    /// The backend a task belongs to: its owner, the displayed one when no
    /// backend has claimed it (oracle: `owner(id).or(displayed)`).
    pub fn owner_of(&self, id: &str) -> BackendId {
        self.owners.get(id).copied().unwrap_or(self.displayed)
    }

    /// One backend's paging: the displayed one's, a parked one, or none read
    /// yet (oracle: `discovery.get(&backend)`, `initialized`).
    pub fn paging_of(&self, backend: BackendId) -> &Paging {
        static NONE: Paging = Paging {
            loading: false,
            failed: false,
            has_more: false,
            initialized: false,
            cursor: None,
            buffer: Vec::new(),
            refill: 0,
            want: 0,
        };
        if backend == self.displayed {
            &self.paging
        } else {
            self.parked.get(&backend).unwrap_or(&NONE)
        }
    }

    /// Whether the backend has more to list; true while nothing was read
    /// from it (oracle: `has_more`, `is_none_or`).
    pub fn has_more(&self, backend: BackendId) -> bool {
        let paging = self.paging_of(backend);
        !paging.initialized || paging.more()
    }

    /// Makes `backend` the displayed one: its paging becomes `paging` and
    /// the one it replaces is kept.
    pub fn display(&mut self, backend: BackendId) {
        if backend == self.displayed {
            return;
        }
        let next = self.parked.remove(&backend).unwrap_or_default();
        let previous = std::mem::replace(&mut self.paging, next);
        self.parked.insert(self.displayed, previous);
        self.displayed = backend;
        self.cloud = self.clouds.contains(&backend);
    }

    /// The backends the chat can move to and from: the first and the clouds
    /// (oracle: the listening ones, in registration order).
    fn connected(&self) -> BTreeSet<BackendId> {
        let mut all = self.clouds.clone();
        all.insert(BackendId::default());
        all.insert(self.displayed);
        all
    }

    /// The backend after the displayed one, wrapping; none with only one
    /// connected (oracle: `next_listening_backend`).
    pub fn next_backend(&self) -> Option<BackendId> {
        let all = self.connected();
        all.range((
            std::ops::Bound::Excluded(self.displayed),
            std::ops::Bound::Unbounded,
        ))
        .chain(all.iter())
        .copied()
        .find(|backend| *backend != self.displayed)
    }

    /// What the backend control offers (a second backend is connected): the displayed backend's
    /// name; none otherwise.
    pub fn cycle_offer(&self) -> Option<crate::app::BackendLabel> {
        self.next_backend().map(|_| self.label())
    }

    /// The displayed backend's name (oracle: `displayed_name`).
    pub fn label(&self) -> crate::app::BackendLabel {
        use crate::app::BackendLabel;
        if self.is_cloud(self.displayed) {
            BackendLabel::Cloud
        } else if self.displayed == BackendId::default() {
            BackendLabel::Local
        } else {
            BackendLabel::Connection(self.displayed.0 + 1)
        }
    }

    /// Whether the tasks of `backend` are the cloud's.
    pub fn is_cloud(&self, backend: BackendId) -> bool {
        self.clouds.contains(&backend) || (backend == self.displayed && self.cloud)
    }

    /// The pin action works when the displayed backend has pins (oracle:
    /// `pin_action_supported`; true while it was not listed).
    pub fn pin_action_supported(&self) -> bool {
        self.pins_by_backend
            .get(&self.displayed)
            .is_none_or(Option::is_some)
    }

    /// True when at least one backend advertised pin support.
    pub fn pins_available(&self) -> bool {
        self.pins_by_backend.values().any(Option::is_some)
    }

    /// Pinned threads across the backends in rank order.
    pub fn pinned_ids(&self) -> impl Iterator<Item = &str> {
        self.pins_by_backend
            .values()
            .flatten()
            .flatten()
            .map(ThreadId::as_str)
    }

    /// Sets one backend's pins in rank order.
    pub fn set_pins(&mut self, owner: BackendId, pins: Option<Vec<String>>) {
        self.pins_by_backend.insert(
            owner,
            pins.map(|pins| pins.into_iter().map(ThreadId::new).collect()),
        );
    }
}

/// The backend an RPC of this screen goes to: a task's owner for what is
/// about a task, the displayed backend for the rest (oracle: the owner's
/// session answers `thread/*` for its tasks).
pub fn backend_of(model: &Model, command: &Command) -> BackendId {
    match command {
        Command::Rename { thread, .. }
        | Command::Archive { thread }
        | Command::Delete { thread }
        | Command::Pin { thread, .. }
        | Command::Usage { thread } => model.owner_of(thread),
        Command::Thread(ask) => ask
            .thread()
            .map_or(model.displayed, |thread| model.owner_of(thread)),
        Command::List { .. }
        | Command::NewWorktree { .. }
        | Command::Search { .. }
        | Command::Loaded
        | Command::SaveGrouping { .. } => model.displayed,
    }
}

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
    CycleGrouping,
    SetGrouping(Grouping),
    /// The search line's text changed (the view owns whether it is open).
    SetSearch {
        text: String,
        /// What the view shows: it decides which rows count as visible.
        lens: Lens,
    },
    /// The search line closed: the query and what it found are dropped.
    ClearSearch,
    /// Enter on the "Show more" row.
    ShowMore,
    /// Esc on the loading page: the opening is given up and the resume's
    /// late answer is ignored.
    CancelOpen,
    /// Enter on the "Show more" row while a search is open: more matches.
    MoreMatches(Lens),
    /// The rename line was accepted: the task is named `name`.
    Rename {
        thread: String,
        name: String,
    },
    Stop(String),
    /// Archive / delete the task (the view asked the user first).
    Archive(String),
    Delete(String),
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
    Pin {
        thread: String,
        pinned: bool,
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
    /// The tasks the server holds loaded (`thread/loaded/list`).
    Loaded,
    /// Writes the grouping to the config: not an RPC, `app::send_agents`
    /// hands it to the shell.
    SaveGrouping {
        grouping: String,
    },
    /// A thread RPC: the app makes it (ADR-003, `threads::ThreadAsk`).
    Thread(ThreadAsk),
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
    /// The task as read, and whether it has unread output.
    Read {
        thread: Box<Thread>,
        unread: bool,
    },
    /// The previews of a task's last turn (oracle: `latest_prompt` and
    /// `last_message` of the refresh).
    Turns {
        prompt: Option<String>,
        last_message: Option<String>,
    },
    Loaded(Vec<String>),
    /// A page of turns, newest first, and the cursor of the older page.
    History {
        turns: Vec<codex_app_server_protocol::Turn>,
        next: Option<String>,
    },
    /// The MCP servers of the effective config (`config/read`).
    McpServers(serde_json::Map<String, serde_json::Value>),
    /// The ephemeral thread of a structured turn started; `as_asked` is whether it has the
    /// permissions that were asked for (`Temporary::started_as_asked`).
    Temporary {
        thread: String,
        as_asked: bool,
    },
    /// The id of the turn a structured turn started.
    StructuredTurn {
        turn: String,
    },
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
        thread: ThreadId,
    },
    /// The read of the turn to stop.
    Stop {
        thread: String,
    },
    Interrupt,
    Start,
    /// The fork of `thread` (a failure is told to the page of that thread, not to every page).
    Fork {
        thread: String,
    },
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
    Read {
        thread: String,
    },
    Turns {
        thread: String,
    },
    /// A page of the opened session's history.
    History {
        thread: String,
        read: HistoryRead,
    },
    Loaded,
    SaveGrouping,
}

/// Why the app requested this page of history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryRead {
    Older,
    Latest,
    /// The newest page requested after recovering the thread's connection or stream.
    SubmissionRecovery,
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
    crate::protocol::agents::wire_request(command, id)
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    crate::protocol::agents::decode_reply(command, result)
}

/// A script's `screen` step for this directory. The forms: `{"listed":
/// {"threads":[<protocol Thread>..],"unread":[..],"next_cursor":..}}`,
/// `{"clock":n}`, `{"current":"id"}`, `{"pinned":["id"..]}`, `{"request":["id","text"]}`.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    crate::protocol::agents::seed_ev(value).map_err(crate::screens::SeedError::Bad)
}

/// The oracle's recency order for discovery and detail reads.
pub(super) fn compare_recency(left: &Thread, right: &Thread) -> std::cmp::Ordering {
    left.recency_at
        .unwrap_or(left.updated_at)
        .cmp(&right.recency_at.unwrap_or(right.updated_at))
        .then_with(|| left.id.cmp(&right.id))
}
