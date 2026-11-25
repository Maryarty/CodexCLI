//! `usage` (S20, the `/usage` dashboard): the core half of the screen's
//! directory. The model, its events and intents and the asks the host runs
//! live here with no rtwk; the view half is `src/pages/usage/` in the UI
//! crate. Oracle: `codex-rs/tui/src/analytics*` (the pure logic of its files is
//! carried into the modules below, each naming its file).
//!
//! The model holds the facts and the requests: the account, and the reports as a cache by
//! `ReportKey` (section, days, grouping, model filter). What the person chose (the tab, the
//! range per range group, the group per section, the model filter, the chat sort) is the page's
//! `UsageChoice` (`core/src/usage_choice.rs`), a value its widget state holds; it says what to load with the
//! verb `Intent::Load(key)`. The cursor, the detail, the scroll position and the modes
//! (dashboard, help) are the page's widget state too (R14). Every load is started with a
//! `Stamp`; its reply settles only the load of that stamp, so a refresh, a range change or a
//! close drops what is still in flight (R1).

use std::collections::BTreeMap;
use std::rc::Rc;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_protocol::account::PlanType;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub mod activity;
pub mod dashboard;
pub mod chats;
pub mod data;
pub mod date;
pub mod models;
pub mod normalize;
pub mod plan;
pub mod plot;
pub mod profile;
pub mod reset;
pub mod report;
pub mod report_data;
pub mod sections;
pub mod tokens;

pub use activity::TokenActivityView;
pub use data::{GroupName, Load, LoadError, Note, ReportKey, Stamp};
pub use date::NaiveDate;
pub use models::{AccountAnalyticsHistory as History, AccountKind};
pub use sections::{Control, RangeGroup, Section};
pub use crate::usage_choice::{Chosen, UsageChoice, View};

pub const NAME: &str = "usage";
pub const ID: &str = "S20";
pub const KIND: Kind = Kind::Route;

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    /// The number of the open session: a refresh or a new open starts the
    /// next one, and the host rebuilds its account session when it changes.
    pub session: u32,
    /// A session is live (reports load only then; leaving the page cancels).
    pub live: bool,
    next_stamp: u32,
    pub account: Load<PlanType>,
    /// The signed-in account's label beside the heading (the e-mail).
    pub account_label: Option<String>,
    pub plan_enabled: bool,
    /// The times are shown on a 24-hour clock (the user's preference).
    pub hour24: bool,
    /// Model id -> display name, from the model list.
    pub model_names: Rc<BTreeMap<String, String>>,
    /// The last day every report asks for, fixed per session.
    pub end_date: NaiveDate,
    /// The models the token report knows, for the `m` control.
    pub token_models: Vec<String>,
    pub credit_groups: Vec<usize>,
    pub attributed_usage: bool,
    /// The reports asked for, as a cache by what was asked.
    pub histories: BTreeMap<ReportKey, Load<Rc<History>>>,
    /// What the fixture script that seeded the model chose, for the page it opens (read once by
    /// the page's state; nothing here reads it).
    pub seeded: Option<Rc<Seed>>,
    /// The Plan section's allowance history (oracle: `plan::State::report`).
    pub plan: Load<Rc<plan::Report>>,
    /// The Chats section's listing of a business account (oracle: `chats`).
    pub chats: Load<Rc<chats::ChatList>>,
    /// The Chats section's listing of a consumer account (oracle: `tasks`).
    pub tasks: Load<Rc<chats::TaskList>>,
    pub profile: Load<Rc<profile::Profile>>,
    /// The usage-limit resets: the count, the credits, the redemption (`reset`).
    pub reset: reset::State,
    reports_started: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            session: 0,
            live: false,
            next_stamp: 0,
            account: Load::Unavailable,
            account_label: None,
            plan_enabled: false,
            hour24: false,
            model_names: Rc::default(),
            end_date: NaiveDate::from_unix_secs(0),
            token_models: Vec::new(),
            credit_groups: vec![0],
            attributed_usage: true,
            histories: BTreeMap::new(),
            seeded: None,
            plan: Load::Unavailable,
            chats: Load::Unavailable,
            tasks: Load::Unavailable,
            profile: Load::Unavailable,
            reset: reset::State::default(),
            reports_started: false,
        }
    }
}

