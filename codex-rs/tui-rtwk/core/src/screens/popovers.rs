//! `popovers` (S12, composer popovers): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/popovers/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::PluginInstalledParams;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::SkillsListParams;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::screens::Shared;
use crate::screens::composer::clipboard::Pasted;
use crate::screens::composer::history::{History, HistoryEntry, Lookup, SearchResult, Store};
use crate::screens::composer::mentions::candidate::Capability;
use crate::screens::composer::mentions::files::{FileMatch, FileMatches, FileSearchAsk, MatchType};
use crate::screens::composer::mentions::session::Sources;
use crate::screens::composer::mentions::sources;
use crate::screens::composer::mentions::task::TaskMention;
use crate::screens::composer::mentions::usage::MentionUsage;
use crate::screens::select::apps::App;
pub use crate::screens::composer::mentions::catalog::{Mention, MentionKind};
use crate::threads::ThreadFx;

pub const NAME: &str = "popovers";
pub const ID: &str = "S12";
pub const KIND: Kind = Kind::Part;

/// The app-scoped model state of this screen, changed only by `apply`: the
/// data the `@` and `$` popups list and what the message history file said
/// (Ctrl+R, Up and Down read it). The backend supplies the data; scripts
/// seed it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// Skills, apps and plugins (`$`, `@`).
    pub mentions: Vec<Mention>,
    /// What the server's lists were when they last made `mentions` (`Ev::Catalog`): a seed
    /// stays until the lists change.
    pub listed: Listed,
    /// The tasks the last task search found (`@`).
    pub tasks: Vec<TaskMention>,
    /// What the file search last answered: the query and its matches.
    pub file_matches: FileMatches,
    /// The window of recent `$` picks.
    pub usage: MentionUsage,
    /// The images the host took from the clipboard (Ctrl+V).
    pub pasted: Pasted,
    /// The message history file as far as it was read. Shared: `ScreenStates`
    /// is cloned on every change of any screen, a long history is not.
    pub history: Shared<Store>,
    /// What the host is asked to do with the history file, drained by
    /// [`Model::take_asks`] after each `apply`.
    asks: Vec<Ask>,
}

/// What the catalog of the popups was made of: the thread whose lists they are, its skills and
/// plugins, and the apps of the directory (`app/mentions.rs`). The app compares it with the lists
/// it holds, event by event, and builds the catalog only when they differ.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listed {
    pub thread: Option<ThreadId>,
    pub skills: Option<Vec<Mention>>,
    pub plugins: Option<Vec<Mention>>,
    pub apps: Option<Vec<App>>,
}

impl Listed {
    /// The server listed nothing yet.
    pub fn is_empty(&self) -> bool {
        self.skills.is_none() && self.plugins.is_none() && self.apps.is_none()
    }
}

/// What the composer needs of the host: the history file (`codex-message-history`),
/// the file search and the mention usage file (IO). The answers are the
/// `Ev::History*`, `Ev::FileMatches` and `Ev::MentionUsage` events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Run or end a file search (`mentions::files::FileSearchBackend`);
    /// answered with `Ev::FileMatches`.
    FileSearch(FileSearchAsk),
    /// A `$` pick was made: the usage file keeps its identity.
    RecordMention(String),
    /// Ctrl+V: write the clipboard's image to a file; answered with
    /// `Ev::ImagePasted`. A clipboard with no image, or none at all, is the
    /// host's to tell the user (`Failed to paste image: ...`), no answer.
    ClipboardImage,
    /// Append a submitted prompt to the file (`AppendMessageHistoryEntry`);
    /// no answer.
    Append(String),
    /// Read an entry or a batch; answered with `Ev::HistoryEntry`,
    /// `Ev::HistoryBatch` or `Ev::HistoryBatchFailed`.
    Lookup(Lookup),
}

impl Model {
    /// The asks since the last call, oldest first: what the host runs.
    pub fn take_asks(&mut self) -> Vec<Ask> {
        std::mem::take(&mut self.asks)
    }

    /// What the mention popups are fed from.
    pub fn sources(&self) -> Sources<'_> {
        Sources {
            mentions: &self.mentions,
            tasks: &self.tasks,
            usage: &self.usage,
            file_matches: &self.file_matches,
        }
    }

    /// The asks not taken yet, oldest first.
    pub fn pending_asks(&self) -> &[Ask] {
        &self.asks
    }
}

