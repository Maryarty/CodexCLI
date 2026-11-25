//! `popovers` (S12, composer popovers): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/popovers/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::PluginInstalledParams;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::SkillsListParams;
use codex_app_server_protocol::SortDirection;
use codex_app_server_protocol::ThreadListParams;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadSearchParams;
use codex_app_server_protocol::ThreadSearchResponse;
use codex_app_server_protocol::ThreadSearchResult;
use codex_app_server_protocol::ThreadSearchSortKey;
use codex_app_server_protocol::ThreadSortKey;
use codex_app_server_protocol::ThreadSource;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::screens::Shared;
use crate::screens::composer::clipboard::Pasted;
use crate::data::history::{Lookup, Store};
use crate::data::mention::Capability;
use crate::data::files::{FileMatch, FileMatches, FileSearchAsk, MatchType};
use crate::kit::mentions::session::Sources;
use crate::screens::composer::mentions::sources;
use crate::data::task::TaskMention;
use crate::data::usage::MentionUsage;
use crate::screens::select::apps::App;
pub use crate::data::mention::{Mention, MentionKind};
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
    /// Which answer of a thread's task search `tasks` came from (`Ev::TasksFound`), so the app
    /// takes each answer once.
    pub tasks_from: Option<(ThreadId, u64)>,
    /// `tasks` is a seed (`Ev::Tasks`): an empty search answer leaves it.
    pub tasks_seeded: bool,
    /// Text a sheet left for the composer (`Intent::InsertText`) until the composer took it.
    pub insert: Option<Insert>,
    /// Numbers the inserts, so a composer takes each once.
    inserts: u64,
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
    /// The answer `seq` of a task search of `thread` (`ThreadPart::found`).
    TasksFound {
        thread: ThreadId,
        seq: u64,
        tasks: Vec<TaskMention>,
    },
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
    /// The `@` token is now this query: the tasks of the thread in use are searched (the app
    /// hands it to the active thread, `ThreadEv::SearchTasks`).
    SearchTasks(String),
    /// The `$` popup came up: the directory of apps is read, once, if it is not (the oracle
    /// reads it when the account and the thread are known; here it waits for the first `$`, so a
    /// session that never mentions an app asks no `app/list` of its own, `app/mentions.rs`).
    ListApps,
    /// A task was picked: the list of tasks the search found is spent.
    SpendTasks,
    /// A sheet puts `text` in the composer at its caret (`/skills`: `List skills` inserts `$`),
    /// as the external editor's text goes in: the model holds it until the composer says it took
    /// it (`Inserted`).
    InsertText(String),
    /// The composer took the insert numbered so.
    Inserted(u64),
    /// Ctrl+V: the clipboard's image is wanted (`Ask::ClipboardImage`).
    PasteImage,
}

/// Text for the composer's caret, numbered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Insert {
    pub seq: u64,
    pub text: String,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Mentions(mentions) => model.mentions = mentions,
        Ev::Catalog { listed, mentions } => {
            model.listed = listed;
            model.mentions = mentions;
        }
        // a seeded list stays until a search finds something (as `listed` does for a seed)
        Ev::Tasks(tasks) => {
            model.tasks_seeded = !tasks.is_empty();
            model.tasks = tasks;
        }
        Ev::TasksFound { thread, seq, tasks } => {
            model.tasks_from = Some((thread, seq));
            if !(model.tasks_seeded && tasks.is_empty()) {
                model.tasks_seeded = false;
                model.tasks = tasks;
            }
        }
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
        // the thread's: the app routes it (`app::apply_intent`)
        Intent::SearchTasks(_) | Intent::ListApps => {}
        Intent::SpendTasks => model.tasks.clear(),
        Intent::InsertText(text) => {
            model.inserts += 1;
            model.insert = Some(Insert {
                seq: model.inserts,
                text,
            });
        }
        Intent::Inserted(seq) => {
            if model.insert.as_ref().is_some_and(|insert| insert.seq == seq) {
                model.insert = None;
            }
        }
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
    /// The directory of the thread (what its own tasks are sorted first by).
    pub cwd: String,
    /// The task searches asked so far: the number of the last one, which only its answers count.
    pub generation: u64,
    /// The task search in flight: its two RPCs and what each answered.
    pub tasks_ask: Option<TasksAsk>,
    /// The answer of the last task search that came, for the app to take (`Model::tasks_from`).
    pub found: Option<TasksFound>,
}

