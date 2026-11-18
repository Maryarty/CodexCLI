//! `pager` (S16, transcript pager): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/pager/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "pager";
pub const ID: &str = "S16";
pub const KIND: Kind = Kind::Route;

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The static page (`/diff`, a patch approval) when one is open; the
    /// transcript pager (Ctrl+T) shows the active thread when it is `None`.
    pub page: Option<StaticPage>,
    /// How much of the transcript's older history is loaded.
    pub history: HistoryState,
}

/// Fixed content shown in the generic pager (oracle: `StaticOverlay`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticPage {
    pub title: String,
    pub lines: Vec<String>,
}

/// The pagination of the transcript's history (oracle:
/// `pager_overlay::TranscriptHistoryState`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HistoryState {
    LoadingOlder,
    LoadingBeginning,
    Partial,
    Failed,
    #[default]
    Complete,
}

impl HistoryState {
    /// Oracle: `TranscriptHistoryState::has_unloaded_history`.
    pub fn has_unloaded_history(self) -> bool {
        !matches!(self, Self::Complete)
    }
}

impl Model {
    /// The transcript pager shows no key hints while older history loads or
    /// after it failed (oracle: `transcript_view/footer.rs`).
    pub fn history_blocks_hints(&self) -> bool {
        matches!(
            self.history,
            HistoryState::LoadingOlder | HistoryState::LoadingBeginning | HistoryState::Failed
        )
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A static page was opened (`None`: closed).
    Page(Option<StaticPage>),
    /// The history pagination moved.
    History(HistoryState),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Page(page) => model.page = page,
        Ev::History(state) => model.history = state,
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {}
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

/// The seed form of this screen's events:
/// `{"page":{"title":"..","lines":[..]}}` and `{"history":"partial"}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    Page { title: String, lines: Vec<String> },
    History(SeedHistory),
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedHistory {
    LoadingOlder,
    LoadingBeginning,
    Partial,
    Failed,
    Complete,
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Page { title, lines } => Ev::Page(Some(StaticPage { title, lines })),
        SeedEv::History(state) => Ev::History(match state {
            SeedHistory::LoadingOlder => HistoryState::LoadingOlder,
            SeedHistory::LoadingBeginning => HistoryState::LoadingBeginning,
            SeedHistory::Partial => HistoryState::Partial,
            SeedHistory::Failed => HistoryState::Failed,
            SeedHistory::Complete => HistoryState::Complete,
        }),
    })
}
