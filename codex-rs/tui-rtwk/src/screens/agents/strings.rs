//! The strings of `agents` (S21): a table of `&'static str` fields and typed
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

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// Oracle: `agent_center/render.rs`.
    pub header: &'static str,
    group: Msg<2>,
    pub grouping_project: &'static str,
    pub grouping_key: &'static str,
    pub filters: [&'static str; 5],
    filter_count: Msg<2>,
    pub filter_keys: &'static str,
    pub filter_label: &'static str,
    /// Oracle: `agent_center/rows.rs`.
    pub tasks: &'static str,
    group_heading: Msg<2>,
    group_heading_of: Msg<3>,
    pub no_tasks: &'static str,
    pub no_match: &'static str,
    /// Oracle: `agents_overview_view.rs` `status` and `render_details`.
    pub needs_input: &'static str,
    pub working: &'static str,
    pub ready: &'static str,
    pub inactive: &'static str,
    pub details: &'static str,
    pub model: &'static str,
    pub reasoning: &'static str,
    pub unknown: &'static str,
    pub project: &'static str,
    agent: Msg<1>,
    pub open_task: &'static str,
    pub wait_approval: &'static str,
    pub wait_response: &'static str,
    pub prompt: &'static str,
    pub no_prompt: &'static str,
    /// Oracle: `agent_center/hints.rs` `center_footer_hints`.
    pub key_help: &'static str,
    pub help: &'static str,
    pub key_back: &'static str,
    pub back: &'static str,
    pub key_move: &'static str,
    pub move_: &'static str,
    pub key_open: &'static str,
    pub open: &'static str,
    pub key_new: &'static str,
    pub new: &'static str,
    /// Oracle: `agent_center/render.rs`, `rows.rs`.
    pub grouping: [&'static str; 4],
    pub rename: &'static str,
    pub agent_label: &'static str,
    pub status_header: &'static str,
    pub updated_header: &'static str,
    pub pinned: &'static str,
    pub error: &'static str,
    pub show_more: &'static str,
    pub loading_more: &'static str,
    pub retry: &'static str,
    pub searching_history: &'static str,
    pub show_more_matches: &'static str,
    pub reconnecting_empty: &'static str,
    pub loading: &'static str,
    pub load_failed_empty: &'static str,
    pub archiving: &'static str,
    pub deleting: &'static str,
    pub creating_worktree: &'static str,
    pub load_failed: &'static str,
    pub reconnecting: &'static str,
    pub reconnect_failed: &'static str,
    pub search_label: &'static str,
    pub rename_label: &'static str,
    pub key_filter_label: &'static str,
    pub scroll_up: &'static str,
    pub scroll_down: &'static str,
    pub tab_left: &'static str,
    pub tab_right: &'static str,
    /// Oracle: `agents_overview_view.rs` `render_details`, `agents_overview_details.rs`.
    pub tokens: &'static str,
    pub tokens_in: Msg<1>,
    pub tokens_out: Msg<1>,
    pub tokens_sep: &'static str,
    pub est_usage: &'static str,
    credits: Msg<1>,
    pub branch: &'static str,
    pub last_message: &'static str,
    pub latest_activity: &'static str,
    pub needs_attention: &'static str,
    pub task_error: &'static str,
    pub elision: &'static str,
    pub ago: [&'static str; 4],
    pub now: &'static str,
    /// Oracle: `agent_center/hints.rs` `center_help_lines`.
    pub help_title: &'static str,
    pub help_resize: &'static str,
    pub help_navigate: &'static str,
    pub help_tasks: &'static str,
    pub help_view: &'static str,
    pub help_entries_navigate: [(&'static str, &'static str); 6],
    pub help_entries_tasks: [(&'static str, &'static str); 10],
    pub help_entries_view: [(&'static str, &'static str); 2],
    pub help_filter: &'static str,
    /// Oracle: `agents_overview_actions.rs` `confirm_agents_overview_action`.
    archive_title: Msg<1>,
    delete_title: Msg<1>,
    pub untitled_task: &'static str,
    pub archive_description_cloud: &'static str,
    pub archive_description: &'static str,
    pub delete_description: &'static str,
    pub archive_label_cloud: &'static str,
    pub archive_label: &'static str,
    pub delete_label: &'static str,
    pub cancel: &'static str,
    pub select_hint: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "agents command center",
    header: "Agent command center",
    group: msg!("  Group: {0}  {1}"),
    grouping_project: "Project",
    grouping_key: "g",
    filters: ["All", "Needs you", "Working", "Ready", "Inactive"],
    filter_count: msg!("{0} {1}"),
    // The oracle's `key_hint` labels Shift with a glyph on macOS and with
    // the word elsewhere (`MODIFIER_LABELS`).
    filter_keys: if cfg!(target_os = "macos") {
        "tab/\u{21e7}tab"
    } else {
        "tab/shift+tab"
    },
    filter_label: "filter",
    tasks: "Tasks",
    group_heading: msg!("{0}  {1}"),
    group_heading_of: msg!("{0}  {1} of {2}"),
    no_tasks: "No tasks yet",
    no_match: "No matching tasks",
    needs_input: "Needs input",
    working: "Working",
    ready: "Ready",
    inactive: "Inactive",
    details: "Task details",
    model: "Model: ",
    reasoning: "Reasoning: ",
    unknown: "Unknown",
    project: "Project",
    agent: msg!("Agent: {0}"),
    open_task: "Open task to review.",
    wait_approval: "Waiting for approval.",
    wait_response: "Waiting for your response.",
    prompt: "Prompt",
    no_prompt: "No prompt available.",
    key_help: "?",
    help: "help",
    key_back: "esc",
    back: "back",
    key_move: "\u{2191}/\u{2193}",
    move_: "move",
    key_open: "enter",
    open: "open",
    key_new: "n",
    new: "new",
    grouping: ["Project", "Directory", "Status", "Model"],
    rename: "rename",
    agent_label: "Agent: ",
    status_header: "Status",
    updated_header: "Updated",
    pinned: "Pinned",
    error: "Error",
    show_more: "Show more",
    loading_more: "Loading more\u{2026}",
    retry: "Show more (retry)",
    searching_history: "Searching history\u{2026}",
    show_more_matches: "Show more matches",
    reconnecting_empty: "Reconnecting\u{2026}",
    loading: "Loading tasks\u{2026}",
    load_failed_empty: "Could not load tasks",
    archiving: "Archiving task\u{2026}  You can still browse.",
    deleting: "Deleting task\u{2026}  You can still browse.",
    creating_worktree: "Creating worktree\u{2026}",
    load_failed: "Error loading tasks",
    reconnecting: "Reconnecting \u{2014} agent list is stale",
    reconnect_failed: "Reconnect failed \u{2014} agent list is stale; relaunch to retry",
    search_label: "Search \u{203a} ",
    rename_label: "Rename \u{203a} ",
    key_filter_label: "filter",
    scroll_up: "\u{2191}",
    scroll_down: "\u{2193}",
    tab_left: "\u{2039}",
    tab_right: "\u{203a}",
    tokens: "Tokens: ",
    tokens_in: msg!("{0} in"),
    tokens_out: msg!("{0} out"),
    tokens_sep: " \u{b7} ",
    est_usage: "Est. usage: ",
    credits: msg!("{0} credits"),
    branch: "Branch",
    last_message: "Last message",
    latest_activity: "Latest activity",
    needs_attention: "Needs attention",
    task_error: "Task encountered an error.",
    elision: "\u{2026}",
    ago: ["s ago", "m ago", "h ago", "d ago"],
    now: "now",
    help_title: "Task shortcuts",
    help_resize: "\u{2026} resize to see all",
    help_navigate: "Navigate",
    help_tasks: "Tasks",
    help_view: "View",
    help_entries_navigate: [
        ("\u{2191}", "Up"),
        ("\u{2193}", "Down"),
        ("enter", "Open"),
        ("pgup", "Page up"),
        ("pgdn", "Page down"),
        (
            if cfg!(target_os = "macos") {
                "\u{2303}c"
            } else if cfg!(target_os = "linux") {
                "^c"
            } else {
                "ctrl+c"
            },
            "Quit",
        ),
    ],
    help_entries_tasks: [
        ("n", "New"),
        ("w", "New worktree"),
        ("o", "Resume"),
        ("f", "Fork"),
        ("r", "Rename"),
        ("x", "Stop"),
        ("a", "Archive"),
        ("h", "Hide"),
        ("backspace", "Delete"),
        ("p", "Pin/unpin"),
    ],
    help_entries_view: [("f3", "Search"), ("g", "Group")],
    help_filter: "Filter",
    archive_title: msg!("Archive \u{201c}{0}\u{201d}?"),
    delete_title: msg!("Permanently delete \u{201c}{0}\u{201d}?"),
    untitled_task: "Untitled task",
    archive_description_cloud: "This stops any running work in this task and archives it. Child agents stay unchanged. Its history can be restored from the resume picker.",
    archive_description: "This stops any running work in this task and its child agents, then archives them. Their history can be restored from the resume picker.",
    delete_description: "Tasks with saved side chats stay intact. Otherwise, deleting this task stops its running work and permanently deletes its history and its child agents\u{2019} history.",
    archive_label_cloud: "Archive task",
    archive_label: "Archive task and child agents",
    delete_label: "Permanently delete task and child agents",
    cancel: "Cancel",
    select_hint: "enter select \u{b7} esc back",
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
    pub fn archive_title(&self, name: &str, num: &NumSyms) -> String {
        Fmt::new(self.archive_title, [Arg::Text(name)], num).to_string()
    }

    pub fn delete_title(&self, name: &str, num: &NumSyms) -> String {
        Fmt::new(self.delete_title, [Arg::Text(name)], num).to_string()
    }

    pub fn group(&self, grouping: &str, key: &str, num: &NumSyms) -> String {
        Fmt::new(self.group, [Arg::Text(grouping), Arg::Text(key)], num).to_string()
    }

    pub fn filter_count(&self, label: &str, count: usize, num: &NumSyms) -> String {
        let count = u64::try_from(count).unwrap_or(0);
        Fmt::new(self.filter_count, [Arg::Text(label), Arg::Int(count)], num).to_string()
    }

    pub fn group_heading(&self, heading: &str, shown: usize, total: usize, num: &NumSyms) -> String {
        let shown = u64::try_from(shown).unwrap_or(0);
        if shown as usize == total {
            return Fmt::new(self.group_heading, [Arg::Text(heading), Arg::Int(shown)], num).to_string();
        }
        let total = u64::try_from(total).unwrap_or(0);
        Fmt::new(
            self.group_heading_of,
            [Arg::Text(heading), Arg::Int(shown), Arg::Int(total)],
            num,
        )
        .to_string()
    }

    /// `12 in · 3 out`: the parts that are known.
    pub fn tokens_line(&self, input: Option<&str>, output: Option<&str>, num: &NumSyms) -> String {
        let a = input.map(|input| Fmt::new(self.tokens_in, [Arg::Text(input)], num).to_string());
        let b = output.map(|output| Fmt::new(self.tokens_out, [Arg::Text(output)], num).to_string());
        [a, b].into_iter().flatten().collect::<Vec<_>>().join(self.tokens_sep)
    }

    pub fn credits(&self, amount: &str, num: &NumSyms) -> String {
        Fmt::new(self.credits, [Arg::Text(amount)], num).to_string()
    }

    /// The values of the `Est. usage:` line, joined.
    pub fn join_values(&self, values: &[String]) -> String {
        values.join(self.tokens_sep)
    }

    pub fn agent(&self, name: &str, num: &NumSyms) -> String {
        Fmt::new(self.agent, [Arg::Text(name)], num).to_string()
    }
}
