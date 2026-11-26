//! `chat` (S10, main chat screen): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

mod banner;
pub mod facts;
mod mcp;
pub mod plan_mode;
pub mod reasoning;

use std::rc::Rc;

pub use banner::{
    AutomaticSwitch, Banner, BannerAction, BannerCta, BannerOccurrence, Nudge, NudgeOutcome,
    Reserve, SwitchReason,
};
pub use facts::{Ev as FactEv, Facts, Footer, Probe};
pub use mcp::{McpHeader, McpStartup, McpStatus, McpWarning};

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::commands::CommandFlags;
use crate::commands::ServiceTierCommand;
use crate::commands::SessionIntent;
use crate::commands::SlashCommand;
use crate::ids::{ThreadId, TurnId};
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::screens::select::collaboration::{self, ModelChange};
use crate::threads::ThreadReply;
use crate::threads::asks::ThreadRpc;
use crate::threads::{Note, ThreadFx, UserMessage};

pub const NAME: &str = "chat";
pub const ID: &str = "S10";
pub const KIND: Kind = Kind::Home;

/// The app-scoped model state of this screen, changed only by `apply`: what
/// the composer's commands are gated by, and the last thing a command said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    /// The feature gates of the `/` popup and of dispatch.
    pub flags: CommandFlags,
    /// The model service tiers offered after `/model` (`/fast`).
    pub service_tiers: Vec<ServiceTierCommand>,
    /// Signed in with a ChatGPT backend (`/usage`). True until session-core
    /// carries the account; a script seeds the signed-out case.
    pub signed_in: bool,
    /// What commands answered before any thread existed: shown above the
    /// composer, where a thread's notes would be in its transcript.
    pub notes: Vec<Note>,
    /// `/raw`: the transcript is shown as text for terminal selection
    /// (`raw_output_mode`). The rendering reads it.
    pub raw_output: bool,
    /// What `/title` and `/statusline` set for this session: it outlives the host's facts, which
    /// replace the status whole.
    pub status_override: StatusOverride,
    /// What the status line shows, read by the host (config, cwd, git).
    pub status: Rc<StatusFacts>,
    /// The footer's and the status line's other facts: the workspace probes, the side and
    /// agent labels (`facts`).
    pub facts: Facts,
}

/// The oracle's default list (`DEFAULT_STATUS_LINE_ITEMS`) when the config
/// names none.
pub const DEFAULT_STATUS_LINE_ITEMS: [&str; 3] =
    ["model-with-reasoning", "current-dir", "thread-name"];

/// `LUNA_RESERVE_MODEL` (`model_catalog.rs`).
const LUNA_RESERVE_MODEL: &str = "gpt-reserve";

/// The display name supplied by the configuration for a model provider.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ModelProviderName {
    pub id: String,
    pub name: String,
}

/// The facts of the status line that need no session: the loaded config
/// (model, reasoning effort, approval mode, `tui.status_line`,
/// `tui.status_line_use_colors`) and what the host reads of the workspace
/// (cwd, project root, git branch). The host sends them as one event
/// (`Ev::Status`); the texts are the oracle's
/// (`status_line_value_for_item`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct StatusFacts {
    pub model: Option<String>,
    /// The reasoning effort's name (`high`); none or `none` reads `default`.
    pub effort: Option<String>,
    /// The approval policy as the config writes it (`on-request`).
    pub approval_policy: Option<String>,
    /// A granular approval policy as the config's JSON (`/presets` recalls it).
    pub approval_granular: Option<String>,
    /// The active permission profile's id (`:workspace`, a named profile's name).
    pub permission_profile: Option<String>,
    /// The approvals reviewer is the auto reviewer.
    pub auto_review: bool,
    pub profile: super::status::Profile,
    /// The working directory as shown (`~/dev/app`).
    pub cwd: Option<String>,
    pub home_directory: Option<String>,
    pub service_tier: Option<String>,
    /// `Feature::FastMode`: the tiers other than flex and ultrafast need it.
    pub fast_mode: bool,
    /// `Feature::UltrafastMode`: the `ultrafast` tier needs it.
    pub ultrafast_mode: bool,
    /// `Feature::Collab`: whether new subagents may be started.
    pub collab_enabled: bool,
    /// The name of the project root (the git root's directory).
    pub project_root: Option<String>,
    pub branch: Option<String>,
    /// `tui.status_line`: item ids in the order shown; none is the default.
    pub items: Option<Vec<String>>,
    /// `tui.status_line_use_colors`.
    pub colors: bool,
    pub terminal_title: Option<Vec<String>>,
    pub hostname: Option<String>,
    pub profile_name: Option<String>,
    pub provider: Option<String>,
    /// Configured display names for the providers a thread refusal can name.
    pub provider_names: Vec<ModelProviderName>,
    pub version: Option<String>,
    pub animations: bool,
    pub progress_effect: bool,
    pub shimmer_effect: bool,
    /// `tui.effects.starfield`: the composer sparkle (oracle `settings.effects.starfield`).
    pub starfield_effect: bool,
    pub account_usage: bool,
    /// What a `/status` card reads of the loaded config.
    pub card: super::status::CardFacts,
    pub empty_state: bool,
    pub notification_policy: NotificationPolicy,
    /// What the config says of `/permissions` (guardian, requirements, limits).
    pub permissions: super::select::permissions::Facts,
    /// `multi_agent_v2.max_concurrent_threads_per_session`: an Ultra pick warns from 8 on.
    pub agent_threads: Option<usize>,
}

