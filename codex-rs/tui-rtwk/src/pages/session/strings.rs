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
use codex_tui_core::screens::chat::{McpHeader, McpWarning, Retry, StatusHeader};
use codex_tui_core::screens::composer;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

pub(super) const INLINE_GAP: &str = " ";
pub(super) const MORE: &str = " …";
pub(super) const LINE_BREAK: &str = "\n";
const LIST_SEPARATOR: &str = ", ";
const ELLIPSIS: &str = "…";

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
    /// `USER_SHELL_COMMAND_HELP_TITLE` and `_HINT` (`chatwidget.rs`): a bare `!`.
    pub shell_help: &'static str,
    /// `user_input_too_large_message` (`chat_composer.rs`): the limit and the size.
    pub input_too_large: Msg<2>,
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
    /// `slash_dispatch.rs`, `chatwidget.rs`: what a command that runs at once
    /// says (`add_info_message`).
    pub working_directory: Msg<1>,
    pub rollout: Msg<1>,
    pub rollout_missing: &'static str,
    pub desktop_opened: &'static str,
    pub stopping_terminals: &'static str,
    pub raw_output_on: &'static str,
    pub raw_output_off: &'static str,
    /// `add_app_server_stub_message`: the feature, then the message.
    pub stub: Msg<1>,
    pub stub_memory_maintenance: &'static str,
    /// `PARENT_OWNED_INPUT_MESSAGE`.
    pub parent_owned: &'static str,
    /// `working_directory.rs`, `event_dispatch.rs`: why `/cd` did nothing.
    pub cd_not_started: &'static str,
    pub cd_not_idle: &'static str,
    pub cd_history_not_saved: &'static str,
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
    /// `actionable_banner.rs` / `set_inline_banner`: the hint under the
    /// banner's actions, and under an information-only one.
    pub banner_choose: &'static str,
    pub banner_choose_dismiss: &'static str,
    pub banner_dismiss: &'static str,
    /// The Luna Reserve selector (`backend_banners.rs`): its local row, and the footer that
    /// names the keys (`accept_cancel_hint_line`).
    pub reserve_continue: &'static str,
    reserve_hint: Msg<2>,
    pub warning_singular: &'static str,
    pub warning_plural: &'static str,
    pub warning_key: &'static str,
    pub warning_view: &'static str,
    /// `mcp_startup.rs`: the status header of one server, of several (the
    /// answered, the total, the servers still starting), and the warnings.
    pub mcp_booting: Msg<1>,
    pub mcp_starting: Msg<3>,
    pub mcp_failed: Msg<1>,
    pub mcp_interrupted: Msg<1>,
    pub mcp_incomplete: Msg<1>,
    /// `protocol.rs`: a 429 reconnect reads as the server being busy.
    pub retry_busy: Msg<1>,
    /// `on_server_overloaded_error`: an overloaded server that said nothing.
    pub high_load: &'static str,
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
    shell_help: "Prefix a command with ! to run it locally\nExample: !ls",
    input_too_large: msg!("Message exceeds the maximum length of {0} characters ({1} provided)."),
    usage_needs_login: "Sign in with ChatGPT to use /usage.",
    usage_pwd: "Usage: /pwd",
    usage_usage: "Usage: /usage [daily|weekly|cumulative]",
    usage_raw: "Usage: /raw [on|off]",
    usage_voice: "Usage: /voice [settings|mute|stop]",
    usage_keymap: "Usage: /keymap [debug]",
    usage_mcp: "Usage: /mcp [verbose | login <name>]",
    mcp_login_needs_session: "MCP sign-in requires an active session.",
    not_available: msg!("'/{0}' is not available yet."),
    working_directory: msg!("Current working directory: {0}"),
    rollout: msg!("Current rollout path: {0}"),
    rollout_missing: "Rollout path is not available yet.",
    desktop_opened: "Opened this session in the Desktop app.",
    stopping_terminals: "Stopping all background terminals.",
    raw_output_on: "Raw output mode on: transcript text is shown for clean terminal selection.",
    raw_output_off: "Raw output mode off: rich transcript rendering restored.",
    stub: msg!("{0}: Not available in TUI yet."),
    stub_memory_maintenance: "Memory maintenance",
    parent_owned: "This sub-agent is controlled by its parent. Direct input is disabled.",
    cd_not_started: "The session must start before you can change its working directory.",
    cd_not_idle: "Changing directories requires an idle primary session without queued input.",
    cd_history_not_saved: "Conversation history is not saved.",
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
    banner_choose: "Press a number to choose",
    banner_choose_dismiss: "Press a number to choose \u{b7} esc to dismiss \u{b7} type to continue",
    banner_dismiss: "esc to dismiss \u{b7} type to continue",
    reserve_continue: "Continue with Luna Reserve",
    reserve_hint: msg!("Press {0} to confirm or {1} to continue working"),
    warning_singular: "warning",
    warning_plural: "warnings",
    warning_key: "f2",
    warning_view: " to view",
    mcp_booting: msg!("Booting MCP server: {0}"),
    mcp_starting: msg!("Starting MCP servers ({0}/{1}): {2}"),
    mcp_failed: msg!("MCP client for `{0}` failed to start"),
    mcp_interrupted: msg!(
        "MCP startup interrupted. The following servers were not initialized: {0}"
    ),
    mcp_incomplete: msg!("MCP startup incomplete (failed: {0})"),
    retry_busy: msg!("Server is busy, reconnecting {0}"),
    high_load: "Codex is currently experiencing high load.",
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
    /// The hint under a backend banner.
    pub(super) fn banner_hint(
        &self,
        hint: crate::pages::session::banners::BannerHint,
    ) -> Option<&'static str> {
        use crate::pages::session::banners::BannerHint;
        match hint {
            BannerHint::Choose => Some(self.banner_choose),
            BannerHint::ChooseOrDismiss => Some(self.banner_choose_dismiss),
            BannerHint::Dismiss => Some(self.banner_dismiss),
            BannerHint::None => None,
        }
    }

    /// The Luna selector's footer, with the keys the list's keymap names.
    pub(super) fn reserve_hint(&self, accept: &str, cancel: &str) -> String {
        Fmt::new(
            self.reserve_hint,
            [Arg::Text(accept), Arg::Text(cancel)],
            &self.num,
        )
        .to_string()
    }

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

    fn one(&self, message: Msg<1>, arg: &str) -> String {
        Fmt::new(message, [Arg::Text(arg)], &self.num).to_string()
    }

    /// The status row's header when a retry or MCP startup owns it.
    pub fn status_header(&self, header: &StatusHeader) -> String {
        match header {
            StatusHeader::Retry(Retry::Server(message)) => message.clone(),
            StatusHeader::Retry(Retry::Busy { attempt }) => self.one(self.retry_busy, attempt),
            StatusHeader::Mcp(McpHeader::Booting { server }) => self.one(self.mcp_booting, server),
            StatusHeader::Mcp(McpHeader::Starting {
                completed,
                total,
                servers,
                more,
            }) => {
                let mut named = servers.join(LIST_SEPARATOR);
                if *more {
                    named.push_str(LIST_SEPARATOR);
                    named.push_str(ELLIPSIS);
                }
                Fmt::new(
                    self.mcp_starting,
                    [
                        Arg::Text(&completed.to_string()),
                        Arg::Text(&total.to_string()),
                        Arg::Text(&named),
                    ],
                    &self.num,
                )
                .to_string()
            }
        }
    }

    /// The words of a warning MCP startup added to the history.
    pub fn mcp_warning(&self, warning: &McpWarning) -> String {
        match warning {
            McpWarning::Failed { server, error } => match error {
                Some(error) => error.clone(),
                None => self.one(self.mcp_failed, server),
            },
            McpWarning::Interrupted { servers } => {
                self.one(self.mcp_interrupted, &servers.join(LIST_SEPARATOR))
            }
            McpWarning::Incomplete { servers } => {
                self.one(self.mcp_incomplete, &servers.join(LIST_SEPARATOR))
            }
        }
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

    /// The words of a note of the transcript (a command line's answer).
    pub fn note(&self, key: &codex_tui_core::threads::NoteKey) -> String {
        use codex_tui_core::threads::{CdRefusal, NoteKey, Stub};
        let text = |msg: Msg<1>, arg: &str| Fmt::new(msg, [Arg::Text(arg)], &self.num).to_string();
        match key {
            NoteKey::Command(notice) => self.notice(notice),
            NoteKey::WorkingDirectory(dir) => text(self.working_directory, dir),
            NoteKey::Rollout(Some(path)) => text(self.rollout, path),
            NoteKey::Rollout(None) => self.rollout_missing.to_string(),
            NoteKey::DesktopOpened => self.desktop_opened.to_string(),
            NoteKey::StoppingTerminals => self.stopping_terminals.to_string(),
            NoteKey::Stub(Stub::MemoryMaintenance) => text(self.stub, self.stub_memory_maintenance),
            NoteKey::RawOutput(true) => self.raw_output_on.to_string(),
            NoteKey::RawOutput(false) => self.raw_output_off.to_string(),
            NoteKey::ParentOwned => self.parent_owned.to_string(),
            NoteKey::CdRefused(CdRefusal::NotStarted) => self.cd_not_started.to_string(),
            NoteKey::CdRefused(CdRefusal::NotIdle) => self.cd_not_idle.to_string(),
            NoteKey::CdRefused(CdRefusal::HistoryNotSaved) => self.cd_history_not_saved.to_string(),
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
            Notice::ShellHelp => self.shell_help.to_string(),
            Notice::InputTooLarge { provided } => Fmt::new(
                self.input_too_large,
                [
                    Arg::Text(&composer::MAX_INPUT_CHARS.to_string()),
                    Arg::Text(&provided.to_string()),
                ],
                &self.num,
            )
            .to_string(),
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
