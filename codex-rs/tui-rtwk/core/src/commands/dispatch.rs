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
use crate::features::{FeatureTarget, target_of};

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

/// Why a command did not run, or what is wrong with its arguments: the
/// oracle's error and info messages as keys.
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
}

impl Notice {
    /// Whether the oracle shows the line as information
    /// (`add_info_message`) and not as an error (`add_error_message`).
    pub fn is_info(&self) -> bool {
        matches!(self, Notice::ModelStartup | Notice::ShellHelp)
    }
}

/// The view `/usage <arg>` opens the dashboard on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenActivityView {
    Daily,
    Weekly,
    Cumulative,
}

impl TokenActivityView {
    /// Parses the optional `/usage` argument into a supported chart view.
    /// Carried from `analytics/activity_chart.rs` (`TokenActivityView::parse`).
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "day" | "daily" => Some(Self::Daily),
            "week" | "weekly" => Some(Self::Weekly),
            "cumulative" => Some(Self::Cumulative),
            _ => None,
        }
    }

    /// The argument the dashboard opens with.
    pub fn arg(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Cumulative => "cumulative",
        }
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
    Recap,
    Archive,
    Delete,
    Worktree,
    Rename {
        name: Option<String>,
    },
    Plan {
        prompt: Option<String>,
    },
    Goal {
        args: Option<String>,
    },
    Side {
        prompt: Option<String>,
    },
    Copy,
    /// `/export <path>`: a bare `/export` is the feature's own sheet (`Target::Feature`).
    Export {
        path: String,
    },
    Raw {
        on: Option<bool>,
    },
    Tui,
    Cd {
        path: String,
    },
    Pwd,
    /// `/mcp verbose` and `/mcp login <server>` (a bare `/mcp` is the feature's, `Feature::OPENS`).
    Mcp {
        args: Option<String>,
    },
    Apps,
    Logout,
    Vim,
    Ide {
        args: Option<String>,
    },
    Daybreak,
    ElevateSandbox,
    Hooks,
    DebugConfig,
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
    Model {
        args: Option<String>,
    },
    Permissions {
        args: Option<String>,
    },
    Title {
        args: Option<String>,
    },
    Statusline {
        args: Option<String>,
    },
    VoiceControl(VoiceControl),
    ResumeNamed {
        name: String,
    },
    MemoryMaintenance,
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
    /// The thread exists (the session is configured).
    pub thread_started: bool,
    /// Signed in with a ChatGPT backend (`/usage`).
    pub has_backend_auth: bool,
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

fn open(target: Target, command: SlashCommand, args: &str) -> Outcome {
    Outcome::Open(Open {
        target,
        command,
        args: args.to_string(),
    })
}

fn session(command: SlashCommand, intent: SessionIntent) -> Outcome {
    Outcome::Session { command, intent }
}

fn some(args: &str) -> Option<String> {
    (!args.is_empty()).then(|| args.to_string())
}

/// Dispatches a recognized command with the text after its name.
pub fn dispatch_command(command: SlashCommand, args: &str, cx: &DispatchCx) -> Outcome {
    if cx.side_conversation && !command.available_in_side_conversation() {
        return Outcome::Notice(Notice::UnavailableInSide(command));
    }
    if cx.review_mode && matches!(command, SlashCommand::Side | SlashCommand::Btw) {
        return Outcome::Notice(Notice::UnavailableInReview(command));
    }
    // `slash_dispatch.rs:163-165`: `/resume` and `/cd` are also blocked while
    // a turn runs or is about to start. (`task_running` is both facts.)
    if cx.task_running
        && (!command.available_during_task()
            || matches!(command, SlashCommand::Resume | SlashCommand::Cd))
    {
        return Outcome::Notice(Notice::DisabledDuringTask(command));
    }
    if !command.supports_inline_args() || args.is_empty() {
        dispatch_bare(command, cx)
    } else {
        dispatch_with_args(command, args, cx)
    }
}

