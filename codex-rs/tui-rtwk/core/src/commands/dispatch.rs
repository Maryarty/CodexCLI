//! The decision a slash command leads to (`chatwidget/slash_dispatch.rs`:
//! `dispatch_command`, `dispatch_prepared_command_with_args`): open a screen
//! of the app, ask the session for something (a typed intent), insert text,
//! quit, or say why not (a `Notice`, a key the UI's tables turn into words).
//! A pure function of the line and of a few facts of the session.

use super::CommandFlags;
use super::ServiceTierCommand;
use super::SlashCommand;
use super::SlashCommandItem;
use super::find_slash_command;
use super::parse_slash_name;
use crate::features::{FeatureTarget, command_rule, runs_of, target_of};
pub use crate::screens::usage::TokenActivityView;

/// A screen directory a command opens. The UI opens it through that
/// directory's own opener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The resume picker (S18).
    Resume,
    /// The agents command center and the subagent switcher (S21).
    Agents,
    /// The import from other agents (S06).
    Import,
    /// The usage dashboard (S20).
    Usage,
    /// The static pager (`/diff`) and the transcript pager (S16).
    Pager,
    /// The generic selection dialog (S14).
    Select,
    /// The settings dialogs (S15).
    Settings,
    /// The small dialogs (S23).
    Dialogs,
    /// The pets overlay (S24).
    Pets,
    /// The voice strip and its settings (S25).
    Voice,
    /// The approval dialogs (S13).
    Approvals,
    /// Backtrack: browsing earlier prompts (S17).
    Backtrack,
    /// `/subagents`: the picker of this session's subagent threads (oracle:
    /// `AppEvent::OpenAgentPicker`), a sheet over the chat.
    AgentPicker,
    /// A feature's own sheet (`/plugins`, a bare `/export`): the feature's UI half opens it
    /// (`src/features/<f>/`).
    Feature(FeatureTarget),
}

/// How a target shows up in the app (architecture §1, Amendment 2026-11-19b):
/// only the five pages are routes; the UI opens the rest over the page that
/// asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opens {
    /// A page: Startup (import is its step), Agents, Resume, Usage.
    Page,
    /// A modal sheet over the page that opened it.
    Sheet,
    /// A small centred dialog over the page.
    Dialog,
    /// A mode of the session page (its expanded transcript).
    Mode,
}

impl Target {
    /// How the UI shows the target.
    pub fn opens(self) -> Opens {
        match self {
            Target::Resume | Target::Agents | Target::Import | Target::Usage => Opens::Page,
            Target::Select
            | Target::Settings
            | Target::Approvals
            | Target::Pets
            | Target::Voice
            | Target::AgentPicker
            | Target::Feature(_) => Opens::Sheet,
            Target::Dialogs => Opens::Dialog,
            Target::Pager | Target::Backtrack => Opens::Mode,
        }
    }

    /// The name of the screen directory (`screens::dirs()`).
    pub fn dir(self) -> &'static str {
        match self {
            Target::Resume => "resume",
            Target::Agents => "agents",
            Target::Import => "import",
            Target::Usage => "usage",
            Target::Pager => "pager",
            Target::Select => "select",
            Target::Settings => "settings",
            Target::Dialogs => "dialogs",
            Target::Pets => "pets",
            Target::Voice => "voice",
            Target::Approvals => "approvals",
            Target::Backtrack => "backtrack",
            // The agents directory owns the entry: the picker is its sheet.
            Target::AgentPicker => "agents",
            Target::Feature(feature) => feature.dir(),
        }
    }
}