/// What the host is asked, answered with the matching `Ev` under the same
/// session and stamp. The host runs `Ask`s as `Command::Usage { ask }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    pub session: u32,
    pub stamp: Stamp,
    pub what: AskKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AskKind {
    /// The account behind the session (oracle: `Live::session`), with the
    /// facts the page needs of the configuration.
    Account,
    /// A report of the section: `days` days ending at the model's end date,
    /// grouped by the `group`-th grouping, filtered to `model` (the token
    /// report's `m` control).
    Report {
        section: Section,
        days: u32,
        group: usize,
        model: Option<String>,
    },
    Profile,
    /// The Plan section's allowance history (oracle: `Live::plan_history`).
    Plan,
    /// The recent root chats with their estimates (a business account).
    Chats,
    /// The recent root chats with their task usage (a consumer account).
    Tasks,
    /// Redeem a usage-limit reset (`account/rateLimitResetCredit/consume`) under the idempotency
    /// `key`; `credit_id` none lets the backend choose. Answered by `Ev::Redeemed`, whatever the
    /// session: the redemption is the account's, not the page's.
    Redeem { key: String, credit_id: Option<String> },
}

/// The settled account: the plan, and what the host knows of the session.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct AccountReply {
    pub plan: Option<PlanType>,
    #[serde(default)]
    pub label: Option<String>,
    /// The day the reports end on.
    pub today: NaiveDate,
    #[serde(default)]
    pub plan_enabled: bool,
    /// Model id and display name.
    #[serde(default)]
    pub model_names: Vec<(String, String)>,
    /// The groupings the account's credit report supports.
    #[serde(default)]
    pub credit_groups: Vec<usize>,
    /// The 24-hour clock is the user's preference (oracle: `ClockFormat`,
    /// detected once; unknown keeps 12-hour): the times of the page.
    #[serde(default)]
    pub hour24: bool,
}

/// A report's answer: the history, and what the host learned making it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ReportReply {
    pub history: Option<History>,
    /// The models of the token report, when this one listed them.
    #[serde(default)]
    pub token_models: Option<Vec<String>>,
    /// Whether the usage report attributes every day to a surface.
    #[serde(default)]
    pub attributed_usage: Option<bool>,
}

/// The events of this screen (the host's answers); a variant of the root
/// `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    Account {
        session: u32,
        stamp: Stamp,
        result: Result<AccountReply, LoadError>,
    },
    Report {
        session: u32,
        stamp: Stamp,
        section: Section,
        result: Result<ReportReply, LoadError>,
    },
    Profile {
        session: u32,
        stamp: Stamp,
        result: Result<Option<Box<profile::Profile>>, LoadError>,
    },
    Plan {
        session: u32,
        stamp: Stamp,
        result: Result<Option<plan::Data>, LoadError>,
    },
    Chats {
        session: u32,
        stamp: Stamp,
        result: Result<Option<chats::ChatList>, LoadError>,
    },
    Tasks {
        session: u32,
        stamp: Stamp,
        result: Result<Option<chats::TaskList>, LoadError>,
    },
    /// The backend's answer to the redemption of `key`, or why the ask failed.
    Redeemed {
        key: String,
        result: Result<reset::Outcome, String>,
    },
    /// The signed-in identity changed under the session: everything again
    /// (oracle: `identity_invalidated` -> `refresh`).
    Invalidated { session: u32 },
    /// A script's seed (`seed`): settles loads directly, bypassing the asks.
    Seed(Box<Seed>),
}