impl Default for StatusFacts {
    fn default() -> Self {
        Self {
            model: None,
            effort: None,
            approval_policy: None,
            approval_granular: None,
            permission_profile: None,
            auto_review: false,
            profile: super::status::Profile::Workspace,
            cwd: None,
            home_directory: None,
            service_tier: None,
            fast_mode: true,
            ultrafast_mode: true,
            collab_enabled: true,
            project_root: None,
            branch: None,
            items: None,
            colors: true,
            terminal_title: None,
            hostname: None,
            profile_name: None,
            provider: None,
            provider_names: Default::default(),
            version: None,
            animations: true,
            progress_effect: true,
            shimmer_effect: true,
            starfield_effect: true,
            account_usage: false,
            card: super::status::CardFacts::default(),
            empty_state: false,
            notification_policy: NotificationPolicy::default(),
            permissions: Default::default(),
            agent_threads: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct NotificationPolicy {
    pub enabled: bool,
    pub always: bool,
    pub kinds: Option<Vec<String>>,
}

impl NotificationPolicy {
    pub fn allows(&self, kind: &str) -> bool {
        self.enabled
            && self
                .kinds
                .as_ref()
                .is_none_or(|kinds| kinds.iter().any(|name| name == kind))
    }
}

/// A desktop notification emitted once for an accepted live event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopNotice {
    pub kind: NoticeKind,
    pub preview: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    TurnComplete,
    CommandApproval,
    EditApproval,
    Elicitation,
    Question,
}

impl NoticeKind {
    pub fn config_id(self) -> &'static str {
        match self {
            Self::TurnComplete => "agent-turn-complete",
            Self::CommandApproval | Self::EditApproval | Self::Elicitation => "approval-requested",
            Self::Question => "async-question",
        }
    }
}

impl StatusFacts {
    /// Configured status items in their original order.
    pub fn items(&self) -> Vec<(String, String)> {
        match &self.items {
            Some(items) => items.iter().map(String::as_str).collect::<Vec<_>>(),
            None => DEFAULT_STATUS_LINE_ITEMS.to_vec(),
        }
        .into_iter()
        .filter_map(|id| self.value(id).map(|text| (id.to_string(), text)))
        .collect()
    }

    pub fn model_name(&self) -> Option<&str> {
        let model = self.model.as_deref()?;
        Some(if model.eq_ignore_ascii_case(LUNA_RESERVE_MODEL) {
            "Luna Reserve"
        } else {
            model
        })
    }

    /// `status_line_reasoning_effort_label`.
    pub fn effort_label(&self) -> &str {
        match self.effort.as_deref() {
            None | Some("none") => "default",
            Some(effort) => effort,
        }
    }

    /// `approval_mode_display`.
    fn approval(&self) -> Option<String> {
        let policy = self.approval_policy.as_deref()?;
        Some(match (policy, self.auto_review) {
            ("on-request", true) => "Approve for me".to_string(),
            ("on-request", false) => "Ask for approval".to_string(),
            (other, _) => other.to_string(),
        })
    }

