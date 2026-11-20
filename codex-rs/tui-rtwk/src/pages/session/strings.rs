//! The strings of `chat` (S10): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use codex_tui_core::commands::{Notice, SlashCommand, UsageKey};

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

pub(super) const INLINE_GAP: &str = " ";
pub(super) const MORE: &str = " …";
pub(super) const LINE_BREAK: &str = "\n";

/// The strings of this screen.
#[derive(Clone, Copy, Debug)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// What the composer starts its line with (`chat_composer.rs`).
    pub prompt: &'static str,
    /// The description of a command in the popup (`slash_command.rs`).
    pub description: fn(SlashCommand) -> &'static str,
    /// `slash_dispatch.rs`: the messages a command line can end in.
    pub unrecognized: Msg<1>,
    pub disabled_during_task: Msg<1>,
    pub unavailable_in_side: Msg<1>,
    pub unavailable_in_review: Msg<1>,
    pub session_starting: Msg<1>,
    pub model_startup: &'static str,
    pub usage_needs_login: &'static str,
    pub usage_pwd: &'static str,
    pub usage_usage: &'static str,
    pub usage_raw: &'static str,
    pub usage_voice: &'static str,
    pub usage_keymap: &'static str,
    pub usage_mcp: &'static str,
    pub mcp_login_needs_session: &'static str,
    /// The part of a command that needs the session (session-core) is not
    /// built yet: the slice's own text, not the oracle's.
    pub not_available: Msg<1>,
    /// `chat_composer/history_search.rs`: the footer while searching.
    pub search_idle: Msg<1>,
    pub search_match: Msg<1>,
    pub search_no_match: Msg<1>,
    /// `skill_popup.rs`, `mentions_v2/candidate.rs`: the tags of a row.
    pub tag_skill: &'static str,
    pub tag_app: &'static str,
    pub tag_plugin: &'static str,
    /// `chat_composer/warning_notice.rs` (`warning_labels`): the footer's
    /// `⚠ N warnings · f2 to view`.
    pub warning_badge: &'static str,
    /// `bottom_pane`/`chatwidget/rendering.rs`: the backend control above the
    /// composer, `Cloud  ⌃x b to cycle` (`BackendSessions::displayed_name`).
    pub backend_local: &'static str,
    pub backend_cloud: &'static str,
    pub backend_connection: Msg<1>,
    pub backend_key: &'static str,
    /// Oracle: `chatwidget/rendering.rs` (the external writer view): the
    /// session is open in another process and shown read-only.
    pub read_only_title: &'static str,
    pub header_prompt: &'static str,
    pub header_title: &'static str,
    pub header_version: Msg<1>,
    pub header_permissions: &'static str,
    pub header_yolo: &'static str,
    pub read_only_hint: &'static str,
    pub running_command: &'static str,
    pub ran_command: &'static str,
    pub user_ran_command: &'static str,
    pub calling_tool: &'static str,
    pub called_tool: &'static str,
    pub failed_tool: &'static str,
    pub tool_result_image: &'static str,
    pub tool_result_audio: &'static str,
    pub image_label: Msg<1>,
    pub explored: &'static str,
    pub exploring: &'static str,
    pub explored_read: Msg<1>,
    pub explored_list: Msg<1>,
    pub explored_search: Msg<1>,
    pub explored_search_in: Msg<2>,
    pub tool_no_output: &'static str,
    pub show_tool_details: &'static str,
    pub hide_tool_details: &'static str,
    pub hidden_output_lines: Msg<1>,
    pub hidden_output_line: &'static str,
    pub command_exited: Msg<1>,
    pub shell_prompt: &'static str,
    pub exit_ok: &'static str,
    pub exit_failed: &'static str,
    pub exit_status: Msg<1>,
    pub worked_for: Msg<1>,
    pub writer_retry_key: &'static str,
    pub writer_retry_detail: &'static str,
    pub writer_retry: &'static str,
    pub writer_fork: &'static str,
    pub writer_center: &'static str,
    pub writer_exit: &'static str,
    pub writer_transcript: &'static str,
    pub backend_cycle: &'static str,
    pub warning_singular: &'static str,
    pub warning_plural: &'static str,
    pub warning_key: &'static str,
    pub warning_view: &'static str,
    /// The hint of the `/` popup is the framework's; this is the empty one.
    pub num: NumSyms,
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "main chat screen",
    prompt: "› ",
    description: description_en,
    unrecognized: msg!("Unrecognized command '/{0}'. Type \"/\" for a list of supported commands."),
    disabled_during_task: msg!("'/{0}' is disabled while a task is in progress."),
    unavailable_in_side: msg!(
        "'/{0}' is unavailable in side conversations. Press Ctrl+C to return to the main thread first."
    ),
    unavailable_in_review: msg!("'/{0}' is unavailable while code review is running."),
    session_starting: msg!("Session is still starting; try /{0} again in a moment."),
    model_startup: "Model selection is disabled until startup completes.",
    usage_needs_login: "Sign in with ChatGPT to use /usage.",
    usage_pwd: "Usage: /pwd",
    usage_usage: "Usage: /usage [daily|weekly|cumulative]",
    usage_raw: "Usage: /raw [on|off]",
    usage_voice: "Usage: /voice [settings|mute|stop]",
    usage_keymap: "Usage: /keymap [debug]",
    usage_mcp: "Usage: /mcp [verbose | login <name>]",
    mcp_login_needs_session: "MCP sign-in requires an active session.",
    not_available: msg!("'/{0}' is not available yet."),
    search_idle: msg!("reverse-i-search: {0}"),
    search_match: msg!("reverse-i-search: {0}  enter accept \u{b7} esc cancel"),
    search_no_match: msg!("reverse-i-search: {0}  no match"),
    tag_skill: "[Skill]",
    tag_app: "[App]",
    tag_plugin: "Plugin",
    warning_badge: "\u{26a0} ",
    backend_local: "Local",
    backend_cloud: "Cloud",
    backend_connection: msg!("Connection {0}"),
    backend_key: "\u{2303}x b",
    read_only_title: "This conversation is open in another app",
    header_prompt: ">_ ",
    header_title: "OpenAI Codex",
    header_version: msg!(" (v{0})"),
    header_permissions: "permissions: ",
    header_yolo: "YOLO mode",
    read_only_hint: "Close it there and press R to continue here.",
    running_command: "Running",
    ran_command: "Ran",
    user_ran_command: "You ran",
    calling_tool: "Calling",
    called_tool: "Called",
    failed_tool: "Failed",
    tool_result_image: "Returned image",
    tool_result_audio: "Returned audio",
    image_label: msg!("[Image #{0}]"),
    explored: "Explored",
    exploring: "Exploring",
    explored_read: msg!("Read {0}"),
    explored_list: msg!("List {0}"),
    explored_search: msg!("Search {0}"),
    explored_search_in: msg!("Search {0} in {1}"),
    tool_no_output: "(no output)",
    show_tool_details: "+ Show details",
    hide_tool_details: "− Hide details",
    hidden_output_lines: msg!("+ {0} lines (⌃t to expand)"),
    hidden_output_line: "+ 1 line (⌃t to expand)",
    command_exited: msg!("Exited (status {0})"),
    shell_prompt: "$",
    exit_ok: "✓",
    exit_failed: "✗",
    exit_status: msg!(" ({0})"),
    worked_for: msg!("Worked for {0}"),
    writer_retry_key: "r",
    writer_retry_detail: " to retry",
    writer_retry: "retry",
    writer_fork: "fork",
    writer_center: "command center",
    writer_exit: "exit",
    writer_transcript: "transcript",
    backend_cycle: " to cycle",
    warning_singular: "warning",
    warning_plural: "warnings",
    warning_key: "f2",
    warning_view: " to view",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The title of the screen in `lang`, for code with no tree (the debug
/// menu's rows).
pub fn title(lang: Lang) -> &'static str {
    TABLES.pick(lang).title
}