/// The seed form (`{"step":"screen","dir":"usage","ev":{..}}`): the state a
/// fixture wants the model in, as the loads would leave it.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seed {
    #[serde(default)]
    pub account: Option<AccountReply>,
    /// The account's name before its plan is known (oracle: `Live::session` is the page's
    /// identity from the start); a settled account brings its own.
    #[serde(default)]
    pub account_label: Option<String>,
    #[serde(default)]
    pub section: Option<Section>,
    /// The group per section, as the user's choices left them.
    #[serde(default)]
    pub groups: Vec<GroupSeed>,
    #[serde(default)]
    pub ranges: Option<[usize; 3]>,
    #[serde(default)]
    pub token_models: Vec<String>,
    #[serde(default)]
    pub token_model: Option<String>,
    #[serde(default)]
    pub attributed_usage: Option<bool>,
    /// Each section's load: `{"section":"usage","load":{"ready":{..}}}`.
    #[serde(default)]
    pub histories: Vec<HistorySeed>,
    #[serde(default)]
    pub profile: Option<SeedLoad<profile::Profile>>,
    /// The Plan section's load: the wire's history, checked as the host's
    /// reply is.
    #[serde(default)]
    pub plan: Option<SeedLoad<plan::Data>>,
    #[serde(default)]
    pub chats: Option<SeedLoad<chats::ChatList>>,
    #[serde(default)]
    pub tasks: Option<SeedLoad<chats::TaskList>>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct GroupSeed {
    pub section: Section,
    pub group: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct HistorySeed {
    pub section: Section,
    pub load: SeedLoad<History>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedLoad<T> {
    Loading,
    Unavailable,
    Error(String),
    Ready(T),
}

impl<T> SeedLoad<T> {
    fn into_load<U>(self, ready: impl FnOnce(T) -> U) -> Load<U> {
        match self {
            SeedLoad::Loading => Load::Loading(Stamp::default()),
            SeedLoad::Unavailable => Load::Unavailable,
            SeedLoad::Error(message) => Load::Error(LoadError::Host(message)),
            SeedLoad::Ready(value) => Load::Ready(ready(value)),
        }
    }
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Kept for the chat's `/usage` open (the app refreshes the rate limits).
    RefreshRateLimits,
    /// The usage menu opened (oracle `open_usage_menu`): the app checks a count known to be zero
    /// again. The menu's `Redeem reset` entry reads `reset`.
    Menu,
    /// The reset flow opened (the entry, or `Try again` of a failed list): the app reads the
    /// credits, and `reset.listing` says how the list stands.
    Resets,
    /// `Yes, use reset` (`credit_id` none: the backend's next credit): a new idempotency key.
    Redeem { credit_id: Option<String> },
    /// `Try again` of a failed redemption: the same key.
    Retry,
    /// The page opened: a fresh session (oracle: `open`).
    Open,
    /// `R`: every report again (oracle: `refresh`).
    Refresh,
    /// The page was left: pending loads are dropped (oracle: `cancel_loads`).
    Leave,
    /// The report `key` is wanted, again: it is asked for, and the cache entry is `Loading`
    /// until the answer of that ask settles it (a range, grouping or model change, a first
    /// load of each tab).
    Load(ReportKey),
}

impl Model {
    fn next_stamp(&mut self) -> Stamp {
        self.next_stamp += 1;
        Stamp(self.next_stamp)
    }

    /// The host ask a reset step made, if it made one.
    pub fn redemption_asks(&mut self, next: reset::Next) -> Vec<Ask> {
        match next {
            reset::Next::Redeem { key, credit_id } => {
                let stamp = self.next_stamp();
                vec![self.ask(stamp, AskKind::Redeem { key, credit_id })]
            }
            reset::Next::Nothing | reset::Next::Read => Vec::new(),
        }
    }

    fn ask(&self, stamp: Stamp, what: AskKind) -> Ask {
        Ask {
            session: self.session,
            stamp,
            what,
        }
    }

    /// A fresh session: drops every account-owned load and asks for the
    /// account (oracle: `refresh`).
    fn open_session(&mut self) -> Vec<Ask> {
        self.session += 1;
        self.live = true;
        self.histories.clear();
        self.profile = Load::Unavailable;
        self.plan = Load::Unavailable;
        self.chats = Load::Unavailable;
        self.tasks = Load::Unavailable;
        self.reports_started = false;
        self.token_models.clear();
        let stamp = self.next_stamp();
        self.account = Load::Loading(stamp);
        vec![self.ask(stamp, AskKind::Account)]
    }

    /// Once the account is known, the profile and the reports that are not a section's own
    /// (plan, chats) start, once (oracle: `start_reports`); the page asks for the sections'
    /// reports with `Intent::Load`, from its `UsageChoice`.
    fn start_reports(&mut self) -> Vec<Ask> {
        if self.reports_started || self.account.ready().is_none() {
            return Vec::new();
        }
        self.reports_started = true;
        let visible = self.visible_sections();
        if visible.is_empty() {
            return Vec::new();
        }
        let mut asks = vec![self.load_profile()];
        if visible.contains(&Section::Plan) {
            let stamp = self.next_stamp();
            self.plan = Load::Loading(stamp);
            asks.push(self.ask(stamp, AskKind::Plan));
        }
        if visible.contains(&Section::Chats) {
            let stamp = self.next_stamp();
            if self.business() {
                self.chats = Load::Loading(stamp);
                asks.push(self.ask(stamp, AskKind::Chats));
            } else {
                self.tasks = Load::Loading(stamp);
                asks.push(self.ask(stamp, AskKind::Tasks));
            }
        }
        asks
    }

    fn load_profile(&mut self) -> Ask {
        let stamp = self.next_stamp();
        self.profile = Load::Loading(stamp);
        self.ask(stamp, AskKind::Profile)
    }

    /// Starts the report `key` again (oracle: `load_report`); a key of a section without a
    /// report of its own starts nothing.
    fn load_report(&mut self, key: ReportKey) -> Option<Ask> {
        key.section.report()?;
        let stamp = self.next_stamp();
        self.histories.insert(key.clone(), Load::Loading(stamp));
        Some(self.ask(
            stamp,
            AskKind::Report {
                section: key.section,
                days: key.days,
                group: key.group,
                model: key.model,
            },
        ))
    }

    /// What every settled load checks (oracle: `poll_reports`): a consumer account whose usage
    /// is not attributed has no per-surface groups, so the usage report it was asked for by
    /// one answers the ungrouped question.
    fn after_settle(&mut self) {
        if self.business() || self.consumer_attribution() {
            return;
        }
        let moved: Vec<ReportKey> = self
            .histories
            .keys()
            .filter(|key| key.section == Section::Usage && ![0, 2].contains(&key.group))
            .cloned()
            .collect();
        for key in moved {
            if let Some(load) = self.histories.remove(&key) {
                self.histories.insert(ReportKey { group: 0, ..key }, load);
            }
        }
    }

    fn settle_account(&mut self, reply: AccountReply) {
        self.account = Load::Ready(reply.plan.unwrap_or(PlanType::Unknown));
        self.account_label = reply.label;
        self.end_date = reply.today;
        self.plan_enabled = reply.plan_enabled;
        self.hour24 = reply.hour24;
        self.model_names = Rc::new(reply.model_names.into_iter().collect());
        self.credit_groups = if reply.credit_groups.is_empty() {
            vec![0]
        } else {
            reply.credit_groups
        };
    }

    /// The model's name for display (oracle: `model_name`).
    pub fn model_name<'a>(&'a self, model: &'a str) -> &'a str {
        self.model_names
            .get(model)
            .filter(|name| !name.is_empty())
            .map(String::as_str)
            .unwrap_or(model)
    }

    fn seed(&mut self, seed: Seed) {
        self.live = true;
        self.reports_started = true;
        if let Some(label) = seed.account_label.clone() {
            self.account_label = Some(label);
        }
        if let Some(account) = seed.account.clone() {
            self.settle_account(account);
        }
        if let Some(attributed) = seed.attributed_usage {
            self.attributed_usage = attributed;
        }
        self.token_models.clone_from(&seed.token_models);
        let choice = UsageChoice::seeded(&seed, self);
        for HistorySeed { section, load } in seed.histories.clone() {
            if let Some(key) = choice.key(self, section) {
                self.histories.insert(key, load.into_load(Rc::new));
            }
        }
        if let Some(profile) = seed.profile.clone() {
            self.profile = profile.into_load(Rc::new);
        }
        if let Some(chats) = seed.chats.clone() {
            self.chats = chats.into_load(Rc::new);
        }
        if let Some(tasks) = seed.tasks.clone() {
            self.tasks = tasks.into_load(Rc::new);
        }
        if let Some(history) = seed.plan.clone() {
            self.plan = match history {
                SeedLoad::Ready(data) => match plan::Report::parse(data) {
                    Ok(Some(report)) => Load::Ready(Rc::new(report)),
                    Ok(None) => Load::Unavailable,
                    Err(error) => Load::Error(error.into()),
                },
                other => other.into_load(|_: plan::Data| unreachable!("`Ready` is handled above")),
            };
        }
        self.seeded = Some(Rc::new(seed));
    }
}

/// The intent's change and the asks it makes (pure; `apply_intent` emits).
pub fn step(model: &mut Model, intent: Intent) -> Vec<Ask> {
    match intent {
        Intent::RefreshRateLimits => Vec::new(),
        // The reads are the app's (`app/reset.rs` runs these two before it gets here).
        Intent::Menu | Intent::Resets => Vec::new(),
        Intent::Redeem { credit_id } => {
            let next = model.reset.redeem(credit_id);
            model.redemption_asks(next)
        }
        Intent::Retry => {
            let next = model.reset.retry();
            model.redemption_asks(next)
        }
        Intent::Open | Intent::Refresh => model.open_session(),
        Intent::Leave => {
            model.live = false;
            model.histories.clear();
            model.profile = Load::Unavailable;
            model.plan = Load::Unavailable;
            model.chats = Load::Unavailable;
            model.tasks = Load::Unavailable;
            model.account = Load::Unavailable;
            Vec::new()
        }
        Intent::Load(key) => model.load_report(key).into_iter().collect(),
    }
}

/// The answer's change and the asks it makes (pure; `apply` emits). A reply
/// of another session or stamp changes nothing.
pub fn settle(model: &mut Model, ev: Ev) -> Vec<Ask> {
    match ev {
        Ev::Seed(seed) => {
            model.seed(*seed);
            Vec::new()
        }
        Ev::Redeemed { key, result } => {
            // The app reads the credits again after a reset (`app/reset.rs` takes this first).
            model.reset.redeemed(&key, result);
            Vec::new()
        }
        Ev::Invalidated { session } if session == model.session && model.live => {
            model.open_session()
        }
        Ev::Invalidated { .. } => Vec::new(),
        Ev::Account {
            session,
            stamp,
            result,
        } if session == model.session => {
            if model.account.stamp() != Some(stamp) {
                return Vec::new();
            }
            match result {
                Ok(reply) => model.settle_account(reply),
                Err(message) => model.account = Load::Error(message),
            }
            let asks = model.start_reports();
            model.after_settle();
            asks
        }
        Ev::Report {
            session,
            stamp,
            section,
            result,
        } if session == model.session => {
            let _ = section;
            let Some(key) = model
                .histories
                .iter()
                .find(|(_, load)| load.stamp() == Some(stamp))
                .map(|(key, _)| key.clone())
            else {
                return Vec::new();
            };
            let result = result.map(|reply| {
                if let Some(models) = reply.token_models {
                    model.token_models = models;
                }
                if let Some(attributed) = reply.attributed_usage {
                    model.attributed_usage = attributed;
                }
                reply.history.map(Rc::new)
            });
            if let Some(load) = model.histories.get_mut(&key) {
                load.settle(stamp, result);
            }
            model.after_settle();
            Vec::new()
        }
        Ev::Profile {
            session,
            stamp,
            result,
        } if session == model.session => {
            model
                .profile
                .settle(stamp, result.map(|profile| profile.map(|profile| Rc::new(*profile))));
            Vec::new()
        }
        Ev::Plan {
            session,
            stamp,
            result,
        } if session == model.session => {
            let result = result.and_then(|data| match data {
                Some(data) => plan::Report::parse(data).map_err(LoadError::from),
                None => Ok(None),
            });
            model.plan.settle(stamp, result.map(|report| report.map(Rc::new)));
            Vec::new()
        }
        Ev::Chats {
            session,
            stamp,
            result,
        } if session == model.session => {
            model
                .chats
                .settle(stamp, result.map(|list| list.map(Rc::new)));
            Vec::new()
        }
        Ev::Tasks {
            session,
            stamp,
            result,
        } if session == model.session => {
            model
                .tasks
                .settle(stamp, result.map(|list| list.map(Rc::new)));
            Vec::new()
        }
        Ev::Account { .. }
        | Ev::Report { .. }
        | Ev::Profile { .. }
        | Ev::Plan { .. }
        | Ev::Chats { .. }
        | Ev::Tasks { .. } => Vec::new(),
    }
}

/// Sends the asks to the host as `Command::Usage { ask }`, answered with the
/// matching `Ev` under the ask's session and stamp.
fn emit(fx: &mut AppFx, asks: Vec<Ask>) {
    fx.commands
        .extend(asks.into_iter().map(|ask| crate::app::Command::Usage { ask }));
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let asks = settle(model, ev);
    emit(fx, asks);
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let asks = step(model, intent);
    emit(fx, asks);
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

/// A script's `screen` step for this directory: the seed's serde form.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    serde_json::from_value::<Seed>(value)
        .map(|seed| Ev::Seed(Box::new(seed)))
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))
}
