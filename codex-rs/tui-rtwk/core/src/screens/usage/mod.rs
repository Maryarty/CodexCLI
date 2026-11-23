//! `usage` (S20, the `/usage` dashboard): the core half of the screen's
//! directory. The model, its events and intents and the asks the host runs
//! live here with no rtwk; the view half is `src/pages/usage/` in the UI
//! crate. Oracle: `codex-rs/tui/src/analytics*` (the pure logic of its files is
//! carried into the modules below, each naming its file).
//!
//! The model holds what chooses the requests: the account, the selected
//! section, the range per range group, the group per section, the model
//! filter and the loads. The cursor, the detail, the scroll position and the
//! modes (dashboard, help) are the page's widget state (R14). Every load is
//! started with a `Stamp`; its reply settles only the load of that stamp, so a
//! refresh, a range change or a close drops what is still in flight (R1).

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
pub mod report;
pub mod report_data;
pub mod sections;
pub mod tokens;

pub use activity::TokenActivityView;
pub use data::{GroupName, Load, LoadError, Note, Stamp};
pub use date::NaiveDate;
pub use models::{AccountAnalyticsHistory as History, AccountKind};
pub use sections::{Control, RangeGroup, Section};

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
    pub section: Section,
    /// The range per `RangeGroup`: 0 is 7 days, 1 is 30.
    pub ranges: [usize; 3],
    /// The group (an index of `data::GROUPINGS`) per section.
    pub groups: [usize; Section::COUNT],
    pub token_model: Option<String>,
    /// The models the token report knows, for the `m` control.
    pub token_models: Vec<String>,
    pub credit_groups: Vec<usize>,
    pub attributed_usage: bool,
    pub chat_metric: usize,
    pub histories: [Load<Rc<History>>; Section::COUNT],
    /// The Plan section's allowance history (oracle: `plan::State::report`).
    pub plan: Load<Rc<plan::Report>>,
    /// The Chats section's listing of a business account (oracle: `chats`).
    pub chats: Load<Rc<chats::ChatList>>,
    /// The Chats section's listing of a consumer account (oracle: `tasks`).
    pub tasks: Load<Rc<chats::TaskList>>,
    pub profile: Load<Rc<profile::Profile>>,
    reports_started: bool,
}

impl Default for Model {
    fn default() -> Self {
        let mut groups = [0; Section::COUNT];
        groups[Section::Usage as usize] = 1;
        groups[Section::Plan as usize] = 1;
        groups[Section::Activity as usize] = 2;
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
            section: Section::Summary,
            ranges: [0; 3],
            groups,
            token_model: None,
            token_models: Vec::new(),
            credit_groups: vec![0],
            attributed_usage: true,
            chat_metric: 0,
            histories: std::array::from_fn(|_| Load::Unavailable),
            plan: Load::Unavailable,
            chats: Load::Unavailable,
            tasks: Load::Unavailable,
            profile: Load::Unavailable,
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
    /// The page opened: a fresh session (oracle: `open`), on the Overview's
    /// `view` when `/usage daily|weekly|cumulative` asked for one.
    Open { view: Option<TokenActivityView> },
    /// `R`: every report again (oracle: `refresh`).
    Refresh,
    /// The page was left: pending loads are dropped (oracle: `cancel_loads`).
    Leave,
    /// A tab (digits, Tab, a click).
    Select(Section),
    /// `r`: 7 or 30 days for the selected section's range group.
    Range,
    /// `g`: the next grouping of the selected section.
    Group,
    /// `m`: the next model of the token report.
    Model,
    /// `s`: the next chat sort metric of the `count` the page offers.
    TaskMetric,
}

impl Model {
    fn next_stamp(&mut self) -> Stamp {
        self.next_stamp += 1;
        Stamp(self.next_stamp)
    }

    fn ask(&self, stamp: Stamp, what: AskKind) -> Ask {
        Ask {
            session: self.session,
            stamp,
            what,
        }
    }

    /// A fresh session: drops every account-owned load and asks for the
    /// account (oracle: `refresh`). `view` picks the Overview's chart.
    fn open_session(&mut self, view: Option<TokenActivityView>) -> Vec<Ask> {
        self.session += 1;
        self.live = true;
        self.histories = std::array::from_fn(|_| Load::Unavailable);
        self.profile = Load::Unavailable;
        self.plan = Load::Unavailable;
        self.chats = Load::Unavailable;
        self.tasks = Load::Unavailable;
        self.reports_started = false;
        self.token_model = None;
        self.token_models.clear();
        if let Some(view) = view {
            self.section = Section::Summary;
            self.groups[Section::Summary as usize] = view.index();
        }
        let stamp = self.next_stamp();
        self.account = Load::Loading(stamp);
        vec![self.ask(stamp, AskKind::Account)]
    }

