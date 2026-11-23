//! The strings of `resume` (S18): a table of `&'static str` fields and typed
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
use crate::widgets::age::Age;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// An empty line, one cell wide.
    pub blank: &'static str,
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// Oracle: `SessionPickerAction::title`.
    pub header: &'static str,
    /// Oracle: `search_line`.
    pub search_hint: &'static str,
    /// Oracle: `render_empty_state_line`.
    pub loading: &'static str,
    pub empty: &'static str,
    /// Oracle: `render_list`: the indicators above and below the rows.
    pub more_above: &'static str,
    pub more_below: &'static str,
    /// Oracle: `FooterPart::text` for a missing cwd and branch.
    pub no_cwd: &'static str,
    pub no_branch: &'static str,
    /// Oracle: `format_relative_time`.
    pub age_missing: &'static str,
    pub age_now: &'static str,
    age_seconds: Msg<1>,
    age_minutes: Msg<1>,
    age_hours: Msg<1>,
    age_days: Msg<1>,
    /// Oracle: `toolbar_line`.
    pub filter: &'static str,
    pub filter_all: &'static str,
    pub status: &'static str,
    pub status_active: &'static str,
    pub status_archived: &'static str,
    pub sort: &'static str,
    pub sort_updated: &'static str,
    pub sort_created: &'static str,
    /// Oracle: `footer_hint_lines`: the keys and their labels, wide and
    /// compact.
    pub key_accept: &'static str,
    pub key_cancel: &'static str,
    pub key_quit: &'static str,
    pub key_focus: &'static str,
    pub key_option: &'static str,
    pub key_density: &'static str,
    pub key_transcript: &'static str,
    pub key_expand: &'static str,
    pub key_browse: &'static str,
    pub accept: &'static str,
    pub cancel_wide: &'static str,
    pub cancel_compact: &'static str,
    pub quit: &'static str,
    pub focus_wide: &'static str,
    pub focus_compact: &'static str,
    pub option_wide: &'static str,
    pub option_compact: &'static str,
    pub density_wide: &'static str,
    pub density_compact: &'static str,
    pub transcript_wide: &'static str,
    pub transcript_compact: &'static str,
    pub expand_wide: &'static str,
    pub expand_compact: &'static str,
    pub browse: &'static str,
    /// Oracle: `search_line`: the label before a typed query.
    pub search_label: &'static str,
    /// Oracle: `render_empty_state_line`.
    pub searching: &'static str,
    pub no_results: &'static str,
    pub loading_older: &'static str,
    /// Oracle: `render_list`: the line under the rows while more load.
    pub more_loading: &'static str,
    /// Oracle: `render_transcript_preview_lines`.
    pub preview_loading: &'static str,
    pub preview_failed: &'static str,
    pub preview_none: &'static str,
    /// The connectors before the expanded row's details and conversation.
    pub bar: &'static str,
    pub bar_end: &'static str,
    pub bar_bare: &'static str,
    pub marker_expanded: &'static str,
    /// Oracle: `render_expanded_session_details`.
    pub detail_session: &'static str,
    pub detail_created: &'static str,
    pub detail_updated: &'static str,
    pub detail_directory: &'static str,
    pub detail_branch: &'static str,
    pub detail_conversation: &'static str,
    pub detail_none: &'static str,
    /// Oracle: `render_transcript_loading_overlay`.
    pub overlay: &'static str,
    /// Oracle: `filter_mode_label`.
    pub filter_cwd: &'static str,
    pub filter_repo: &'static str,
    /// Oracle: `SessionPickerAction`.
    pub header_fork: &'static str,
    pub accept_fork: &'static str,
    pub accept_restore: &'static str,
    /// Oracle: `footer_hint_lines`.
    pub key_archive: &'static str,
    pub archive: &'static str,
    pub exit: &'static str,
    pub cancel_clear: &'static str,
    pub cancel_clear_compact: &'static str,
    pub density_comfy_wide: &'static str,
    pub density_comfy_compact: &'static str,
    pub key_loading: &'static str,
    pub loading_transcript: &'static str,
    /// Oracle: `inline_error` and `archive.rs`.
    pub problem_no_thread_id: &'static str,
    pub problem_no_metadata: &'static str,
    problem_no_metadata_at: Msg<1>,
    pub problem_no_transcript: &'static str,
    pub problem_transcript_failed: &'static str,
    pub problem_current_session: &'static str,
    pub problem_one_backend: &'static str,
    pub problem_more_failed: &'static str,
    problem_archive_failed: Msg<1>,
    problem_restore_failed: Msg<1>,
    /// Oracle: `format_relative_time_long` (the singular is its own text).
    long_second: &'static str,
    long_minute: &'static str,
    long_hour: &'static str,
    long_day: &'static str,
    long_seconds: Msg<1>,
    long_minutes: Msg<1>,
    long_hours: Msg<1>,
    long_days: Msg<1>,
    /// Oracle: `format_timestamp`: the strftime of each clock.
    pub time_12h: &'static str,
    pub time_24h: &'static str,
    /// Oracle: `render_list`, `SESSION_META_*_ICON`: the row's marker (the
    /// selected one and the others), the unread dot and the icons before a
    /// cwd and a branch.
    pub marker_selected: &'static str,
    pub marker_other: &'static str,
    pub unread_dot: &'static str,
    pub cwd_icon: &'static str,
    pub branch_icon: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    blank: " ",
    search_label: "Search: ",
    searching: "Searching…",
    no_results: "No results for your search",
    loading_older: "Loading older sessions…",
    more_loading: "↓ loading more",
    preview_loading: "Loading recent transcript...",
    preview_failed: "Could not load transcript preview",
    preview_none: "No transcript preview available",
    bar: "  │ ",
    bar_end: "  └ ",
    bar_bare: "  │",
    marker_expanded: "⌄ ",
    detail_session: "Session:",
    detail_created: "Created:",
    detail_updated: "Updated:",
    detail_directory: "Directory:",
    detail_branch: "Branch:",
    detail_conversation: "Conversation:",
    detail_none: "-",
    overlay: "Loading transcript…",
    filter_cwd: "Cwd",
    filter_repo: "Repo",
    header_fork: "Fork a previous session",
    accept_fork: "fork",
    accept_restore: "restore",
    key_archive: "⌃a",
    archive: "archive",
    exit: "exit",
    cancel_clear: "clear search",
    cancel_clear_compact: "clear",
    density_comfy_wide: "comfortable view",
    density_comfy_compact: "comfy",
    key_loading: "loading",
    loading_transcript: "transcript",
    problem_no_thread_id: "Selected session does not have a thread ID.",
    problem_no_metadata: "Failed to read session metadata from selected session",
    problem_no_metadata_at: msg!("Failed to read session metadata from {0}"),
    problem_no_transcript: "No transcript available for this session",
    problem_transcript_failed: "Could not load transcript preview",
    problem_current_session: "Use /archive to archive the current session and exit.",
    problem_one_backend: "Could not load one backend. Reopen the picker to retry.",
    problem_more_failed: "Could not load more sessions",
    problem_archive_failed: msg!("Failed to archive session: {0}"),
    problem_restore_failed: msg!("Failed to restore archived session: {0}"),
    long_second: "1 second ago",
    long_minute: "1 minute ago",
    long_hour: "1 hour ago",
    long_day: "1 day ago",
    long_seconds: msg!("{0} seconds ago"),
    long_minutes: msg!("{0} minutes ago"),
    long_hours: msg!("{0} hours ago"),
    long_days: msg!("{0} days ago"),
    time_12h: "%Y-%m-%d %-I:%M:%S %p",
    time_24h: "%Y-%m-%d %H:%M:%S",
    marker_selected: "› ",
    marker_other: "  ",
    unread_dot: "● ",
    cwd_icon: "⌁ ",
    branch_icon: "\u{e0a0} ",
    title: "resume and fork picker",
    header: "Resume a previous session",
    search_hint: "Type to search",
    loading: "Loading sessions…",
    empty: "No sessions yet",
    more_above: "↑ more",
    more_below: "↓ more",
    no_cwd: "no cwd",
    no_branch: "no branch",
    age_missing: "-",
    age_now: "now",
    age_seconds: msg!("{0}s ago"),
    age_minutes: msg!("{0}m ago"),
    age_hours: msg!("{0}h ago"),
    age_days: msg!("{0}d ago"),
    filter: "Filter:",
    filter_all: "All",
    status: "Status:",
    status_active: "Active",
    status_archived: "Archived",
    sort: "Sort:",
    sort_updated: "Updated",
    sort_created: "Created",
    key_accept: "enter",
    key_cancel: "esc",
    key_quit: "⌃c",
    key_focus: "tab",
    key_option: "←/→",
    key_density: "⌃o",
    key_transcript: "⌃t",
    key_expand: "⌃e",
    key_browse: "↑/↓",
    accept: "resume",
    cancel_wide: "start new",
    cancel_compact: "new",
    quit: "quit",
    focus_wide: "focus sort/filter",
    focus_compact: "focus",
    option_wide: "change option",
    option_compact: "option",
    density_wide: "dense view",
    density_compact: "dense",
    transcript_wide: "transcript",
    transcript_compact: "preview",
    expand_wide: "expand",
    expand_compact: "exp",
    browse: "browse",
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
    /// An age as the row's date column shows it.
    /// One value of a toolbar control: the name between two blanks.
    pub fn value(&self, text: &str) -> String {
        format!(" {text} ")
    }

    pub fn age(&self, age: Age, num: &NumSyms) -> String {
        let number = |value: i64| [Arg::Int(u64::try_from(value).unwrap_or(0))];
        match age {
            Age::Missing => self.age_missing.to_string(),
            Age::Now => self.age_now.to_string(),
            Age::Seconds(n) => Fmt::new(self.age_seconds, number(n), num).to_string(),
            Age::Minutes(n) => Fmt::new(self.age_minutes, number(n), num).to_string(),
            Age::Hours(n) => Fmt::new(self.age_hours, number(n), num).to_string(),
            Age::Days(n) => Fmt::new(self.age_days, number(n), num).to_string(),
        }
    }

    /// An age as the expanded details write it (oracle:
    /// `format_relative_time_long`).
    pub fn age_long(&self, age: Age, num: &NumSyms) -> String {
        let plural = |one: &'static str, many: Msg<1>, value: i64| {
            if value == 1 {
                one.to_string()
            } else {
                Fmt::new(many, [Arg::Int(u64::try_from(value).unwrap_or(0))], num).to_string()
            }
        };
        match age {
            Age::Missing => self.age_missing.to_string(),
            Age::Now => self.age_now.to_string(),
            Age::Seconds(n) => plural(self.long_second, self.long_seconds, n),
            Age::Minutes(n) => plural(self.long_minute, self.long_minutes, n),
            Age::Hours(n) => plural(self.long_hour, self.long_hours, n),
            Age::Days(n) => plural(self.long_day, self.long_days, n),
        }
    }

    /// A time of the details (oracle: `format_timestamp`), in UTC like the
    /// oracle's.
    pub fn timestamp(&self, epoch: i64, h24: bool) -> String {
        let format = if h24 { self.time_24h } else { self.time_12h };
        chrono::DateTime::from_timestamp(epoch, 0)
            .map(|time| time.format(format).to_string())
            .unwrap_or_default()
    }

    /// The words of what the line under the list reports.
    pub fn problem(
        &self,
        problem: &codex_tui_core::screens::resume::Problem,
        num: &NumSyms,
    ) -> String {
        use codex_tui_core::screens::resume::Problem;
        let text = |msg: Msg<1>, arg: &str| Fmt::new(msg, [Arg::Text(arg)], num).to_string();
        match problem {
            Problem::NoThreadId => self.problem_no_thread_id.to_string(),
            Problem::NoMetadata(None) => self.problem_no_metadata.to_string(),
            Problem::NoMetadata(Some(path)) => text(self.problem_no_metadata_at, path),
            Problem::NoTranscript => self.problem_no_transcript.to_string(),
            Problem::TranscriptFailed => self.problem_transcript_failed.to_string(),
            Problem::CurrentSession => self.problem_current_session.to_string(),
            Problem::OneBackend => self.problem_one_backend.to_string(),
            Problem::MoreFailed => self.problem_more_failed.to_string(),
            Problem::ArchiveFailed(error) => text(self.problem_archive_failed, error),
            Problem::RestoreFailed(error) => text(self.problem_restore_failed, error),
        }
    }
}
