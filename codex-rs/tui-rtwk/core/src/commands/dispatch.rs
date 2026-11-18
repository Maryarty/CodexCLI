//! The decision a slash command leads to (`chatwidget/slash_dispatch.rs`:
//! `dispatch_command`, `dispatch_prepared_command_with_args`): open a screen
//! of the app, ask the session for something (a typed intent), insert text,
//! quit, or say why not (a `Notice`, a key the UI's tables turn into words).
//! A pure function of the line and of a few facts of the session.

use super::CommandFlags;
use super::SlashCommand;
use super::find_builtin_command;
use super::parse_slash_name;

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
    /// The status card (S19).
    Status,
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
}

impl Target {
    /// The name of the screen directory (`screens::dirs()`).
    pub fn dir(self) -> &'static str {
        match self {
            Target::Resume => "resume",
            Target::Agents => "agents",
            Target::Import => "import",
            Target::Usage => "usage",
            Target::Pager => "pager",
            Target::Status => "status",
            Target::Select => "select",
            Target::Settings => "settings",
            Target::Dialogs => "dialogs",
            Target::Pets => "pets",
            Target::Voice => "voice",
            Target::Approvals => "approvals",
            Target::Backtrack => "backtrack",
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
    /// `'/cmd' is unavailable while code review is running.`
    UnavailableInReview(SlashCommand),
    /// `Session is still starting; try /cmd again in a moment.`
    SessionStarting(SlashCommand),
    /// `Sign in with ChatGPT to use /usage.`
    UsageNeedsLogin,
    /// A usage line: `Usage: /pwd`, `Usage: /raw [on|off]`, ...
    Usage(UsageKey),
    /// `MCP sign-in requires an active session.`
    McpLoginNeedsSession,
    /// The session's part of the command is not built yet (session-core).
    NotAvailable(SlashCommand),
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
    McpLogin,
}

/// What the session is asked for: the commands that act on the thread or the
/// connection. session-core gives each its effect; until then the chat
/// answers `Notice::NotAvailable`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionIntent {
    New { name: Option<String> },
    Clear { name: Option<String> },
    Fork { name: Option<String> },
    Compact,
    Init,
    Recap,
    Archive,
    Delete,
    Worktree,
    Rename { name: Option<String> },
    Plan { prompt: Option<String> },
    Goal { args: Option<String> },
    Side { prompt: Option<String> },
    Copy,
    Export { path: Option<String> },
    Raw { on: Option<bool> },
    Tui,
    Cd { path: String },
    Pwd,
    Mcp { args: Option<String> },
    Apps,
    Plugins,
    Logout,
    Vim,
    Ide { args: Option<String> },
    Daybreak,
    ElevateSandbox,
    Hooks,
    DebugConfig,
    Ps,
    Stop,
    Rollout,
    App,
    Review { args: Option<String> },
    Model { args: Option<String> },
    Permissions { args: Option<String> },
    Title { args: Option<String> },
    Statusline { args: Option<String> },
    VoiceControl(VoiceControl),
    ResumeNamed { name: String },
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

/// Dispatches a composer line that starts with `/`.
pub fn dispatch(line: &str, cx: &DispatchCx) -> Outcome {
    let Some((name, rest, _)) = parse_slash_name(line) else {
        return Outcome::NotACommand;
    };
    // `/usr/bin/env`: a path, not a command.
    if name.contains('/') {
        return Outcome::NotACommand;
    }
    match find_builtin_command(name, cx.flags) {
        Some(command) => dispatch_command(command, rest.trim_end(), cx),
        None => Outcome::Notice(Notice::Unrecognized(name.to_string())),
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
    if cx.task_running && !command.available_during_task() {
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
                Outcome::Notice(Notice::SessionStarting(command))
            }
        }
        Presets | AutoReview | Archive | Delete | Daemon | Review | Permissions => {
            open(Target::Select, command, "")
        }
        Skills | Config | Theme | Title | Statusline | Experimental | Memories | Keymap | Tui => {
            open(Target::Settings, command, "")
        }
        Agents | MultiAgents => open(Target::Agents, command, ""),
        Resume => open(Target::Resume, command, ""),
        Import => open(Target::Import, command, ""),
        Diff => open(Target::Pager, command, ""),
        Status => open(Target::Status, command, ""),
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
        Export => session(command, SessionIntent::Export { path: None }),
        Raw => session(command, SessionIntent::Raw { on: None }),
        Cd => session(
            command,
            SessionIntent::Cd {
                path: String::new(),
            },
        ),
        Pwd => session(command, SessionIntent::Pwd),
        Mcp => session(command, SessionIntent::Mcp { args: None }),
        Apps => session(command, SessionIntent::Apps),
        Plugins => session(command, SessionIntent::Plugins),
        Logout => session(command, SessionIntent::Logout),
        Vim => session(command, SessionIntent::Vim),
        Ide => session(command, SessionIntent::Ide { args: None }),
        Daybreak => session(command, SessionIntent::Daybreak),
        ElevateSandbox => session(command, SessionIntent::ElevateSandbox),
        Hooks => session(command, SessionIntent::Hooks),
        DebugConfig => session(command, SessionIntent::DebugConfig),
        Ps => session(command, SessionIntent::Ps),
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
        Export => session(command, SessionIntent::Export { path: some(args) }),
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
            match args {
                "daily" | "weekly" | "cumulative" => open(Target::Usage, command, args),
                _ => Outcome::Notice(Notice::Usage(UsageKey::Usage)),
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
        let has_environment = name
            .split_whitespace()
            .any(|w| w == "--environment" || w.starts_with("--environment="));
        // With `--environment` the name is what is left of the words: none
        // left is the login usage line.
        if has_environment {
            let mut words = name.split_whitespace();
            let mut positionals = 0;
            while let Some(word) = words.next() {
                if word == "--environment" {
                    words.next();
                } else if !word.starts_with("--environment=") {
                    positionals += 1;
                }
            }
            if positionals == 0 {
                return Outcome::Notice(Notice::Usage(UsageKey::McpLogin));
            }
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
