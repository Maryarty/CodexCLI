//! Slash commands (reuse-map ADAPT rows), carried from codex-tui's
//! `slash_command.rs`, `bottom_pane/slash_commands.rs`, `command_popup.rs`
//! (the filter) and `bottom_pane/prompt_args.rs` (`parse_slash_name`): the
//! registry, the parse, the popup's rows and, in `dispatch`, the decision a
//! typed or selected command leads to. Pure data and functions with no prose:
//! the descriptions and messages are the UI's string tables'.

pub mod args;
mod dispatch;

pub use crate::kit::fuzzy::fuzzy_match;
pub use dispatch::*;

impl Notice {
    /// The composer keeps the line that ended in this notice, as the oracle
    /// does (`chat_composer.rs`, `try_dispatch_*slash_command*`): a command it
    /// does not know, a command refused while a task runs, and a command
    /// dispatch validates before it takes the draft (`/fork`, `/review`). Every
    /// other answer comes after the line was taken.
    pub fn keeps_draft(&self) -> bool {
        match self {
            Notice::Unrecognized(_) | Notice::InputTooLarge { .. } => true,
            Notice::DisabledDuringTask(command) => {
                command.requires_dispatch_validation() || !command.available_during_task()
            }
            Notice::UnavailableInSide(command)
            | Notice::UnavailableInReview(command)
            | Notice::SessionStarting(command)
            | Notice::NotAvailable(command) => command.requires_dispatch_validation(),
            _ => false,
        }
    }

    /// The line is in the composer's local recall: an unknown command and a
    /// message over the limit are not
    /// (`unrecognized_slash_command_is_not_added_to_local_recall`).
    pub fn is_recalled(&self) -> bool {
        !matches!(self, Notice::Unrecognized(_) | Notice::InputTooLarge { .. })
    }
}

impl SlashCommand {
    /// `parent_owned_command_is_allowed` (`chat_composer.rs`): what a thread
    /// its parent controls still runs. `/export`, `/title` and `/statusline`
    /// with any arguments; the others bare only.
    pub fn allowed_when_parent_owned(self, args: &str) -> bool {
        use SlashCommand::*;
        if matches!(self, Export | Title | Statusline) {
            return true;
        }
        args.is_empty()
            && matches!(
                self,
                Feedback
                    | New
                    | Clear
                    | Resume
                    | App
                    | Side
                    | Btw
                    | Agents
                    | MultiAgents
                    | Vim
                    | Keymap
                    | Config
                    | ElevateSandbox
                    | Experimental
                    | Memories
                    | Quit
                    | Exit
                    | Logout
                    | Copy
                    | Raw
                    | Diff
                    | Mention
                    | Skills
                    | Import
                    | Hooks
                    | Status
                    | Warnings
                    | Daemon
                    | Usage
                    | Ide
                    | DebugConfig
                    | Theme
                    | Pets
                    | Ps
                    | Stop
                    | MemoryDrop
                    | MemoryUpdate
                    | Mcp
                    | Apps
                    | Plugins
                    | Rollout
            )
    }
}

/// `AbsolutePathBuf::resolve_path_against_base`: the directory `/cd` names,
/// `~` expanded to `home`, a relative path joined to `base`, `.` and `..`
/// folded away (no file system is read).
pub fn resolve_directory(requested: &str, base: &str, home: Option<&str>) -> String {
    use std::path::{Component, Path, PathBuf};
    let expanded = match home {
        Some(home) if requested == "~" => PathBuf::from(home),
        Some(home) => match requested.strip_prefix("~/") {
            Some(rest) => Path::new(home).join(rest),
            None => PathBuf::from(requested),
        },
        None => PathBuf::from(requested),
    };
    let mut folded = PathBuf::new();
    for part in Path::new(base).join(expanded).components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                folded.pop();
            }
            part => folded.push(part.as_os_str()),
        }
    }
    folded.display().to_string()
}

