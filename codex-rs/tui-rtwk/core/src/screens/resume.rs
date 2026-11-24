//! `resume` (S18, resume and fork picker): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/resume/` in the UI crate.
//!
//! The picker lists saved threads page by page from every connected backend
//! (its sources), filtered by directory and status, sorted by update or
//! creation time, and searched by a typed query. Every `thread/list` page
//! carries the listing generation, its request number and the search token
//! it serves (`listing`): a reply of an older cycle is dropped whole, a page
//! of an older search ingests its rows but starts no further page for it.
//! Opening a thread is not done here: `Intent::Resume` names the thread and
//! the app opens it through `OpenThread` (ADR-003).

mod listing;
mod proto;
mod toolbar;

use std::collections::{BTreeMap, BTreeSet};

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Turn;

use crate::app::AppFx;
use crate::ids::{BackendId, ThreadId};
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub use listing::{Listing, PageMode, Request, Source};
pub use proto::{PAGE_SIZE, PREVIEW_LINES, preview_lines, row_of};
pub use toolbar::{Action, Control, Dir, Filter, Pick, Sort, Status};

pub const NAME: &str = "resume";
pub const ID: &str = "S18";
pub const KIND: Kind = Kind::Route;

/// The cursor is this near the end of the loaded rows: the page asks for the
/// next page (oracle: `LOAD_NEAR_THRESHOLD`).
pub const LOAD_NEAR: usize = 5;

/// What a thread is called across the picker's caches: its backend and id.
pub type Key = (BackendId, ThreadId);

/// Where the picker was opened from (oracle: `SessionPickerLaunchContext`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Launch {
    #[default]
    Startup,
    /// From a running session: archiving that session here is refused.
    Session { current: ThreadId },
}

/// The directory the picker is scoped to (oracle: `filter_cwd` and
/// `local_filter_cwd`), and the checkouts of its repository the shell found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub cwd: Option<String>,
    /// Every checkout of the working directory's repository, itself included.
    pub repo_cwds: Vec<String>,
}

impl Scope {
    /// The repository filter is offered when the repository has other
    /// checkouts (oracle: `local_repo_filter_available`).
    pub fn repo_available(&self) -> bool {
        self.repo_cwds.len() > 1
    }
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The rows loaded so far, deduplicated and sorted by the sort key,
    /// newest first. What the list shows is `visible`.
    pub rows: Vec<Row>,
    /// The reference time of the relative ages, epoch seconds (R11: the
    /// clock is a model input).
    pub clock: i64,
    pub load: Load,
    /// The thread the user asked to resume (the app opens it).
    pub resuming: Option<ThreadId>,
    /// The thread a restore made available: the app opens it too.
    pub restored: Option<ThreadId>,
    /// The picker was opened on the live backend.
    pub live: bool,
    pub action: Action,
    pub launch: Launch,
    pub scope: Scope,
    /// The backends the listing reads (the connected ones).
    pub sources: Vec<BackendId>,
    pub filter: Filter,
    pub status: Status,
    pub sort: Sort,
    /// The clock the expanded details write times with (oracle:
    /// `ClockFormat`): the 24-hour one, else the 12-hour one.
    pub clock_24h: bool,
    /// The typed search (oracle: `query`).
    pub query: String,
    pub listing: Listing,
    /// A one-line error under the list (oracle: `inline_error`), cleared by
    /// the next thing the user does. The words are the view's.
    pub inline_error: Option<Problem>,
    /// The conversation previews asked of the backend, by the thread they
    /// are for (the request record; which row is open is the page's).
    pub previews: BTreeMap<Key, Preview>,
    pub archive: ArchiveState,
    /// The transcript being loaded or last opened (ctrl+t).
    pub transcript: Option<Transcript>,
    seen: BTreeSet<String>,
}

/// What the line under the list reports (oracle: `inline_error`). Core says
/// what happened; the page's strings say it in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The row has no readable thread id.
    NoThreadId,
    /// The row's session cannot be resumed; the rollout file it names, if any.
    NoMetadata(Option<String>),
    /// The row has no transcript to open.
    NoTranscript,
    /// The transcript could not be read.
    TranscriptFailed,
    /// The current session is not archived from the picker.
    CurrentSession,
    /// One of several backends failed a page.
    OneBackend,
    /// A page after the first failed.
    MoreFailed,
    /// Archiving failed, with the server's message.
    ArchiveFailed(String),
    /// Restoring an archived session failed, with the server's message.
    RestoreFailed(String),
}