/// Where reverse history search stands (`HistorySearchStatus`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchStatus {
    /// The query is empty (or a read failed): the draft shows as it was.
    #[default]
    Idle,
    /// A persistent read is outstanding.
    Searching,
    /// A match is previewed.
    Match,
    NoMatch,
}

/// What the draft shows after a search step.
#[derive(Clone, Debug, PartialEq)]
pub enum Preview {
    /// The entry that was found.
    Show(HistoryEntry),
    /// The draft as it was when the search opened.
    Original,
    /// What is shown stays.
    Keep,
}

/// Reverse history search: the query and where it stands
/// (`chat_composer/history_search.rs`); the walk over the entries is the
/// composer's [`History`]. The composer owns it with the draft it will
/// restore (UI state, one per thread's draft).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistorySearch {
    pub query: String,
    pub status: SearchStatus,
}

/// Which way the search walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Older,
    Newer,
}

impl HistorySearch {
    /// Opens without previewing anything: Ctrl+R on an empty composer must not
    /// replace it with the latest prompt.
    pub fn new() -> Self {
        Self::default()
    }

    /// The query as the footer shows it: a newline and a tab as marks
    /// (`display_query`); matching keeps the query as it is.
    pub fn display_query(&self) -> String {
        self.query.replace('\n', "\u{21b5}").replace('\t', "\u{21e5}")
    }

    /// The query changed (`update_history_search_query`): the draft goes
    /// back to its original first, the search restarts from the newest
    /// entry, and an empty query ends the walk and stays open.
    pub fn set_query(
        &mut self,
        query: String,
        history: &mut History,
        store: &Store,
        fx: &mut Vec<Lookup>,
    ) -> Preview {
        self.query = query;
        self.status = SearchStatus::Searching;
        if self.query.is_empty() {
            history.reset_search();
            self.status = SearchStatus::Idle;
            return Preview::Original;
        }
        let result = history.search(store, &self.query, Direction::Older, true, fx);
        match self.apply(result) {
            Preview::Show(entry) => Preview::Show(entry),
            _ => Preview::Original,
        }
    }

    /// The next older or newer match (`history_search_in_direction`); at the
    /// end of the matches the current one stays (no "no match" flicker).
    pub fn step(
        &mut self,
        direction: Direction,
        history: &mut History,
        store: &Store,
        fx: &mut Vec<Lookup>,
    ) -> Preview {
        if self.query.is_empty() {
            history.reset_search();
            self.status = SearchStatus::Idle;
            return Preview::Original;
        }
        let result = history.search(store, &self.query, direction, false, fx);
        self.apply(result)
    }

