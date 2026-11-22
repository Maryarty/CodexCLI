//! `chat` (S10, main chat screen): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

mod banner;
mod mcp;

use std::rc::Rc;

pub use banner::{Banner, BannerAction, BannerCta, BannerOccurrence, Nudge, NudgeOutcome};
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
use crate::threads::{Note, ThreadFx};

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
    /// What the status line shows, read by the host (config, cwd, git).
    pub status: Rc<StatusFacts>,
}

/// The oracle's default list (`DEFAULT_STATUS_LINE_ITEMS`) when the config
/// names none.
pub const DEFAULT_STATUS_LINE_ITEMS: [&str; 3] = ["model-with-reasoning", "current-dir", "thread-name"];

/// `LUNA_RESERVE_MODEL` (`model_catalog.rs`).
const LUNA_RESERVE_MODEL: &str = "gpt-reserve";

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
    /// The approvals reviewer is the auto reviewer.
    pub auto_review: bool,
    pub profile: super::status::Profile,
    /// The working directory as shown (`~/dev/app`).
    pub cwd: Option<String>,
    pub home_directory: Option<String>,
    pub service_tier: Option<String>,
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
    pub version: Option<String>,
    pub animations: bool,
    pub progress_effect: bool,
    pub shimmer_effect: bool,
    pub account_usage: bool,
    pub empty_state: bool,
    pub notification_policy: NotificationPolicy,
}

impl Default for StatusFacts {
    fn default() -> Self {
        Self {
            model: None,
            effort: None,
            approval_policy: None,
            auto_review: false,
            profile: super::status::Profile::Workspace,
            cwd: None,
            home_directory: None,
            service_tier: None,
            project_root: None,
            branch: None,
            items: None,
            colors: true,
            terminal_title: None,
            hostname: None,
            profile_name: None,
            provider: None,
            version: None,
            animations: true,
            progress_effect: true,
            shimmer_effect: true,
            account_usage: false,
            empty_state: false,
            notification_policy: NotificationPolicy::default(),
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
        self.enabled && self.kinds.as_ref().is_none_or(|kinds| kinds.iter().any(|name| name == kind))
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
            status: Rc::default(),
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
    /// A command line was answered (refused, or its words): a note of the
    /// thread shown when it was submitted, else of the screen.
    Note {
        thread: Option<ThreadId>,
        note: Note,
    },
    /// `/raw`: the render mode of the transcript (`set_raw_output_mode`).
    RawOutput(bool),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Flags(flags) => model.flags = flags,
        Ev::ServiceTiers(tiers) => model.service_tiers = tiers,
        Ev::SignedIn(on) => model.signed_in = on,
        Ev::Status(facts) => model.status = Rc::from(facts),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        // the app routes these to the thread (`apply_app`); the screen keeps nothing
        Intent::Session { .. } | Intent::Shell { .. } => {}
        Intent::Note { thread: None, note } => model.notes.push(note),
        Intent::RawOutput(enabled) => model.raw_output = enabled,
        // a thread's note is an entry of its transcript (`session::note`)
        Intent::Note { .. } => {}
    }
}

/// Per-thread state and intents for the chat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    compact_pending: bool,
    pub error: Option<RpcError>,
    pub name: Option<String>,
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
    pub activity: ActivityPhase,
    live_compaction: bool,
    pub progress: Option<(usize, usize)>,
    pub step: Option<String>,
    /// MCP startup of this thread (oracle `mcp_startup_status`).
    pub mcp: McpStartup,
    /// A retrying error's header, until the thread's next notification
    /// (oracle `retry_status_header`).
    retry_header: Option<Retry>,
    /// A code review runs in this thread (oracle `review.is_review_mode`).
    review_mode: bool,
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
        self.progress = None;
        self.activity = ActivityPhase::Working;
        self.live_compaction = false;
        self.step = None;
    }

    pub(crate) fn end_turn(&mut self) {
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

    /// A code review runs in this thread (oracle `review.is_review_mode`).
    pub fn review_mode(&self) -> bool {
        self.review_mode
    }

    pub(crate) fn set_review_mode(&mut self, on: bool) {
        self.review_mode = on;
    }

    /// Any notification of the thread but a retrying error ends the retry header.
    pub(crate) fn clear_retry(&mut self) {
        self.retry_header = None;
    }

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
    Rename(String),
    /// `!cmd` from the composer: a user shell command (the oracle's
    /// `AppCommand::RunUserShellCommand`), run while a turn runs too.
    Shell(String),
    /// Tab: the message waits behind the running turn (the oracle's
    /// `InputResult::Queued`); `threads::apply` owns the queue.
    Queue(crate::threads::UserMessage),
    /// Esc while steers are pending: interrupt the turn, then send the
    /// steers as one message (`submit_pending_steers_after_interrupt`).
    InterruptForSteers,
    /// The edit-queued key: the newest queued message goes back to the
    /// composer (`pop_latest_queued_composer_state`).
    EditQueued,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    Metadata {
        name: Option<String>,
        cwd: String,
        path: Option<String>,
        parent_owned: bool,
        ephemeral: bool,
        model: Option<String>,
        effort: Option<String>,
        provider: String,
        daybreak: Option<bool>,
    },
    Settings {
        cwd: String,
        model: String,
        effort: Option<String>,
        provider: String,
        service_tier: Option<String>,
    },
    Phase { turn: TurnId, phase: ActivityPhase },
    Compaction { turn: TurnId, running: bool },
    Name(Option<String>),
    Plan { turn: TurnId, completed: usize, total: usize, step: Option<String> },
    /// The server retries the turn's request: the header says so.
    Retry { turn: TurnId, retry: Retry },
}