/// The oracle's error and info messages for command dispatch and composer input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// `Unrecognized command '/name'`.
    Unrecognized(String),
    /// `'/cmd' is disabled while a task is in progress.`
    DisabledDuringTask(SlashCommand),
    /// `'/cmd' is unavailable in side conversations.`
    UnavailableInSide(SlashCommand),
    /// The same for a service-tier command (`/fast`): its name.
    TierUnavailableInSide(String),
    /// `'/cmd' is unavailable while code review is running.`
    UnavailableInReview(SlashCommand),
    /// `Session is still starting; try /cmd again in a moment.`
    SessionStarting(SlashCommand),
    /// `Model selection is disabled until startup completes.` (an info line,
    /// `model_popups.rs:19-24`: `/model` before the thread exists).
    ModelStartup,
    /// `Sign in with ChatGPT to use /usage.`
    UsageNeedsLogin,
    /// A usage line: `Usage: /pwd`, `Usage: /raw [on|off]`, ...
    Usage(UsageKey),
    /// `MCP sign-in requires an active session.`
    McpLoginNeedsSession,
    /// The session's part of the command is not built yet (session-core).
    NotAvailable(SlashCommand),
    /// `Prefix a command with ! to run it locally` / `Example: !ls`: a bare
    /// `!` was submitted (`USER_SHELL_COMMAND_HELP_*`, an info line).
    ShellHelp,
    /// `Message exceeds the maximum length of N characters (M provided).`
    InputTooLarge { provided: usize },
    /// `Thread model is unavailable. Wait for the thread to finish syncing or choose a model
    /// before sending input.`: a submit, or the queue's next message, met a thread whose model is
    /// empty; the message goes back to the composer.
    ThreadModelUnavailable,
    /// The selected model advertises no image input.
    ImageInputsUnsupported(String),
    /// The host could not read the clipboard image.
    ClipboardReadFailed(String),
}

impl Notice {
    /// Whether the oracle shows the line as information
    /// (`add_info_message`) and not as an error (`add_error_message`).
    pub fn is_info(&self) -> bool {
        matches!(self, Notice::ModelStartup | Notice::ShellHelp)
    }
}

/// Which usage line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageKey {
    Pwd,
    Usage,
    Raw,
    Voice,
    Keymap,
    Mcp,
}

/// What the session is asked for: the commands that act on the thread or the
/// connection. session-core gives each its effect; until then the chat
/// answers `Notice::NotAvailable`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionIntent {
    New {
        name: Option<String>,
    },
    /// The provider-error notice's continue key: a new chat whose composer
    /// holds `message` (the view words it from the saved chat's name and path).
    ContinueChat {
        message: String,
    },
    Clear {
        name: Option<String>,
    },
    Fork {
        name: Option<String>,
    },
    Compact,
    /// `/status`: a card in the thread's transcript.
    Status,
    Init,
    Archive,
    Delete,
    Worktree,
    Plan {
        prompt: Option<String>,
    },
    Raw {
        on: Option<bool>,
    },
    Cd {
        path: String,
    },
    Pwd,
    Vim,
    Stop,
    Rollout,
    App,
    Review {
        args: Option<String>,
    },
    /// The review popup's pick (or `/review <text>` once typed): `review/start`
    /// inline on this thread, with the target chosen.
    StartReview {
        target: crate::threads::ReviewTarget,
    },
    Permissions {
        args: Option<String>,
    },
    VoiceControl(VoiceControl),
    MemoryMaintenance,
    /// A command a feature runs (`Feature::RUNS`: `/rename`, `/goal`, `/model <name>`, `/mcp
    /// verbose`, ...) with the text after its name (trimmed, empty when bare): the feature that
    /// owns the command reads it (`features::apply_slash`), the hub names no slice's verb.
    Feature {
        args: String,
    },
}

/// `/voice` with an argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceControl {
    Toggle,
    Mute,
    Stop,
}

/// A screen to open, with the command and arguments that asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    pub target: Target,
    pub command: SlashCommand,
    pub args: String,
}

/// What a dispatched line leads to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The line is not a command (plain text, a path): it is submitted.
    NotACommand,
    Open(Open),
    Session {
        command: SlashCommand,
        intent: SessionIntent,
    },
    /// Text for the composer (`/mention` inserts `@`).
    Insert(&'static str),
    /// A service-tier command of the model (`/fast`): the tier is switched.
    Tier(ServiceTierCommand),
    Quit,
    Notice(Notice),
}

