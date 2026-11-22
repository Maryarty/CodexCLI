//! `status` (S19, /status card): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/cells/status/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

mod card;
mod helpers;
mod limits;

pub use card::{
    Account, ApprovalLabel, ModelDetail, Permissions, PermissionsLabel, Reasoning, Remote,
    SandboxKind, SandboxText, StatusCard, model_details,
};
pub use helpers::{
    AgentsSource, PlanLabel, ResetDate, ResetTime, compose_agents_summary,
    format_directory_display, plan_label, reset_time,
};
pub use limits::{
    Credits, CreditsText, Data as LimitData, Limits, Row as LimitRow, RowLabel, RowValue,
    Snapshot, SpendLimit, SpendUsed, Window, WindowName, window_name,
};

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
    /// The card of a `/status` cell a script seeded: the facts of the oracle's
    /// cell, shown by `StatusCell`. Only a seed writes it; the live app moves
    /// the card into the thread's transcript (`Item::Status`) and drops this
    /// and `card` with the departure (tm debt task status-card-adr003).
    pub cell: Option<StatusCard>,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
// The dialog's `Card` goes with the dialog (tm status-card-adr003).
#[allow(clippy::large_enum_variant)]
pub enum Ev {
    /// `/status` ran: the card to show.
    Shown(Card),
    /// A `/status` cell was seeded (a script's `status_card`).
    CellShown(Box<StatusCard>),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Open,
    /// The card was closed.
    Close,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Shown(card) => model.card = Some(card),
        Ev::CellShown(card) => model.cell = Some(*card),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Open => {}
        Intent::Close => model.card = None,
    }
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    pub received: bool,
    pub tokens: Tokens,
    pub context: Option<Context>,
}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    TokenUsage { tokens: Tokens, context: Option<Context> },
}

pub(crate) fn project(
    notification: &codex_app_server_protocol::ServerNotification,
) -> Option<(ThreadId, ThreadEv)> {
    let codex_app_server_protocol::ServerNotification::ThreadTokenUsageUpdated(note) = notification else {
        return None;
    };
    Some((ThreadId::new(&note.thread_id), token_usage(&note.token_usage)))
}

fn token_usage(usage: &codex_app_server_protocol::ThreadTokenUsage) -> ThreadEv {
    let total = &usage.total;
    let tokens = Tokens {
        input: total.input_tokens,
        cached_input: total.cached_input_tokens,
        output: total.output_tokens,
        total: total.total_tokens,
    };
    // Match the oracle's effective context window, reserved baseline included.
    let context = usage.model_context_window.filter(|window| *window > 0).map(|window| {
        let used = usage.last.total_tokens.max(0);
        let usage = codex_protocol::protocol::TokenUsage { total_tokens: used, ..Default::default() };
        Context { window, used, percent_left: usage.percent_of_context_window_remaining(window) }
    });
    ThreadEv::TokenUsage { tokens, context }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RateLimits {
    pub primary: Option<RateWindow>,
    pub secondary: Option<RateWindow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateWindow {
    pub window_minutes: Option<i64>,
    pub used_percent: i32,
    pub resets_at: Option<i64>,
}

impl From<codex_app_server_protocol::RateLimitSnapshot> for RateLimits {
    fn from(snapshot: codex_app_server_protocol::RateLimitSnapshot) -> Self {
        let window = |w: codex_app_server_protocol::RateLimitWindow| RateWindow {
            window_minutes: w.window_duration_mins,
            used_percent: w.used_percent,
            resets_at: w.resets_at,
        };
        Self { primary: snapshot.primary.map(window), secondary: snapshot.secondary.map(window) }
    }
}

impl RateLimits {
    pub fn card_limits(&self) -> Vec<Limit> {
        self.primary.iter().chain(&self.secondary).filter_map(|rate| {
            Some(Limit {
                window_minutes: rate.window_minutes?,
                used_percent: i64::from(rate.used_percent),
                reset: None,
            })
        }).collect()
    }
}

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
    let _ = fx;
    let ThreadEv::TokenUsage { tokens, context } = ev;
    part.received = true;
    part.tokens = tokens;
    part.context = context;
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

/// The seed forms: `{"kind":"card", ..the fields of `Card`..}` (the dialog's
/// card) and `{"kind":"status_card", ..the fields of `StatusCard`..}` (the
/// cell's).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Card(Box<Card>),
    StatusCard(Box<StatusCard>),
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Card(card) => Ev::Shown(*card),
        SeedEv::StatusCard(card) => Ev::CellShown(card),
    })
}
