//! `usage` (S20, /usage dashboard): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/usage/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "usage";
pub const ID: &str = "S20";
pub const KIND: Kind = Kind::Route;

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    pub account: Option<Account>,
    /// The overview's numbers (oracle: `analytics::summary`).
    pub overview: Panel<Overview>,
    /// The token history by day (oracle: `analytics::data::Remote`).
    pub history: Panel<Vec<Day>>,
    /// The credit history by day.
    pub credits: Panel<Vec<Day>>,
    /// The usage report's token series.
    pub usage: Panel<Vec<Day>>,
    /// The day the statistics are as of, `YYYY-MM-DD`.
    pub as_of: Option<String>,
    /// The report the user is on (`1`-`6` of the tab row).
    pub report: Report,
}

/// The signed-in account (oracle: the dashboard's header and profile block).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Account {
    pub name: String,
    pub handle: String,
    pub plan: String,
}

/// The reports of the tab row, in order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Report {
    #[default]
    Overview,
    Usage,
    Messages,
    Plugins,
    Skills,
    Chats,
}

impl Report {
    pub const ALL: [Report; 6] = [
        Report::Overview,
        Report::Usage,
        Report::Messages,
        Report::Plugins,
        Report::Skills,
        Report::Chats,
    ];
}

/// What a report shows of one remote value (oracle: `analytics::data::Remote`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Panel<T> {
    Loading,
    /// The backend reported no history.
    Unavailable,
    /// The request failed.
    Failed(Failure),
    Ready(T),
}

// Not derived: a derive would ask `T: Default` of the payload.
#[allow(clippy::derivable_impls)]
impl<T> Default for Panel<T> {
    fn default() -> Self {
        Panel::Loading
    }
}

/// Which request failed (the oracle keeps a fixed sentence per report).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    Analytics,
    Credits,
}

/// The five numbers of the overview.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Overview {
    pub lifetime_tokens: u64,
    pub peak_tokens: u64,
    pub longest_chat_secs: u64,
    pub current_streak_days: u32,
    pub longest_streak_days: u32,
    /// The heat map of the last year (oracle: `activity_chart`).
    #[serde(default)]
    pub activity: Activity,
    /// The "Activity insights" rows, label and shown value.
    #[serde(default)]
    pub insights: Vec<Insight>,
    /// The most used plugins and skills.
    #[serde(default)]
    pub top: Vec<TopUse>,
}

/// The weeks of the heat map and its month marks; `cells` holds the days that
/// have activity (week, weekday from Sunday, level 1..=4), the rest are empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Activity {
    pub weeks: usize,
    /// The weekday (from Sunday) of the last day: the last week has no cells
    /// after it.
    #[serde(default = "last_day_default")]
    pub last_day: usize,
    #[serde(default)]
    pub months: Vec<MonthMark>,
    #[serde(default)]
    pub cells: Vec<(usize, usize, u8)>,
}

fn last_day_default() -> usize {
    6
}

/// A month label over the week the month starts in.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct MonthMark {
    pub week: usize,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Insight {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct TopUse {
    pub name: String,
    pub runs: u64,
}

/// One day of a history.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Day {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub total: u64,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`. Also the seed form: `{"account":{..}}`, `{"overview":
/// {"ready":{..}}}`, `{"history":"loading"}`, `{"credits":{"failed":"credits"}}`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ev {
    Account(Account),
    Overview(Panel<Overview>),
    History(Panel<Vec<Day>>),
    Credits(Panel<Vec<Day>>),
    Usage(Panel<Vec<Day>>),
    AsOf(String),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A report of the tab row (`1`-`6`).
    Report(Report),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Account(account) => model.account = Some(account),
        Ev::Overview(panel) => model.overview = panel,
        Ev::History(panel) => model.history = panel,
        Ev::Credits(panel) => model.credits = panel,
        Ev::Usage(panel) => model.usage = panel,
        Ev::AsOf(date) => model.as_of = Some(date),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Report(report) => model.report = report,
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

/// A script's `screen` step for this directory: the event in its serde form.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))
}
