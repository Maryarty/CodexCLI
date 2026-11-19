//! The strings of `usage` (S20): a table of `&'static str` fields and typed
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
    /// An empty line, one cell wide.
    pub blank: &'static str,
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// Oracle: `render.rs`: the heading, with and without a plan.
    pub heading: &'static str,
    heading_plan: Msg<1>,
    /// Oracle: `chrome.rs` `tab_label`, in tab order.
    pub tabs: [&'static str; 6],
    /// Oracle: `controls.rs`, the period control.
    pub control_daily: &'static str,
    /// Oracle: `hints.rs` `footer_lines`.
    pub back: &'static str,
    pub hints: &'static str,
    /// Oracle: `data.rs` `Remote::message`, `chart.rs` and the errors.
    pub loading: &'static str,
    pub unavailable: &'static str,
    pub failed_analytics: &'static str,
    pub failed_credits: &'static str,
    pub empty_range: &'static str,
    /// Oracle: `summary.rs` (the five numbers, the activity block).
    pub lifetime_tokens: &'static str,
    pub peak_tokens: &'static str,
    pub longest_chat: &'static str,
    pub current_streak: &'static str,
    pub longest_streak: &'static str,
    days: Msg<1>,
    duration: Msg<2>,
    /// The suffixes of thousands, millions, billions and trillions.
    pub units: [&'static str; 4],
    pub activity: &'static str,
    pub last_year: &'static str,
    pub weekdays: [&'static str; 7],
    pub less: &'static str,
    pub more: &'static str,
    /// The heat map's cells and the legend's marks (oracle: `chart.rs`).
    pub cell_none: &'static str,
    pub cell_some: &'static str,
    pub legend_none: &'static str,
    pub legend_scale: &'static str,
    pub legend_gap: &'static str,
    pub insights: &'static str,
    pub top_uses: &'static str,
    runs: Msg<1>,
    as_of: Msg<1>,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    blank: " ",
    title: "/usage dashboard",
    heading: "Usage",
    heading_plan: msg!("Usage · {0}"),
    tabs: ["Overview", "Usage", "Messages", "Plugins", "Skills", "Chats"],
    control_daily: "g Daily",
    back: "esc back",
    hints: "tab/1\u{2013}6 report · z dashboard · ? help · R refresh",
    loading: "Loading…",
    unavailable: "No history has been reported.",
    failed_analytics: "Couldn't load analytics. Press R to retry.",
    failed_credits: "Couldn't load credits. Press R to retry.",
    empty_range: "No data reported for this range.",
    lifetime_tokens: "Lifetime tokens",
    peak_tokens: "Peak tokens",
    longest_chat: "Longest chat",
    current_streak: "Current streak",
    longest_streak: "Longest streak",
    days: msg!("{0} days"),
    duration: msg!("{0}h {1}m"),
    units: ["K", "M", "B", "T"],
    activity: "Token activity",
    last_year: "Last 12 months",
    weekdays: ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
    less: "Less",
    more: "More",
    cell_none: " □",
    cell_some: " ■",
    legend_none: " □ ",
    legend_scale: "■ ■ ■ ■",
    legend_gap: " ",
    insights: "Activity insights",
    top_uses: "Most used plugins and skills",
    runs: msg!("{0} runs"),
    as_of: msg!("Statistics as of {0}"),
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
    pub fn heading_with(&self, plan: &str, num: &NumSyms) -> String {
        Fmt::new(self.heading_plan, [Arg::Text(plan)], num).to_string()
    }

    pub fn days(&self, count: u64, num: &NumSyms) -> String {
        Fmt::new(self.days, [Arg::Int(count)], num).to_string()
    }

    pub fn duration(&self, hours: u64, minutes: u64, num: &NumSyms) -> String {
        Fmt::new(self.duration, [Arg::Int(hours), Arg::Int(minutes)], num).to_string()
    }

    pub fn runs(&self, count: u64, num: &NumSyms) -> String {
        Fmt::new(self.runs, [Arg::Grouped(count)], num).to_string()
    }

    pub fn as_of(&self, date: &str, num: &NumSyms) -> String {
        Fmt::new(self.as_of, [Arg::Text(date)], num).to_string()
    }
}
