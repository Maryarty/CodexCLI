//! `popovers` (S12, composer popovers): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/popovers/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::commands::fuzzy_match;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "popovers";
pub const ID: &str = "S12";
pub const KIND: Kind = Kind::Part;

/// The app-scoped model state of this screen, changed only by `apply`: the
/// data the `@` and `$` popups list, the prompts history search walks, and
/// the search in progress. The backend would supply the data; scripts seed it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// Paths relative to the working directory (`@` file search).
    pub files: Vec<String>,
    /// Skills, apps and plugins (`$`, `@`).
    pub mentions: Vec<Mention>,
    /// Submitted prompts, oldest first (Ctrl+R).
    pub history: Vec<String>,
    /// Reverse history search, while it is open.
    pub search: Option<HistorySearch>,
}

/// What a mention is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MentionKind {
    Skill,
    App,
    Plugin,
}

/// A skill, app or plugin that can be mentioned (`mentions_v2/candidate.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub kind: MentionKind,
    pub name: String,
    pub description: String,
    /// What follows the sigil when it is picked (`$` + `insert`).
    pub insert: String,
}

/// What a popup row stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateKind {
    File,
    Skill,
    App,
    Plugin,
}

/// One row of the `$` or `@` popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub kind: CandidateKind,
    /// The name, or the path of a file.
    pub label: String,
    pub description: String,
    /// What follows the sigil when it is picked.
    pub insert: String,
}

impl Model {
    /// The rows of a popup for `query`: best match first, ties in the data's
    /// order. `$` lists skills and apps; `@` files, plugins and apps.
    pub fn candidates(&self, sigil: Sigil, query: &str) -> Vec<Candidate> {
        let mut scored: Vec<(i32, Candidate)> = Vec::new();
        let mut push = |name: &str, candidate: Candidate| {
            if let Some((_, score)) = fuzzy_match(name, query) {
                scored.push((score, candidate));
            }
        };
        if sigil == Sigil::At {
            for path in &self.files {
                // The file name decides the match, the path is what is shown.
                let file_name = path.rsplit('/').next().unwrap_or(path);
                push(
                    file_name,
                    Candidate {
                        kind: CandidateKind::File,
                        label: path.clone(),
                        description: String::new(),
                        insert: path.clone(),
                    },
                );
            }
        }
        for mention in &self.mentions {
            let wanted = match (sigil, mention.kind) {
                (Sigil::Dollar, MentionKind::Skill | MentionKind::App) => true,
                (Sigil::At, MentionKind::Plugin | MentionKind::App) => true,
                _ => false,
            };
            if !wanted {
                continue;
            }
            push(
                &mention.name,
                Candidate {
                    kind: match mention.kind {
                        MentionKind::Skill => CandidateKind::Skill,
                        MentionKind::App => CandidateKind::App,
                        MentionKind::Plugin => CandidateKind::Plugin,
                    },
                    label: mention.name.clone(),
                    description: mention.description.clone(),
                    insert: mention.insert.clone(),
                },
            );
        }
        scored.sort_by_key(|(score, _)| *score);
        scored.into_iter().map(|(_, c)| c).collect()
    }
}

/// The sigil of a mention popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sigil {
    /// `$`: skills.
    Dollar,
    /// `@`: files and plugins.
    At,
}

/// Where reverse history search stands (`history_search.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchStatus {
    /// The query is empty: nothing is previewed.
    Idle,
    /// A match is previewed.
    Match,
    NoMatch,
}

/// Reverse history search: the query, the draft it will restore and the
/// history entry the preview shows (`chat_composer/history_search.rs`, with
/// the history of `chat_composer_history.rs`: a case-insensitive substring,
/// newest first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistorySearch {
    pub query: String,
    /// The composer's text when the search opened.
    pub draft: String,
    pub status: SearchStatus,
    cursor: Option<usize>,
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
    pub fn new(draft: String) -> Self {
        Self {
            query: String::new(),
            draft,
            status: SearchStatus::Idle,
            cursor: None,
        }
    }

    /// The entry the preview shows; none while idle or without a match, and
    /// the composer then shows `draft`.
    pub fn preview<'a>(&self, history: &'a [String]) -> Option<&'a str> {
        match self.status {
            SearchStatus::Match => self.cursor.and_then(|i| history.get(i)).map(String::as_str),
            SearchStatus::Idle | SearchStatus::NoMatch => None,
        }
    }

    /// The query changed: the search restarts from the newest entry.
    pub fn set_query(&mut self, query: String, history: &[String]) {
        self.query = query;
        self.cursor = None;
        if self.query.is_empty() {
            self.status = SearchStatus::Idle;
        } else {
            self.step(Direction::Older, history);
        }
    }

    /// Walks to the next older or newer match; at the end of the matches the
    /// current one stays (no "no match" flicker).
    pub fn step(&mut self, direction: Direction, history: &[String]) {
        if self.query.is_empty() {
            self.status = SearchStatus::Idle;
            return;
        }
        let needle = self.query.to_lowercase();
        let hit = |i: &usize| history[*i].to_lowercase().contains(&needle);
        let found = match (direction, self.cursor) {
            (Direction::Older, None) => (0..history.len()).rev().find(hit),
            (Direction::Older, Some(at)) => (0..at.min(history.len())).rev().find(hit),
            (Direction::Newer, None) => None,
            (Direction::Newer, Some(at)) => (at + 1..history.len()).find(hit),
        };
        match (found, self.cursor) {
            (Some(i), _) => {
                self.cursor = Some(i);
                self.status = SearchStatus::Match;
            }
            (None, Some(_)) => self.status = SearchStatus::Match,
            (None, None) => self.status = SearchStatus::NoMatch,
        }
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`. Only scripts make them yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    Files(Vec<String>),
    Mentions(Vec<Mention>),
    History(Vec<String>),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A prompt was submitted: history search finds it later.
    Record(String),
    SearchStart { draft: String },
    SearchEdit(String),
    SearchStep(Direction),
    /// The search ended (accepted or cancelled: the composer keeps the text).
    SearchEnd,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Files(files) => model.files = files,
        Ev::Mentions(mentions) => model.mentions = mentions,
        Ev::History(history) => model.history = history,
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Record(text) => {
            if !text.trim().is_empty() && model.history.last() != Some(&text) {
                model.history.push(text);
            }
        }
        Intent::SearchStart { draft } => model.search = Some(HistorySearch::new(draft)),
        Intent::SearchEdit(query) => {
            let history = model.history.clone();
            if let Some(search) = model.search.as_mut() {
                search.set_query(query, &history);
            }
        }
        Intent::SearchStep(direction) => {
            let history = model.history.clone();
            if let Some(search) = model.search.as_mut() {
                search.step(direction, &history);
            }
        }
        Intent::SearchEnd => model.search = None,
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

/// The seed form of this screen's events: `{"files":["src/a.rs"]}`,
/// `{"mentions":[{kind,name,description,insert}]}`, `{"history":["..."]}`
/// (history oldest first).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    Files(Vec<String>),
    Mentions(Vec<SeedMention>),
    History(Vec<String>),
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedMention {
    kind: SeedKind,
    name: String,
    description: String,
    insert: String,
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
        SeedEv::Files(files) => Ev::Files(files),
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
                })
                .collect(),
        ),
    })
}