/// Where the list stands (oracle: `PaginationState` and `inline_error`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Load {
    #[default]
    Idle,
    /// The first page is loading (`Loading sessions…`).
    Loading,
    /// The first page failed; the message shows on the search line.
    Failed(String),
}

/// One saved thread in the picker's list (oracle: `resume_picker::Row`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Empty when the listing gave no readable thread id.
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
    /// The backend that listed the row.
    pub backend: BackendId,
    /// The rollout file, when the backend reports one.
    pub path: Option<String>,
}

impl Row {
    /// The name the thread was given, when it has one.
    pub fn name(&self) -> Option<&str> {
        self.thread_name.as_deref()
    }

    /// Oracle: `Row::display_preview`: the thread's name when it has one.
    pub fn display_preview(&self) -> &str {
        self.thread_name.as_deref().unwrap_or(&self.preview)
    }

    pub fn key(&self) -> Key {
        (self.backend, self.id.clone())
    }

    /// Oracle: `Row::seen_key`: the rollout path, else the thread id.
    fn seen_key(&self) -> Option<String> {
        self.path
            .clone()
            .or_else(|| (!self.id.as_str().is_empty()).then(|| self.id.to_string()))
    }

    /// Oracle: `Row::matches_query` (`query` is lower case).
    fn matches_query(&self, query: &str) -> bool {
        let has = |text: &str| text.to_lowercase().contains(query);
        has(&self.preview)
            || self.thread_name.as_deref().is_some_and(has)
            || has(self.id.as_str())
            || self.branch.as_deref().is_some_and(has)
            || self.cwd.as_deref().is_some_and(has)
    }
}

/// One line of an expanded row's conversation preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker {
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewLine {
    pub speaker: Speaker,
    pub text: String,
}

/// The preview of an expanded row (oracle: `TranscriptPreviewState`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preview {
    Loading,
    Loaded(Vec<PreviewLine>),
    Failed,
}

/// The transcript of a row, asked for with ctrl+t (oracle:
/// `pending_transcript_open` and `SessionTranscriptState`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transcript {
    pub key: Key,
    pub state: TranscriptState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TranscriptState {
    Loading,
    /// Read: this many turns.
    Loaded { turns: usize },
}

/// Archiving and restoring act on one session at a time (oracle:
/// `archive::ArchiveState`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ArchiveState {
    #[default]
    Idle,
    Pending(Key),
    Restoring(Key),
}

/// The directories a page is asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CwdFilter {
    One(String),
    Many(Vec<String>),
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// The listing, as one finished page (a script's seed; the live picker
    /// reads pages through `Command::List`).
    Listed(Vec<Row>),
    /// The clock the details write times with: `true` for the 24-hour one.
    Clock24(bool),
    /// The conversation preview of a row (a script's seed of an answered
    /// `ThreadAsk::History`).
    Previewed {
        key: Key,
        lines: Vec<PreviewLine>,
    },
    /// What the line under the list reports (a script's seed of a failure the
    /// app's answers make).
    Problem(Problem),
    /// A listing started.
    Loading,
    /// A listing failed with this message.
    Failed(String),
    /// The directory the picker is scoped to, and the checkouts the shell
    /// found for its repository.
    Scope(Scope),
    /// What a launch asks of the picker.
    Launched {
        action: Action,
        launch: Launch,
        db_first: bool,
    },
    /// A page of a source, as the backend answered (a script's seed).
    Paged {
        backend: BackendId,
        rows: Vec<Row>,
        next: Option<String>,
    },
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The picker opened on the live backend: the sessions are listed.
    Open,
    /// Enter on a row: resume (or fork, or restore an archived) this thread.
    Resume(ThreadId),
    /// The search text is now this (typing, backspace).
    Query(String),
    /// Text pasted into the search.
    Paste(String),
    /// Esc with a query: the search is cleared.
    ClearQuery,
    /// Left and right change the value of the toolbar control with the
    /// keyboard focus (which one has it is the page's: UI state).
    Change(Control, Dir),
    /// A tap on a value of the toolbar: that value is picked.
    Pick(Pick),
    /// The page opened this row's details: its conversation preview is asked
    /// for, unless it was already.
    LoadPreview(ThreadId),
    /// Ctrl+T on this row.
    OpenTranscript(ThreadId),
    /// Esc while the transcript loads.
    CancelTranscript,
    /// Ctrl+A on this row.
    Archive(ThreadId),
    /// The list reaches for more rows (its cursor is near the last loaded
    /// one, it has room left, it is scrolled to its end): the next page of
    /// every source that has one is asked for.
    LoadMore,
    /// The user acted: the line under the list is cleared.
    DismissProblem,
}

