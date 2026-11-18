//! The decision a slash command leads to (`chatwidget/slash_dispatch.rs`:
//! `dispatch_command`, `dispatch_prepared_command_with_args`): open a screen
//! of the app, ask the session for something (a typed intent), insert text,
//! quit, or say why not (a `Notice`, a key the UI's tables turn into words).
//! A pure function of the line and of a few facts of the session.

use super::CommandFlags;
use super::SlashCommand;

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
        todo!()
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
    let _ = (line, cx);
    todo!()
}

/// Dispatches a recognized command with the text after its name.
pub fn dispatch_command(command: SlashCommand, args: &str, cx: &DispatchCx) -> Outcome {
    let _ = (command, args, cx);
    todo!()
}