    /// Once the account is known, every visible report starts, once
    /// (oracle: `start_reports`).
    fn start_reports(&mut self) -> Vec<Ask> {
        if self.reports_started || self.account.ready().is_none() {
            return Vec::new();
        }
        self.reports_started = true;
        let visible = self.visible_sections();
        let Some(first) = visible.first() else {
            return Vec::new();
        };
        if !visible.contains(&self.section) {
            self.section = *first;
        }
        let usage_groups: &[usize] = if self.business() {
            &[6, 2]
        } else {
            &[1, 2, 0, 3]
        };
        if !usage_groups.contains(&self.groups[Section::Usage as usize]) {
            self.groups[Section::Usage as usize] =
                if matches!(self.account_kind(), Some(AccountKind::Business)) {
                    2
                } else {
                    usage_groups[0]
                };
        }
        if visible.contains(&Section::Credits)
            && !self
                .credit_groups
                .contains(&self.groups[Section::Credits as usize])
        {
            self.groups[Section::Credits as usize] = self.credit_groups[0];
        }
        let mut asks = vec![self.load_profile()];
        for section in visible {
            asks.extend(self.load_report(*section));
        }
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

    /// Starts the section's report again (oracle: `load_report`); the sections
    /// without a report of their own start nothing.
    fn load_report(&mut self, section: Section) -> Option<Ask> {
        section.report()?;
        let stamp = self.next_stamp();
        self.histories[section as usize] = Load::Loading(stamp);
        let model = (section == Section::Usage)
            .then(|| self.token_model.clone())
            .flatten();
        Some(self.ask(
            stamp,
            AskKind::Report {
                section,
                days: self.days_of(section),
                group: self.groups[section as usize],
                model,
            },
        ))
    }

    /// What every settled load checks (oracle: `poll_reports`): a consumer
    /// account whose usage is not attributed has no per-surface groups.
    fn after_settle(&mut self) {
        if !self.business()
            && !self.consumer_attribution()
            && ![0, 2].contains(&self.groups[Section::Usage as usize])
        {
            self.groups[Section::Usage as usize] = 0;
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
        if let Some(account) = seed.account {
            self.settle_account(account);
        }
        if let Some(ranges) = seed.ranges {
            self.ranges = ranges;
        }
        for GroupSeed { section, group } in seed.groups {
            self.groups[section as usize] = group;
        }
        if let Some(attributed) = seed.attributed_usage {
            self.attributed_usage = attributed;
        }
        self.token_models = seed.token_models;
        self.token_model = seed.token_model;
        for HistorySeed { section, load } in seed.histories {
            self.histories[section as usize] = load.into_load(Rc::new);
        }
        if let Some(profile) = seed.profile {
            self.profile = profile.into_load(Rc::new);
        }
        if let Some(chats) = seed.chats {
            self.chats = chats.into_load(Rc::new);
        }
        if let Some(tasks) = seed.tasks {
            self.tasks = tasks.into_load(Rc::new);
        }
        if let Some(history) = seed.plan {
            self.plan = match history {
                SeedLoad::Ready(data) => match plan::Report::parse(data) {
                    Ok(Some(report)) => Load::Ready(Rc::new(report)),
                    Ok(None) => Load::Unavailable,
                    Err(error) => Load::Error(error.into()),
                },
                other => other.into_load(|_: plan::Data| unreachable!("`Ready` is handled above")),
            };
        }
        let visible = self.visible_sections();
        self.section = seed
            .section
            .filter(|section| visible.contains(section))
            .or_else(|| visible.first().copied())
            .unwrap_or(Section::Summary);
    }
}

/// The intent's change and the asks it makes (pure; `apply_intent` emits).
pub fn step(model: &mut Model, intent: Intent) -> Vec<Ask> {
    match intent {
        Intent::RefreshRateLimits => Vec::new(),
        Intent::Open { view } => model.open_session(view),
        Intent::Refresh => model.open_session(None),
        Intent::Leave => {
            model.live = false;
            model.histories = std::array::from_fn(|_| Load::Unavailable);
            model.profile = Load::Unavailable;
            model.plan = Load::Unavailable;
            model.chats = Load::Unavailable;
            model.tasks = Load::Unavailable;
            model.account = Load::Unavailable;
            Vec::new()
        }
        Intent::Select(section) => {
            if model.visible_sections().contains(&section) {
                model.section = section;
            }
            Vec::new()
        }
        Intent::Range => {
            if !model.control_available(Control::Range, false) {
                return Vec::new();
            }
            let group = model.range_group(model.section);
            model.ranges[group as usize] ^= 1;
            model
                .visible_sections()
                .iter()
                .filter(|section| model.range_group(**section) == group)
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .filter_map(|section| model.load_report(section))
                .collect()
        }
        Intent::Group => {
            if !model.control_available(Control::Group, false) {
                return Vec::new();
            }
            let options = model.group_options();
            let current = model.groups[model.section as usize];
            let next = options
                .iter()
                .position(|group| *group == current)
                .map_or(0, |index| (index + 1) % options.len());
            model.groups[model.section as usize] = options[next];
            model.load_report(model.section).into_iter().collect()
        }
        Intent::Model => {
            if !model.control_available(Control::Model, false) {
                return Vec::new();
            }
            let models = &model.token_models;
            model.token_model = match model
                .token_model
                .as_ref()
                .and_then(|current| models.iter().position(|candidate| candidate == current))
            {
                Some(index) => models.get(index + 1).cloned(),
                None => models.first().cloned(),
            };
            model.load_report(Section::Usage).into_iter().collect()
        }
        Intent::TaskMetric => {
            if model.control_available(Control::TaskMetric, false) {
                let metrics = model.task_metrics();
                let at = metrics
                    .iter()
                    .position(|metric| *metric == model.task_metric())
                    .unwrap_or_default();
                model.chat_metric = metrics[(at + 1) % metrics.len()].index();
            }
            Vec::new()
        }
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
        Ev::Invalidated { session } if session == model.session && model.live => {
            model.open_session(None)
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
            if model.histories[section as usize].stamp() != Some(stamp) {
                return Vec::new();
            }
            let result = result.map(|reply| {
                if let Some(models) = reply.token_models {
                    model.token_models = models;
                }
                if let Some(attributed) = reply.attributed_usage {
                    model.attributed_usage = attributed;
                }
                reply.history.map(Rc::new)
            });
            model.histories[section as usize].settle(stamp, result);
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
