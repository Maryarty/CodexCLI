//! The strings of `settings` (S15): a table of `&'static str` fields and typed
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

use codex_tui_core::commands::args::ArgError;
use codex_tui_core::features::settings::args::{Refusal, Surface};
use codex_tui_core::features::settings::capture::Said;
use codex_tui_core::features::settings::{Note, Setting};

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// How a line of the theme preview changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Context,
    Added,
    Removed,
}

/// One line of the theme preview's sample diff (`theme_picker.rs`:
/// `PreviewRow`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewRow {
    pub no: &'static str,
    pub change: Change,
    pub code: &'static str,
}

const fn row(no: &'static str, change: Change, code: &'static str) -> PreviewRow {
    PreviewRow { no, change, code }
}

/// A choice editor of the configuration panel (`config_panel/render.rs`): a title, the line
/// under it and the names of its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub items: [&'static str; 3],
}

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    pub num: NumSyms,
    // config_panel/render.rs, config_panel/mod.rs: the panel.
    pub config_title: &'static str,
    pub config_subtitle: &'static str,
    /// The group tabs, in `Section::ALL` order.
    pub config_groups: [&'static str; 5],
    pub config_search_placeholder: &'static str,
    pub config_search_prefix: &'static str,
    pub value_on: &'static str,
    pub value_off: &'static str,
    pub value_choice: Msg<1>,
    pub value_configure: &'static str,
    pub value_test: &'static str,
    pub value_unavailable: &'static str,
    /// A row of a search result: its group, then its description.
    pub search_description: Msg<2>,
    /// The marker of the row with the cursor, and the blank under it.
    pub cursor_marker: &'static str,
    pub indent: &'static str,
    /// The cell before the sign of a diff line.
    pub diff_gutter: &'static str,
    pub more_above: &'static str,
    pub more_below: &'static str,
    pub no_matches: &'static str,
    pub ellipsis: &'static str,
    /// `Search \u{203a} <typed text>`: the part before the text.
    pub config_search_line: &'static str,
    pub config_hint_open: &'static str,
    pub config_hint_test: &'static str,
    pub config_hint_plain: &'static str,
    pub config_search_hint_test: &'static str,
    pub config_search_hint_plain: &'static str,
    pub config_hint_toggle: &'static str,
    pub config_search_hint_open: &'static str,
    pub config_search_hint_toggle: &'static str,
    // theme_picker.rs
    pub theme_title: &'static str,
    pub theme_subtitle: &'static str,
    pub theme_search_placeholder: &'static str,
    /// The subtitle when the custom themes' directory is known and fits.
    pub theme_custom_dir: Msg<1>,
    pub theme_custom_mark: &'static str,
    /// The narrow preview (stacked below the rows): a diff of a Rust function.
    pub theme_narrow: [PreviewRow; 4],
    /// The wide preview (beside the rows).
    pub theme_wide: [PreviewRow; 8],
    /// The footer of the theme picker: the oracle gives it `standard_popup_hint_line`, which the
    /// bottom pane turns into the list keymap's accept and cancel hints
    /// (`apply_standard_popup_hint`, `picker_hint_line_for_keymap`).
    pub popup_hint: &'static str,
    // bottom_pane/status_line_setup.rs
    pub status_title: &'static str,
    pub status_subtitle: &'static str,
    pub status_search_placeholder: &'static str,
    pub use_theme_colors: &'static str,
    pub use_theme_colors_description: &'static str,
    pub status_hint: &'static str,
    // bottom_pane/title_setup.rs: the same view with a title of its own.
    pub title_setup_title: &'static str,
    pub title_setup_subtitle: &'static str,
    // bottom_pane/skills_toggle_view.rs
    pub skills_title: &'static str,
    pub skills_subtitle: &'static str,
    pub skills_search_placeholder: &'static str,
    pub skills_hint: &'static str,
    // bottom_pane/experimental_features_view.rs
    pub features_title: &'static str,
    pub features_subtitle: &'static str,
    pub features_loading: &'static str,
    pub features_saving: &'static str,
    pub features_none: &'static str,
    pub features_unavailable: &'static str,
    pub features_no_rows: &'static str,
    pub features_read_only: &'static str,
    pub features_hint: &'static str,
    pub features_hint_saving: &'static str,
    pub features_hint_retained: &'static str,
    pub check_on: &'static str,
    pub check_off: &'static str,
    pub section_break: &'static str,
    // bottom_pane/memories_settings_view.rs
    pub memories_title: &'static str,
    pub memories_subtitle: &'static str,
    pub memories_use: &'static str,
    pub memories_use_description: &'static str,
    pub memories_generate: &'static str,
    pub memories_generate_description: &'static str,
    pub memories_reset: &'static str,
    pub memories_reset_description: &'static str,
    pub memories_docs: &'static str,
    pub memories_docs_url: &'static str,
    pub memories_hint: &'static str,
    pub reset_title: &'static str,
    pub reset_subtitle: &'static str,
    pub reset_yes: &'static str,
    pub reset_yes_description: &'static str,
    pub reset_no: &'static str,
    pub reset_no_description: &'static str,
    pub reset_hint: &'static str,
    // bottom_pane/hooks_browser_view.rs, hooks_browser_render.rs
    pub hooks_title: &'static str,
    pub hooks_subtitle: &'static str,
    pub hooks_col_event: &'static str,
    pub hooks_col_installed: &'static str,
    pub hooks_col_active: &'static str,
    pub hooks_col_review: &'static str,
    pub hooks_col_description: &'static str,
    /// The narrow table's second column and the mark of an event to review.
    pub hooks_col_counts: &'static str,
    pub hooks_review_mark: &'static str,
    /// `{active}/{installed}` of the narrow table.
    pub hooks_counts: Msg<2>,
    pub hooks_warning_mark: &'static str,
    pub hooks_error_mark: &'static str,
    pub hooks_issues: &'static str,
    /// The error of a file: its path, then the message.
    pub hooks_error: Msg<2>,
    pub hooks_review_one: &'static str,
    pub hooks_review_many: Msg<1>,
    /// The events, in `HookEvent::ALL` order.
    pub hooks_event_labels: [&'static str; 14],
    pub hooks_event_descriptions: [&'static str; 14],
    /// `{event} hooks`.
    pub hooks_handlers_title: Msg<1>,
    pub hooks_handlers_subtitle: &'static str,
    pub hooks_unnamed: &'static str,
    pub hooks_suffix_modified: &'static str,
    pub hooks_suffix_new: &'static str,
    /// The check of a hook waiting for a review.
    pub hooks_check_review: &'static str,
    pub hooks_none: &'static str,
    // The detail rows' labels and values.
    pub hooks_label_event: &'static str,
    pub hooks_label_matcher: &'static str,
    pub hooks_label_source: &'static str,
    pub hooks_label_command: &'static str,
    pub hooks_label_mode: &'static str,
    pub hooks_label_server: &'static str,
    pub hooks_label_tool: &'static str,
    pub hooks_label_handler: &'static str,
    pub hooks_label_timeout: &'static str,
    pub hooks_label_context: &'static str,
    pub hooks_label_trust: &'static str,
    pub hooks_mode_sync: &'static str,
    pub hooks_mode_async: &'static str,
    pub hooks_handler_prompt: &'static str,
    pub hooks_handler_agent: &'static str,
    pub hooks_timeout: Msg<1>,
    pub hooks_context_unlimited: &'static str,
    pub hooks_context_limit: Msg<1>,
    pub hooks_trust_managed: &'static str,
    pub hooks_trust_trusted: &'static str,
    pub hooks_trust_untrusted: &'static str,
    pub hooks_trust_modified: &'static str,
    pub hooks_source_admin: &'static str,
    pub hooks_source_user: &'static str,
    pub hooks_source_project: &'static str,
    pub hooks_source_session: &'static str,
    pub hooks_source_cloud: &'static str,
    pub hooks_source_unknown: &'static str,
    pub hooks_source_plugin: &'static str,
    /// `Plugin - {id}`, `Environment - {id}` and `{label} - {path}`.
    pub hooks_source_plugin_id: Msg<1>,
    pub hooks_source_environment: Msg<1>,
    pub hooks_source_path: Msg<2>,
    // The footers (the oracle's key hints, with the default keymap).
    pub hooks_hint_events: &'static str,
    pub hooks_hint_events_review: &'static str,
    pub hooks_hint_handlers_toggle: &'static str,
    pub hooks_hint_handlers_trust: &'static str,
    pub hooks_hint_handlers_managed: &'static str,
    pub hooks_hint_handlers: &'static str,
    // keymap_setup/picker.rs
    pub keymap_search_placeholder: &'static str,
    pub keymap_hint: &'static str,
    pub keymap_empty: &'static str,
    pub keymap_title: &'static str,
    pub keymap_actions_one: &'static str,
    pub keymap_actions_many: Msg<1>,
    /// The marker before an unbound action, and before a bound one.
    pub keymap_unbound: &'static str,
    pub keymap_bound: &'static str,
    /// bottom_pane/list_selection_view.rs: the marker of the current row.
    pub current_marker: &'static str,
    // config_panel/mod.rs: why a row is unavailable (its description while it is).
    pub reason_notifications: &'static str,
    pub reason_notify_when: &'static str,
    pub reason_deselect: &'static str,
    /// The line under the title after a save (`ConfigPanelUpdate::notice`).
    pub notice_could_not_save: Msg<1>,
    pub notice_new_composers: &'static str,
    pub notice_test_scheduled: &'static str,
    pub notice_test_already: &'static str,
    // config_panel/render.rs: the editors of the rows.
    pub editor_notify_when: Choice,
    pub editor_copy_on_select: Choice,
    pub editor_right_click_paste: Choice,
    pub editor_session_layout: Choice,
    pub editor_resume_directory: Choice,
    pub effects_title: &'static str,
    pub effects_subtitle: &'static str,
    pub effects_animations: &'static str,
    pub effects_animations_description: &'static str,
    pub effects_stars: &'static str,
    pub effects_stars_description: &'static str,
    pub effects_stars_reason: &'static str,
    pub effects_hint: &'static str,
    pub resume_current_reason: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "settings dialogs",
    num: NumSyms::ENGLISH,
    config_title: "Configuration",
    config_subtitle: "Changes save automatically",
    config_groups: [
        "Appearance",
        "Notifications",
        "Input",
        "Sessions",
        "Text selection",
    ],
    config_search_placeholder: "Type to search settings",
    config_search_prefix: "Search",
    value_on: "[x] On",
    value_off: "[ ] Off",
    value_choice: msg!("{0} \u{203a}"),
    value_configure: "Configure \u{203a}",
    value_test: "Send in 5s",
    value_unavailable: "Unavailable",
    search_description: msg!("{0} \u{b7} {1}"),
    cursor_marker: "\u{203a} ",
    indent: "  ",
    diff_gutter: " ",
    more_above: "\u{2191}",
    more_below: "\u{2193}",
    no_matches: "no matches",
    ellipsis: "\u{2026}",
    config_search_line: "Search \u{203a} ",
    config_hint_test: "left/right group \u{b7} / search \u{b7} enter test \u{b7} esc clear search/close",
    config_hint_plain: "left/right group \u{b7} / search \u{b7} esc clear search/close",
    config_search_hint_test: "enter test \u{b7} esc clear search",
    config_search_hint_plain: "esc clear search",
    config_hint_open: "left/right group \u{b7} / search \u{b7} enter open \u{b7} esc clear search/close",
    config_hint_toggle: "left/right group \u{b7} / search \u{b7} enter toggle \u{b7} esc clear search/close",
    config_search_hint_open: "enter open \u{b7} esc clear search",
    config_search_hint_toggle: "enter toggle \u{b7} esc clear search",
    theme_title: "Select Syntax Theme",
    theme_subtitle: "Move up/down to live preview themes",
    theme_search_placeholder: "Type to filter themes...",
    theme_custom_dir: msg!("Custom .tmTheme files can be added to the {0} directory."),
    theme_custom_mark: " (custom)",
    theme_narrow: [
        row("12", Change::Context, "fn greet(name: &str) -> String {"),
        row("13", Change::Removed, "    format!(\"Hello, {}!\", name)"),
        row("13", Change::Added, "    format!(\"Hello, {name}!\")"),
        row("14", Change::Context, "}"),
    ],
    theme_wide: [
        row(
            "31",
            Change::Context,
            "fn summarize(users: &[User]) -> String {",
        ),
        row(
            "32",
            Change::Removed,
            "    let active = users.iter().filter(|u| u.is_active).count();",
        ),
        row(
            "32",
            Change::Added,
            "    let active = users.iter().filter(|u| u.is_active()).count();",
        ),
        row(
            "33",
            Change::Context,
            "    let names: Vec<&str> = users.iter().map(User::name).take(3).collect();",
        ),
        row(
            "34",
            Change::Removed,
            "    format!(\"{} active: {}\", active, names.join(\", \"))",
        ),
        row(
            "34",
            Change::Added,
            "    format!(\"{active} active users: {}\", names.join(\", \"))",
        ),
        row("35", Change::Added, "        .trim()"),
        row("36", Change::Context, "}"),
    ],
    popup_hint: "enter select \u{b7} esc back",
    status_title: "Configure Status Line",
    status_subtitle: "Select which items to display in the status line.",
    status_search_placeholder: "Type to search",
    use_theme_colors: "Use theme colors",
    use_theme_colors_description: "Apply colors from the active /theme",
    check_on: "[x] ",
    check_off: "[ ] ",
    section_break: "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
    memories_title: "Memories",
    memories_subtitle: "Choose how Codex uses and creates memories. Changes are saved to config.toml",
    memories_use: "Use memories",
    memories_use_description: "Use memories starting with the next thread",
    memories_generate: "Generate memories",
    memories_generate_description: "Generate memories from this thread and future threads",
    memories_reset: "Reset all memories",
    memories_reset_description: "Clear local memory files and summaries without changing existing threads",
    memories_docs: "Learn more: ",
    memories_docs_url: "https://developers.openai.com/codex/memories",
    memories_hint: "space toggle \u{b7} enter save/select \u{b7} esc cancel",
    reset_title: "Reset all memories?",
    reset_subtitle: "This clears local memory files and rollout summaries for the current Codex home.",
    reset_yes: "Reset all memories",
    reset_yes_description: "Delete local memory files and rollout summaries",
    reset_no: "Go back",
    reset_no_description: "Return to memory settings",
    reset_hint: "enter select \u{b7} esc back",
    status_hint: "space toggle \u{b7} \u{2190}/\u{2192} reorder \u{b7} enter save \u{b7} esc cancel",
    title_setup_title: "Configure Terminal Title",
    title_setup_subtitle: "Select which items to display in the terminal title.",
    skills_title: "Enable/Disable Skills",
    skills_subtitle: "Turn skills on or off. Your changes are saved automatically.",
    skills_search_placeholder: "Type to search skills",
    skills_hint: "space/enter toggle \u{b7} esc close",
    features_title: "Experimental features",
    features_subtitle: "Checked features are configured on. Some experimental features take effect only in new tasks or after restarting the Codex server.",
    features_loading: "Loading server experiments\u{2026}",
    features_saving: "Saving experimental features\u{2026}",
    features_none: "No server experiments available.",
    features_unavailable: "Server experiments unavailable. Reopen /experimental to retry; restart this Codex client if requests remain unanswered.",
    features_no_rows: "No experimental features available for now",
    features_read_only: " (read-only)",
    features_hint: "space toggle \u{b7} enter save \u{b7} esc save/close",
    features_hint_saving: "Saving\u{2026} Closing this popup will not cancel the write.",
    features_hint_retained: "Selections retained. Save to retry, or cancel to close.",
    hooks_title: "Hooks",
    hooks_subtitle: "Lifecycle hooks from config and enabled plugins.",
    hooks_col_event: "Event",
    hooks_col_installed: "Installed",
    hooks_col_active: "Active",
    hooks_col_review: "Review",
    hooks_col_description: "Description",
    hooks_col_counts: "Active/Installed",
    hooks_review_mark: " !",
    hooks_counts: msg!("{0}/{1}"),
    hooks_warning_mark: "\u{26a0} ",
    hooks_error_mark: "\u{25a0} ",
    hooks_issues: "Issues",
    hooks_error: msg!("{0}: {1}"),
    hooks_review_one: "1 hook needs review before it can run.",
    hooks_review_many: msg!("{0} hooks need review before they can run."),
    hooks_event_labels: [
        "PreToolUse",
        "PermissionRequest",
        "PostToolUse",
        "PreCompact",
        "PostCompact",
        "SessionStart",
        "SessionEnd",
        "UserPromptSubmit",
        "SubagentStart",
        "SubagentStop",
        "Stop",
        "Interrupt",
        "PreModelRequest",
        "PostModelRequest",
    ],
    hooks_event_descriptions: [
        "Before a tool executes",
        "When permission is requested",
        "After a tool executes",
        "Before context compaction",
        "After context compaction",
        "When a new session starts",
        "Right before a session ends",
        "When the user submits a prompt",
        "When a subagent is created",
        "Right before a subagent ends its turn",
        "Right before Codex ends its turn",
        "Right before an interrupted turn is aborted",
        "Before a model request",
        "After a model request, including failures",
    ],
    hooks_handlers_title: msg!("{0} hooks"),
    hooks_handlers_subtitle: "Turn hooks on or off. Your changes are saved automatically.",
    hooks_unnamed: "Unnamed hook",
    hooks_suffix_modified: " \u{b7} modified",
    hooks_suffix_new: " \u{b7} new",
    hooks_check_review: "[!] ",
    hooks_none: "No hooks installed for this event.",
    hooks_label_event: "Event",
    hooks_label_matcher: "Matcher",
    hooks_label_source: "Source",
    hooks_label_command: "Command",
    hooks_label_mode: "Mode",
    hooks_label_server: "MCP Server",
    hooks_label_tool: "MCP Tool",
    hooks_label_handler: "Handler",
    hooks_label_timeout: "Timeout",
    hooks_label_context: "Context",
    hooks_label_trust: "Trust",
    hooks_mode_sync: "Sync",
    hooks_mode_async: "Async",
    hooks_handler_prompt: "Prompt",
    hooks_handler_agent: "Agent",
    hooks_timeout: msg!("{0}s"),
    hooks_context_unlimited: "unlimited",
    hooks_context_limit: msg!("limit: {0} approximate tokens"),
    hooks_trust_managed: "Managed",
    hooks_trust_trusted: "Trusted",
    hooks_trust_untrusted: "New hook - review required",
    hooks_trust_modified: "Modified since last trusted - review required",
    hooks_source_admin: "Admin config",
    hooks_source_user: "User config",
    hooks_source_project: "Project config",
    hooks_source_session: "Session flags",
    hooks_source_cloud: "Cloud-managed config",
    hooks_source_unknown: "Unknown source",
    hooks_source_plugin: "Plugin",
    hooks_source_plugin_id: msg!("Plugin - {0}"),
    hooks_source_environment: msg!("Environment - {0}"),
    hooks_source_path: msg!("{0} - {1}"),
    hooks_hint_events: "enter details \u{b7} esc close",
    hooks_hint_events_review: "t trust all \u{b7} enter review \u{b7} esc close",
    hooks_hint_handlers_toggle: "space/enter toggle \u{b7} esc back",
    hooks_hint_handlers_trust: "t trust \u{b7} esc back",
    hooks_hint_handlers_managed: "Managed (always on) \u{b7} esc back",
    hooks_hint_handlers: "esc back",
    keymap_search_placeholder: "Type to search shortcuts",
    keymap_hint: "left/right group \u{b7} enter edit shortcut \u{b7} * custom \u{b7} - unbound \u{b7} esc close",
    keymap_empty: "No shortcuts in this group",
    keymap_title: "Keymap",
    keymap_actions_one: "1 action.",
    keymap_actions_many: msg!("{0} actions."),
    keymap_unbound: "-",
    keymap_bound: " ",
    current_marker: " (current)",
    reason_notifications: "Event-specific notifications are configured in config.toml",
    reason_notify_when: "Turn on notifications to choose when they appear",
    reason_deselect: "Turn on Copy on select to deselect after copying",
    notice_could_not_save: msg!("Could not save: {0}"),
    notice_new_composers: "Saved \u{b7} applies to new composers",
    notice_test_scheduled: "Test scheduled in 5s \u{b7} switch apps",
    notice_test_already: "Test already scheduled \u{b7} switch apps",
    editor_notify_when: Choice {
        title: "Notify when",
        subtitle: "Notify only while you are away or while this terminal is focused too",
        items: ["Unfocused", "Always", ""],
    },
    editor_copy_on_select: Choice {
        title: "Copy on select",
        subtitle: "Copy transcript, footer, and composer selections on release; Auto follows terminal defaults",
        items: ["Auto (default)", "On", "Off"],
    },
    editor_right_click_paste: Choice {
        title: "Right-click paste",
        subtitle: "Paste when nothing is selected, with Auto following platform defaults and On also enabling macOS",
        items: ["Auto (default)", "On", "Off"],
    },
    editor_session_layout: Choice {
        title: "Session picker layout",
        subtitle: "Choose tighter rows or a roomier list when resuming and forking conversations",
        items: ["Dense", "Comfortable", ""],
    },
    editor_resume_directory: Choice {
        title: "Resume directory",
        subtitle: "Choose the working directory when resuming, with --cd taking precedence for this session",
        items: ["Ask", "Current directory", "Session directory"],
    },
    effects_title: "Effects",
    effects_subtitle: "Configure visual effects",
    effects_animations: "Animations",
    effects_animations_description: "Animate the welcome screen, working indicator, and transitions",
    effects_stars: "Composer stars",
    effects_stars_description: "Show stars briefly for new Astra tasks",
    effects_stars_reason: "Turn on Animations to show Astra task stars",
    effects_hint: "enter toggle \u{b7} esc back",
    resume_current_reason: "Requires --cd for remote sessions",
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The title of the screen in `lang`, for code with no tree (the debug
/// menu's rows).
/// The words of a note of the feature (`add_error_message` of `app/event_dispatch.rs`,
/// `app/config_persistence.rs`): the oracle's English until the catalog has these entries.
pub fn note(note: &Note) -> String {
    match note {
        Note::SaveFailed { setting, error } => match setting {
            Setting::Theme => format!("Failed to save theme: {error}"),
            Setting::StatusLine => format!("Failed to save status line settings: {error}"),
            Setting::Title => format!("Failed to save terminal title items: {error}"),
            Setting::Memories => format!("Failed to save memory settings: {error}"),
            Setting::TuiMode => format!("Failed to save TUI mode: {error}"),
            Setting::Keymap => format!("Failed to save shortcut: {error}"),
            Setting::PetSelection => format!("Failed to save pet selection: {error}"),
            Setting::PetsDisabled => format!("Failed to disable pets: {error}"),
            Setting::RateLimitReminder => {
                format!("Failed to save rate limit reminder preference: {error}")
            }
        },
        Note::SkillWriteFailed { path, error } => {
            format!("Failed to update skill config for {path}: {error}")
        }
        Note::SkillsChanged { enabled, disabled } => {
            let plural = |count: &usize| if *count == 1 { "" } else { "s" };
            match (enabled, disabled) {
                (0, _) => format!("Disabled {disabled} skill{}", plural(disabled)),
                (_, 0) => format!("Enabled {enabled} skill{}", plural(enabled)),
                _ => format!(
                    "Enabled {enabled} skill{}, disabled {disabled} skill{}",
                    plural(enabled),
                    plural(disabled)
                ),
            }
        }
        Note::SkillsFailed { error } => format!("Failed to load skills: {error}"),
        Note::HookWriteFailed { error } => format!("Failed to update hook config: {error}"),
        Note::HookTrustFailed { many: false, error } => format!("Failed to trust hook: {error}"),
        Note::HookTrustFailed { many: true, error } => format!("Failed to trust hooks: {error}"),
        Note::HooksFailed { error } => format!("Failed to load hooks: {error}"),
        Note::ExperimentalUnavailable => {
            "Experimental features are unavailable until startup completes.".to_string()
        }
        Note::TuiModeSaved { fullscreen } => {
            let mode = if *fullscreen {
                "Fullscreen"
            } else {
                "Scrollback"
            };
            format!("Saved TUI mode: {mode}. Restart Codex to apply; launch overrides still apply.")
        }
        Note::Refused(refusal) => refusal_words(refusal),
        Note::KeymapEdited {
            context,
            action,
            key,
            said,
        } => match said {
            Said::Remapped => format!("Remapped `{context}.{action}` to `{key}`."),
            Said::Added => format!("Added `{key}` to `{context}.{action}`."),
            Said::Replaced { old_key } => {
                format!("Replaced `{old_key}` with `{key}` for `{context}.{action}`.")
            }
            Said::Unchanged => format!("No change: `{context}.{action}` already uses `{key}`."),
        },
        Note::KeymapRemoved { context, action } => {
            format!("Removed custom shortcut for `{context}.{action}`.")
        }
        Note::KeymapUnknown { context, action } => format!(
            "Unknown keymap action `{context}.{action}`. Reopen /keymap and choose an action."
        ),
        Note::KeymapStale {
            context,
            action,
            old_key,
        } => format!(
            "`{context}.{action}` no longer uses `{old_key}`. Reopen /keymap and choose a binding again."
        ),
        Note::NoSavedChat { name } => format!("No saved chat found matching '{name}'."),
        Note::AmbiguousSession {
            name,
            first,
            second,
        } => format!(
            "Multiple sessions match '{name}' (including {first} and {second}); use a session UUID to disambiguate."
        ),
        Note::UnverifiedSession { id } => format!(
            "Cannot verify a unique session label across server pages; matching session UUID: {id}. Use it only if this is the session you want."
        ),
        Note::ResumeFailed { error } => {
            format!("failed to list sessions while resolving session label: {error}")
        }
    }
}

/// The lines of `status_surface_commands.rs` and `model_command.rs`.
fn refusal_words(refusal: &Refusal) -> String {
    let surface = |surface: &Surface| match surface {
        Surface::Title => ("terminal-title", "/title"),
        Surface::StatusLine => ("status-line", "/statusline"),
    };
    match refusal {
        Refusal::Args(ArgError::Quoting) => {
            "Invalid command arguments. Check the quoting.".to_string()
        }
        Refusal::Args(ArgError::NeedsValue(name)) => {
            format!("Command setting `{name}` requires a value.")
        }
        Refusal::Args(ArgError::TakesNoValue(name)) => {
            format!("Command setting `{name}` does not take a value.")
        }
        Refusal::Args(ArgError::OnlyOne(name)) => {
            format!("Specify only one of --{name} or --no-{name}.")
        }
        Refusal::Args(ArgError::Repeated(name)) => {
            format!("Command setting `{name}` was provided more than once.")
        }
        Refusal::Args(ArgError::Unknown(name)) => format!("Unknown command setting `{name}`."),
        Refusal::ClearWithItems(which) => {
            format!("Cannot combine --clear with {} items.", surface(which).0)
        }
        Refusal::UnknownItem {
            surface: which,
            token,
        } => {
            let (name, command) = surface(which);
            format!("Unknown {name} item '{token}'. Use {command} to see available items.")
        }
        Refusal::Unexpected(argument) => format!(
            "Unexpected argument '{argument}'. Use '--reasoning=<level>' or '--speed=<tier>'."
        ),
        Refusal::Starting => "Session is still starting; try /model again in a moment.".to_string(),
        Refusal::Reserve => {
            "Model arguments are unavailable while using Reserve. Use /model to adjust effort."
                .to_string()
        }
        Refusal::ModelUnavailable(model) => {
            format!("Model '{model}' is unavailable. Use /model to see available models.")
        }
        Refusal::EffortUnavailable {
            model,
            effort,
            available,
        } => format!(
            "Reasoning level '{effort}' is unavailable for {model}. Available: {available}."
        ),
        Refusal::SpeedUnavailable {
            model,
            speed,
            available,
        } => {
            format!("Speed '{speed}' is unavailable for {model}. Available: {available}.")
        }
    }
}

pub fn title(lang: Lang) -> &'static str {
    TABLES.pick(lang).title
}

impl Strings {
    /// "Could not save: permission denied".
    pub fn could_not_save<'a>(&'a self, error: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.notice_could_not_save, [Arg::Text(error)], &self.num)
    }

    /// "Notifications · Notify only while ...".
    pub fn search_desc<'a>(&'a self, group: &'a str, description: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.search_description,
            [Arg::Text(group), Arg::Text(description)],
            &self.num,
        )
    }

    /// "Automatic ›".
    pub fn choice<'a>(&'a self, label: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.value_choice, [Arg::Text(label)], &self.num)
    }

    /// "Custom .tmTheme files can be added to the ~/.codex/themes directory.".
    pub fn custom_dir<'a>(&'a self, dir: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.theme_custom_dir, [Arg::Text(dir)], &self.num)
    }

    /// "12 actions.".
    pub fn actions_many(&self, count: usize) -> Fmt<'_, 1> {
        Fmt::new(
            self.keymap_actions_many,
            [Arg::Int(count as u64)],
            &self.num,
        )
    }

    /// "1/2".
    pub fn counts(&self, active: usize, installed: usize) -> Fmt<'_, 2> {
        Fmt::new(
            self.hooks_counts,
            [Arg::Int(active as u64), Arg::Int(installed as u64)],
            &self.num,
        )
    }

    /// "3 hooks need review before they can run.".
    pub fn review_many(&self, count: usize) -> Fmt<'_, 1> {
        Fmt::new(self.hooks_review_many, [Arg::Int(count as u64)], &self.num)
    }

    /// "/tmp/hooks.json: failed to parse hooks config".
    pub fn hooks_error<'a>(&'a self, path: &'a str, message: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.hooks_error,
            [Arg::Text(path), Arg::Text(message)],
            &self.num,
        )
    }

    /// "PreToolUse hooks".
    pub fn handlers_title<'a>(&'a self, event: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.hooks_handlers_title, [Arg::Text(event)], &self.num)
    }

    /// "30s".
    pub fn timeout(&self, seconds: u64) -> Fmt<'_, 1> {
        Fmt::new(self.hooks_timeout, [Arg::Int(seconds)], &self.num)
    }

    /// "limit: 400 approximate tokens".
    pub fn context_limit(&self, limit: usize) -> Fmt<'_, 1> {
        Fmt::new(
            self.hooks_context_limit,
            [Arg::Int(limit as u64)],
            &self.num,
        )
    }

    /// "Plugin - superpowers@openai-curated", "Environment - {id}".
    pub fn source_of<'a>(&'a self, msg: Msg<1>, id: &'a str) -> Fmt<'a, 1> {
        Fmt::new(msg, [Arg::Text(id)], &self.num)
    }

    /// "User config - /tmp/h.json".
    pub fn source_path<'a>(&'a self, label: &'a str, path: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.hooks_source_path,
            [Arg::Text(label), Arg::Text(path)],
            &self.num,
        )
    }

    /// "Notifications · Notify only while ...".
    pub fn search_description<'a>(&'a self, group: &'a str, description: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.search_description,
            [Arg::Text(group), Arg::Text(description)],
            &self.num,
        )
    }
}