    /// What a step or a read that came in says (`apply_history_search_result`):
    /// `Found` previews, `Pending` waits, `AtBoundary` keeps the match,
    /// `NotFound` puts the original back with "no match" and `Unavailable`
    /// with nothing said.
    pub fn apply(&mut self, result: SearchResult) -> Preview {
        match result {
            SearchResult::Found(entry) => {
                self.status = SearchStatus::Match;
                Preview::Show(entry)
            }
            SearchResult::Pending => {
                self.status = SearchStatus::Searching;
                Preview::Keep
            }
            SearchResult::AtBoundary => {
                self.status = SearchStatus::Match;
                Preview::Keep
            }
            SearchResult::NotFound => {
                self.status = SearchStatus::NoMatch;
                Preview::Original
            }
            SearchResult::Unavailable => {
                self.status = SearchStatus::Idle;
                Preview::Original
            }
        }
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`. The history ones are the host's answers to [`Ask`]s
/// (and the metadata read when a thread opens).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    Mentions(Vec<Mention>),
    /// What the server's lists make of the mentions now (skills, plugins, apps) from `listed`:
    /// replaces `mentions` (`app/mentions.rs`).
    Catalog {
        listed: Listed,
        mentions: Vec<Mention>,
    },
    /// The tasks a task search found.
    Tasks(Vec<TaskMention>),
    /// The file search answered `query`.
    FileMatches { query: String, matches: Vec<FileMatch> },
    /// The recent `$` picks the usage file held, oldest first.
    MentionUsage(Vec<String>),
    /// The clipboard's image was written to this file (`Ask::ClipboardImage`).
    ImagePasted(std::path::PathBuf),
    /// A history file of these prompts, oldest first, all read (a script's
    /// seed).
    History(Vec<String>),
    /// The file was opened: its log id and entry count
    /// (`history_metadata`).
    HistoryMetadata { log_id: u64, entry_count: usize },
    /// One entry read, `None` when the row could not be read.
    HistoryEntry {
        log_id: u64,
        offset: usize,
        text: Option<String>,
    },
    /// A batch read: the rows (offset, text) and where the next older batch
    /// ends.
    HistoryBatch {
        log_id: u64,
        end_offset: usize,
        rows: Vec<(usize, Option<String>)>,
        next_older: Option<usize>,
    },
    /// A batch read failed.
    HistoryBatchFailed { log_id: u64, end_offset: usize },
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A prompt was submitted: the file keeps it for the next sessions.
    Record(String),
    /// A composer's history needs a read it did not find in the store.
    Lookup(Lookup),
    /// The mention popup wants a file search run or ended.
    SearchFiles(FileSearchAsk),
    /// A `$` mention was picked (by the skill's path).
    RecordMention(String),
    /// A task was picked: the list of tasks the search found is spent.
    SpendTasks,
    /// Ctrl+V: the clipboard's image is wanted (`Ask::ClipboardImage`).
    PasteImage,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Mentions(mentions) => model.mentions = mentions,
        Ev::Catalog { listed, mentions } => {
            model.listed = listed;
            model.mentions = mentions;
        }
        Ev::Tasks(tasks) => model.tasks = tasks,
        Ev::FileMatches { query, matches } => model.file_matches = FileMatches { query, matches },
        Ev::MentionUsage(recent) => model.usage = MentionUsage::from_recent(recent),
        Ev::ImagePasted(path) => model.pasted.on_image(path),
        Ev::History(history) => *model.history = Store::seeded(history),
        Ev::HistoryMetadata {
            log_id,
            entry_count,
        } => model.history.set_metadata(log_id, entry_count),
        Ev::HistoryEntry {
            log_id,
            offset,
            text,
        } => {
            model.history.on_entry(log_id, offset, text);
        }
        Ev::HistoryBatch {
            log_id,
            end_offset,
            rows,
            next_older,
        } => {
            model.history.on_batch(log_id, end_offset, rows, next_older);
        }
        Ev::HistoryBatchFailed { log_id, end_offset } => {
            model.history.on_batch_error(log_id, end_offset);
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Record(text) => {
            if !text.trim().is_empty() {
                model.asks.push(Ask::Append(text));
            }
        }
        Intent::Lookup(lookup) => model.asks.push(Ask::Lookup(lookup)),
        Intent::SearchFiles(ask) => model.asks.push(Ask::FileSearch(ask)),
        Intent::SpendTasks => model.tasks.clear(),
        Intent::PasteImage => model.asks.push(Ask::ClipboardImage),
        Intent::RecordMention(identity) => {
            model.usage.record(&identity);
            model.asks.push(Ask::RecordMention(identity));
        }
    }
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`. What the server lists to mention for this thread's
/// directory: the skills and the installed plugins, asked once when the thread is in use
/// (the oracle's startup `skills/list` and `plugin/installed`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    /// The lists were asked for (once per thread).
    pub asked: bool,
    /// `skills/list`'s enabled skills of the thread's directory, once answered.
    pub skills: Option<Vec<Mention>>,
    /// `plugin/installed`'s eligible plugins, once answered.
    pub plugins: Option<Vec<Mention>>,
}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    /// The thread is in use, in `cwd`: its lists are asked for, once.
    Load { cwd: String },
}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `skills/list` of the directory, re-scanned from disk.
    Skills { cwd: String },
    /// `plugin/installed` for the directory.
    Plugins { cwd: String },
}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The enabled skills of the asked directory, as mentions.
    Skills(Vec<Mention>),
    /// The installed plugins, as mentions.
    Plugins(Vec<Mention>),
}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Skills,
    Plugins,
}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    match ev {
        ThreadEv::Load { cwd } => {
            if std::mem::replace(&mut part.asked, true) {
                return;
            }
            fx.request(Ctx::Skills, Command::Skills { cwd: cwd.clone() });
            fx.request(Ctx::Plugins, Command::Plugins { cwd });
        }
    }
}