/// The facts of the session a dispatch reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchCx {
    pub flags: CommandFlags,
    /// A turn is running or about to start.
    pub task_running: bool,
    /// A side conversation is active.
    pub side_conversation: bool,
    /// Code review is running.
    pub review_mode: bool,
    /// A steer was rejected or a message waits in the queue (`/review` is refused then).
    pub queued_follow_up: bool,
    /// The thread exists (the session is configured).
    pub thread_started: bool,
    /// Signed in with a ChatGPT backend (`/usage`).
    pub has_backend_auth: bool,
}

/// A slice's addition to the common dispatch checks.
#[derive(Clone, Copy)]
pub enum Need {
    OutsideReview,
    NoRunningTask,
    ThreadWhenBare(fn(SlashCommand) -> Notice),
    BackendAuth(fn(SlashCommand) -> Notice),
}

type ArgumentDispatch = fn(SlashCommand, &str, &DispatchCx) -> Outcome;

/// A command's special dispatch policy. Ordinary sheets and verbs use `OPENS` and `RUNS`.
pub struct CommandRule {
    pub command: SlashCommand,
    need: Option<Need>,
    bare_target: Option<Target>,
    arguments: Option<ArgumentDispatch>,
}

impl CommandRule {
    pub const fn new(command: SlashCommand) -> Self {
        Self {
            command,
            need: None,
            bare_target: None,
            arguments: None,
        }
    }

    pub const fn requiring(mut self, need: Need) -> Self {
        self.need = Some(need);
        self
    }

    pub const fn opens(mut self, bare_target: Target) -> Self {
        self.bare_target = Some(bare_target);
        self
    }

    pub const fn arguments(mut self, arguments: ArgumentDispatch) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

/// The bare screen routes declared by the feature registry.
pub fn feature_openings() -> impl Iterator<Item = (SlashCommand, Target)> {
    SlashCommand::ALL.into_iter().filter_map(|command| {
        feature_opening(command, command_rule(command)).map(|target| (command, target))
    })
}

fn feature_opening(command: SlashCommand, rule: Option<&CommandRule>) -> Option<Target> {
    rule.and_then(|rule| rule.bare_target)
        .or_else(|| target_of(command).map(Target::Feature))
}

/// Dispatches a composer line that starts with `/`, for a model that offers no service tier.
pub fn dispatch(line: &str, cx: &DispatchCx) -> Outcome {
    dispatch_with_tiers(line, cx, &[])
}

/// Dispatches a composer line that starts with `/`; `tiers` are the commands the model's service
/// tiers add (`find_slash_command`).
pub fn dispatch_with_tiers(line: &str, cx: &DispatchCx, tiers: &[ServiceTierCommand]) -> Outcome {
    let Some((name, rest, _)) = parse_slash_name(line) else {
        return Outcome::NotACommand;
    };
    // `/usr/bin/env`: a path, not a command.
    if name.contains('/') {
        return Outcome::NotACommand;
    }
    match find_slash_command(name, cx.flags, tiers) {
        Some(SlashCommandItem::Builtin(command)) => dispatch_command(command, rest.trim_end(), cx),
        // a tier command takes no arguments: with some, the line is a message
        Some(SlashCommandItem::ServiceTier(_)) if !rest.trim().is_empty() => Outcome::NotACommand,
        Some(SlashCommandItem::ServiceTier(tier)) => {
            if cx.side_conversation {
                Outcome::Notice(Notice::TierUnavailableInSide(tier.name))
            } else {
                Outcome::Tier(tier)
            }
        }
        None => Outcome::Notice(Notice::Unrecognized(name.to_string())),
    }
}

/// A queued line dispatched when its turn comes (`submit_queued_slash_prompt`): what the
/// outcome is and the command that was found, for the rule of the next item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedDispatch {
    pub outcome: Outcome,
    pub command: Option<SlashCommand>,
}

