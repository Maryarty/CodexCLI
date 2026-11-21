//! The strings of `import` (S06): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`). Core's
//! `screens::import` holds no word: the group and item labels, the nouns, the
//! notices of the history and the errors are told here from its typed data.

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// Oracle: `render_ref`, the heading of each view.
    pub heading_marker: &'static str,
    pub heading_summary: &'static str,
    pub heading_customize: &'static str,
    /// Oracle: `render_ref`, the intro of each view.
    pub intro_summary: [&'static str; 3],
    pub intro_customize: [&'static str; 3],
    /// Oracle: `ActionMenuOption::label`.
    pub action_proceed: &'static str,
    pub action_customize: &'static str,
    pub action_skip: &'static str,
    pub action_back: &'static str,
    /// Oracle: `picker_option_row`, `build_*_render_lines`.
    pub cursor: &'static str,
    pub no_cursor: &'static str,
    pub detail_indent: &'static str,
    pub check_all: &'static str,
    pub check_some: &'static str,
    pub check_none: &'static str,
    pub importing_none: &'static str,
    /// Oracle: `section_title`.
    pub section_home: &'static str,
    pub section_project: &'static str,
    /// Oracle: `render_ref`, the footers: the keys and the words between.
    pub hint_use: &'static str,
    pub hint_press: &'static str,
    pub key_up: &'static str,
    pub key_down: &'static str,
    pub key_enter: &'static str,
    pub key_space: &'static str,
    pub key_customize: &'static str,
    pub key_back: &'static str,
    pub slash: &'static str,
    pub hint_move: &'static str,
    pub hint_select: &'static str,
    pub hint_customize: &'static str,
    pub hint_continue: &'static str,
    pub hint_toggle: &'static str,
    pub hint_back: &'static str,
    /// Oracle: `external_agent_config_migration_groups`.
    pub tools_and_setup: &'static str,
    pub tools_description: &'static str,
    pub current_project: &'static str,
    pub project_description: &'static str,
    pub chat_description: &'static str,
    /// Oracle: `external_agent_config_migration_item_label` (the checklist).
    pub label_instructions: &'static str,
    pub label_settings: &'static str,
    pub label_skills: &'static str,
    pub label_plugins: &'static str,
    pub label_mcp_servers: &'static str,
    pub label_agents: &'static str,
    pub label_hooks: &'static str,
    pub label_commands: &'static str,
    pub label_memory: &'static str,
    pub label_sessions: &'static str,
    /// Oracle: `external_agent_config_migration_type_label` where it differs.
    pub type_sessions: &'static str,
    /// Oracle: `external_agent_config_migration_item_detail`: `[one, many]`.
    pub noun_skill: [&'static str; 2],
    pub noun_mcp_server: [&'static str; 2],
    pub noun_agent: [&'static str; 2],
    pub noun_hook: [&'static str; 2],
    pub noun_command: [&'static str; 2],
    pub noun_session: [&'static str; 2],
    pub noun_memory: [&'static str; 2],
    pub noun_marketplace: [&'static str; 2],
    pub noun_plugin: [&'static str; 2],
    /// Oracle: `display_description`: what the app server says and what is shown.
    pub migrate_prefix: &'static str,
    pub import_prefix: &'static str,
    pub into: &'static str,
    pub to: &'static str,
    pub skills_from: &'static str,
    pub plugins_from: &'static str,
    /// Oracle: `external_agent_config_migration_started_lines` and `_finished_lines`.
    pub notice_bullet: &'static str,
    pub started: &'static str,
    pub started_tail: &'static str,
    pub apply_to_new_chats: &'static str,
    pub importing_title: &'static str,
    pub dash: &'static str,
    pub remaining_one: &'static str,
    pub finished: &'static str,
    pub results_by_type: &'static str,
    pub run_again: &'static str,
    /// Oracle: `flow.rs`, `app_server_session.rs`.
    pub no_items: &'static str,
    pub in_progress: &'static str,
    pub source_gone: &'static str,
    /// Oracle: `ExternalAgentConfigMigrationSource::label` and `source.rs`.
    pub source_claude: &'static str,
    pub source_cursor: &'static str,
    pub source_title: &'static str,
    pub source_subtitle: &'static str,
    pub source_hint: &'static str,
    projects: Msg<1>,
    chat_sessions: Msg<1>,
    counted_bare: Msg<2>,
    counted_names: Msg<3>,
    plugin_summary: Msg<4>,
    more_plugins: Msg<1>,
    more_marketplaces: Msg<1>,
    names_more: Msg<2>,
    remaining_many: Msg<1>,
    imported: Msg<1>,
    failed: Msg<1>,
    detect_failed: Msg<1>,
    source_error: Msg<2>,
    import_failed: Msg<1>,
    selected_one: Msg<2>,
    selected_many: Msg<2>,
    importing: Msg<1>,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "import from other agents",
    heading_marker: "> ",
    heading_summary: "Import setup",
    heading_customize: "Choose what to import",
    intro_summary: [
        "Bring over supported setup from another coding agent.",
        "Codex may add files to your current project folder.",
        "Your existing setup will not be changed.",
    ],
    intro_customize: [
        "Choose items to import.",
        "Codex may add files to your current project folder.",
        "Your existing setup will not be changed.",
    ],
    action_proceed: "Import selected",
    action_customize: "Customize selection",
    action_skip: "Cancel",
    action_back: "Review selection",
    cursor: "› ",
    no_cursor: "  ",
    detail_indent: "      ",
    check_all: "x",
    check_some: "-",
    check_none: " ",
    importing_none: "Importing: none",
    section_home: "Home",
    section_project: "Current project: ",
    hint_use: "Use ",
    hint_press: "Press ",
    key_up: "↑",
    key_down: "↓",
    key_enter: "enter",
    key_space: "space",
    key_customize: "c",
    key_back: "b",
    slash: "/",
    hint_move: " to move, ",
    hint_select: " to select, ",
    hint_customize: " to customize",
    hint_continue: " to continue, ",
    hint_toggle: " to toggle, ",
    hint_back: " to go back",
    tools_and_setup: "Tools & setup",
    tools_description: "Settings, instructions, integrations, agents, commands, and skills",
    current_project: "Current project",
    project_description: "Add Codex files alongside your existing project files",
    chat_description: "Last 30 days of chats",
    label_instructions: "Instructions",
    label_settings: "Settings",
    label_skills: "Skills",
    label_plugins: "Plugins",
    label_mcp_servers: "MCP servers",
    label_agents: "Agents",
    label_hooks: "Hooks",
    label_commands: "Slash commands",
    label_memory: "Memory",
    label_sessions: "Recent chat sessions",
    type_sessions: "Chat sessions",
    noun_skill: ["skill", "skills"],
    noun_mcp_server: ["MCP server", "MCP servers"],
    noun_agent: ["agent", "agents"],
    noun_hook: ["hook", "hooks"],
    noun_command: ["slash command", "slash commands"],
    noun_session: ["chat session", "chat sessions"],
    noun_memory: ["memory", "memories"],
    noun_marketplace: ["marketplace", "marketplaces"],
    noun_plugin: ["plugin", "plugins"],
    migrate_prefix: "Migrate ",
    import_prefix: "Import ",
    into: " into ",
    to: " to ",
    skills_from: "Import skills from ",
    plugins_from: "Import enabled plugins from ",
    notice_bullet: "• ",
    started: "Import started.",
    started_tail: " You can keep working while it finishes.",
    apply_to_new_chats: "Imported setup will apply to new chats.",
    importing_title: "Importing:",
    dash: " — ",
    remaining_one: "1 additional item remains. After it finishes, run /import again to review it.",
    finished: "Import finished: ",
    results_by_type: "Results by type:",
    run_again: "Run /import again to check for additional items.",
    no_items: "No compatible setup was found to import.",
    in_progress: "A previous external agent import is still running. Wait for it to finish before importing again.",
    source_gone: "Selected import source is no longer available.",
    source_claude: "Claude Code",
    source_cursor: "Cursor",
    source_title: "Choose an import source",
    source_subtitle: "Select the app whose setup you want to import.",
    source_hint: "enter continue · esc cancel",
    projects: msg!("Projects ({0})"),
    chat_sessions: msg!("Chat sessions ({0})"),
    counted_bare: msg!("{0} {1}"),
    counted_names: msg!("{0} {1}: {2}"),
    plugin_summary: msg!("({0} {1}, {2} {3})"),
    more_plugins: msg!("+{0} more"),
    more_marketplaces: msg!("+{0} more marketplaces"),
    names_more: msg!("{0}, +{1} more"),
    remaining_many: msg!(
        "{0} additional items remain. After it finishes, run /import again to review them."
    ),
    imported: msg!("{0} imported"),
    failed: msg!("{0} failed"),
    detect_failed: msg!("Could not check for importable setup: {0}"),
    source_error: msg!("{0}: {1}"),
    import_failed: msg!("Import failed: {0}"),
    selected_one: msg!("Selected {0} of {1} item."),
    selected_many: msg!("Selected {0} of {1} items."),
    importing: msg!("Importing: {0}"),
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

impl Strings {
    /// `Selected 3 of 4 items.` (the noun follows the number of items).
    pub fn selected(&self, chosen: usize, total: usize, num: &NumSyms) -> String {
        let count = |value: usize| Arg::Int(u64::try_from(value).unwrap_or(0));
        let msg = if total == 1 {
            self.selected_one
        } else {
            self.selected_many
        };
        Fmt::new(msg, [count(chosen), count(total)], num).to_string()
    }

    /// `Importing: Settings 1, Memory 1`.
    pub fn importing(&self, summary: &str, num: &NumSyms) -> String {
        Fmt::new(self.importing, [Arg::Text(summary)], num).to_string()
    }
}

fn int(value: usize) -> Arg<'static> {
    Arg::Int(u64::try_from(value).unwrap_or(0))
}

fn one<const N: usize>(msg: Msg<N>, args: [Arg<'_>; N], num: &NumSyms) -> String {
    Fmt::new(msg, args, num).to_string()
}

impl Strings {
    pub fn projects(&self, count: usize, num: &NumSyms) -> String {
        one(self.projects, [int(count)], num)
    }

    pub fn chat_sessions(&self, count: usize, num: &NumSyms) -> String {
        one(self.chat_sessions, [int(count)], num)
    }

    /// `2 skills: triage, release-notes`, or `1 skill` with no names.
    pub fn counted(&self, noun: [&str; 2], count: usize, names: &[&str], num: &NumSyms) -> String {
        let noun = if count == 1 { noun[0] } else { noun[1] };
        if names.is_empty() {
            one(self.counted_bare, [int(count), Arg::Text(noun)], num)
        } else {
            let names = names.join(", ");
            one(
                self.counted_names,
                [int(count), Arg::Text(noun), Arg::Text(&names)],
                num,
            )
        }
    }

    /// `(1 marketplace, 2 plugins)`.
    pub fn plugin_summary(&self, marketplaces: usize, plugins: usize, num: &NumSyms) -> String {
        let (market, plugin) = (
            if marketplaces == 1 {
                self.noun_marketplace[0]
            } else {
                self.noun_marketplace[1]
            },
            if plugins == 1 {
                self.noun_plugin[0]
            } else {
                self.noun_plugin[1]
            },
        );
        one(
            self.plugin_summary,
            [
                int(marketplaces),
                Arg::Text(market),
                int(plugins),
                Arg::Text(plugin),
            ],
            num,
        )
    }

    pub fn more_plugins(&self, hidden: usize, num: &NumSyms) -> String {
        one(self.more_plugins, [int(hidden)], num)
    }

    pub fn more_marketplaces(&self, hidden: usize, num: &NumSyms) -> String {
        one(self.more_marketplaces, [int(hidden)], num)
    }

    pub fn names_more(&self, shown: &str, hidden: usize, num: &NumSyms) -> String {
        one(self.names_more, [Arg::Text(shown), int(hidden)], num)
    }

    pub fn remaining(&self, count: usize, num: &NumSyms) -> String {
        if count == 1 {
            self.remaining_one.to_owned()
        } else {
            one(self.remaining_many, [int(count)], num)
        }
    }

    pub fn imported(&self, count: usize, num: &NumSyms) -> String {
        one(self.imported, [int(count)], num)
    }

    pub fn failed(&self, count: usize, num: &NumSyms) -> String {
        one(self.failed, [int(count)], num)
    }

    pub fn detect_failed(&self, errors: &str, num: &NumSyms) -> String {
        one(self.detect_failed, [Arg::Text(errors)], num)
    }

    pub fn source_error(&self, source: &str, error: &str, num: &NumSyms) -> String {
        one(
            self.source_error,
            [Arg::Text(source), Arg::Text(error)],
            num,
        )
    }

    pub fn import_failed(&self, error: &str, num: &NumSyms) -> String {
        one(self.import_failed, [Arg::Text(error)], num)
    }
}