/// A task search in flight (`task_mentions::spawn_search`): `thread/search` by the query joined
/// with the titles of `thread/list`, both asked at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TasksAsk {
    pub generation: u64,
    pub query: String,
    /// The thread in use: no task of its own to reference.
    pub current: ThreadId,
    /// What each RPC answered: nothing yet, an error, or its threads.
    pub search: Option<Option<Vec<Found>>>,
    pub titles: Option<Option<Vec<Found>>>,
}

/// The tasks a search found for `query`, the `generation` of the search naming the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TasksFound {
    pub generation: u64,
    pub query: String,
    pub tasks: Vec<TaskMention>,
}

/// One thread an RPC of the task search listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub id: ThreadId,
    /// The first of the name, the preview and the id that is not blank.
    pub title: String,
    pub cwd: String,
    pub snippet: String,
    /// An ephemeral thread or an ambient suggestion: never a task to mention.
    pub hidden: bool,
}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    /// The thread is in use, in `cwd`: its lists are asked for, once.
    Load { cwd: String },
    /// The `@` token is `query` in `current`'s composer: its tasks are searched. Only the last
    /// search's answer counts, a stale one is dropped (the oracle also waits 100 ms for the
    /// token to settle; the host's search of files does that here, a typed word costs two
    /// requests more).
    SearchTasks { query: String, current: ThreadId },
}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `skills/list` of the directory, re-scanned from disk.
    Skills { cwd: String },
    /// `plugin/installed` for the directory.
    Plugins { cwd: String },
    /// `thread/search` for `query`, by recency.
    TaskSearch { query: String },
    /// `thread/list` of the newest threads (their titles are matched by the query's words in the
    /// popup).
    TaskTitles,
}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The enabled skills of the asked directory, as mentions.
    Skills(Vec<Mention>),
    /// The installed plugins, as mentions.
    Plugins(Vec<Mention>),
    /// The threads of a task search or of the title list.
    Threads(Vec<Found>),
}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Skills,
    Plugins,
    /// The `thread/search` of the task search `generation`.
    TaskSearch { generation: u64 },
    /// The `thread/list` of the task search `generation`.
    TaskTitles { generation: u64 },
}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    match ev {
        ThreadEv::Load { cwd } => {
            if std::mem::replace(&mut part.asked, true) {
                return;
            }
            part.cwd = cwd.clone();
            fx.request(Ctx::Skills, Command::Skills { cwd: cwd.clone() });
            fx.request(Ctx::Plugins, Command::Plugins { cwd });
        }
        ThreadEv::SearchTasks { query, current } => {
            part.generation += 1;
            let generation = part.generation;
            // a blank query lists nothing, at once
            if query.trim().is_empty() {
                part.tasks_ask = None;
                part.found = Some(TasksFound {
                    generation,
                    query,
                    tasks: Vec::new(),
                });
                return;
            }
            fx.request(
                Ctx::TaskSearch { generation },
                Command::TaskSearch {
                    query: query.clone(),
                },
            );
            fx.request(Ctx::TaskTitles { generation }, Command::TaskTitles);
            part.tasks_ask = Some(TasksAsk {
                generation,
                query,
                current,
                search: None,
                titles: None,
            });
        }
    }
}