/// `(prefix of the live value, name, description)` of a limit item once its window is known
/// (`rate_limit_preview_copy`): the row is named after the window the value tells.
pub(super) const RATE_LIMIT_COPY: &[(&str, &str, &str)] = &[
    (
        "secondary usage ",
        "secondary-usage-limit",
        "Remaining usage on the secondary usage limit (omitted when unavailable)",
    ),
    (
        "usage ",
        "usage-limit",
        "Remaining usage on the primary usage limit (omitted when unavailable)",
    ),
    (
        "5h ",
        "five-hour-limit",
        "Remaining usage on the 5-hour usage limit (omitted when unavailable)",
    ),
    (
        "daily ",
        "daily-limit",
        "Remaining usage on the daily usage limit (omitted when unavailable)",
    ),
    (
        "weekly ",
        "weekly-limit",
        "Remaining usage on the weekly usage limit (omitted when unavailable)",
    ),
    (
        "monthly ",
        "monthly-limit",
        "Remaining usage on the monthly usage limit (omitted when unavailable)",
    ),
    (
        "annual ",
        "annual-limit",
        "Remaining usage on the annual usage limit (omitted when unavailable)",
    ),
];

/// `(id, description, preview)` of each item of the status line, in the oracle's order.
pub(super) const STATUS_LINE_ITEMS: &[(&str, &str, &str)] = &[
    ("model", "Current model name", "gpt-5.2-codex"),
    (
        "model-with-reasoning",
        "Current model name with reasoning level",
        "gpt-5.2-codex medium",
    ),
    ("reasoning", "Current reasoning level", "medium"),
    (
        "current-dir",
        "Current working directory",
        "~/my-project/subdir",
    ),
    (
        "project-name",
        "Project name (omitted when unavailable)",
        "my-project",
    ),
    (
        "hostname",
        "Current machine hostname (omitted when unavailable)",
        "my-host",
    ),
    (
        "git-branch",
        "Current Git branch (omitted when unavailable)",
        "feat/awesome-feature",
    ),
    (
        "pull-request-number",
        "Open pull request number for the current branch (omitted when unavailable)",
        "PR #123",
    ),
    (
        "branch-changes",
        "Committed branch changes against the default branch (omitted when unavailable)",
        "+12 -3",
    ),
    (
        "run-state",
        "Compact session run-state text (Ready, Working, Thinking)",
        "Working",
    ),
    (
        "permissions",
        "Active permission profile or sandbox mode",
        "Workspace",
    ),
    (
        "approval-mode",
        "Active command approval mode",
        "on-request",
    ),
    (
        "context-remaining",
        "Percentage of context window remaining (omitted when unknown)",
        "Context 0% left",
    ),
    (
        "context-used",
        "Percentage of context window used (omitted when unknown)",
        "Context 0% used",
    ),
    (
        "five-hour-limit",
        "Remaining usage on the primary usage limit (omitted when unavailable)",
        "primary 0%",
    ),
    (
        "weekly-limit",
        "Remaining usage on the secondary usage limit (omitted when unavailable)",
        "secondary 0%",
    ),
    ("codex-version", "Codex application version", "0.0.0"),
    (
        "context-window-size",
        "Total context window size in tokens (omitted when unknown)",
        "0 window",
    ),
    (
        "used-tokens",
        "Total tokens used in session (omitted when zero)",
        "0 used",
    ),
    (
        "total-input-tokens",
        "Total input tokens used in session",
        "0 in",
    ),
    (
        "total-output-tokens",
        "Total output tokens used in session",
        "0 out",
    ),
    (
        "thread-credits",
        "Estimated current-thread credits (Enterprise workspaces only; omitted when unavailable)",
        "5.2 credits",
    ),
    (
        "estimated-thread-cost",
        "Estimated current-thread cost in USD (Enterprise workspaces only; omitted when unavailable)",
        "~$1.82",
    ),
    (
        "thread-id",
        "Current thread identifier (omitted until thread starts)",
        "550e8400-e29b-41d4",
    ),
    (
        "fast-mode",
        "Effective processing tier for the current model",
        "Fast",
    ),
    (
        "daybreak",
        "Whether Daybreak is enabled for this thread",
        "Daybreak off",
    ),
    (
        "raw-output",
        "Whether raw scrollback mode is active",
        "raw output",
    ),
    (
        "thread-name",
        "Current thread name (omitted when unnamed)",
        "thread name",
    ),
    (
        "thread-title",
        "Current thread title, or thread identifier when unnamed",
        "thread title",
    ),
    (
        "workspace-headline",
        "Workspace notification headline (Enterprise workspaces only; omitted when unavailable)",
        "Workspace headline",
    ),
    (
        "task-progress",
        "Latest task progress from update_plan (omitted until available)",
        "Tasks 0/0",
    ),
    (
        "profile",
        "Config profile selected with --profile (omitted when unset)",
        "profile: <name>",
    ),
    (
        "provider",
        "Model provider for the active thread",
        "provider: <id>",
    ),
];

