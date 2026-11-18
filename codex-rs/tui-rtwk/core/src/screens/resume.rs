//! `resume` (S18, resume and fork picker): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/resume/` in the UI crate. Nothing
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
}

/// One saved thread in the picker's list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: ThreadId,
    pub preview: String,
    pub model_provider: String,
    /// RFC 3339, when the thread started.
    pub started: String,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// The backend listed the saved threads.
    Listed(Vec<Row>),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Listed(rows) => model.rows = rows,
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