/// An RPC to make and what it is for (the app mints the ticket).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Out {
    pub ask: Ask,
    pub ctx: Ctx,
}

/// What the picker asks of a backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// An RPC of this screen (`Command`).
    Rpc(Command),
    /// The newest turns of a thread (the preview and the transcript): a thread
    /// RPC, so the app asks it as the thread's own `ThreadAsk` (ADR-003).
    Turns { backend: BackendId, thread: ThreadId },
}

/// The RPCs of this screen: app-scoped, sent by `app::send_resume`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    List {
        backend: BackendId,
        cursor: Option<String>,
        mode: PageMode,
        sort: Sort,
        archived: bool,
        cwd: Option<CwdFilter>,
    },
    Archive {
        backend: BackendId,
        thread: ThreadId,
    },
    Unarchive {
        backend: BackendId,
        thread: ThreadId,
    },
}

impl Command {
    /// The backend that answers.
    pub fn backend(&self) -> BackendId {
        match self {
            Self::List { backend, .. }
            | Self::Archive { backend, .. }
            | Self::Unarchive { backend, .. } => *backend,
        }
    }
}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    Page {
        rows: Vec<Row>,
        next: Option<String>,
        /// The ids of the rows with unread output.
        unread: Vec<String>,
    },
    /// The newest turns of a thread, newest first (the app turns the
    /// thread's history reply into this).
    Turns(Vec<Turn>),
    Archived,
    Unarchived {
        thread: ThreadId,
        cwd: Option<String>,
    },
}

// The protocol's `Turn` compares only by value, as the agents replies do.
impl Eq for Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Page {
        backend: BackendId,
        generation: u64,
        request: u64,
        search: Option<u64>,
    },
    Preview(Key),
    Transcript(Key),
    Archive(Key),
    Unarchive(Key),
}

impl Ctx {
    /// The backend the RPC went to.
    pub fn backend(&self) -> BackendId {
        match self {
            Self::Page { backend, .. } => *backend,
            Self::Preview(key) | Self::Transcript(key) | Self::Archive(key) | Self::Unarchive(key) => {
                key.0
            }
        }
    }
}

impl Model {
    /// The rows the list shows: the directory filter, then the query
    /// (oracle: `apply_filter`).
    pub fn visible(&self) -> Vec<&Row> {
        self.visible_for(&self.query)
    }

    /// The rows the list would show for another query (the page keeps the
    /// cursor on its row when a search is cleared).
    pub fn visible_for(&self, query: &str) -> Vec<&Row> {
        self.visible_indices_for(query)
            .into_iter()
            .map(|at| &self.rows[at])
            .collect()
    }

    /// Where in `rows` the rows the list shows are.
    pub fn visible_indices(&self) -> Vec<usize> {
        self.visible_indices_for(&self.query)
    }

