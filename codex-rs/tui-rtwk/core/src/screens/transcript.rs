//! `transcript` (S10, transcript view): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/transcript/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

pub mod diff;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use serde::Deserialize;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "transcript";
pub const ID: &str = "S10";
pub const KIND: Kind = Kind::Part;

/// What a static pager offers besides closing (oracle `pager_overlay.rs`
/// `PagerAction`): the key and label are the view's, the effect is the
/// opener's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PagerAction {
    /// Enter: "Edit configuration" (provider details).
    EditConfiguration,
}

/// The document of a static pager (oracle `StaticOverlay`): a title and the
/// text of a program, with its ANSI escapes, one line per `\n`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PagerDoc {
    pub title: String,
    pub text: String,
    #[serde(default)]
    pub action: Option<PagerAction>,
}

/// One thing the domain wants shown, once (ADR-013 Ж): the document of a static pager. The
/// presenter shows it and says so (`Intent::Shown`); whether the pager is still open afterwards
/// is the page's, never kept here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Directive<T> {
    /// Numbers the directives in the order they were made; what a presenter says it showed.
    pub seq: u64,
    pub value: T,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The documents (`/diff`, the full-screen approvals, provider details) nobody has shown
    /// yet, oldest first.
    queue: Vec<Directive<PagerDoc>>,
    seq: u64,
}

impl Model {
    /// The documents still to show, oldest first.
    pub fn pending(&self) -> impl Iterator<Item = &Directive<PagerDoc>> {
        self.queue.iter()
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A static pager is to open on `doc`.
    Pager(PagerDoc),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The presenter showed the directive `seq` (or gave it up): it is gone from the queue.
    Shown(u64),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Pager(doc) => {
            model.seq += 1;
            let seq = model.seq;
            model.queue.push(Directive { seq, value: doc });
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Shown(seq) => model.queue.retain(|directive| directive.seq != seq),
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

/// A script's `screen` step for this directory: the event `value` stands for
/// (`fixtures::script`). None is seedable yet; the slice gives its events a
/// serde form here and reads it, so the shared loader never changes.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    let SeedEv::Pager(doc) = seed;
    Ok(Ev::Pager(doc))
}

/// The serde form of this directory's seedable events.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Pager(PagerDoc),
}