/// An answer of the server: its list, or nothing when it failed (the popups then list what they
/// had; the oracle logs the failure and shows no warning of its own).
pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = fx;
    match (ctx, result) {
        (Ctx::Skills, Ok(Reply::Skills(skills))) => part.skills = Some(skills),
        (Ctx::Plugins, Ok(Reply::Plugins(plugins))) => part.plugins = Some(plugins),
        _ => {}
    }
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = thread;
    match command {
        Command::Skills { cwd } => ClientRequest::SkillsList {
            request_id: id,
            params: SkillsListParams {
                cwds: vec![cwd.into()],
                force_reload: true,
            },
        },
        Command::Plugins { cwd } => ClientRequest::PluginInstalled {
            request_id: id,
            // A directory that is not absolute names no repository marketplace.
            params: serde_json::from_value(serde_json::json!({ "cwds": [cwd] })).unwrap_or(
                PluginInstalledParams {
                    cwds: None,
                    install_suggestion_plugin_names: None,
                },
            ),
        },
    }
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
    match command {
        Command::Skills { cwd } => serde_json::from_value(result)
            .map(|response| Reply::Skills(sources::skills(response, Some(cwd))))
            .map_err(malformed),
        Command::Plugins { .. } => serde_json::from_value(result)
            .map(|response| Reply::Plugins(sources::plugins(response)))
            .map_err(malformed),
    }
}

/// The seed form of this screen's events:
/// `{"mentions":[{kind,name,description,insert,path?,plugin_id?,capabilities?}]}` (capabilities: `"skills"`, `{"mcp_servers":2}`, `{"apps":1}`),
/// `{"tasks":[{thread,title,cwd?,snippet?}]}`,
/// `{"file_matches":{"query":"q","matches":[{path,dir?,score?,indices?}]}}`,
/// `{"mention_usage":["path"]}`, `{"image_pasted":"/path/to.png"}` and
/// `{"history":["..."]}` (oldest first).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    Mentions(Vec<SeedMention>),
    Tasks(Vec<SeedTask>),
    FileMatches(SeedFileMatches),
    MentionUsage(Vec<String>),
    ImagePasted(std::path::PathBuf),
    History(Vec<String>),
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedMention {
    kind: SeedKind,
    name: String,
    description: String,
    insert: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    plugin_id: Option<String>,
    #[serde(default)]
    capabilities: Vec<SeedCapability>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedCapability {
    Skills,
    McpServers(usize),
    Apps(usize),
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedTask {
    thread: String,
    title: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    snippet: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedMatch {
    path: String,
    #[serde(default)]
    dir: bool,
    #[serde(default)]
    score: u32,
    #[serde(default)]
    indices: Option<Vec<u32>>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedFileMatches {
    query: String,
    matches: Vec<SeedMatch>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedKind {
    Skill,
    App,
    Plugin,
}

pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::History(history) => Ev::History(history),
        SeedEv::Mentions(mentions) => Ev::Mentions(
            mentions
                .into_iter()
                .map(|m| Mention {
                    kind: match m.kind {
                        SeedKind::Skill => MentionKind::Skill,
                        SeedKind::App => MentionKind::App,
                        SeedKind::Plugin => MentionKind::Plugin,
                    },
                    name: m.name,
                    description: m.description,
                    insert: m.insert,
                    path: m.path,
                    plugin_id: m.plugin_id,
                    capabilities: m
                        .capabilities
                        .into_iter()
                        .map(|c| match c {
                            SeedCapability::Skills => Capability::Skills,
                            SeedCapability::McpServers(n) => Capability::McpServers(n),
                            SeedCapability::Apps(n) => Capability::Apps(n),
                        })
                        .collect(),
                })
                .collect(),
        ),
        SeedEv::Tasks(tasks) => Ev::Tasks(
            tasks
                .into_iter()
                .map(|t| TaskMention {
                    thread_id: ThreadId::new(t.thread),
                    title: t.title,
                    cwd: t.cwd,
                    snippet: t.snippet,
                })
                .collect(),
        ),
        SeedEv::FileMatches(found) => Ev::FileMatches {
            query: found.query,
            matches: found
                .matches
                .into_iter()
                .map(|m| FileMatch {
                    path: m.path,
                    match_type: if m.dir {
                        MatchType::Directory
                    } else {
                        MatchType::File
                    },
                    score: m.score,
                    indices: m.indices,
                })
                .collect(),
        },
        SeedEv::MentionUsage(recent) => Ev::MentionUsage(recent),
        SeedEv::ImagePasted(path) => Ev::ImagePasted(path),
    })
}
