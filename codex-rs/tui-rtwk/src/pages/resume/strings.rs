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
}