    /// Where in `rows` the rows a query would show are.
    pub fn visible_indices_for(&self, query: &str) -> Vec<usize> {
        let query = query.to_lowercase();
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| self.in_scope(row))
            .filter(|(_, row)| query.is_empty() || row.matches_query(&query))
            .map(|(at, _)| at)
            .collect()
    }

    /// Whether the directory filter lets `row` in (oracle: `row_matches_filter`).
    fn in_scope(&self, row: &Row) -> bool {
        let Some(cwd) = self.scope.cwd.as_deref() else {
            return true;
        };
        match self.filter {
            Filter::All => true,
            Filter::Cwd => row.cwd.as_deref().is_some_and(|row| same_dir(row, cwd)),
            Filter::Repo => row.cwd.as_deref().is_some_and(|row| {
                same_dir(row, cwd) || self.scope.repo_cwds.iter().any(|dir| same_dir(row, dir))
            }),
        }
    }

    /// The directory filter the backend is asked with.
    fn cwd_filter(&self) -> Option<CwdFilter> {
        let cwd = self.scope.cwd.clone()?;
        match self.filter {
            Filter::All => None,
            Filter::Cwd => Some(CwdFilter::One(cwd)),
            Filter::Repo if self.scope.repo_available() => {
                Some(CwdFilter::Many(self.scope.repo_cwds.clone()))
            }
            Filter::Repo => Some(CwdFilter::One(cwd)),
        }
    }

    pub fn row(&self, id: &ThreadId) -> Option<&Row> {
        self.rows.iter().find(|row| &row.id == id)
    }

    /// Whether another page is loading or could be read (the list's `↓ more`).
    pub fn has_more(&self) -> bool {
        self.listing.has_more()
    }

    /// Whether a row's meta line names its directory: unless the list is
    /// already that directory's (oracle: `show_cwd` in
    /// `render_comfortable_session_lines`).
    pub fn shows_cwd(&self, row: &Row) -> bool {
        self.filter == Filter::All
            || row
                .cwd
                .as_deref()
                .zip(self.scope.cwd.as_deref())
                .is_some_and(|(row, cwd)| !same_dir(row, cwd))
    }

    /// The archive shortcut is offered when resuming active sessions
    /// (oracle: `archive_shortcut_available`).
    pub fn archive_available(&self) -> bool {
        self.action == Action::Resume && self.status == Status::Active
    }

    fn command(&self, request: Request) -> Out {
        Out {
            ctx: Ctx::Page {
                backend: request.backend,
                generation: self.listing.generation,
                request: request.request,
                search: request.search,
            },
            ask: Ask::Rpc(Command::List {
                backend: request.backend,
                cursor: request.cursor,
                mode: request.mode,
                sort: self.sort,
                archived: self.status == Status::Archived,
                cwd: self.cwd_filter(),
            }),
        }
    }

    fn outs(&self, requests: Vec<Request>) -> Vec<Out> {
        requests.into_iter().map(|request| self.command(request)).collect()
    }

    /// A new listing cycle (oracle: `start_initial_load`).
    fn restart(&mut self) -> Vec<Out> {
        self.rows.clear();
        self.seen.clear();
        self.load = Load::Loading;
        let search = if self.query.is_empty() {
            None
        } else {
            Some(self.listing.next_search_token())
        };
        self.listing.search = search;
        let sources = if self.sources.is_empty() {
            vec![BackendId::default()]
        } else {
            self.sources.clone()
        };
        let requests = self.listing.restart(&sources, search);
        self.outs(requests)
    }

    /// The next page of every source that has one (oracle: `load_more_if_needed`).
    fn load_more(&mut self, search: Option<u64>) -> Vec<Out> {
        let requests = self.listing.begin_all(search);
        self.outs(requests)
    }

    /// Adds a page's rows: new ones only, the rest sorted again (oracle: `ingest_page`).
    fn ingest(&mut self, backend: BackendId, rows: Vec<Row>, unread: &[String]) {
        for mut row in rows {
            row.backend = backend;
            row.unread = row.unread || unread.iter().any(|id| id == row.id.as_str());
            let duplicate = row.seen_key().is_some_and(|key| !self.seen.insert(key));
            if !duplicate {
                self.rows.push(row);
            }
        }
        let sort = self.sort;
        self.rows.sort_by_key(|row| {
            std::cmp::Reverse(match sort {
                Sort::Created => row.created,
                Sort::Updated => row.updated.or(row.created),
            })
        });
    }

    /// While a search finds nothing in the loaded rows, more pages are read
    /// until it does or the sources end (oracle: `continue_search_if_needed`).
    fn continue_search(&mut self) -> Vec<Out> {
        let Some(token) = self.listing.search else {
            return Vec::new();
        };
        if !self.visible().is_empty() || !self.listing.has_more() {
            self.listing.search = None;
            return Vec::new();
        }
        if !self.listing.can_page() {
            return Vec::new();
        }
        self.load_more(Some(token))
    }

    fn set_query(&mut self, query: String) -> Vec<Out> {
        if self.query == query {
            return Vec::new();
        }
        self.query = query;
        self.listing.search = None;
        if self.query.is_empty() || !self.visible().is_empty() || !self.listing.has_more() {
            return Vec::new();
        }
        let token = self.listing.next_search_token();
        self.listing.search = Some(token);
        if self.listing.can_page() {
            self.load_more(Some(token))
        } else {
            Vec::new()
        }
    }

    /// The row asked for, or the inline error the oracle shows.
    fn row_or_error(&mut self, id: &ThreadId, problem: Problem) -> Option<Row> {
        if id.as_str().is_empty() {
            self.inline_error = Some(problem);
            return None;
        }
        let row = self.row(id).cloned();
        if row.is_none() {
            self.inline_error = Some(problem);
        }
        row
    }
}