/// The threads of the search and of the title list as the tasks to mention: the search's rows,
/// then the listed ones it lacks; not the `current` thread, nor an ephemeral or ambient one;
/// the first 50; those of `cwd` (or under it) first, otherwise in order. `None` when both RPCs
/// failed.
pub fn merge_tasks(
    search: Option<Vec<Found>>,
    titles: Option<Vec<Found>>,
    current: &ThreadId,
    cwd: &str,
) -> Option<Vec<TaskMention>> {
    if search.is_none() && titles.is_none() {
        return None;
    }
    let mut found = search.unwrap_or_default();
    for thread in titles.unwrap_or_default() {
        if found.iter().all(|other| other.id != thread.id) {
            found.push(thread);
        }
    }
    let mut tasks: Vec<TaskMention> = found
        .into_iter()
        .filter(|thread| !thread.hidden && thread.id != *current)
        .take(MAX_SEARCH_RESULTS)
        .map(|thread| TaskMention {
            thread_id: thread.id,
            title: thread.title,
            cwd: thread.cwd,
            snippet: thread.snippet,
        })
        .collect();
    let windows = cwd.as_bytes().get(1) == Some(&b':') || cwd.starts_with(r"\\");
    tasks.sort_by_key(|task| {
        let Some((prefix, suffix)) = task.cwd.get(..cwd.len()).zip(task.cwd.get(cwd.len()..))
        else {
            return true;
        };
        !(prefix == cwd || windows && prefix.eq_ignore_ascii_case(cwd))
            || !(suffix.is_empty() || suffix.starts_with(['/', '\\']))
    });
    Some(tasks)
}

/// The most threads one search asks for and lists.
const MAX_SEARCH_RESULTS: usize = 50;

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
        (Ctx::TaskSearch { generation }, result) => {
            task_reply(part, generation, true, result.ok().and_then(threads))
        }
        (Ctx::TaskTitles { generation }, result) => {
            task_reply(part, generation, false, result.ok().and_then(threads))
        }
        _ => {}
    }
}

fn threads(reply: Reply) -> Option<Vec<Found>> {
    match reply {
        Reply::Threads(found) => Some(found),
        _ => None,
    }
}

