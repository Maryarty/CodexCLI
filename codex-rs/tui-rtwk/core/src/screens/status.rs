//! `status` (S19, /status card): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/status/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "status";
pub const ID: &str = "S19";
pub const KIND: Kind = Kind::Sheet;

/// The sandbox profile of the session (`status_permission_summary`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    ReadOnly,
    Workspace,
    Full,
}

/// When approvals are asked (`status_approval_label`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Approval {
    OnRequest,
    Never,
    OnFailure,
    UnlessTrusted,
}

/// The token counters of the thread (`TokenUsage`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Tokens {
    pub input: i64,
    pub cached_input: i64,
    pub output: i64,
    pub total: i64,
}

impl Tokens {
    /// The total the card shows: the input that was not cached and the output
    /// (`TokenUsage::blended_total`).
    pub fn blended_total(&self) -> i64 {
        (self.input - self.cached_input).max(0) + self.output.max(0)
    }
}

/// The context window (`ContextInfo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Context {
    pub window: i64,
    pub used: i64,
    pub percent_left: i64,
}

/// One rate-limit window; its reset in the user's local time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Limit {
    pub window_minutes: i64,
    pub used_percent: i64,
    pub reset: Option<Reset>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Reset {
    pub hour: u8,
    pub minute: u8,
}

/// What `/status` shows: the raw values at the moment it ran; every shown
/// string is derived by the view.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Card {
    pub version: String,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub reasoning_summary: Option<String>,
    pub directory: String,
    pub profile: Profile,
    pub approval: Approval,
    #[serde(default)]
    pub auto_review: bool,
    /// The instruction files in use (`Agents.md`), none when empty.
    #[serde(default)]
    pub agents: Vec<String>,
    pub tokens: Tokens,
    pub context: Option<Context>,
    #[serde(default)]
    pub limits: Vec<Limit>,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The card of the last `/status`.
    pub card: Option<Card>,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// `/status` ran: the card to show.
    Shown(Card),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The card was closed.
    Close,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Shown(card) => model.card = Some(card),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Close => model.card = None,
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

/// The seed form: `{"kind":"card", ..the fields of `Card`..}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Card(Card),
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    let SeedEv::Card(card) = seed;
    Ok(Ev::Shown(card))
}
