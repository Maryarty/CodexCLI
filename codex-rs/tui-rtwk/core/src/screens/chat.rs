//! `chat` (S10, main chat screen): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/chat/` in the UI crate. Nothing
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
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A command the session has to carry out. Until session-core gives each
    /// its effect the chat answers `ChatNotice::NotAvailable`.
    Session {
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
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Session { command, intent: _ } => {
            model.notice = Some(ChatNotice::NotAvailable(command));
        }
        Intent::Notice(notice) => model.notice = Some(ChatNotice::Command(notice)),
        Intent::ClearNotice => model.notice = None,
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

/// The seed form of this screen's events:
/// `{"flags":{"voice_command_enabled":true}}` (the named gates switch on, the
/// others keep their defaults), `{"service_tiers":[{id,name,description}]}`,
/// `{"signed_in":false}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedEv {
    Flags(SeedFlags),
    ServiceTiers(Vec<SeedTier>),
    SignedIn(bool),
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
    })
}