/// One of the two answers of the search `generation`; the second one merges them. A search
/// that is not the last is dropped.
fn task_reply(part: &mut ThreadPart, generation: u64, search: bool, found: Option<Vec<Found>>) {
    let Some(ask) = part.tasks_ask.as_mut().filter(|ask| ask.generation == generation) else {
        return;
    };
    if search {
        ask.search = Some(found);
    } else {
        ask.titles = Some(found);
    }
    if ask.search.is_none() || ask.titles.is_none() {
        return;
    }
    let ask = part.tasks_ask.take().unwrap_or_else(|| unreachable!("checked above"));
    // both failed: the list stays as it was
    if let Some(tasks) = merge_tasks(
        ask.search.flatten(),
        ask.titles.flatten(),
        &ask.current,
        &part.cwd,
    ) {
        part.found = Some(TasksFound {
            generation,
            query: ask.query,
            tasks,
        });
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
        Command::TaskSearch { query } => ClientRequest::ThreadSearch {
            request_id: id,
            params: ThreadSearchParams {
                include_attached_threads: false,
                current_account_only: true,
                cursor: None,
                limit: Some(MAX_SEARCH_RESULTS as u32),
                sort_key: Some(ThreadSearchSortKey::RecencyAt),
                sort_direction: Some(SortDirection::Desc),
                source_kinds: Some(Vec::new()),
                archived: Some(false),
                search_term: query.trim().to_string(),
            },
        },
        Command::TaskTitles => ClientRequest::ThreadList {
            request_id: id,
            params: ThreadListParams {
                include_attached_threads: false,
                current_account_only: true,
                excluded_thread_ids: None,
                attached_to_thread_id: None,
                originators: None,
                cursor: None,
                limit: Some(MAX_SEARCH_RESULTS as u32),
                sort_key: Some(ThreadSortKey::UpdatedAt),
                sort_direction: Some(SortDirection::Desc),
                model_providers: Some(Vec::new()),
                source_kinds: Some(Vec::new()),
                archived: Some(false),
                section_id: None,
                project_id: None,
                cwd: None,
                use_state_db_only: true,
                search_term: None,
                parent_thread_id: None,
                ancestor_thread_id: None,
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
        Command::TaskSearch { .. } => serde_json::from_value::<ThreadSearchResponse>(result)
            .map(|response| Reply::Threads(response.data.into_iter().map(found).collect()))
            .map_err(malformed),
        Command::TaskTitles => serde_json::from_value::<ThreadListResponse>(result)
            .map(|response| {
                Reply::Threads(
                    response
                        .data
                        .into_iter()
                        .map(|thread| {
                            found(ThreadSearchResult {
                                thread,
                                snippet: String::new(),
                            })
                        })
                        .collect(),
                )
            })
            .map_err(malformed),
    }
}

/// A thread of an answer as the search lists it: titled by its name, else its preview, else its id.
fn found(result: ThreadSearchResult) -> Found {
    let thread = result.thread;
    let title = [
        thread.name.as_deref(),
        Some(thread.preview.as_str()),
        Some(thread.id.as_str()),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|title| !title.is_empty())
    .unwrap_or_default()
    .to_string();
    Found {
        hidden: thread.ephemeral
            || matches!(
                thread.thread_source,
                Some(ThreadSource::Feature(ref source)) if source == "ambient_suggestions"
            ),
        id: ThreadId::new(&thread.id),
        title,
        cwd: thread.cwd.to_string_lossy().into_owned(),
        snippet: result.snippet,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::TicketMint;

    fn found(id: &str) -> Found {
        Found {
            id: ThreadId::new(id),
            title: id.into(),
            cwd: "/w".into(),
            snippet: String::new(),
            hidden: false,
        }
    }

    fn search(part: &mut ThreadPart, query: &str) {
        let mut out = AppFx::default();
        let mut mint = TicketMint::default();
        let thread = ThreadId::new("cur");
        let mut fx = ThreadFx::new(&thread, &mut out, &mut mint);
        let ev = ThreadEv::SearchTasks {
            query: query.into(),
            current: thread.clone(),
        };
        apply_thread(part, ev, &mut fx);
    }

    fn answer(part: &mut ThreadPart, ctx: Ctx, threads: Option<Vec<Found>>) {
        let mut out = AppFx::default();
        let mut mint = TicketMint::default();
        let thread = ThreadId::new("cur");
        let mut fx = ThreadFx::new(&thread, &mut out, &mut mint);
        let result = threads
            .map(Reply::Threads)
            .ok_or(RpcError::Remote("failed".into()));
        apply_reply(part, ctx, result, &mut fx);
    }

    #[test]
    fn both_answers_of_the_last_search_make_the_list_and_a_stale_one_is_dropped() {
        let mut part = ThreadPart::default();
        search(&mut part, "fi");
        search(&mut part, "fix");
        // the first search answers late: dropped
        answer(&mut part, Ctx::TaskSearch { generation: 1 }, Some(vec![found("old")]));
        assert!(part.found.is_none());
        answer(&mut part, Ctx::TaskSearch { generation: 2 }, Some(vec![found("a")]));
        assert!(part.found.is_none(), "one half is not the answer");
        answer(&mut part, Ctx::TaskTitles { generation: 2 }, Some(vec![found("b")]));
        let found = part.found.expect("both are in");
        assert_eq!((found.generation, found.query.as_str()), (2, "fix"));
        assert_eq!(found.tasks.len(), 2);
        assert!(part.tasks_ask.is_none());
    }

    #[test]
    fn a_blank_query_empties_the_list_at_once_and_two_failures_leave_it() {
        let mut part = ThreadPart::default();
        search(&mut part, "  ");
        assert_eq!(part.found.as_ref().map(|found| found.tasks.len()), Some(0));
        let before = part.found.clone();
        search(&mut part, "x");
        answer(&mut part, Ctx::TaskSearch { generation: 2 }, None);
        answer(&mut part, Ctx::TaskTitles { generation: 2 }, None);
        assert_eq!(part.found, before);
    }
}
