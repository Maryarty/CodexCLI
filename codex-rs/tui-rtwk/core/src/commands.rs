//! Slash commands (reuse-map ADAPT rows), carried from codex-tui's
//! `slash_command.rs`, `bottom_pane/slash_commands.rs`, `command_popup.rs`
//! (the filter) and `bottom_pane/prompt_args.rs` (`parse_slash_name`): the
//! registry, the parse, the popup's rows and, in `dispatch`, the decision a
//! typed or selected command leads to. Pure data and functions with no prose:
//! the descriptions and messages are the UI's string tables'.

mod dispatch;
mod fuzzy;

pub use dispatch::*;
pub use fuzzy::fuzzy_match;

/// Commands that can be invoked by starting a message with a leading slash.
/// Enum order is presentation order in the popup (not alphabetical: the more
/// frequently used first), as in `slash_command.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlashCommand {
    Model,
    Presets,
    Daybreak,
    Ide,
    Permissions,
    Keymap,
    Vim,
    ElevateSandbox,
    Experimental,
    AutoReview,
    Memories,
    Skills,
    Import,
    Hooks,
    Review,
    Rename,
    New,
    Archive,
    Delete,
    Resume,
    Fork,
    Worktree,
    App,
    Init,
    Compact,
    Config,
    Recap,
    Plan,
    Voice,
    Goal,
    Agents,
    Side,
    Btw,
    Copy,
    Export,
    Raw,
    Tui,
    Diff,
    Mention,
    Status,
    Daemon,
    Warnings,
    Cd,
    Pwd,
    Usage,
    DebugConfig,
    Title,
    Statusline,
    Theme,
    Pets,
    Mcp,
    Apps,
    Plugins,
    Logout,
    Quit,
    Exit,
    Feedback,
    Rollout,
    Ps,
    Stop,
    Clear,
    TestApproval,
    MultiAgents,
    MemoryDrop,
    MemoryUpdate,
}

impl SlashCommand {
    /// Every command, in enum order.
    pub const ALL: [SlashCommand; 65] = [
        SlashCommand::Model,
        SlashCommand::Presets,
        SlashCommand::Daybreak,
        SlashCommand::Ide,
        SlashCommand::Permissions,
        SlashCommand::Keymap,
        SlashCommand::Vim,
        SlashCommand::ElevateSandbox,
        SlashCommand::Experimental,
        SlashCommand::AutoReview,
        SlashCommand::Memories,
        SlashCommand::Skills,
        SlashCommand::Import,
        SlashCommand::Hooks,
        SlashCommand::Review,
        SlashCommand::Rename,
        SlashCommand::New,
        SlashCommand::Archive,
        SlashCommand::Delete,
        SlashCommand::Resume,
        SlashCommand::Fork,
        SlashCommand::Worktree,
        SlashCommand::App,
        SlashCommand::Init,
        SlashCommand::Compact,
        SlashCommand::Config,
        SlashCommand::Recap,
        SlashCommand::Plan,
        SlashCommand::Voice,
        SlashCommand::Goal,
        SlashCommand::Agents,
        SlashCommand::Side,
        SlashCommand::Btw,
        SlashCommand::Copy,
        SlashCommand::Export,
        SlashCommand::Raw,
        SlashCommand::Tui,
        SlashCommand::Diff,
        SlashCommand::Mention,
        SlashCommand::Status,
        SlashCommand::Daemon,
        SlashCommand::Warnings,
        SlashCommand::Cd,
        SlashCommand::Pwd,
        SlashCommand::Usage,
        SlashCommand::DebugConfig,
        SlashCommand::Title,
        SlashCommand::Statusline,
        SlashCommand::Theme,
        SlashCommand::Pets,
        SlashCommand::Mcp,
        SlashCommand::Apps,
        SlashCommand::Plugins,
        SlashCommand::Logout,
        SlashCommand::Quit,
        SlashCommand::Exit,
        SlashCommand::Feedback,
        SlashCommand::Rollout,
        SlashCommand::Ps,
        SlashCommand::Stop,
        SlashCommand::Clear,
        SlashCommand::TestApproval,
        SlashCommand::MultiAgents,
        SlashCommand::MemoryDrop,
        SlashCommand::MemoryUpdate,
    ];

    /// The command string without the leading `/`.
    pub fn command(self) -> &'static str {
        todo!()
    }

    /// The command a name or alias stands for (`FromStr` of the oracle).
    pub fn parse(name: &str) -> Option<SlashCommand> {
        let _ = name;
        todo!()
    }

    /// Whether this command supports inline args (`/review ...`).
    pub fn supports_inline_args(self) -> bool {
        todo!()
    }

    /// Whether this command remains available inside a side conversation.
    pub fn available_in_side_conversation(self) -> bool {
        todo!()
    }

    /// Whether dispatch needs thread state to validate this command before
    /// it consumes the draft.
    pub fn requires_dispatch_validation(self) -> bool {
        todo!()
    }

    /// Commands that do not require a writable current thread.
    pub fn available_when_thread_unavailable(self) -> bool {
        todo!()
    }

    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        todo!()
    }

    fn is_visible(self) -> bool {
        todo!()
    }
}

/// Every visible built-in command, with its command string, in enum order.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    todo!()
}

