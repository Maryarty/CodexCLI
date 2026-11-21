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
    /// bottom_pane/popup_consts.rs: `standard_popup_hint_line`.
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
    pub check_on: &'static str,
    pub check_off: &'static str,
    pub section_break: &'static str,
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
    popup_hint: "Press enter to confirm or esc to go back",
    status_title: "Configure Status Line",
    status_subtitle: "Select which items to display in the status line.",
    status_search_placeholder: "Type to search",
    use_theme_colors: "Use theme colors",
    use_theme_colors_description: "Apply colors from the active /theme",
    check_on: "[x] ",
    check_off: "[ ] ",
    section_break: "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
    status_hint: "space toggle \u{b7} \u{2190}/\u{2192} reorder \u{b7} enter save \u{b7} esc cancel",
    title_setup_title: "Configure Terminal Title",
    title_setup_subtitle: "Select which items to display in the terminal title.",
    keymap_search_placeholder: "Type to search shortcuts",
    keymap_hint: "left/right group \u{b7} enter edit shortcut \u{b7} * custom \u{b7} - unbound \u{b7} esc close",
    keymap_empty: "No shortcuts in this group",
    keymap_title: "Keymap",
    keymap_actions_one: "1 action.",
    keymap_actions_many: msg!("{0} actions."),
    keymap_unbound: "-",
    keymap_bound: " ",
    current_marker: " (current)",
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

    /// "Notifications · Notify only while ...".
    pub fn search_description<'a>(&'a self, group: &'a str, description: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.search_description,
            [Arg::Text(group), Arg::Text(description)],
            &self.num,
        )
    }
}