/// The description of a command in the popup (`slash_command.rs`).
pub fn description(lang: Lang, command: SlashCommand) -> &'static str {
    (TABLES.pick(lang).description)(command)
}

impl Strings {
    pub(super) fn search_in(&self, query: &str, path: &str) -> String {
        Fmt::new(
            self.explored_search_in,
            [Arg::Text(query), Arg::Text(path)],
            &self.num,
        )
        .to_string()
    }

    pub(super) fn transcript_label(&self, message: Msg<1>, arg: &str) -> String {
        Fmt::new(message, [Arg::Text(arg)], &self.num).to_string()
    }

    pub(super) fn worked_for<'a>(&'a self, elapsed: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.worked_for, [Arg::Text(elapsed)], &self.num)
    }

    /// The name of a backend on the control.
    pub fn backend(&self, label: codex_tui_core::app::BackendLabel) -> String {
        use codex_tui_core::app::BackendLabel;
        match label {
            BackendLabel::Local => self.backend_local.to_owned(),
            BackendLabel::Cloud => self.backend_cloud.to_owned(),
            BackendLabel::Connection(n) => Fmt::new(
                self.backend_connection,
                [Arg::Text(&n.to_string())],
                &self.num,
            )
            .to_string(),
        }
    }

    /// The words of a notice a command line ended in.
    pub fn notice(&self, notice: &Notice) -> String {
        let text =
            |msg: Msg<1>, name: &str| Fmt::new(msg, [Arg::Text(name)], &self.num).to_string();
        match notice {
            Notice::Unrecognized(name) => text(self.unrecognized, name),
            Notice::DisabledDuringTask(c) => text(self.disabled_during_task, c.command()),
            Notice::UnavailableInSide(c) => text(self.unavailable_in_side, c.command()),
            Notice::UnavailableInReview(c) => text(self.unavailable_in_review, c.command()),
            Notice::SessionStarting(c) => text(self.session_starting, c.command()),
            Notice::NotAvailable(c) => text(self.not_available, c.command()),
            Notice::ModelStartup => self.model_startup.to_string(),
            Notice::UsageNeedsLogin => self.usage_needs_login.to_string(),
            Notice::McpLoginNeedsSession => self.mcp_login_needs_session.to_string(),
            Notice::Usage(key) => match key {
                UsageKey::Pwd => self.usage_pwd,
                UsageKey::Usage => self.usage_usage,
                UsageKey::Raw => self.usage_raw,
                UsageKey::Voice => self.usage_voice,
                UsageKey::Keymap => self.usage_keymap,
                UsageKey::Mcp => self.usage_mcp,
            }
            .to_string(),
        }
    }

    /// The footer while history search is open.
    pub fn search_footer(&self, query: &str, status: Search) -> String {
        let msg = match status {
            Search::Idle => self.search_idle,
            Search::Match => self.search_match,
            Search::NoMatch => self.search_no_match,
        };
        Fmt::new(msg, [Arg::Text(query)], &self.num).to_string()
    }
}