impl ThreadEv {
    pub(crate) fn turn(&self) -> Option<&TurnId> {
        match self {
            Self::Plan { turn, .. }
            | Self::Phase { turn, .. }
            | Self::Compaction { turn, .. }
            | Self::Retry { turn, .. } => Some(turn),
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
fn blocks_direct_input(thread: &codex_app_server_protocol::Thread) -> bool {
    use codex_app_server_protocol::SessionSource;
    use codex_protocol::protocol::SubAgentSource;
    thread.can_accept_direct_input.map_or_else(
        || {
            matches!(
                &thread.source,
                SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_path: Some(_), .. })
            )
        },
        |can_accept| !can_accept,
    )
}

pub(crate) fn metadata(thread: &codex_app_server_protocol::Thread) -> ThreadEv {
    ThreadEv::Metadata {
        name: thread.name.clone(),
        cwd: thread.cwd.display().to_string(),
        path: thread.path.as_ref().map(|path| path.display().to_string()),
        parent_owned: blocks_direct_input(thread),
        ephemeral: thread.ephemeral,
        model: thread.model.clone(),
        effort: thread.reasoning_effort.as_ref().map(|effort| effort.as_str().to_string()),
        provider: thread.model_provider.clone(),
        daybreak: thread.daybreak_enabled,
    }
}

pub(crate) fn project(notification: &codex_app_server_protocol::ServerNotification) -> Option<(ThreadId, ThreadEv)> {
    use codex_app_server_protocol::{ServerNotification, ThreadItemPayload};
    let (id, event) = match notification {
        ServerNotification::ThreadStarted(note) => (&note.thread.id, metadata(&note.thread)),
        ServerNotification::ThreadNameUpdated(note) => (&note.thread_id, ThreadEv::Name(note.thread_name.clone())),
        ServerNotification::ThreadSettingsUpdated(note) => {
            let settings = &note.thread_settings;
            (&note.thread_id, ThreadEv::Settings {
                cwd: settings.cwd.display().to_string(),
                model: settings.model.clone(),
                effort: settings.effort.as_ref().map(|effort| effort.as_str().to_string()),
                provider: settings.model_provider.clone(),
                service_tier: settings.service_tier.clone(),
            })
        }
        ServerNotification::Error(note) if note.will_retry => {
            // `Reconnecting... 2/5` after a 429 reads as the server being busy.
            let busy = matches!(
                note.error.codex_error_info,
                Some(codex_app_server_protocol::CodexErrorInfo::ResponseStreamDisconnected {
                    http_status_code: Some(429)
                })
            );
            let retry = match note.error.message.strip_prefix("Reconnecting... ") {
                Some(attempt) if busy => Retry::Busy { attempt: attempt.to_string() },
                _ => Retry::Server(note.error.message.clone()),
            };
            (&note.thread_id, ThreadEv::Retry { turn: TurnId::new(&note.turn_id), retry })
        }
        ServerNotification::ReasoningSummaryPartAdded(note) => (&note.thread_id, ThreadEv::Phase {
            turn: TurnId::new(&note.turn_id),
            phase: ActivityPhase::Thinking,
        }),
        ServerNotification::ItemStarted(note) if matches!(note.item.payload, ThreadItemPayload::ContextCompaction { .. }) => {
            (&note.thread_id, ThreadEv::Compaction { turn: TurnId::new(&note.turn_id), running: true })
        }
        ServerNotification::ItemCompleted(note) => {
            let turn = TurnId::new(&note.turn_id);
            let event = match &note.item.payload {
                ThreadItemPayload::ContextCompaction { .. } => ThreadEv::Compaction { turn, running: false },
                ThreadItemPayload::Reasoning { .. } => ThreadEv::Phase { turn, phase: ActivityPhase::Working },
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
    Rename(String),
    Shell(String),
    /// `turn/steer`: the message joins the running turn `turn`.
    Steer {
        turn: TurnId,
        message: crate::threads::UserMessage,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Ack,
    /// The turn the steer joined.
    Steered(TurnId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Compact,
    CleanTerminals,
    Rename,
    Shell,
    /// A steer in flight: `threads::apply` reads its reply (the queue and the
    /// previews are the thread's, not the slot's); `retried` once the turn id
    /// was corrected.
    Steer {
        turn: TurnId,
        retried: bool,
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
        ThreadIntent::Rename(name) if !name.trim().is_empty() => {
            part.error = None;
            fx.request(Ctx::Rename, Command::Rename(name));
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
        ThreadEv::Metadata { name, cwd, path, parent_owned, ephemeral, model, effort, provider, daybreak } => {
            part.name = name;
            part.daybreak = daybreak;
            part.cwd = Some(cwd);
            part.path = path;
            part.parent_owned = parent_owned;
            part.ephemeral = ephemeral;
            part.model = model;
            part.effort = effort;
            part.provider = (!provider.is_empty()).then_some(provider);
        }
        ThreadEv::Settings { cwd, model, effort, provider, service_tier } => {
            part.cwd = Some(cwd);
            part.model = Some(model);
            part.effort = effort;
            part.provider = Some(provider);
            part.service_tier = Some(service_tier);
        }
        ThreadEv::Phase { phase, .. } => part.activity = phase,
        ThreadEv::Compaction { running, .. } => part.live_compaction = running,
        ThreadEv::Name(name) => part.name = name,
        ThreadEv::Retry { retry, .. } => part.retry_header = Some(retry),
        ThreadEv::Plan { completed, total, step, .. } => {
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
    if matches!(ctx, Ctx::Compact) {
        part.compact_pending = false;
    }
    // a steer's reply is read by `threads::apply`, never here
    if !matches!(ctx, Ctx::Steer { .. }) {
        part.error = result.err();
    }
}

pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let ask = match command {
        Command::Compact => crate::threads::ThreadAsk::Compact { thread: thread.to_string() },
        Command::CleanTerminals => crate::threads::ThreadAsk::CleanTerminals { thread: thread.to_string() },
        Command::Rename(name) => crate::threads::ThreadAsk::Rename {
            thread: thread.to_string(),
            name: name.clone(),
        },
        Command::Shell(command) => crate::threads::ThreadAsk::Shell {
            thread: thread.to_string(),
            command: command.clone(),
        },
        Command::Steer { turn, message } => crate::threads::ThreadAsk::Steer {
            thread: thread.to_string(),
            turn: turn.to_string(),
            message: message.clone(),
        },
    };
    ask.wire(id)
}

pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    use codex_app_server_protocol::{
        ThreadBackgroundTerminalsCleanResponse, ThreadCompactStartResponse, ThreadSetNameResponse,
        ThreadShellCommandResponse, TurnSteerResponse,
    };
    let decoded = match command {
        Command::Compact => serde_json::from_value::<ThreadCompactStartResponse>(result).map(|_| Reply::Ack),
        Command::CleanTerminals => {
            serde_json::from_value::<ThreadBackgroundTerminalsCleanResponse>(result).map(|_| Reply::Ack)
        }
        Command::Rename(_) => serde_json::from_value::<ThreadSetNameResponse>(result).map(|_| Reply::Ack),
        Command::Shell(_) => serde_json::from_value::<ThreadShellCommandResponse>(result).map(|_| Reply::Ack),
        Command::Steer { .. } => serde_json::from_value::<TurnSteerResponse>(result)
            .map(|response| Reply::Steered(TurnId::new(response.turn_id))),
    };
    decoded.map_err(|error| RpcError::Malformed(error.to_string()))
}

/// The seed form of this screen's events:
/// `{"flags":{"voice_command_enabled":true}}` (the named gates switch on, the
/// others keep their defaults), `{"service_tiers":[{id,name,description}]}`,
/// `{"signed_in":false}`, `{"status":{StatusFacts}}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    Flags(SeedFlags),
    ServiceTiers(Vec<SeedTier>),
    SignedIn(bool),
    Status(Box<StatusFacts>),
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
    })
}