/// `chatwidget/slash_dispatch.rs`, `dispatch_command` (bare commands).
fn dispatch_bare(command: SlashCommand, cx: &DispatchCx) -> Outcome {
    use SlashCommand::*;
    match command {
        Model => {
            if cx.thread_started {
                open(Target::Select, command, "")
            } else {
                // `model_popups.rs:19-24`: an info line.
                Outcome::Notice(Notice::ModelStartup)
            }
        }
        Presets | AutoReview | Archive | Delete | Daemon | Review | Permissions | Apps => {
            open(Target::Select, command, "")
        }
        Skills | Config | Theme | Title | Statusline | Experimental | Memories | Keymap | Tui => {
            open(Target::Settings, command, "")
        }
        // a feature's own sheet: the feature says which command opens it (`Feature::OPENS`)
        Export | Plugins | Ps | Mcp => match target_of(command) {
            Some(feature) => open(Target::Feature(feature), command, ""),
            None => Outcome::Notice(Notice::NotAvailable(command)),
        },
        Agents => open(Target::Agents, command, ""),
        MultiAgents => open(Target::AgentPicker, command, ""),
        Resume => open(Target::Resume, command, ""),
        Import => open(Target::Import, command, ""),
        Diff => open(Target::Pager, command, ""),
        Status => session(command, SessionIntent::Status),
        Warnings | Feedback | Rename => open(Target::Dialogs, command, ""),
        Pets => open(Target::Pets, command, ""),
        TestApproval => open(Target::Approvals, command, ""),
        Usage => {
            if cx.has_backend_auth {
                open(Target::Usage, command, "")
            } else {
                Outcome::Notice(Notice::UsageNeedsLogin)
            }
        }
        Voice => session(command, SessionIntent::VoiceControl(VoiceControl::Toggle)),
        Mention => Outcome::Insert("@"),
        Quit | Exit => Outcome::Quit,
        App | Recap if !cx.thread_started => Outcome::Notice(Notice::SessionStarting(command)),
        App => session(command, SessionIntent::App),
        Recap => session(command, SessionIntent::Recap),
        New => session(command, SessionIntent::New { name: None }),
        Clear => session(command, SessionIntent::Clear { name: None }),
        Fork => session(command, SessionIntent::Fork { name: None }),
        Compact => session(command, SessionIntent::Compact),
        Init => session(command, SessionIntent::Init),
        Worktree => session(command, SessionIntent::Worktree),
        Plan => session(command, SessionIntent::Plan { prompt: None }),
        Goal => session(command, SessionIntent::Goal { args: None }),
        Side | Btw => session(command, SessionIntent::Side { prompt: None }),
        Copy => session(command, SessionIntent::Copy),
        Raw => session(command, SessionIntent::Raw { on: None }),
        // `slash_dispatch.rs:557-559`: a bare `/cd` is `/cd ~`.
        Cd => session(
            command,
            SessionIntent::Cd {
                path: "~".to_string(),
            },
        ),
        Pwd => session(command, SessionIntent::Pwd),
        Logout => session(command, SessionIntent::Logout),
        Vim => session(command, SessionIntent::Vim),
        Ide => session(command, SessionIntent::Ide { args: None }),
        Daybreak => session(command, SessionIntent::Daybreak),
        ElevateSandbox => session(command, SessionIntent::ElevateSandbox),
        Hooks => session(command, SessionIntent::Hooks),
        DebugConfig => session(command, SessionIntent::DebugConfig),
        Stop => session(command, SessionIntent::Stop),
        Rollout => session(command, SessionIntent::Rollout),
        MemoryDrop | MemoryUpdate => session(command, SessionIntent::MemoryMaintenance),
    }
}

/// `chatwidget/slash_dispatch.rs`, commands with inline arguments.
fn dispatch_with_args(command: SlashCommand, args: &str, cx: &DispatchCx) -> Outcome {
    use SlashCommand::*;
    let args = args.trim();
    match command {
        Review => session(command, SessionIntent::Review { args: some(args) }),
        Model => session(command, SessionIntent::Model { args: some(args) }),
        Permissions => session(command, SessionIntent::Permissions { args: some(args) }),
        Title => session(command, SessionIntent::Title { args: some(args) }),
        Statusline => session(command, SessionIntent::Statusline { args: some(args) }),
        Rename => session(command, SessionIntent::Rename { name: some(args) }),
        New => session(command, SessionIntent::New { name: some(args) }),
        Clear => session(command, SessionIntent::Clear { name: some(args) }),
        Fork => session(command, SessionIntent::Fork { name: some(args) }),
        Plan => session(command, SessionIntent::Plan { prompt: some(args) }),
        Goal => session(command, SessionIntent::Goal { args: some(args) }),
        Side | Btw => session(command, SessionIntent::Side { prompt: some(args) }),
        Ide => session(command, SessionIntent::Ide { args: some(args) }),
        Export if args.is_empty() => dispatch_bare(command, cx),
        Export => session(
            command,
            SessionIntent::Export {
                path: args.to_string(),
            },
        ),
        Cd => session(
            command,
            SessionIntent::Cd {
                path: args.to_string(),
            },
        ),
        Resume => session(
            command,
            SessionIntent::ResumeNamed {
                name: args.to_string(),
            },
        ),
        Pets => open(Target::Pets, command, args),
        Pwd => Outcome::Notice(Notice::Usage(UsageKey::Pwd)),
        Usage => {
            if !cx.has_backend_auth {
                return Outcome::Notice(Notice::UsageNeedsLogin);
            }
            match TokenActivityView::parse(args) {
                Some(view) => open(Target::Usage, command, view.arg()),
                None => Outcome::Notice(Notice::Usage(UsageKey::Usage)),
            }
        }
        Voice => match args.to_ascii_lowercase().as_str() {
            "settings" => open(Target::Voice, command, "settings"),
            "mute" => session(command, SessionIntent::VoiceControl(VoiceControl::Mute)),
            "stop" => session(command, SessionIntent::VoiceControl(VoiceControl::Stop)),
            _ => Outcome::Notice(Notice::Usage(UsageKey::Voice)),
        },
        Keymap => match args.to_ascii_lowercase().as_str() {
            "debug" => open(Target::Settings, command, "debug"),
            _ => Outcome::Notice(Notice::Usage(UsageKey::Keymap)),
        },
        Raw => match args.to_ascii_lowercase().as_str() {
            "on" => session(command, SessionIntent::Raw { on: Some(true) }),
            "off" => session(command, SessionIntent::Raw { on: Some(false) }),
            _ => Outcome::Notice(Notice::Usage(UsageKey::Raw)),
        },
        Mcp => dispatch_mcp(args, cx),
        // A command without inline arguments never gets here.
        _ => dispatch_bare(command, cx),
    }
}

fn dispatch_mcp(args: &str, cx: &DispatchCx) -> Outcome {
    let command = SlashCommand::Mcp;
    if args.eq_ignore_ascii_case("verbose") {
        return session(
            command,
            SessionIntent::Mcp {
                args: Some(args.to_string()),
            },
        );
    }
    if let Some((word, name)) = args.split_once(' ')
        && word.eq_ignore_ascii_case("login")
        && !name.trim().is_empty()
    {
        if !cx.thread_started {
            return Outcome::Notice(Notice::McpLoginNeedsSession);
        }
        return session(
            command,
            SessionIntent::Mcp {
                args: Some(args.to_string()),
            },
        );
    }
    Outcome::Notice(Notice::Usage(UsageKey::Mcp))
}