/// Which footer of history search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    Idle,
    Match,
    NoMatch,
}

/// The oracle's `SlashCommand::description` (`slash_command.rs`), exhaustive
/// so a new command cannot go without a description.
fn description_en(command: SlashCommand) -> &'static str {
    match command {
        SlashCommand::Config => "configure Codex preferences",
        SlashCommand::Feedback => "send logs to maintainers or save a ZIP",
        SlashCommand::New => "start a new chat during a conversation",
        SlashCommand::Init => "create an AGENTS.md file with instructions for Codex",
        SlashCommand::Compact => "summarize conversation to prevent hitting the context limit",
        SlashCommand::Recap => "summarize the current conversation now",
        SlashCommand::Review => "review my current changes and find issues",
        SlashCommand::Rename => "rename the current thread",
        SlashCommand::Resume => "resume a saved chat",
        SlashCommand::Archive => "archive this session",
        SlashCommand::Delete => "permanently delete this session",
        SlashCommand::Clear => "clear the terminal and start a new chat",
        SlashCommand::Fork => "fork the current chat",
        SlashCommand::Worktree => "start or continue a conversation in a new worktree",
        SlashCommand::App => "continue this session in the Desktop app",
        SlashCommand::Quit | SlashCommand::Exit => "exit Codex",
        SlashCommand::Copy => "copy the last response or part of it",
        SlashCommand::Export => "export the conversation as markdown",
        SlashCommand::Raw => "toggle raw scrollback mode for copy-friendly terminal selection",
        SlashCommand::Tui => "choose the TUI mode for the next launch",
        SlashCommand::Diff => "show git diff (including untracked files)",
        SlashCommand::Mention => "mention a file",
        SlashCommand::Skills => "use skills to improve how Codex performs specific tasks",
        SlashCommand::Import => "import setup, this project, and recent chats from Claude Code",
        SlashCommand::Hooks => "view and manage lifecycle hooks",
        SlashCommand::Daemon => "Manage the local background server",
        SlashCommand::Warnings => "view retained warnings and diagnostic details",
        SlashCommand::Status => "show current session configuration and token usage",
        SlashCommand::Cd => "change the current working directory",
        SlashCommand::Pwd => "show the current working directory",
        SlashCommand::Usage => "view account usage or use a usage limit reset",
        SlashCommand::DebugConfig => "show config layers and requirement sources for debugging",
        SlashCommand::Title => "configure which items appear in the terminal title",
        SlashCommand::Statusline => "configure which items appear in the status line",
        SlashCommand::Theme => "choose a syntax highlighting theme",
        SlashCommand::Pets => "choose or hide the terminal pet",
        SlashCommand::Ps => "list background terminals",
        SlashCommand::Stop => "stop all background terminals",
        SlashCommand::MemoryDrop => "DO NOT USE",
        SlashCommand::MemoryUpdate => "DO NOT USE",
        SlashCommand::Model => "choose what model and reasoning effort to use",
        SlashCommand::Presets => "run or save reusable prompts and slash commands",
        SlashCommand::Daybreak => "turn Daybreak on or off",
        SlashCommand::Ide => {
            "include current selection, open files, and other context from your IDE"
        }
        SlashCommand::Plan => "switch to Plan mode",
        SlashCommand::Voice => "start or stop voice; use /voice settings to choose a voice",
        SlashCommand::Goal => "set or view the goal for a long-running task",
        SlashCommand::Agents => "open the agent command center",
        SlashCommand::MultiAgents => "switch between this session's subagents",
        SlashCommand::Side | SlashCommand::Btw => "start a side conversation",
        SlashCommand::Permissions => "choose what Codex is allowed to do",
        SlashCommand::Keymap => "remap TUI shortcuts",
        SlashCommand::Vim => "toggle Vim mode for the composer",
        SlashCommand::ElevateSandbox => "set up elevated agent sandbox",
        SlashCommand::Experimental => "toggle experimental features",
        SlashCommand::AutoReview => "approve one retry of a recent auto-review denial",
        SlashCommand::Memories => "configure memory use and generation",
        SlashCommand::Mcp => "list MCP tools; use /mcp verbose or /mcp login <name>",
        SlashCommand::Apps => "manage apps",
        SlashCommand::Plugins => "browse plugins",
        SlashCommand::Logout => "log out of Codex",
        SlashCommand::Rollout => "print the rollout file path",
        SlashCommand::TestApproval => "test approval request",
    }
}