    /// `status_line_value_for_item` for the items that need no session; the
    /// ids are `StatusLineItem`'s names and their serialize aliases.
    pub fn value(&self, id: &str) -> Option<String> {
        match id {
            "model" | "model-name" => self.model_name().map(ToString::to_string),
            "model-with-reasoning" => self
                .model_name()
                .map(|model| format!("{model} {}", self.effort_label())),
            "reasoning" => Some(self.effort_label().to_string()),
            "current-dir" => self.cwd.clone(),
            "project-name" | "project" | "project-root" => self.project_root.clone(),
            "git-branch" => self.branch.clone(),
            "approval-mode" | "approval" => self.approval(),
            _ => None,
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
            flags: CommandFlags {
                collaboration_modes_enabled: true,
                connectors_enabled: true,
                plugins_command_enabled: true,
                token_activity_command_enabled: true,
                service_tier_commands_enabled: true,
                goal_command_enabled: true,
                ..CommandFlags::default()
            },
            service_tiers: Vec::new(),
            signed_in: true,
            notes: Vec::new(),
            raw_output: false,
            status_override: StatusOverride::default(),
            status: Rc::default(),
            facts: Facts::default(),
        }
    }
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`. Only scripts make them yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    Flags(CommandFlags),
    ServiceTiers(Vec<ServiceTierCommand>),
    SignedIn(bool),
    /// The host read the config and the workspace.
    Status(Box<StatusFacts>),
    /// An app-wide fact of the footer or the status line (`facts`).
    Facts(FactEv),
    /// The details under `thread`'s status row, set as a fixture does (the
    /// app routes it to the thread, which owns the fact).
    Details {
        thread: String,
        details: Option<StatusDetails>,
    },
    /// What `thread` holds of the user's input during a turn, set as a fixture
    /// does (the app routes it to the thread, which owns the lists).
    Pending {
        thread: String,
        pending: PendingTexts,
    },
    /// `thread` is refused for its model provider, set as a fixture does (the
    /// app routes it to the thread, which owns the fact).
    ProviderError {
        thread: String,
        error: codex_app_server_protocol::ModelProviderError,
    },
}

/// The three lists the pending-input preview shows, as texts.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct PendingTexts {
    pub steers: Vec<String>,
    pub rejected: Vec<String>,
    pub queued: Vec<String>,
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A session command addressed to the thread shown when it was submitted.
    Session {
        thread: Option<ThreadId>,
        command: SlashCommand,
        intent: SessionIntent,
    },
    /// A user shell command (`!cmd`) for the thread shown when it was submitted.
    Shell {
        thread: Option<ThreadId>,
        command: String,
    },
    /// A verb of the thread's safety state: the retry of a buffered turn, a rewrite after a
    /// refusal, the continuation after a misalignment stop, the receipt of a request
    /// (`threads/safety.rs`; `threads::apply` is the writer, the screen keeps nothing of it).
    Safety {
        thread: ThreadId,
        verb: crate::threads::safety::Verb,
    },
    /// A command line was answered (refused, or its words): a note of the
    /// thread shown when it was submitted, else of the screen.
    Note {
        thread: Option<ThreadId>,
        note: Note,
    },
    /// `/raw`: the render mode of the transcript (`set_raw_output_mode`).
    RawOutput(bool),
    /// `/title` and `/statusline` with items: session-only, nothing is written to the config.
    StatusOverride(StatusOverride),
}

/// The status surfaces a command set for this session (`StatusSurfaceOverrides`): what is none
/// is the config's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusOverride {
    pub terminal_title: Option<Vec<String>>,
    pub status_line: Option<Vec<String>>,
    pub status_line_colors: Option<bool>,
}

impl StatusOverride {
    /// What `later` changes on top of this.
    fn merge(&mut self, later: StatusOverride) {
        self.terminal_title = later.terminal_title.or(self.terminal_title.take());
        self.status_line = later.status_line.or(self.status_line.take());
        self.status_line_colors = later.status_line_colors.or(self.status_line_colors);
    }

    /// The facts with the overrides on them.
    fn on(&self, facts: &mut StatusFacts) {
        if let Some(items) = &self.terminal_title {
            facts.terminal_title = Some(items.clone());
        }
        if let Some(items) = &self.status_line {
            facts.items = Some(items.clone());
        }
        if let Some(colors) = self.status_line_colors {
            facts.colors = colors;
        }
    }
}

/// The app's clock moved (`ScreenStates::set_now`): what is due by it is asked for, as after any
/// other change of the facts.
pub(crate) fn set_now(model: &mut Model, now: u64) {
    model.facts.set_now(now);
    model.facts.sync(&model.status, model.signed_in);
}

/// When the chat wants the clock told (`AppFx::wake`): the workspace headline's next read, only
/// while a read would be made then (a ChatGPT sign-in; the facts say none while the item is off,
/// the backend lacks the feature or a read is in flight).
pub fn wake_at(model: &Model) -> Option<crate::app::Now> {
    if !model.signed_in {
        return None;
    }
    let due = model.facts.headline_due_at()?;
    Some(crate::app::Now::at_secs(
        i64::try_from(due).unwrap_or(i64::MAX),
    ))
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Flags(flags) => model.flags = flags,
        Ev::ServiceTiers(tiers) => model.service_tiers = tiers,
        Ev::SignedIn(on) => model.signed_in = on,
        Ev::Status(mut facts) => {
            model.status_override.on(&mut facts);
            model.status = Rc::from(facts);
        }
        Ev::Facts(ev) => model.facts.apply(ev),
        // the thread owns it (`apply_app` routes it)
        Ev::Details { .. } | Ev::Pending { .. } | Ev::ProviderError { .. } => {}
    }
    // what the configured items need is asked for after every change that may alter it
    model.facts.sync(&model.status, model.signed_in);
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        // the app routes these to the thread (`apply_app`); the screen keeps nothing
        Intent::Session { .. } | Intent::Shell { .. } | Intent::Safety { .. } => {}
        Intent::Note { thread: None, note } => model.notes.push(note),
        Intent::RawOutput(enabled) => model.raw_output = enabled,
        Intent::StatusOverride(set) => {
            model.status_override.merge(set);
            model.status_override.on(Rc::make_mut(&mut model.status));
        }
        // a thread's note is an entry of its transcript (`session::note`)
        Intent::Note { .. } => {}
    }
}

/// Per-thread state and intents for the chat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    compact_pending: bool,
    pub error: Option<RpcError>,
    /// Why the thread is locked: the server refused it for its model provider.
    /// Set until the thread is attached again.
    pub provider_error: Option<codex_app_server_protocol::ModelProviderError>,
    pub name: Option<String>,
    /// The thread this one was forked from (`/status` shows it).
    pub forked_from: Option<String>,
    pub cwd: Option<String>,
    /// The file the thread's history is written to (`/rollout`), once the
    /// server has named it.
    pub path: Option<String>,
    /// The thread is controlled by its parent: it takes no direct input
    /// (`thread_blocks_direct_input`).
    pub parent_owned: bool,
    /// The thread is not kept (`config.ephemeral`).
    pub ephemeral: bool,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub provider: Option<String>,
    pub service_tier: Option<Option<String>>,
    pub daybreak: Option<bool>,
    /// The collaboration mode (`thread/settings/updated`, or the user's switch); read through `mode()`.
    mode: codex_protocol::config_types::ModeKind,
    /// What the thread runs with outside of a mask: its model and effort as last seen in Default
    /// mode (`current_collaboration_mode`); the mask of a switch lies over it.
    base: collaboration::Base,
    /// Plan mode: the proposed plan and the question asked when the turn ends.
    pub plan: plan_mode::PlanMode,
    /// Resumed with history: the token usage is pending until `status` counts one.
    awaiting_usage: bool,
    pub activity: ActivityPhase,
    live_compaction: bool,
    pub progress: Option<(usize, usize)>,
    pub step: Option<String>,
    /// The lines under the status row's header; read through `details()`.
    details: Option<StatusDetails>,
    /// MCP startup of this thread (oracle `mcp_startup_status`).
    pub mcp: McpStartup,
    /// A retrying error's header, until the thread's next notification
    /// (oracle `retry_status_header`).
    retry_header: Option<Retry>,
    /// A code review runs in this thread (oracle `review.is_review_mode`).
    review_mode: bool,
    /// A new primary thread that has run nothing yet (oracle `mark_fresh_task_for_sparkle`: no
    /// turns, not a fork, not controlled by a parent); the first turn ends it.
    fresh: bool,
    /// A turn started in this thread.
    ran_turn: bool,
    /// Turns the goal's budget stopped, until their interrupted completion
    /// is seen (oracle `budget_limited_turn_ids`).
    budget_limited: Vec<TurnId>,
    /// The streamed deltas of the reasoning item in flight, which the status row's header reads
    /// besides the transcript (`reasoning_header`).
    live_reasoning: reasoning::Live,
}

/// The default of `STATUS_DETAILS_DEFAULT_MAX_LINES`.
pub const STATUS_DETAILS_MAX_LINES: usize = 3;

/// The lines under the status row's header (oracle `StatusIndicatorState`):
/// the text trimmed at the start and capitalized unless the source says its
/// case is its own, the lines it may take at most.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusDetails {
    pub text: String,
    pub max_lines: usize,
}

impl StatusDetails {
    /// Capitalizes the first letter (oracle `StatusDetailsCapitalization::CapitalizeFirst`).
    pub fn new(text: String, max_lines: usize) -> Self {
        let trimmed = text.trim_start();
        let mut chars = trimmed.chars();
        let text = match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        };
        Self {
            text,
            max_lines: max_lines.max(1),
        }
    }

    /// Keeps the case as given (`Preserve`).
    pub fn preserved(text: &str, max_lines: usize) -> Self {
        Self {
            text: text.trim_start().to_string(),
            max_lines: max_lines.max(1),
        }
    }
}

/// What owns the status row's header besides the activity phase; the UI's
/// string tables hold the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusHeader {
    Retry(Retry),
    Mcp(McpHeader),
}

/// A retrying error (oracle `on_error` with `will_retry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Retry {
    /// The server's own words (`Reconnecting... 2/5`).
    Server(String),
    /// A 429 reconnect: the attempt (`2/5`) of a busy server.
    Busy { attempt: String },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivityPhase {
    #[default]
    Working,
    Thinking,
}

impl ThreadPart {
    pub(crate) fn begin_turn(&mut self) {
        self.fresh = false;
        self.ran_turn = true;
        self.details = None;
        self.progress = None;
        self.activity = ActivityPhase::Working;
        // the turn the compact request asked for has started: it holds the thread busy now
        self.compact_pending = false;
        self.live_compaction = false;
        self.step = None;
        self.live_reasoning.clear();
    }

    pub(crate) fn end_turn(&mut self) {
        self.details = None;
        self.live_reasoning.clear();
        // a compaction the server never turned into a turn must not hold the thread for good
        self.compact_pending = false;
        self.live_compaction = false;
        self.retry_header = None;
    }

    /// The status row's header when something other than the phase owns it:
    /// a retry, else MCP startup (oracle `set_status_header`).
    pub fn header(&self) -> Option<StatusHeader> {
        self.retry_header
            .clone()
            .map(StatusHeader::Retry)
            .or_else(|| self.mcp.header().map(StatusHeader::Mcp))
    }

    /// The status row's header the model's reasoning gives the running `turn` (oracle
    /// `reasoning_header`): its latest usable summary line, none before there is one.
    /// `show_raw` is `show_raw_agent_reasoning`: the raw reasoning counts too.
    pub fn reasoning_header(
        &self,
        transcript: &crate::threads::Transcript,
        turn: &TurnId,
        show_raw: bool,
    ) -> Option<String> {
        reasoning::header(transcript, turn, &self.live_reasoning, show_raw)
    }

    /// The lines under the status row's header, if the thread sets any.
    pub fn details(&self) -> Option<&StatusDetails> {
        self.details.as_ref()
    }

    pub(crate) fn mark_fresh(&mut self, fresh: bool) {
        self.fresh = fresh;
    }

    /// A new primary thread that has run nothing yet.
    pub fn fresh(&self) -> bool {
        self.fresh
    }

    /// The composer sparkle may be offered here (oracle `mark_fresh_task_for_sparkle`,
    /// `stop_ineligible_sparkle`): a fresh thread of an Astra model with animations and the
    /// starfield effect on. `facts` is the config's model when the thread names none.
    pub fn sparkle_eligible(&self, facts: &StatusFacts) -> bool {
        self.fresh
            && facts.animations
            && facts.starfield_effect
            && self
                .model
                .as_deref()
                .or(facts.model.as_deref())
                .is_some_and(is_sparkle_model)
    }

    /// A code review runs in this thread (oracle `review.is_review_mode`).
    pub fn review_mode(&self) -> bool {
        self.review_mode
    }

    pub(crate) fn set_review_mode(&mut self, on: bool) {
        self.review_mode = on;
    }

    /// Any notification of the thread but a retrying error ends the retry header.
    /// A budget-limited goal stopped `turn` (`on_thread_goal_updated`).
    pub(crate) fn mark_budget_limited(&mut self, turn: TurnId) {
        if !self.budget_limited.contains(&turn) {
            self.budget_limited.push(turn);
        }
    }

    /// The turn's interruption is the budget's (`take_budget_limited`): true once.
    pub(crate) fn take_budget_limited(&mut self, turn: &TurnId) -> bool {
        let before = self.budget_limited.len();
        self.budget_limited.retain(|marked| marked != turn);
        self.budget_limited.len() != before
    }

    pub(crate) fn clear_retry(&mut self) {
        self.retry_header = None;
    }

    /// The user's switch of the mode (`set_collaboration_mask_from_user_action`): the mask lies
    /// over the thread's base settings, the thread goes on in what that makes at once (the next
    /// turn is sent in it) and the server's settings follow. What the switch changed of the
    /// model and the effort comes back for the line that says so.
    pub(crate) fn set_mask(
        &mut self,
        mask: &collaboration::Mask,
        fx: &mut ThreadFx<'_>,
    ) -> Option<ModelChange> {
        // a thread resumed in Plan mode has not been seen in Default: what it runs with now stands in
        let base = if self.base.model.is_empty() {
            collaboration::Base::of(self.model.as_deref(), self.effort.as_deref())
        } else {
            self.base.clone()
        };
        let effective = collaboration::effective(&base, mask, None);
        let previous = (self.mode, self.model.clone(), self.effort.clone());
        self.mode = effective.mode;
        self.error = None;
        let named = Some(effective.settings.model.clone());
        // a model nobody has named stays unknown (`None`), it is not an empty one: the thread
        // whose model is KNOWN empty sends nothing (`threads::startup::model_unavailable`)
        self.model = named
            .clone()
            .filter(|model| previous.1.is_some() || !model.is_empty());
        self.effort = effective
            .settings
            .reasoning_effort
            .as_ref()
            .map(|effort| effort.as_str().to_string());
        let changed = previous.0 != self.mode && (previous.1 != named || previous.2 != self.effort);
        let change = changed.then(|| ModelChange {
            model: effective.settings.model.clone(),
            effort: collaboration::effort_label(
                &effective.settings.model,
                effective.settings.reasoning_effort.as_ref(),
            ),
            mode: self.mode,
        });
        fx.request(Ctx::Mode, Command::Mode(effective));
        change
    }

    /// A compaction was asked for or runs: the thread is busy from the dispatch of `/compact`
    /// (`is_task_running`), though no turn runs yet.
    pub fn is_compacting(&self) -> bool {
        self.compact_pending || self.live_compaction
    }

    pub(crate) fn connection_lost(&mut self) {
        self.compact_pending = false;
        self.live_compaction = false;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadIntent {
    Compact,
    /// `/stop`: stop the thread's background terminals
    /// (`AppCommand::clean_background_terminals`).
    CleanTerminals,
    /// `/review <text>` and the review popup's pick: `review/start` inline on
    /// this thread (the oracle's `AppCommand::review`).
    Review(crate::threads::ReviewTarget),
    /// `!cmd` from the composer: a user shell command (the oracle's
    /// `AppCommand::RunUserShellCommand`), run while a turn runs too.
    Shell(String),
    /// Tab: the line waits behind the running turn (the oracle's
    /// `InputResult::Queued`); `threads::apply` owns the queue.
    Queue(crate::threads::Queued),
    /// The screen ran the queued command it was handed (`ThreadModel::due_command`): what the
    /// queue does next reads what it opened.
    QueueRan(crate::threads::queue::QueueRan),
    /// The picker issued for this operation was shown.
    QueueOpened(crate::threads::queue::QueueOperation),
    /// The picker for this operation closed, or its owner declined to open it.
    QueueClosed(crate::threads::queue::QueueOperation),
    /// What stopped the queue is over (the sheet closed): the next item goes on.
    QueueResume,
    /// Esc while steers are pending: interrupt the turn, then send the
    /// steers as one message (`submit_pending_steers_after_interrupt`).
    InterruptForSteers,
    /// The edit-queued key: the newest queued message goes back to the
    /// composer (`pop_latest_queued_composer_state`).
    EditQueued,
    /// Esc Esc found no prompt to edit: the chat says so
    /// (`NO_PREVIOUS_MESSAGE_TO_EDIT`, an info line in the history).
    NoPreviousMessage,
    /// A text for the composer of a fresh thread (the provider-error notice's
    /// continue key): given back as a restore, nothing is sent.
    Draft(String),
    /// The user's switch of the collaboration mode (`/plan`, the prompt's Yes): the thread goes
    /// on in the mode `Mask` makes and the server is told; the app chose the mask from the
    /// server's presets (`select::collaboration`). The switch is `threads::apply`'s, which
    /// says what it changed.
    SetMask(collaboration::Mask),
    /// Shift+Tab: the next preset in the server's list, only while the thread is idle
    /// (`cycle_collaboration_mode`); `threads::apply` chooses it.
    CycleMode,
    /// The fresh thread of the prompt's "clear context" row takes the approved plan: the server
    /// answers with the prompt that starts the work.
    ApprovePlan(String),
    /// The prompt's answer Yes (`submit_user_message_with_mode`): the thread goes on in Default
    /// and `Implement the plan.` is sent; a running turn refuses the switch.
    ImplementPlan,
    /// A settings pick that was to run before the queue went on failed (an inline
    /// `/permissions <profile>`): what waits in the queue and the pending steers stay unsent
    /// until the user sends again (`retain_input_after_failed_permission_selection`).
    HoldQueue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    Metadata {
        name: Option<String>,
        forked_from: Option<String>,
        cwd: String,
        path: Option<String>,
        parent_owned: bool,
        ephemeral: bool,
        model: Option<String>,
        effort: Option<String>,
        provider: String,
        daybreak: Option<bool>,
        /// The thread came with turns (a resume or a fork): its token usage is yet to come.
        resumed: bool,
        /// A new primary thread: no turns, not a fork, not controlled by a parent.
        fresh: bool,
    },
    Settings {
        cwd: String,
        model: String,
        effort: Option<String>,
        provider: String,
        service_tier: Option<String>,
        mode: codex_protocol::config_types::ModeKind,
    },
    /// The model's reasoning streams (`reasoning::Ev`).
    Reasoning(reasoning::Ev),
    Compaction {
        turn: TurnId,
        running: bool,
    },
    /// The thread's own facts of a context compaction (`threads/compaction.rs`): `threads::apply`
    /// takes it before the screen sees it, the screen keeps nothing of it.
    Compacting(crate::threads::compaction::CompactionEv),
    /// The thread's safety facts (`threads/safety.rs`), taken by `threads::apply` the same way.
    Safety(crate::threads::safety::SafetyEv),
    Name(Option<String>),
    Plan {
        turn: TurnId,
        completed: usize,
        total: usize,
        step: Option<String>,
    },
    /// The server retries the turn's request: the header says so, and the
    /// error's own details sit under it.
    Retry {
        turn: TurnId,
        retry: Retry,
        details: Option<String>,
    },
    /// What the status row says under its header (oracle `set_status`'s
    /// `details`); `None` clears it.
    Details(Option<StatusDetails>),
    /// The server refused this thread for its model provider (oracle
    /// `show_provider_error`): the thread takes no input and the notice says why.
    ProviderError(codex_app_server_protocol::ModelProviderError),
    /// The thread's goal reached its token budget during `turn`
    /// (`on_thread_goal_updated` with `BudgetLimited`).
    BudgetLimited {
        turn: TurnId,
    },
}

impl ThreadEv {
    pub(crate) fn turn(&self) -> Option<&TurnId> {
        match self {
            Self::Plan { turn, .. }
            | Self::Compaction { turn, .. }
            | Self::Retry { turn, .. }
            | Self::BudgetLimited { turn } => Some(turn),
            Self::Reasoning(ev) => Some(ev.turn()),
            _ => None,
        }
    }
}

/// The status line's view of a plan report (`threads` applies it with the
/// transcript entry the same report makes): done of total, the step running.
pub(crate) fn plan_progress(turn: &TurnId, update: &crate::threads::PlanUpdate) -> ThreadEv {
    ThreadEv::Plan {
        turn: turn.clone(),
        completed: update.completed(),
        total: update.steps.len(),
        step: update
            .steps
            .iter()
            .find(|step| step.status == crate::threads::StepStatus::InProgress)
            .map(|step| step.text.clone()),
    }
}

/// `thread_blocks_direct_input`: the server's word when it gives one, else a
/// spawned sub-agent's thread.
pub(crate) fn blocks_direct_input(thread: &codex_app_server_protocol::Thread) -> bool {
    use codex_app_server_protocol::SessionSource;
    use codex_protocol::protocol::SubAgentSource;
    thread.can_accept_direct_input.map_or_else(
        || {
            matches!(
                &thread.source,
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    agent_path: Some(_),
                    ..
                })
            )
        },
        |can_accept| !can_accept,
    )
}

/// The Astra models: any word of the name is `astra` (oracle `BottomPane::is_sparkle_model`).
pub fn is_sparkle_model(model: &str) -> bool {
    model
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .any(|part| part.eq_ignore_ascii_case("astra"))
}

pub(crate) fn metadata(thread: &codex_app_server_protocol::Thread) -> ThreadEv {
    ThreadEv::Metadata {
        name: thread.name.clone(),
        forked_from: thread.forked_from_id.clone(),
        cwd: thread.cwd.display().to_string(),
        path: thread.path.as_ref().map(|path| path.display().to_string()),
        parent_owned: blocks_direct_input(thread),
        ephemeral: thread.ephemeral,
        model: thread.model.clone(),
        effort: thread
            .reasoning_effort
            .as_ref()
            .map(|effort| effort.as_str().to_string()),
        provider: thread.model_provider.clone(),
        daybreak: thread.daybreak_enabled,
        resumed: !thread.turns.is_empty(),
        fresh: thread.turns.is_empty()
            && thread.forked_from_id.is_none()
            && !blocks_direct_input(thread),
    }
}

pub(crate) fn project(
    notification: &codex_app_server_protocol::ServerNotification,
) -> Option<(ThreadId, ThreadEv)> {
    use codex_app_server_protocol::{ServerNotification, ThreadItemPayload};
    let (id, event) = match notification {
        ServerNotification::ThreadStarted(note) => (&note.thread.id, metadata(&note.thread)),
        ServerNotification::ThreadNameUpdated(note) => {
            (&note.thread_id, ThreadEv::Name(note.thread_name.clone()))
        }
        ServerNotification::ThreadSettingsUpdated(note) => {
            let settings = &note.thread_settings;
            (
                &note.thread_id,
                ThreadEv::Settings {
                    cwd: settings.cwd.display().to_string(),
                    model: settings.model.clone(),
                    effort: settings
                        .effort
                        .as_ref()
                        .map(|effort| effort.as_str().to_string()),
                    provider: settings.model_provider.clone(),
                    service_tier: settings.service_tier.clone(),
                    mode: settings.collaboration_mode.mode,
                },
            )
        }
        ServerNotification::Error(note) if note.will_retry => {
            // `Reconnecting... 2/5` after a 429 reads as the server being busy.
            let busy = matches!(
                note.error.codex_error_info,
                Some(
                    codex_app_server_protocol::CodexErrorInfo::ResponseStreamDisconnected {
                        http_status_code: Some(429)
                    }
                )
            );
            let retry = match note.error.message.strip_prefix("Reconnecting... ") {
                Some(attempt) if busy => Retry::Busy {
                    attempt: attempt.to_string(),
                },
                _ => Retry::Server(note.error.message.clone()),
            };
            let details = note.error.additional_details.clone();
            (
                &note.thread_id,
                ThreadEv::Retry {
                    turn: TurnId::new(&note.turn_id),
                    retry,
                    details,
                },
            )
        }
        ServerNotification::ThreadGoalUpdated(note)
            if note.goal.status == codex_app_server_protocol::ThreadGoalStatus::BudgetLimited =>
        {
            let turn = TurnId::new(note.turn_id.as_deref()?);
            (&note.thread_id, ThreadEv::BudgetLimited { turn })
        }
        ServerNotification::ReasoningSummaryPartAdded(note) => (
            &note.thread_id,
            ThreadEv::Reasoning(reasoning::Ev::Part {
                turn: TurnId::new(&note.turn_id),
                item: crate::ids::ItemId::new(&note.item_id),
            }),
        ),
        ServerNotification::ReasoningSummaryTextDelta(note) => (
            &note.thread_id,
            ThreadEv::Reasoning(reasoning::Ev::Delta {
                turn: TurnId::new(&note.turn_id),
                item: crate::ids::ItemId::new(&note.item_id),
                kind: reasoning::Kind::Summary,
                delta: note.delta.clone(),
            }),
        ),
        ServerNotification::ReasoningTextDelta(note) => (
            &note.thread_id,
            ThreadEv::Reasoning(reasoning::Ev::Delta {
                turn: TurnId::new(&note.turn_id),
                item: crate::ids::ItemId::new(&note.item_id),
                kind: reasoning::Kind::Content,
                delta: note.delta.clone(),
            }),
        ),
        ServerNotification::ItemStarted(note)
            if matches!(
                note.item.payload,
                ThreadItemPayload::ContextCompaction { .. }
            ) =>
        {
            (
                &note.thread_id,
                ThreadEv::Compaction {
                    turn: TurnId::new(&note.turn_id),
                    running: true,
                },
            )
        }
        ServerNotification::ItemCompleted(note) => {
            let turn = TurnId::new(&note.turn_id);
            let event = match &note.item.payload {
                ThreadItemPayload::ContextCompaction { .. } => ThreadEv::Compaction {
                    turn,
                    running: false,
                },
                _ => return None,
            };
            (&note.thread_id, event)
        }
        _ => return None,
    };
    Some((ThreadId::new(id), event))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Compact,
    CleanTerminals,
    /// `thread/backgroundTerminals/list`: a page of the thread's live terminals.
    ListTerminals {
        cursor: Option<String>,
    },
    Review(crate::threads::ReviewTarget),
    Shell(String),
    /// `turn/steer`: the message joins the running turn `turn`.
    Steer {
        turn: TurnId,
        client_id: String,
        message: crate::threads::UserMessage,
    },
    /// `thread/settings/update` of the collaboration mode.
    Mode(codex_protocol::config_types::CollaborationMode),
    /// `thread/prepareApprovedPlan` with the plan's markdown.
    ApprovePlan(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Ack,
    /// The turn the steer joined.
    Steered(TurnId),
    /// The prompt that starts the work of an approved plan.
    Prepared(String),
    /// A page of the live background terminals, and the cursor of the next.
    Terminals {
        data: Vec<crate::threads::Listed>,
        next: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Mode,
    ApprovePlan,
    Compact,
    CleanTerminals,
    /// The restore of a resumed thread's terminals (`threads::apply` reads its pages): its id,
    /// the pages read, what was listed.
    ListTerminals {
        restore: u32,
        pages: u8,
        listed: Vec<crate::threads::Listed>,
    },
    Review,
    Shell,
    /// A steer in flight: `threads::apply` reads its reply (the queue and the
    /// previews are the thread's, not the slot's); `retried` once the turn id
    /// was corrected.
    Steer {
        turn: TurnId,
        retried: bool,
        client_id: String,
        message: crate::threads::UserMessage,
    },
}

pub fn apply_thread_intent(
    part: &mut ThreadPart,
    intent: ThreadIntent,
    busy: bool,
    fx: &mut ThreadFx<'_>,
) {
    match intent {
        ThreadIntent::Compact if !part.compact_pending && !busy => {
            part.error = None;
            part.compact_pending = true;
            fx.request(Ctx::Compact, Command::Compact);
        }
        ThreadIntent::CleanTerminals => {
            part.error = None;
            fx.request(Ctx::CleanTerminals, Command::CleanTerminals);
        }
        ThreadIntent::ApprovePlan(markdown) => {
            part.error = None;
            fx.request(Ctx::ApprovePlan, Command::ApprovePlan(markdown));
        }
        ThreadIntent::Review(target) => {
            part.error = None;
            fx.request(Ctx::Review, Command::Review(target));
        }
        ThreadIntent::Shell(command) if !command.trim().is_empty() => {
            part.error = None;
            fx.request(Ctx::Shell, Command::Shell(command));
        }
        // the queue and the steers are the thread model's (`threads::apply`)
        _ => {}
    }
}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = fx;
    match ev {
        ThreadEv::Metadata {
            name,
            forked_from,
            cwd,
            path,
            parent_owned,
            ephemeral,
            model,
            effort,
            provider,
            daybreak,
            resumed,
            fresh,
        } => {
            part.name = name;
            part.forked_from = forked_from;
            part.daybreak = daybreak;
            part.cwd = Some(cwd);
            part.path = path;
            part.parent_owned = parent_owned;
            part.ephemeral = ephemeral;
            if part.mode == codex_protocol::config_types::ModeKind::Default {
                part.base = collaboration::Base::of(model.as_deref(), effort.as_deref());
            }
            part.model = model;
            part.effort = effort;
            part.provider = (!provider.is_empty()).then_some(provider);
            // a later `thread/started` without turns does not make a resumed thread fresh
            part.awaiting_usage |= resumed;
            // a later `thread/started` of a thread that ran a turn does not make it fresh again
            part.fresh = fresh && !part.ran_turn;
        }
        ThreadEv::Settings {
            cwd,
            model,
            effort,
            provider,
            service_tier,
            mode,
        } => {
            part.mode = mode;
            if mode == codex_protocol::config_types::ModeKind::Default {
                part.base = collaboration::Base::of(Some(&model), effort.as_deref());
            }
            part.cwd = Some(cwd);
            part.model = Some(model);
            part.effort = effort;
            part.provider = Some(provider);
            part.service_tier = Some(service_tier);
        }
        ThreadEv::Reasoning(ev) => {
            if part.live_reasoning.apply(ev) {
                part.activity = ActivityPhase::Thinking;
            }
        }
        ThreadEv::Compaction { running, .. } => part.live_compaction = running,
        ThreadEv::Compacting(_) | ThreadEv::Safety(_) => {}
        ThreadEv::Name(name) => part.name = name,
        ThreadEv::Retry { retry, details, .. } => {
            part.retry_header = Some(retry);
            part.details = details.map(|text| StatusDetails::new(text, STATUS_DETAILS_MAX_LINES));
        }
        ThreadEv::Details(details) => part.details = details,
        ThreadEv::ProviderError(error) => part.provider_error = Some(error),
        ThreadEv::BudgetLimited { turn } => {
            if !part.budget_limited.contains(&turn) {
                part.budget_limited.push(turn);
            }
        }
        ThreadEv::Plan {
            completed,
            total,
            step,
            ..
        } => {
            part.progress = (total > 0).then_some((completed, total));
            part.step = step;
        }
    }
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = fx;
    // the request is the thread's busy until the turn it starts begins (`begin_turn`): an
    // answer that is no refusal keeps it, so a message typed in between queues
    if matches!(ctx, Ctx::Compact) && result.is_err() {
        part.compact_pending = false;
    }
    // a steer's reply is read by `threads::apply`, never here
    if !matches!(ctx, Ctx::Steer { .. } | Ctx::ListTerminals { .. }) {
        part.error = result.err();
    }
}

fn thread_ask<'a>(command: &'a Command, thread: &ThreadId) -> ThreadRpc<&'a UserMessage> {
    match command {
        Command::Compact => ThreadRpc::Compact {
            thread: thread.to_string(),
        },
        Command::CleanTerminals => ThreadRpc::CleanTerminals {
            thread: thread.to_string(),
        },
        Command::ListTerminals { cursor } => ThreadRpc::ListTerminals {
            thread: thread.clone(),
            cursor: cursor.clone(),
        },
        Command::Review(target) => ThreadRpc::Review {
            thread: thread.to_string(),
            target: target.clone(),
        },
        Command::Shell(command) => ThreadRpc::Shell {
            thread: thread.to_string(),
            command: command.clone(),
        },
        Command::Steer {
            turn,
            client_id,
            message,
        } => ThreadRpc::Steer {
            thread: thread.to_string(),
            turn: turn.to_string(),
            client_id: client_id.clone(),
            message,
        },
        Command::ApprovePlan(markdown) => ThreadRpc::PrepareApprovedPlan {
            thread: thread.to_string(),
            markdown: markdown.clone(),
        },
        Command::Mode(mode) => ThreadRpc::SettingsUpdate {
            thread: thread.to_string(),
            patch: crate::threads::SettingsPatch {
                collaboration_mode: Some(mode.clone()),
                ..crate::threads::SettingsPatch::default()
            },
        },
    }
}

pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    thread_ask(command, thread).wire(id)
}

pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    match thread_ask(command, &ThreadId::default()).decode(result)? {
        ThreadReply::Terminals { data, next } => Ok(Reply::Terminals { data, next }),
        ThreadReply::Done => Ok(Reply::Ack),
        ThreadReply::Prepared(prompt) => Ok(Reply::Prepared(prompt)),
        ThreadReply::Steered(turn) => Ok(Reply::Steered(turn)),
        _ => Err(RpcError::Mismatch),
    }
}

