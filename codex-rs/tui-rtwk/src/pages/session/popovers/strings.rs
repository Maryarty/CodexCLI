//! The strings of `popovers` (S12): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::{Arg, DependCx, Fmt, Msg, NumSyms, msg};

use codex_tui_core::data::mention::Capability;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// The unified `@` popup's header, its query placeholder and tab labels
    /// (`mentions_v2/render.rs`, `search_mode.rs`).
    pub mentions_title: &'static str,
    pub mentions_subtitle: &'static str,
    pub mentions_placeholder: &'static str,
    pub tab_results: &'static str,
    pub tab_filesystem: &'static str,
    pub tab_plugins: &'static str,
    /// The marks of tabs out of view, the popup rows' cursor mark and blank,
    /// the marks of rows scrolled out of view.
    pub tab_left: &'static str,
    pub tab_right: &'static str,
    pub mark_on: &'static str,
    pub mark_off: &'static str,
    pub scroll_up: &'static str,
    pub scroll_down: &'static str,
    /// The type tag at the end of a row (`MentionType::span`): plugin,
    /// skill, task, file, directory.
    pub type_tags: [&'static str; 5],
    /// The `$` popup's category tags (`skill_popup.rs`).
    /// A file row at the working directory's top: its parent path.
    pub current_dir: &'static str,
    /// Between a file row's parent path and its description.
    pub description_gap: &'static str,
    pub tag_skill: &'static str,
    pub tag_app: &'static str,
    /// What an empty list says (`FileSearch::empty_message`).
    pub loading: &'static str,
    pub no_matches: &'static str,
    /// A plugin's description when it has none (`plugin_description`):
    /// `Plugin - skills - 2 MCP servers - 1 app`.
    pub plugin_word: &'static str,
    pub plugin_dash: &'static str,
    pub capability_skills: &'static str,
    pub mcp_server_one: &'static str,
    pub mcp_servers: Msg<1>,
    pub app_one: &'static str,
    pub apps: Msg<1>,
    /// The popups' hint line, each key and its word (`mentions_v2/footer.rs`,
    /// `skill_popup.rs` `skill_popup_hint_line`).
    pub hint_insert_keys: &'static str,
    pub hint_insert: &'static str,
    pub hint_close_key: &'static str,
    pub hint_close_more: &'static str,
    pub hint_select_keys: &'static str,
    pub hint_select: &'static str,
    pub hint_filter_keys: &'static str,
    pub hint_filter: &'static str,
    pub skill_hint_insert_key: &'static str,
    pub skill_hint_close: &'static str,
    pub num: NumSyms,
}

impl Strings {
    /// What a plugin without a description of its own says it carries.
    pub fn plugin_description(&self, capabilities: &[Capability]) -> String {
        let count = |n: usize| n.to_string();
        let words: Vec<String> = capabilities
            .iter()
            .map(|capability| match capability {
                Capability::Skills => self.capability_skills.to_string(),
                Capability::McpServers(1) => self.mcp_server_one.to_string(),
                Capability::McpServers(n) => {
                    Fmt::new(self.mcp_servers, [Arg::Text(&count(*n))], &self.num).to_string()
                }
                Capability::Apps(1) => self.app_one.to_string(),
                Capability::Apps(n) => {
                    Fmt::new(self.apps, [Arg::Text(&count(*n))], &self.num).to_string()
                }
            })
            .collect();
        if words.is_empty() {
            self.plugin_word.to_string()
        } else {
            format!(
                "{}{}{}",
                self.plugin_word,
                self.plugin_dash,
                words.join(self.plugin_dash)
            )
        }
    }
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "composer popovers",
    mentions_title: "Mentions",
    mentions_subtitle: "Files, directories, plugins, skills and tasks",
    mentions_placeholder: "Type to search mentions",
    tab_results: "All Results",
    tab_filesystem: "Filesystem Only",
    tab_plugins: "Plugins",
    tab_left: "\u{2039}",
    tab_right: "\u{203a}",
    mark_on: "\u{203a} ",
    mark_off: "  ",
    scroll_up: "\u{2191}",
    scroll_down: "\u{2193}",
    type_tags: ["Plugin", "Skill", "Task", "File", "Dir"],
    current_dir: "./",
    description_gap: "  ",
    tag_skill: "[Skill]",
    tag_app: "[App]",
    loading: "loading...",
    no_matches: "no matches",
    plugin_word: "Plugin",
    plugin_dash: " - ",
    capability_skills: "skills",
    mcp_server_one: "1 MCP server",
    mcp_servers: msg!("{0} MCP servers"),
    app_one: "1 app",
    apps: msg!("{0} apps"),
    hint_insert_keys: "enter/tab",
    hint_insert: " insert \u{b7} ",
    hint_close_key: "esc",
    hint_close_more: " close \u{b7} ",
    hint_select_keys: "\u{2191}/\u{2193}",
    hint_select: " select \u{b7} ",
    hint_filter_keys: "\u{2190}/\u{2192}",
    hint_filter: " filter",
    skill_hint_insert_key: "enter",
    skill_hint_close: " close",
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
