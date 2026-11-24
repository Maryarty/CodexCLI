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
mod compose;
mod helpers;
mod limits;
mod rates;
mod warnings;

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

pub use warnings::{Limits as WarningLimits, Sample, Source, Used, Warning, warning_threshold};
pub use compose::{CardFacts, CatalogModel, Inputs, ThreadFacts};
pub use rates::{
    Bucket, CreditStatus, RateLimitOrigin, RateLimitStore, RateRead, ResetCredit, ResetCredits,
    ResetKind,
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
    /// Fine-grained approval flows (the protocol's `granular`).
    Granular,
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

/// The app-scoped model state of this screen, changed only by `apply`: the
/// facts a `/status` card is made of that no thread owns. The cards themselves
/// are items of a thread's transcript (`Item::Status`, ADR-003).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The clock the shell moves on, unix seconds (the model never reads it):
    /// when a card is made and the limits it shows were captured.
    pub now: i64,
    /// The local time zone's offset from UTC, seconds east.
    pub utc_offset_secs: i32,
    /// The account the server is signed in to (oracle `status_account_display`).
    pub account: Option<Account>,
    /// The models the server offers: their names and default reasoning.
    pub catalog: Vec<CatalogModel>,
    /// The connection to a server other than the in-process one.
    pub remote: Option<Remote>,
    /// The request number the next `/status` refresh takes (oracle
    /// `next_status_refresh_request_id`).
    pub next_request: u64,
}

impl Model {
    /// The request number of a refresh a `/status` asks for.
    pub(crate) fn take_request(&mut self) -> u64 {
        let request = self.next_request;
        self.next_request = self.next_request.wrapping_add(1);
        request
    }

    /// Whether the session is signed in to a ChatGPT account (oracle
    /// `has_chatgpt_account`): the limits of one can be read.
    pub fn has_chatgpt_account(&self) -> bool {
        matches!(self.account, Some(Account::ChatGpt { .. }))
    }
}

/// The events of this screen; a variant of the root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A card a script seeds (`{"kind":"status_card", ..}`): the app puts it
    /// into the shown thread's transcript as `/status` does.
    Card(Box<StatusCard>),
    /// The account the server is signed in to, as the host read it.
    Account(Option<Account>),
    /// The models the server offers.
    Catalog(Vec<CatalogModel>),
    /// The connection to the server changed.
    Connection(Option<Remote>),
}

/// The intents of this screen; a variant of the root `ScreenIntent`. `/status`
/// is a session command (`SessionIntent::Status`): the card is the thread's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        // The thread's: the app routes it (`app::apply_event`).
        Ev::Card(_) => {}
        Ev::Account(account) => model.account = account,
        Ev::Catalog(catalog) => model.catalog = catalog,
        Ev::Connection(remote) => model.remote = remote,
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {}
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    pub received: bool,
    pub tokens: Tokens,
    pub context: Option<Context>,
    /// The instruction files the session loaded (oracle
    /// `instruction_source_paths`), as the server said at the thread's start.
    pub agents: Vec<AgentsSource>,
}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    TokenUsage { tokens: Tokens, context: Option<Context> },
    /// The instruction files of the session, from the start that opened the
    /// thread.
    InstructionSources(Vec<AgentsSource>),
    /// A refresh of the rate limits ended: the transcript's `/status` cell that asked for it
    /// (`request`) takes `limits`, the cells of other requests wait on (the thread's transcript
    /// owns the cells: `threads.rs` applies it there).
    Refreshed { request: u64, limits: Limits },
}

pub(crate) fn project(
    notification: &codex_app_server_protocol::ServerNotification,
) -> Option<(ThreadId, ThreadEv)> {
    let codex_app_server_protocol::ServerNotification::ThreadTokenUsageUpdated(note) = notification else {
        return None;
    };
    Some((ThreadId::new(&note.thread_id), token_usage(&note.token_usage)))
}

/// The account an `account/updated` notification names (oracle
/// `status_account_display_from_auth_mode`): the notification carries no email
/// or workspace, so the card shows none until the account is read again.
pub fn account_from_auth_mode(
    auth_mode: Option<codex_app_server_protocol::AuthMode>,
    plan: Option<codex_protocol::account::PlanType>,
) -> Option<Account> {
    use codex_app_server_protocol::AuthMode;
    match auth_mode? {
        AuthMode::ApiKey => Some(Account::ApiKey),
        AuthMode::Chatgpt
        | AuthMode::ChatgptAuthTokens
        | AuthMode::AgentIdentity
        | AuthMode::PersonalAccessToken => Some(Account::ChatGpt {
            email: None,
            plan,
            workspace: None,
        }),
        _ => None,
    }
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
    match ev {
        ThreadEv::TokenUsage { tokens, context } => {
            part.received = true;
            part.tokens = tokens;
            part.context = context;
        }
        ThreadEv::InstructionSources(agents) => part.agents = agents,
        // The transcript's: `threads::apply_ev` finishes the cell.
        ThreadEv::Refreshed { .. } => {}
    }
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

/// The seed forms: `{"kind":"status_card", ..the fields of `StatusCard`..}`,
/// `{"kind":"account","account":{..}|null}`, `{"kind":"catalog","models":[..]}`
/// and `{"kind":"connection","remote":{..}|null}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    StatusCard(Box<StatusCard>),
    Account {
        #[serde(default)]
        account: Option<Account>,
    },
    Catalog {
        models: Vec<CatalogModel>,
    },
    Connection {
        #[serde(default)]
        remote: Option<Remote>,
    },
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::StatusCard(card) => Ev::Card(card),
        SeedEv::Account { account } => Ev::Account(account),
        SeedEv::Catalog { models } => Ev::Catalog(models),
        SeedEv::Connection { remote } => Ev::Connection(remote),
    })
}