/// The seed form of this screen's events:
/// `{"flags":{"voice_command_enabled":true}}` (the named gates switch on, the
/// others keep their defaults), `{"service_tiers":[{id,name,description}]}`,
/// `{"signed_in":false}`, `{"status":{StatusFacts}}`, `{"facts":{"cwd":"/w"}}` (`FactEv`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    Flags(SeedFlags),
    ServiceTiers(Vec<SeedTier>),
    SignedIn(bool),
    Status(Box<StatusFacts>),
    Facts(FactEv),
    Details(SeedDetails),
    Pending {
        thread: String,
        #[serde(flatten)]
        pending: PendingTexts,
    },
    ProviderError {
        thread: String,
        error: codex_app_server_protocol::ModelProviderError,
    },
}

/// `{"details":{"thread":"thread-1","text":"a\nb","max_lines":3,"preserve":true}}`
#[derive(Clone, Debug, serde::Deserialize)]
struct SeedDetails {
    thread: String,
    text: String,
    max_lines: Option<usize>,
    #[serde(default)]
    preserve: bool,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
struct SeedFlags {
    collaboration_modes_enabled: Option<bool>,
    connectors_enabled: Option<bool>,
    plugins_command_enabled: Option<bool>,
    token_activity_command_enabled: Option<bool>,
    service_tier_commands_enabled: Option<bool>,
    goal_command_enabled: Option<bool>,
    voice_command_enabled: Option<bool>,
    worktrees_enabled: Option<bool>,
    allow_elevate_sandbox: Option<bool>,
    side_conversation_active: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedTier {
    id: String,
    name: String,
    description: String,
}

pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Flags(f) => {
            let base = Model::default().flags;
            Ev::Flags(CommandFlags {
                collaboration_modes_enabled: f
                    .collaboration_modes_enabled
                    .unwrap_or(base.collaboration_modes_enabled),
                connectors_enabled: f.connectors_enabled.unwrap_or(base.connectors_enabled),
                plugins_command_enabled: f
                    .plugins_command_enabled
                    .unwrap_or(base.plugins_command_enabled),
                token_activity_command_enabled: f
                    .token_activity_command_enabled
                    .unwrap_or(base.token_activity_command_enabled),
                service_tier_commands_enabled: f
                    .service_tier_commands_enabled
                    .unwrap_or(base.service_tier_commands_enabled),
                daybreak_command_description: None,
                goal_command_enabled: f.goal_command_enabled.unwrap_or(base.goal_command_enabled),
                voice_command_enabled: f
                    .voice_command_enabled
                    .unwrap_or(base.voice_command_enabled),
                worktrees_enabled: f.worktrees_enabled.unwrap_or(base.worktrees_enabled),
                allow_elevate_sandbox: f
                    .allow_elevate_sandbox
                    .unwrap_or(base.allow_elevate_sandbox),
                side_conversation_active: f
                    .side_conversation_active
                    .unwrap_or(base.side_conversation_active),
            })
        }
        SeedEv::ServiceTiers(tiers) => Ev::ServiceTiers(
            tiers
                .into_iter()
                .map(|t| ServiceTierCommand {
                    id: t.id,
                    name: t.name,
                    description: t.description,
                })
                .collect(),
        ),
        SeedEv::SignedIn(on) => Ev::SignedIn(on),
        SeedEv::Status(facts) => Ev::Status(facts),
        SeedEv::Facts(ev) => Ev::Facts(ev),
        SeedEv::Pending { thread, pending } => Ev::Pending { thread, pending },
        SeedEv::ProviderError { thread, error } => Ev::ProviderError { thread, error },
        SeedEv::Details(d) => {
            let max = d.max_lines.unwrap_or(STATUS_DETAILS_MAX_LINES);
            Ev::Details {
                thread: d.thread,
                details: Some(if d.preserve {
                    StatusDetails::preserved(&d.text, max)
                } else {
                    StatusDetails::new(d.text, max)
                }),
            }
        }
    })
}