/// Whether two directory paths name the same directory (oracle:
/// `paths_match`, without touching the file system: trailing separators and
/// case-insensitive drive letters are not told apart here).
fn same_dir(left: &str, right: &str) -> bool {
    left.trim_end_matches('/') == right.trim_end_matches('/')
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Listed(rows) => {
            model.seen = rows.iter().filter_map(Row::seen_key).collect();
            model.rows = rows;
            model.load = Load::Idle;
        }
        Ev::Clock24(on) => model.clock_24h = on,
        Ev::Previewed { key, lines } => {
            model.previews.insert(key, Preview::Loaded(lines));
        }
        Ev::Problem(problem) => model.inline_error = Some(problem),
        Ev::Loading => model.load = Load::Loading,
        Ev::Failed(message) => model.load = Load::Failed(message),
        Ev::Scope(scope) => {
            model.filter = Filter::initial(false, scope.cwd.as_deref());
            model.scope = scope;
        }
        Ev::Launched {
            action,
            launch,
            db_first,
        } => {
            model.action = action;
            model.launch = launch;
            model.listing.initial_mode = if db_first {
                PageMode::StateDbOnly
            } else {
                PageMode::StoreDefault
            };
        }
        Ev::Paged { backend, rows, next } => {
            model.ingest(backend, rows, &[]);
            model.sources = vec![backend];
            let source = model.listing.sources.entry(backend).or_default();
            source.exhausted = next.is_none();
            source.cursor = next;
            model.load = Load::Idle;
        }
    }
}

/// The intents of this screen; what the picker needs from the backend comes
/// back as `Out`s the app sends.
pub fn update(model: &mut Model, intent: Intent) -> Vec<Out> {
    model.inline_error = None;
    match intent {
        Intent::Open => {
            model.live = true;
            model.restart()
        }
        Intent::Resume(id) => resume(model, &id),
        Intent::Query(query) => model.set_query(query),
        Intent::Paste(text) => match pasted(&model.query, &text) {
            Some(query) => model.set_query(query),
            None => Vec::new(),
        },
        Intent::ClearQuery => {
            model.query.clear();
            model.listing.search = None;
            Vec::new()
        }
        Intent::Change(control, dir) => change(model, control, dir),
        Intent::Pick(pick) => pick_value(model, pick),
        Intent::LoadPreview(id) => load_preview(model, &id),
        Intent::OpenTranscript(id) => open_transcript(model, &id),
        Intent::CancelTranscript => {
            if let Some(transcript) = model.transcript.take()
                && !matches!(transcript.state, TranscriptState::Loading)
            {
                model.transcript = Some(transcript);
            }
            Vec::new()
        }
        Intent::Archive(id) => archive(model, &id),
        Intent::LoadMore => {
            if model.listing.can_page() {
                let search = model.listing.search;
                return model.load_more(search);
            }
            Vec::new()
        }
        Intent::DismissProblem => Vec::new(),
    }
}

/// Oracle: `normalize_pasted_search_query` and `handle_paste`: the paste's
/// whitespace folded into single spaces, put after the query (one space
/// between them); `None` for a blank paste, which changes nothing.
pub fn pasted(query: &str, text: &str) -> Option<String> {
    let folded = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if folded.is_empty() {
        return None;
    }
    let mut query = query.to_owned();
    if !query.is_empty() && !query.ends_with(char::is_whitespace) {
        query.push(' ');
    }
    query.push_str(&folded);
    Some(query)
}

fn change(model: &mut Model, control: Control, dir: Dir) -> Vec<Out> {
    match control {
        Control::Sort => model.sort = model.sort.toggled(),
        Control::Status => model.status = model.status.toggled(),
        Control::Filter => {
            let next = model.filter.change(
                dir,
                model.scope.cwd.is_some(),
                model.scope.repo_available(),
            );
            if next == model.filter {
                return Vec::new();
            }
            model.filter = next;
        }
    }
    model.restart()
}