/// `handle_parent_owned_submission`: whether a thread its parent controls
/// takes this composer line: a built-in command by its own name (not an
/// alias) that `allowed_when_parent_owned`. Text, `!cmd` and every other
/// command are refused.
pub fn parent_owned_takes(line: &str, flags: CommandFlags) -> bool {
    line.starts_with('/')
        && parse_slash_name(line).is_some_and(|(name, args, _)| {
            find_builtin_command(name, flags).is_some_and(|command| {
                name == command.command() && command.allowed_when_parent_owned(args)
            })
        })
}

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
        use SlashCommand::*;
        match self {
            Model => "model",
            Presets => "presets",
            Daybreak => "daybreak",
            Ide => "ide",
            Permissions => "permissions",
            Keymap => "keymap",
            Vim => "vim",
            ElevateSandbox => "setup-default-sandbox",
            Experimental => "experimental",
            AutoReview => "approve",
            Memories => "memories",
            Skills => "skills",
            Import => "import",
            Hooks => "hooks",
            Review => "review",
            Rename => "rename",
            New => "new",
            Archive => "archive",
            Delete => "delete",
            Resume => "resume",
            Fork => "fork",
            Worktree => "worktree",
            App => "app",
            Init => "init",
            Compact => "compact",
            Config => "config",
            Recap => "recap",
            Plan => "plan",
            Voice => "voice",
            Goal => "goal",
            Agents => "agents",
            Side => "side",
            Btw => "btw",
            Copy => "copy",
            Export => "export",
            Raw => "raw",
            Tui => "tui",
            Diff => "diff",
            Mention => "mention",
            Status => "status",
            Daemon => "daemon",
            Warnings => "warnings",
            Cd => "cd",
            Pwd => "pwd",
            Usage => "usage",
            DebugConfig => "debug-config",
            Title => "title",
            Statusline => "statusline",
            Theme => "theme",
            Pets => "pets",
            Mcp => "mcp",
            Apps => "apps",
            Plugins => "plugins",
            Logout => "logout",
            Quit => "quit",
            Exit => "exit",
            Feedback => "feedback",
            Rollout => "rollout",
            Ps => "ps",
            Stop => "stop",
            Clear => "clear",
            TestApproval => "test-approval",
            MultiAgents => "subagents",
            MemoryDrop => "debug-m-drop",
            MemoryUpdate => "debug-m-update",
        }
    }

    /// The other names a command answers to (`strum(serialize = ..)`).
    fn aliases(self) -> &'static [&'static str] {
        match self {
            SlashCommand::Pwd => &["cwd"],
            SlashCommand::Pets => &["pet"],
            SlashCommand::Stop => &["clean"],
            _ => &[],
        }
    }

    /// The command a name or alias stands for (`FromStr` of the oracle).
    pub fn parse(name: &str) -> Option<SlashCommand> {
        SlashCommand::ALL
            .into_iter()
            .find(|c| c.command() == name || c.aliases().contains(&name))
    }

    /// Whether this command supports inline args (`/review ...`).
    pub fn supports_inline_args(self) -> bool {
        use SlashCommand::*;
        matches!(
            self,
            Review
                | Model
                | Permissions
                | Title
                | Statusline
                | Rename
                | New
                | Clear
                | Fork
                | Plan
                | Goal
                | Voice
                | Ide
                | Keymap
                | Mcp
                | Export
                | Raw
                | Cd
                | Pwd
                | Usage
                | Pets
                | Side
                | Btw
                | Resume
        )
    }

    /// Whether this command remains available inside a side conversation.
    pub fn available_in_side_conversation(self) -> bool {
        use SlashCommand::*;
        matches!(
            self,
            Copy | Agents
                | Export
                | Raw
                | Diff
                | Mention
                | Status
                | Daemon
                | Warnings
                | Pwd
                | Usage
                | Ide
                | Rename
        )
    }

    /// Whether dispatch needs thread state to validate this command before
    /// it consumes the draft.
    pub fn requires_dispatch_validation(self) -> bool {
        matches!(self, SlashCommand::Fork | SlashCommand::Review)
    }

    /// Commands that do not require a writable current thread.
    pub fn available_when_thread_unavailable(self) -> bool {
        use SlashCommand::*;
        matches!(
            self,
            New | Clear
                | Resume
                | Agents
                | MultiAgents
                | Quit
                | Exit
                | Status
                | Warnings
                | DebugConfig
                | Pwd
                | Rollout
                | Copy
                | Raw
        )
    }

    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        use SlashCommand::*;
        !matches!(
            self,
            New | Delete
                | Fork
                | Worktree
                | Init
                | Compact
                | Recap
                | Export
                | Keymap
                | Tui
                | Vim
                | ElevateSandbox
                | Experimental
                | Memories
                | Import
                | Review
                | Plan
                | Cd
                | Clear
                | Logout
                | MemoryDrop
                | MemoryUpdate
                | Config
                | Theme
                | Pets
        )
    }

    fn is_visible(self) -> bool {
        match self {
            SlashCommand::Copy => !cfg!(target_os = "android"),
            SlashCommand::App => cfg!(any(target_os = "macos", target_os = "windows")),
            SlashCommand::Voice => true,
            SlashCommand::Rollout | SlashCommand::TestApproval => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// Every visible built-in command, with its command string, in enum order.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::ALL
        .into_iter()
        .filter(|command| command.is_visible())
        .map(|c| (c.command(), c))
        .collect()
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
        match self {
            Self::Builtin(cmd) => cmd.command(),
            Self::ServiceTier(command) => &command.name,
        }
    }

    pub fn supports_inline_args(&self) -> bool {
        match self {
            Self::Builtin(cmd) => cmd.supports_inline_args(),
            Self::ServiceTier(_) => false,
        }
    }

    pub fn available_in_side_conversation(&self) -> bool {
        match self {
            Self::Builtin(cmd) => cmd.available_in_side_conversation(),
            Self::ServiceTier(_) => false,
        }
    }

    pub fn available_during_task(&self) -> bool {
        match self {
            Self::Builtin(cmd) => cmd.available_during_task(),
            Self::ServiceTier(_) => true,
        }
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
    built_in_slash_commands()
        .into_iter()
        .filter(|(_, cmd)| flags.allow_elevate_sandbox || *cmd != SlashCommand::ElevateSandbox)
        .filter(|(_, cmd)| flags.collaboration_modes_enabled || *cmd != SlashCommand::Plan)
        .filter(|(_, cmd)| flags.connectors_enabled || *cmd != SlashCommand::Apps)
        .filter(|(_, cmd)| flags.plugins_command_enabled || *cmd != SlashCommand::Plugins)
        .filter(|(_, cmd)| flags.token_activity_command_enabled || *cmd != SlashCommand::Usage)
        .filter(|(_, cmd)| flags.goal_command_enabled || *cmd != SlashCommand::Goal)
        .filter(|(_, cmd)| flags.worktrees_enabled || *cmd != SlashCommand::Worktree)
        .filter(|(_, cmd)| flags.voice_command_enabled || *cmd != SlashCommand::Voice)
        .filter(|(_, cmd)| {
            flags.daybreak_command_description.is_some() || *cmd != SlashCommand::Daybreak
        })
        .filter(|(_, cmd)| !flags.side_conversation_active || cmd.available_in_side_conversation())
        .collect()
}

/// Built-ins and service tiers (after `/model`) for the input.
pub fn commands_for_input(
    flags: CommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> Vec<SlashCommandItem> {
    let mut commands = Vec::new();
    let tiers_enabled = flags.service_tier_commands_enabled;
    for (_, cmd) in builtins_for_input(flags) {
        commands.push(SlashCommandItem::Builtin(cmd));
        if cmd == SlashCommand::Model && tiers_enabled {
            commands.extend(
                service_tier_commands
                    .iter()
                    .cloned()
                    .map(SlashCommandItem::ServiceTier),
            );
        }
    }
    commands
        .into_iter()
        .filter(|cmd| !flags.side_conversation_active || cmd.available_in_side_conversation())
        .collect()
}

/// A built-in command by a recognized name or alias, after the feature
/// gating. Side-conversation and token-activity gating are dispatch's, so a
/// typed command can give a specific unavailable message while the popup
/// still hides it.
pub fn find_builtin_command(name: &str, flags: CommandFlags) -> Option<SlashCommand> {
    let cmd = SlashCommand::parse(name).or_else(|| {
        // `/goooal`: the extra os are accepted.
        let repeated_os = name.strip_prefix('g')?.strip_suffix("al")?;
        (!repeated_os.is_empty() && repeated_os.bytes().all(|byte| byte == b'o'))
            .then_some(SlashCommand::Goal)
    })?;
    builtins_for_input(CommandFlags {
        token_activity_command_enabled: true,
        daybreak_command_description: Some(""),
        side_conversation_active: false,
        ..flags
    })
    .into_iter()
    .any(|(_, visible_cmd)| visible_cmd == cmd)
    .then_some(cmd)
}

/// A built-in or a service-tier command by name.
pub fn find_slash_command(
    name: &str,
    flags: CommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> Option<SlashCommandItem> {
    if let Some(cmd) = find_builtin_command(name, flags) {
        return Some(SlashCommandItem::Builtin(cmd));
    }
    flags
        .service_tier_commands_enabled
        .then(|| {
            service_tier_commands
                .iter()
                .find(|command| command.name == name)
                .cloned()
                .map(SlashCommandItem::ServiceTier)
        })
        .flatten()
}

/// Whether `name` is a fuzzy prefix of a known command. A side conversation
/// can describe a known command as unavailable in the popup, though dispatch
/// rejects it, so the side gate is off here.
pub fn has_slash_command_prefix(
    name: &str,
    flags: CommandFlags,
    service_tier_commands: &[ServiceTierCommand],
) -> bool {
    commands_for_input(
        CommandFlags {
            side_conversation_active: false,
            ..flags
        },
        service_tier_commands,
    )
    .into_iter()
    .any(|command| fuzzy_match(command.command(), name).is_some())
}

/// `(name, rest, rest_offset)` of a line starting with `/name`
/// (`prompt_args::parse_slash_name`): `line[rest_offset..] == rest`, `rest`
/// without its leading blanks.
pub fn parse_slash_name(line: &str) -> Option<(&str, &str, usize)> {
    let stripped = line.strip_prefix('/')?;
    let name_end = stripped
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map_or(stripped.len(), |(idx, _)| idx);
    let name = &stripped[..name_end];
    if name.is_empty() {
        return None;
    }
    let rest_untrimmed = &stripped[name_end..];
    let rest = rest_untrimmed.trim_start();
    let rest_start = name_end + (rest_untrimmed.len() - rest.len());
    Some((name, rest, rest_start + 1))
}

/// Rows the popup shows at most (`popup_consts::MAX_POPUP_ROWS`).
pub const MAX_POPUP_ROWS: usize = 8;

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

impl From<CommandPopupFlags> for CommandFlags {
    fn from(value: CommandPopupFlags) -> Self {
        Self {
            collaboration_modes_enabled: value.collaboration_modes_enabled,
            connectors_enabled: value.connectors_enabled,
            plugins_command_enabled: value.plugins_command_enabled,
            token_activity_command_enabled: value.token_activity_command_enabled,
            service_tier_commands_enabled: value.service_tier_commands_enabled,
            daybreak_command_description: value.daybreak_command_description,
            goal_command_enabled: value.goal_command_enabled,
            voice_command_enabled: value.voice_command_enabled,
            worktrees_enabled: value.worktrees_enabled,
            allow_elevate_sandbox: value.windows_degraded_sandbox_active,
            side_conversation_active: value.side_conversation_active,
        }
    }
}

impl From<CommandFlags> for CommandPopupFlags {
    fn from(value: CommandFlags) -> Self {
        Self {
            collaboration_modes_enabled: value.collaboration_modes_enabled,
            connectors_enabled: value.connectors_enabled,
            plugins_command_enabled: value.plugins_command_enabled,
            token_activity_command_enabled: value.token_activity_command_enabled,
            service_tier_commands_enabled: value.service_tier_commands_enabled,
            daybreak_command_description: value.daybreak_command_description,
            goal_command_enabled: value.goal_command_enabled,
            voice_command_enabled: value.voice_command_enabled,
            worktrees_enabled: value.worktrees_enabled,
            windows_degraded_sandbox_active: value.allow_elevate_sandbox,
            side_conversation_active: value.side_conversation_active,
        }
    }
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

/// Hide alias commands in the default list so each action appears once:
/// `quit` is an alias of `exit`, `btw` of `side`.
const ALIAS_COMMANDS: &[SlashCommand] = &[SlashCommand::Quit, SlashCommand::Btw];

/// The `/` popup: the filter, the commands and the selection
/// (`command_popup.rs`, without drawing).
pub struct CommandPopup {
    command_filter: String,
    commands: Vec<CommandItem>,
    side_conversation_active: bool,
    daybreak_command_description: Option<&'static str>,
}

impl CommandPopup {
    pub fn new(flags: CommandPopupFlags, service_tier_commands: Vec<ServiceTierCommand>) -> Self {
        // Built-in availability stays in sync with the composer.
        let commands = commands_for_input(
            CommandFlags {
                side_conversation_active: false,
                ..flags.into()
            },
            &service_tier_commands,
        )
        .into_iter()
        .filter(|command| match command {
            SlashCommandItem::Builtin(cmd) => {
                !cmd.command().starts_with("debug") && *cmd != SlashCommand::Apps
            }
            SlashCommandItem::ServiceTier(_) => true,
        })
        .collect();
        Self {
            command_filter: String::new(),
            commands,
            side_conversation_active: flags.side_conversation_active,
            daybreak_command_description: flags.daybreak_command_description,
        }
    }

    /// The composer text; the filter is the first token after a leading `/`.
    pub fn on_composer_text_change(&mut self, text: String) {
        let first_line = text.lines().next().unwrap_or("");
        if let Some(stripped) = first_line.strip_prefix('/') {
            let token = stripped.trim_start();
            let cmd_token = token.split_whitespace().next().unwrap_or("");
            self.command_filter = cmd_token.to_string();
        } else {
            self.command_filter.clear();
        }

    }

    /// Exact then prefix matches, each in presentation order, with the
    /// indices of the matched characters of the name.
    fn filtered(&self) -> Vec<(CommandItem, Option<Vec<usize>>)> {
        let filter = self.command_filter.trim();
        let mut out: Vec<(CommandItem, Option<Vec<usize>>)> = Vec::new();
        if filter.is_empty() {
            for command in &self.commands {
                if matches!(command, CommandItem::Builtin(cmd) if ALIAS_COMMANDS.contains(cmd)) {
                    continue;
                }
                if self.is_unavailable(command) {
                    continue;
                }
                out.push((command.clone(), None));
            }
            return out;
        }

        let filter_lower = filter.to_lowercase();
        let filter_chars = filter.chars().count();
        let mut exact = Vec::new();
        let mut prefix = Vec::new();
        for command in &self.commands {
            let display_lower = command.command().to_lowercase();
            let indices = Some((0..filter_chars).collect());
            if display_lower == filter_lower {
                exact.push((command.clone(), indices));
            } else if display_lower.starts_with(&filter_lower) {
                prefix.push((command.clone(), indices));
            }
        }
        out.extend(exact);
        out.extend(prefix);
        if self.side_conversation_active {
            out.sort_by_key(|(item, _)| self.is_unavailable(item));
        }
        out
    }

    pub fn filtered_items(&self) -> Vec<CommandItem> {
        self.filtered().into_iter().map(|(c, _)| c).collect()
    }

    pub fn rows(&self) -> Vec<PopupRow> {
        self.filtered()
            .into_iter()
            .map(|(item, indices)| {
                let description = match &item {
                    SlashCommandItem::Builtin(SlashCommand::Daybreak) => {
                        self.daybreak_command_description.map(str::to_string)
                    }
                    SlashCommandItem::Builtin(_) => None,
                    SlashCommandItem::ServiceTier(command) => Some(command.description.clone()),
                };
                let unavailable = self.is_unavailable(&item);
                PopupRow {
                    item,
                    match_indices: indices.map(|v| v.into_iter().map(|i| i + 1).collect()),
                    unavailable,
                    description,
                }
            })
            .collect()
    }

    fn is_unavailable(&self, item: &CommandItem) -> bool {
        self.side_conversation_active
            && match item {
                CommandItem::Builtin(cmd) => !cmd.available_in_side_conversation(),
                CommandItem::ServiceTier(_) => true,
            }
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
/// flags are the facts the oracle's `Cd` and `Worktree` arms read. A turn
/// pending or a popup open stops the queue before any command is asked.
pub fn queued_command_drain_result(
    command: SlashCommand,
    can_change_directory: bool,
    managed_worktree_available: bool,
) -> QueueDrain {
    use SlashCommand::*;
    match command {
        Ide | Status | Daemon | Pwd | Usage | DebugConfig | Ps | Stop | MemoryDrop
        | MemoryUpdate | Mcp | Apps | Plugins | Rollout | Copy | Raw | Vim | Daybreak | Diff
        | App | Rename | Voice | Recap | TestApproval => QueueDrain::Continue,
        Cd if can_change_directory => QueueDrain::Stop,
        Cd => QueueDrain::Continue,
        Worktree if managed_worktree_available => QueueDrain::Stop,
        Worktree => QueueDrain::Continue,
        Feedback | Warnings | Export | New | Archive | Delete | Clear | Resume | Fork | Init
        | Compact | Review | Model | Presets | Plan | Goal | Side | Btw | Keymap | Agents
        | MultiAgents | Permissions | ElevateSandbox | Experimental | AutoReview | Memories
        | Quit | Exit | Logout | Mention | Skills | Import | Hooks | Title | Statusline
        | Config | Theme | Tui | Pets => QueueDrain::Stop,
    }
}