/// Dispatches a queued line. As a live line, except that a command which takes no arguments,
/// given some, is a message for the model (the line is submitted whole).
pub fn dispatch_queued(
    line: &str,
    cx: &DispatchCx,
    tiers: &[ServiceTierCommand],
) -> QueuedDispatch {
    let found = parse_slash_name(line)
        .filter(|(name, _, _)| !name.contains('/'))
        .and_then(|(name, rest, _)| {
            find_slash_command(name, cx.flags, tiers).map(|item| (item, rest))
        });
    let command = match &found {
        Some((SlashCommandItem::Builtin(command), _)) => Some(*command),
        _ => None,
    };
    if let Some((SlashCommandItem::Builtin(command), rest)) = &found
        && !rest.trim().is_empty()
        && !command.supports_inline_args()
    {
        return QueuedDispatch {
            outcome: Outcome::NotACommand,
            command: None,
        };
    }
    QueuedDispatch {
        outcome: dispatch_with_tiers(line, cx, tiers),
        command,
    }
}

pub(crate) fn open(target: Target, command: SlashCommand, args: &str) -> Outcome {
    Outcome::Open(Open {
        target,
        command,
        args: args.to_string(),
    })
}

fn session(command: SlashCommand, intent: SessionIntent) -> Outcome {
    Outcome::Session { command, intent }
}

/// A command a feature runs (`Feature::RUNS`), with the text after its name.
pub(crate) fn run(command: SlashCommand, args: &str) -> Outcome {
    session(
        command,
        SessionIntent::Feature {
            args: args.trim().to_string(),
        },
    )
}

fn some(args: &str) -> Option<String> {
    (!args.is_empty()).then(|| args.to_string())
}

/// Dispatches a recognized command with the text after its name.
pub fn dispatch_command(command: SlashCommand, args: &str, cx: &DispatchCx) -> Outcome {
    if cx.side_conversation && !command.available_in_side_conversation() {
        return Outcome::Notice(Notice::UnavailableInSide(command));
    }
    let rule = command_rule(command);
    let need = rule.and_then(|rule| rule.need);
    if cx.review_mode && matches!(need, Some(Need::OutsideReview)) {
        return Outcome::Notice(Notice::UnavailableInReview(command));
    }
    // `slash_dispatch.rs:150-167`, the validation gate: a command that is not
    // available during a task is refused while a turn runs, a start is in
    // flight (`task_running` is both) or a review runs. A slice can require an
    // idle task without blocking review. `/review` also checks for a queued follow-up.
    let blocked = cx.task_running || cx.review_mode;
    if (!command.available_during_task() && blocked)
        || (cx.task_running && matches!(need, Some(Need::NoRunningTask)))
        || (command == SlashCommand::Review && cx.queued_follow_up)
    {
        return Outcome::Notice(Notice::DisabledDuringTask(command));
    }
    if let Some(Need::BackendAuth(notice)) = need
        && !cx.has_backend_auth
    {
        return Outcome::Notice(notice(command));
    }
    if !command.supports_inline_args() || args.is_empty() {
        dispatch_bare(command, cx, rule)
    } else {
        dispatch_with_args(command, args, cx, rule)
    }
}