/// A tapped value of the toolbar: the list restarts under it unless it is
/// the one already in force (or a filter the picker does not offer).
fn pick_value(model: &mut Model, pick: Pick) -> Vec<Out> {
    match pick {
        Pick::Sort(sort) if sort != model.sort => model.sort = sort,
        Pick::Status(status) if status != model.status => model.status = status,
        Pick::Filter(filter) if filter != model.filter => {
            let has_cwd = model.scope.cwd.is_some();
            let offered = match filter {
                Filter::All => true,
                Filter::Cwd => has_cwd,
                Filter::Repo => has_cwd && model.scope.repo_available(),
            };
            if !offered {
                return Vec::new();
            }
            model.filter = filter;
        }
        _ => return Vec::new(),
    }
    model.restart()
}

fn resume(model: &mut Model, id: &ThreadId) -> Vec<Out> {
    if !matches!(model.archive, ArchiveState::Idle) {
        return Vec::new();
    }
    let path = model.row(id).and_then(|row| row.path.clone());
    let Some(row) = model.row_or_error(id, Problem::NoMetadata(path)) else {
        return Vec::new();
    };
    if model.status == Status::Archived {
        model.archive = ArchiveState::Restoring(row.key());
        return vec![Out {
            ask: Ask::Rpc(Command::Unarchive {
                backend: row.backend,
                thread: row.id.clone(),
            }),
            ctx: Ctx::Unarchive(row.key()),
        }];
    }
    model.resuming = Some(row.id);
    Vec::new()
}

fn load_preview(model: &mut Model, id: &ThreadId) -> Vec<Out> {
    let Some(row) = model.row(id).cloned() else {
        return Vec::new();
    };
    let key = row.key();
    if model.previews.contains_key(&key) {
        return Vec::new();
    }
    model.previews.insert(key.clone(), Preview::Loading);
    vec![Out {
        ask: Ask::Turns {
            backend: row.backend,
            thread: row.id,
        },
        ctx: Ctx::Preview(key),
    }]
}

fn open_transcript(model: &mut Model, id: &ThreadId) -> Vec<Out> {
    let Some(row) = model.row_or_error(id, Problem::NoTranscript) else {
        return Vec::new();
    };
    let key = row.key();
    if matches!(&model.transcript, Some(Transcript { key: loaded, state: TranscriptState::Loaded { .. } }) if loaded == &key)
    {
        return Vec::new();
    }
    model.transcript = Some(Transcript {
        key: key.clone(),
        state: TranscriptState::Loading,
    });
    vec![Out {
        ask: Ask::Turns {
            backend: row.backend,
            thread: row.id,
        },
        ctx: Ctx::Transcript(key),
    }]
}

fn archive(model: &mut Model, id: &ThreadId) -> Vec<Out> {
    if !model.archive_available() || !matches!(model.archive, ArchiveState::Idle) {
        return Vec::new();
    }
    let Some(row) = model.row_or_error(id, Problem::NoThreadId) else {
        return Vec::new();
    };
    if matches!(&model.launch, Launch::Session { current } if current == &row.id) {
        model.inline_error = Some(Problem::CurrentSession);
        return Vec::new();
    }
    model.archive = ArchiveState::Pending(row.key());
    vec![Out {
        ask: Ask::Rpc(Command::Archive {
            backend: row.backend,
            thread: row.id.clone(),
        }),
        ctx: Ctx::Archive(row.key()),
    }]
}