/// A model service-tier command (`/fast`): offered after `/model`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceTierCommand {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// A built-in or a service-tier command (`SlashCommandItem`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlashCommandItem {
    Builtin(SlashCommand),
    ServiceTier(ServiceTierCommand),
}

impl SlashCommandItem {
    pub fn command(&self) -> &str {
        todo!()
    }

    pub fn supports_inline_args(&self) -> bool {
        todo!()
    }

    pub fn available_in_side_conversation(&self) -> bool {
        todo!()
    }

    pub fn available_during_task(&self) -> bool {
        todo!()
    }
}

/// The feature and session gates of the popup and of the lookup
/// (`BuiltinCommandFlags`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandFlags {
    pub collaboration_modes_enabled: bool,
    pub connectors_enabled: bool,
    pub plugins_command_enabled: bool,
    pub token_activity_command_enabled: bool,
    pub service_tier_commands_enabled: bool,
    pub daybreak_command_description: Option<&'static str>,
    pub goal_command_enabled: bool,
    pub voice_command_enabled: bool,
    pub worktrees_enabled: bool,
    pub allow_elevate_sandbox: bool,
    pub side_conversation_active: bool,
}

/// The built-ins visible and usable for the current input.
pub fn builtins_for_input(flags: CommandFlags) -> Vec<(&'static str, SlashCommand)> {
    let _ = flags;
    todo!()
}

/// Built-ins and service tiers (after `/model`) for the input.
pub fn commands_for_input(
    flags: CommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> Vec<SlashCommandItem> {
    let _ = (flags, service_tier_commands);
    todo!()
}

/// A built-in command by a recognized name or alias, after the feature
/// gating (side-conversation and token-activity gating are dispatch's).
pub fn find_builtin_command(name: &str, flags: CommandFlags) -> Option<SlashCommand> {
    let _ = (name, flags);
    todo!()
}

/// A built-in or a service-tier command by name.
pub fn find_slash_command(
    name: &str,
    flags: CommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> Option<SlashCommandItem> {
    let _ = (name, flags, service_tier_commands);
    todo!()
}

/// Whether `name` is a fuzzy prefix of a known command.
pub fn has_slash_command_prefix(
    name: &str,
    flags: CommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> bool {
    let _ = (name, flags, service_tier_commands);
    todo!()
}

/// `(name, rest, rest_offset)` of a line starting with `/name`
/// (`prompt_args::parse_slash_name`).
pub fn parse_slash_name(line: &str) -> Option<(&str, &str, usize)> {
    let _ = line;
    todo!()
}

/// Rows the popup shows at most (`popup_consts::MAX_POPUP_ROWS`).
pub const MAX_POPUP_ROWS: usize = 8;

/// The selection and scroll of a popup list (`scroll_state.rs`).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ScrollState {
    pub selected_idx: Option<usize>,
    pub scroll_top: usize,
}

/// A selectable item in the popup.
pub type CommandItem = SlashCommandItem;

/// The flags of the popup (`CommandPopupFlags`).
#[derive(Clone, Copy, Debug, Default)]
pub struct CommandPopupFlags {
    pub collaboration_modes_enabled: bool,
    pub connectors_enabled: bool,
    pub plugins_command_enabled: bool,
    pub token_activity_command_enabled: bool,
    pub service_tier_commands_enabled: bool,
    pub daybreak_command_description: Option<&'static str>,
    pub goal_command_enabled: bool,
    pub voice_command_enabled: bool,
    pub worktrees_enabled: bool,
    pub windows_degraded_sandbox_active: bool,
    pub side_conversation_active: bool,
}

/// One row of the command popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupRow {
    pub item: CommandItem,
    /// Character positions of the name (after the `/`) the filter matched.
    pub match_indices: Option<Vec<usize>>,
    /// Shown but not selectable (a side conversation hides the command).
    pub unavailable: bool,
    /// A description that replaces the table's (Daybreak's, a service tier's).
    pub description: Option<String>,
}

/// The `/` popup: the filter, the commands and the selection
/// (`command_popup.rs`, without drawing).
pub struct CommandPopup {
    pub state: ScrollState,
}

impl CommandPopup {
    pub fn new(flags: CommandPopupFlags, service_tier_commands: Vec<ServiceTierCommand>) -> Self {
        let _ = (flags, service_tier_commands);
        todo!()
    }

    /// The composer text; the filter is the first token after a leading `/`.
    pub fn on_composer_text_change(&mut self, text: String) {
        let _ = text;
        todo!()
    }

    pub fn filtered_items(&self) -> Vec<CommandItem> {
        todo!()
    }

    pub fn rows(&self) -> Vec<PopupRow> {
        todo!()
    }

    pub fn move_up(&mut self) {
        todo!()
    }

    pub fn move_down(&mut self) {
        todo!()
    }

    pub fn selected_item(&self) -> Option<CommandItem> {
        todo!()
    }
}

/// What the queue does after a queued command was dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueDrain {
    /// The next queued input may be sent.
    Continue,
    /// The command opened something the user answers first.
    Stop,
}

/// The queue rule of a command (`queued_command_drain_result`); the two
/// flags are the facts the oracle's `Cd` and `Worktree` arms read.
pub fn queued_command_drain_result(
    command: SlashCommand,
    can_change_directory: bool,
    managed_worktree_available: bool,
) -> QueueDrain {
    let _ = (command, can_change_directory, managed_worktree_available);
    todo!()
}
