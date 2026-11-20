//! `chat` (S10, main chat screen): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::commands::CommandFlags;
use crate::commands::Notice;
use crate::commands::ServiceTierCommand;
use crate::commands::SessionIntent;
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

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
    /// What the last command answered with: shown above the composer until
    /// the next submit.
    pub notice: Option<ChatNotice>,
    /// What the status line shows, read by the host (config, cwd, git).
    pub status: StatusFacts,
}

/// The oracle's default list (`DEFAULT_STATUS_LINE_ITEMS`) when the config
/// names none.
const DEFAULT_STATUS_LINE_ITEMS: [&str; 3] = ["model-with-reasoning", "current-dir", "thread-name"];

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
    /// The name of the project root (the git root's directory).
    pub project_root: Option<String>,
    pub branch: Option<String>,
    /// `tui.status_line`: item ids in the order shown; none is the default.
    pub items: Option<Vec<String>>,
    /// `tui.status_line_use_colors`.
    pub colors: bool,
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
            project_root: None,
            branch: None,
            items: None,
            colors: true,
        }
    }
}

impl StatusFacts {
    /// The items to draw, in the configured order, each as its id (as the
    /// config wrote it) and text. An item without a value (it needs the
    /// session, or the host found nothing) and an unknown id are left out.
    pub fn items(&self) -> Vec<(String, String)> {
        let ids: Vec<String> = match &self.items {
            Some(items) => items.clone(),
            None => DEFAULT_STATUS_LINE_ITEMS
                .iter()
                .map(ToString::to_string)
                .collect(),
        };
        ids.into_iter()
            .filter_map(|id| self.value(&id).map(|text| (id, text)))
            .collect()
    }

    fn model_name(&self) -> Option<&str> {
        let model = self.model.as_deref()?;
        Some(if model.eq_ignore_ascii_case(LUNA_RESERVE_MODEL) {
            "Luna Reserve"
        } else {
            model
        })
    }

    /// `status_line_reasoning_effort_label`.
    fn effort_label(&self) -> &str {
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
    fn value(&self, id: &str) -> Option<String> {
        match id {
            "model" | "model-name" => self.model_name().map(ToString::to_string),
            "model-with-reasoning" => self
                .model_name()
                .map(|model| format!("{model} {}", self.effort_label())),
            "reasoning" => Some(self.effort_label().to_string()),
            "current-dir" => self.cwd.clone(),
            "project" | "project-root" => self.project_root.clone(),
            "git-branch" => self.branch.clone(),
            "approval-mode" | "approval" => self.approval(),
            _ => None,
        }
    }
}

/// An answer to a command shown in the chat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatNotice {
    /// Dispatch said no, or what is wrong with the arguments.
    Command(Notice),
    /// The command needs the session, which has not been built yet
    /// (`session-core`): the oracle-style "not available" key.
    NotAvailable(SlashCommand),
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
            notice: None,
            status: StatusFacts::default(),
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
    Status(StatusFacts),
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
    /// A command dispatch refused.
    Notice(Notice),
    /// The next submit starts clean.
    ClearNotice,
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Flags(flags) => model.flags = flags,
        Ev::ServiceTiers(tiers) => model.service_tiers = tiers,
        Ev::SignedIn(on) => model.signed_in = on,
        Ev::Status(facts) => model.status = facts,
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Session { command, .. } => {
            model.notice = Some(ChatNotice::NotAvailable(command));
        }
        Intent::Notice(notice) => model.notice = Some(ChatNotice::Command(notice)),
        Intent::ClearNotice => model.notice = None,
    }
}

/// Per-thread state and intents for the chat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    compact_pending: bool,
    pub error: Option<RpcError>,
}

impl ThreadPart {
    pub(crate) fn connection_lost(&mut self) {
        self.compact_pending = false;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadIntent {
    Compact,
    Rename(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Compact,
    Rename(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Ack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Compact,
    Rename,
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
        ThreadIntent::Rename(name) if !name.trim().is_empty() => {
            part.error = None;
            fx.request(Ctx::Rename, Command::Rename(name));
        }
        _ => {}
    }
}

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
    let _ = fx;
    if matches!(ctx, Ctx::Compact) {
        part.compact_pending = false;
    }
    part.error = result.err();
}

pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let ask = match command {
        Command::Compact => crate::threads::ThreadAsk::Compact { thread: thread.to_string() },
        Command::Rename(name) => crate::threads::ThreadAsk::Rename {
            thread: thread.to_string(),
            name: name.clone(),
        },
    };
    ask.wire(id)
}

pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    use codex_app_server_protocol::{ThreadCompactStartResponse, ThreadSetNameResponse};
    let decoded = match command {
        Command::Compact => serde_json::from_value::<ThreadCompactStartResponse>(result).map(|_| Reply::Ack),
        Command::Rename(_) => serde_json::from_value::<ThreadSetNameResponse>(result).map(|_| Reply::Ack),
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
    Status(StatusFacts),
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