/// The reply to an RPC of this screen. A reply that is not the one in
/// flight (an older listing, a cancelled or finished load) changes nothing.
pub fn apply_answer(model: &mut Model, ctx: Ctx, result: Result<Reply, RpcError>) -> Vec<Out> {
    match (ctx, result) {
        (
            Ctx::Page {
                backend,
                generation,
                request,
                search,
            },
            result,
        ) => page_answer(model, backend, generation, request, search, result),
        (Ctx::Preview(key), result) => {
            if !matches!(model.previews.get(&key), Some(Preview::Loading)) {
                return Vec::new();
            }
            model.previews.insert(
                key,
                match result {
                    Ok(Reply::Turns(turns)) => Preview::Loaded(proto::preview_lines(&turns)),
                    _ => Preview::Failed,
                },
            );
            Vec::new()
        }
        (Ctx::Transcript(key), result) => {
            let waiting = matches!(
                &model.transcript,
                Some(Transcript { key: asked, state: TranscriptState::Loading }) if asked == &key
            );
            if !waiting {
                return Vec::new();
            }
            match result {
                Ok(Reply::Turns(turns)) => {
                    model.transcript = Some(Transcript {
                        key,
                        state: TranscriptState::Loaded { turns: turns.len() },
                    });
                }
                _ => {
                    model.transcript = None;
                    model.inline_error = Some(Problem::TranscriptFailed);
                }
            }
            Vec::new()
        }
        (Ctx::Archive(key), result) => archive_answer(model, key, result),
        (Ctx::Unarchive(key), result) => {
            if model.archive != ArchiveState::Restoring(key) {
                return Vec::new();
            }
            model.archive = ArchiveState::Idle;
            match result {
                Ok(Reply::Unarchived { thread, .. }) => model.restored = Some(thread),
                Ok(_) => {}
                Err(error) => {
                    model.inline_error = Some(Problem::RestoreFailed(describe(&error)));
                }
            }
            Vec::new()
        }
    }
}

fn describe(error: &RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) => message.clone(),
        RpcError::Mismatch => "unexpected reply".to_owned(),
        RpcError::Prepare(text) => text.clone(),
        RpcError::Provider(error) => error.to_string(),
    }
}

fn page_answer(
    model: &mut Model,
    backend: BackendId,
    generation: u64,
    request: u64,
    search: Option<u64>,
    result: Result<Reply, RpcError>,
) -> Vec<Out> {
    let first_page = model.rows.is_empty();
    let several = model.listing.sources.len() > 1;
    let (mode, was_first) = {
        let Some(source) = model.listing.finish(backend, generation, request) else {
            return Vec::new();
        };
        (source.mode, source.cursor.is_none())
    };
    match result {
        Ok(Reply::Page { rows, next, unread }) => {
            // Oracle: fall back to the store's own listing only when the
            // first indexed page is empty (an empty later page ends the list).
            if mode == PageMode::StateDbOnly && first_page && was_first && rows.is_empty() {
                if let Some(source) = model.listing.sources.get_mut(&backend) {
                    source.mode = PageMode::StoreDefault;
                }
                let search = model.listing.search.or(search);
                return model
                    .listing
                    .begin(backend, search)
                    .map(|request| model.command(request))
                    .into_iter()
                    .collect();
            }
            if let Some(source) = model.listing.sources.get_mut(&backend) {
                source.exhausted = next.is_none();
                source.cursor = next;
            }
            model.ingest(backend, rows, &unread);
            if !model.listing.is_loading() {
                model.load = Load::Idle;
            }
            // A page asked for by an older search still brings its rows; what
            // is read next serves the search that is current now (oracle:
            // `continue_search_if_token_matches`).
            model.continue_search()
        }
        Ok(_) => Vec::new(),
        Err(error) => {
            if let Some(source) = model.listing.sources.get_mut(&backend) {
                source.failed = true;
            }
            if several {
                model.inline_error = Some(Problem::OneBackend);
            } else if model.rows.is_empty() {
                model.load = Load::Failed(describe(&error));
            } else {
                model.inline_error = Some(Problem::MoreFailed);
            }
            if !model.listing.is_loading() && matches!(model.load, Load::Loading) {
                model.load = Load::Idle;
            }
            model.listing.search = None;
            Vec::new()
        }
    }
}

fn archive_answer(model: &mut Model, key: Key, result: Result<Reply, RpcError>) -> Vec<Out> {
    if model.archive != ArchiveState::Pending(key.clone()) {
        return Vec::new();
    }
    model.archive = ArchiveState::Idle;
    if let Err(error) = result {
        model.inline_error = Some(Problem::ArchiveFailed(describe(&error)));
        return Vec::new();
    }
    // The seen key stays as a tombstone: an older page already in flight
    // cannot put the archived session back.
    model.rows.retain(|row| row.key() != key);
    model.previews.remove(&key);
    if matches!(&model.transcript, Some(transcript) if transcript.key == key) {
        model.transcript = None;
    }
    model.inline_error = None;
    if model.visible().is_empty()
        && !model.query.is_empty()
        && model.listing.has_more()
        && model.listing.search.is_none()
    {
        let token = model.listing.next_search_token();
        model.listing.search = Some(token);
        if model.listing.can_page() {
            return model.load_more(Some(token));
        }
    }
    Vec::new()
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    // The app sends the outs (`app::resume_picker`); the generic path has none.
    let _ = (update(model, intent), fx);
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

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
    // Every RPC of this screen is app-scoped: `apply_answer` takes its reply.
    let _ = (part, ctx, result, fx);
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = thread;
    proto::wire(command, id)
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    proto::decode(command, result)
}