/// `(id, description, preview)` of each item of the terminal title, in the oracle's order.
pub(super) const TERMINAL_TITLE_ITEMS: &[(&str, &str, &str)] = &[
    ("app-name", "Codex app name", "codex"),
    (
        "project-name",
        "Project name (falls back to current directory name)",
        "my-project",
    ),
    (
        "current-dir",
        "Current working directory",
        "~/my-project/subdir",
    ),
    (
        "activity",
        "Spinner while working, action-required message while blocked",
        "",
    ),
    (
        "run-state",
        "Compact session run-state text (Ready, Working, Thinking)",
        "Working",
    ),
    (
        "thread-name",
        "Current thread name (omitted when unnamed)",
        "thread name",
    ),
    (
        "thread-title",
        "Current thread title, or thread identifier when unnamed",
        "thread title",
    ),
    (
        "git-branch",
        "Current Git branch (omitted when unavailable)",
        "feat/awesome-feature",
    ),
    (
        "context-remaining",
        "Percentage of context window remaining (omitted when unknown)",
        "Context 0% left",
    ),
    (
        "context-used",
        "Percentage of context window used (omitted when unknown)",
        "Context 0% used",
    ),
    (
        "five-hour-limit",
        "Remaining usage on the primary usage limit (omitted when unavailable)",
        "primary 0%",
    ),
    (
        "weekly-limit",
        "Remaining usage on the secondary usage limit (omitted when unavailable)",
        "secondary 0%",
    ),
    ("codex-version", "Codex application version", "0.0.0"),
    (
        "used-tokens",
        "Total tokens used in session (omitted when zero)",
        "0 used",
    ),
    (
        "total-input-tokens",
        "Total input tokens used in session",
        "0 in",
    ),
    (
        "total-output-tokens",
        "Total output tokens used in session",
        "0 out",
    ),
    (
        "thread-credits",
        "Estimated current-thread credits (Enterprise workspaces only; omitted when unavailable)",
        "5.2 credits",
    ),
    (
        "estimated-thread-cost",
        "Estimated current-thread cost (Enterprise workspaces only; omitted when unavailable)",
        "~$1.82",
    ),
    (
        "thread-id",
        "Current thread identifier (omitted until thread starts)",
        "550e8400-e29b-41d4",
    ),
    (
        "fast-mode",
        "Effective processing tier for the current model",
        "Fast",
    ),
    (
        "daybreak",
        "Whether Daybreak is enabled for this thread",
        "Daybreak off",
    ),
    ("model", "Current model name", "gpt-5.2-codex"),
    (
        "model-with-reasoning",
        "Current model name with reasoning level",
        "gpt-5.2-codex medium",
    ),
    ("reasoning", "Current reasoning level", "medium"),
    (
        "task-progress",
        "Latest task progress from update_plan (omitted until available)",
        "Tasks 0/0",
    ),
];