/// `chatwidget/slash_dispatch.rs`, `dispatch_command` (bare commands).
fn dispatch_bare(command: SlashCommand, cx: &DispatchCx, rule: Option<&CommandRule>) -> Outcome {
    use SlashCommand::*;
    if let Some(Need::ThreadWhenBare(notice)) = rule.and_then(|rule| rule.need)
        && !cx.thread_started
    {
        return Outcome::Notice(notice(command));
    }
    if let Some(target) = feature_opening(command, rule) {
        return open(target, command, "");
    }
    match command {
        Presets | AutoReview | Archive | Delete | Review | Permissions | Apps => {
            open(Target::Select, command, "")
        }
        Agents => open(Target::Agents, command, ""),
        MultiAgents => open(Target::AgentPicker, command, ""),
        Import => open(Target::Import, command, ""),
        Status => session(command, SessionIntent::Status),
        Warnings => open(Target::Dialogs, command, ""),
        Pets => open(Target::Pets, command, ""),
        TestApproval => open(Target::Approvals, command, ""),
        Voice => session(command, SessionIntent::VoiceControl(VoiceControl::Toggle)),
        Mention => Outcome::Insert("@"),
        Quit | Exit => Outcome::Quit,
        App if !cx.thread_started => Outcome::Notice(Notice::SessionStarting(command)),
        App => session(command, SessionIntent::App),
        New => session(command, SessionIntent::New { name: None }),
        Clear => session(command, SessionIntent::Clear { name: None }),
        Fork => session(command, SessionIntent::Fork { name: None }),
        Compact => session(command, SessionIntent::Compact),
        Init => session(command, SessionIntent::Init),
        Worktree => session(command, SessionIntent::Worktree),
        Plan => session(command, SessionIntent::Plan { prompt: None }),
        Raw => session(command, SessionIntent::Raw { on: None }),
        // `slash_dispatch.rs:557-559`: a bare `/cd` is `/cd ~`.
        Cd => session(
            command,
            SessionIntent::Cd {
                path: "~".to_string(),
            },
        ),
        Pwd => session(command, SessionIntent::Pwd),
        Vim => session(command, SessionIntent::Vim),
        Stop => session(command, SessionIntent::Stop),
        Rollout => session(command, SessionIntent::Rollout),
        MemoryDrop | MemoryUpdate => session(command, SessionIntent::MemoryMaintenance),
        // every other command is a feature's verb (`Feature::RUNS`); one that has no owner is
        // answered "not available" where the app looks for it (`features::apply_slash`)
        _ => run(command, ""),
    }
}

/// `chatwidget/slash_dispatch.rs`, commands with inline arguments.
fn dispatch_with_args(
    command: SlashCommand,
    args: &str,
    cx: &DispatchCx,
    rule: Option<&CommandRule>,
) -> Outcome {
    use SlashCommand::*;
    let args = args.trim();
    if let Some(dispatch) = rule.and_then(|rule| rule.arguments) {
        return dispatch(command, args, cx);
    }
    match command {
        Review => session(command, SessionIntent::Review { args: some(args) }),
        Permissions => session(command, SessionIntent::Permissions { args: some(args) }),
        New => session(command, SessionIntent::New { name: some(args) }),
        Clear => session(command, SessionIntent::Clear { name: some(args) }),
        Fork => session(command, SessionIntent::Fork { name: some(args) }),
        Plan => session(command, SessionIntent::Plan { prompt: some(args) }),
        Cd => session(
            command,
            SessionIntent::Cd {
                path: args.to_string(),
            },
        ),
        Pets => open(Target::Pets, command, args),
        Pwd => Outcome::Notice(Notice::Usage(UsageKey::Pwd)),
        Voice => match args.to_ascii_lowercase().as_str() {
            "settings" => open(Target::Voice, command, "settings"),
            "mute" => session(command, SessionIntent::VoiceControl(VoiceControl::Mute)),
            "stop" => session(command, SessionIntent::VoiceControl(VoiceControl::Stop)),
            _ => Outcome::Notice(Notice::Usage(UsageKey::Voice)),
        },
        Raw => match args.to_ascii_lowercase().as_str() {
            "on" => session(command, SessionIntent::Raw { on: Some(true) }),
            "off" => session(command, SessionIntent::Raw { on: Some(false) }),
            _ => Outcome::Notice(Notice::Usage(UsageKey::Raw)),
        },
        // a feature's verb takes its arguments (`Feature::RUNS`)
        _ if runs_of(command) => run(command, args),
        // A command without inline arguments never gets here.
        _ => dispatch_bare(command, cx, rule),
    }
}