/// The seed form of this screen's events (`{"listed":{"rows":[..]}}`,
/// `{"clock":..}`, `{"loading":null}`, `{"failed":".."}`, `{"scope":{..}}`,
/// `{"launched":{..}}`, `{"paged":{"rows":[..],"next":".."}}`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    /// The backend listed the saved threads: the fields of the app-server
    /// tests' `create_fake_rollout`, and the picker's own.
    Listed { rows: Vec<RolloutSeed> },
    /// The clock the details write times with.
    Clock24(bool),
    /// The conversation preview of a listed row: `{"id": .., "backend": ..,
    /// "lines": [{"speaker": "user", "text": ..}]}`.
    Preview {
        id: String,
        #[serde(default)]
        backend: u32,
        lines: Vec<PreviewSeed>,
    },
    /// What the line under the list reports: `one_backend`, `more_failed`,
    /// `no_transcript`, `no_thread_id`, `current_session`.
    Problem(String),
    Loading(()),
    Failed(String),
    Scope {
        cwd: Option<String>,
        #[serde(default)]
        repo_cwds: Vec<String>,
    },
    Launched {
        #[serde(default)]
        fork: bool,
        #[serde(default)]
        current: Option<String>,
        #[serde(default)]
        db_first: bool,
    },
    Paged {
        #[serde(default)]
        backend: u32,
        rows: Vec<RolloutSeed>,
        #[serde(default)]
        next: Option<String>,
    },
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PreviewSeed {
    speaker: String,
    text: String,
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
    #[serde(default)]
    path: Option<String>,
}

impl RolloutSeed {
    fn row(self) -> Row {
        Row {
            id: ThreadId::new(self.id),
            preview: self.preview,
            model_provider: self.model_provider,
            started: self.timestamp,
            thread_name: self.thread_name,
            created: self.created,
            updated: self.updated,
            cwd: self.cwd,
            branch: self.branch,
            cloud: self.cloud,
            unread: self.unread,
            backend: BackendId::default(),
            path: self.path,
        }
    }
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Listed { rows } => Ev::Listed(rows.into_iter().map(RolloutSeed::row).collect()),
        SeedEv::Clock24(on) => Ev::Clock24(on),
        SeedEv::Preview { id, backend, lines } => Ev::Previewed {
            key: (BackendId(backend), ThreadId::new(id)),
            lines: lines
                .into_iter()
                .map(|line| PreviewLine {
                    speaker: if line.speaker == "assistant" {
                        Speaker::Assistant
                    } else {
                        Speaker::User
                    },
                    text: line.text,
                })
                .collect(),
        },
        SeedEv::Problem(name) => Ev::Problem(match name.as_str() {
            "one_backend" => Problem::OneBackend,
            "more_failed" => Problem::MoreFailed,
            "no_transcript" => Problem::NoTranscript,
            "no_thread_id" => Problem::NoThreadId,
            "current_session" => Problem::CurrentSession,
            other => {
                return Err(crate::screens::SeedError::Bad(format!(
                    "unknown problem {other}"
                )));
            }
        }),
        SeedEv::Loading(()) => Ev::Loading,
        SeedEv::Failed(message) => Ev::Failed(message),
        SeedEv::Scope { cwd, repo_cwds } => Ev::Scope(Scope { cwd, repo_cwds }),
        SeedEv::Launched {
            fork,
            current,
            db_first,
        } => Ev::Launched {
            action: if fork { Action::Fork } else { Action::Resume },
            launch: current.map_or(Launch::Startup, |current| Launch::Session {
                current: ThreadId::new(current),
            }),
            db_first,
        },
        SeedEv::Paged {
            backend,
            rows,
            next,
        } => Ev::Paged {
            backend: BackendId(backend),
            rows: rows.into_iter().map(RolloutSeed::row).collect(),
            next,
        },
    })
}
