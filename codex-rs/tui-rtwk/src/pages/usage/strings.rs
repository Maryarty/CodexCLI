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
    account_suffix: Msg<1>,
    /// Oracle: `render_filled_tab_bar`: the marks of a strip that scrolls.
    pub tab_left: &'static str,
    pub tab_right: &'static str,
    /// Oracle: `render.rs` labels: the digit and the name, from 80 cells.
    tab_numbered: Msg<2>,
    /// Oracle: `chrome.rs` `tab_label`, with the business name of Usage.
    pub tab_overview: &'static str,
    pub tab_usage: &'static str,
    pub tab_tokens: &'static str,
    pub tab_credits: &'static str,
    pub tab_messages: &'static str,
    pub tab_plugins: &'static str,
    pub tab_skills: &'static str,
    pub tab_chats: &'static str,
    pub tab_plan: &'static str,
    /// Oracle: `controls.rs`: the range, group and model controls.
    pub range_key: &'static str,
    pub range_week: &'static str,
    pub range_month: &'static str,
    pub range_slash: &'static str,
    pub group_key: &'static str,
    pub model_key: &'static str,
    pub all_models: &'static str,
    pub control_gap: &'static str,
    interval_same_year: Msg<5>,
    interval_years: Msg<6>,
    interval_numeric: Msg<6>,
    /// The months, abbreviated (chrono's `%b`).
    pub months: [&'static str; 12],
    /// Oracle: `hints.rs` `footer_lines`.
    /// The first row: `esc back`.
    pub back: (&'static str, &'static str),
    /// The second row: the key as the oracle writes it, its hint.
    hint_report: Msg<1>,
    pub hints: [(&'static str, &'static str); 3],
    /// Oracle: `render.rs`: no account, and the message of an account the
    /// dashboard cannot show.
    pub not_available: &'static str,
    /// Oracle: `summary_panel.rs` (the five numbers, the activity block).
    pub lifetime_tokens: &'static str,
    pub peak_tokens: &'static str,
    pub longest_chat: &'static str,
    pub current_streak: &'static str,
    pub longest_streak: &'static str,
    days: Msg<1>,
    pub activity: &'static str,
    pub last_year: &'static str,
    pub weekdays: [&'static str; 7],
    pub less: &'static str,
    pub more: &'static str,
    /// The heat map's cells and the legend's marks (oracle: `activity_chart`).
    pub cell_empty: &'static str,
    pub cell_active: &'static str,
    pub cell_bar: &'static str,
    pub cell_gap: &'static str,
    pub caption_indent: &'static str,
    pub axis_max: &'static str,
    pub axis_zero: &'static str,
    pub axis_none: &'static str,
    pub no_activity: &'static str,
    pub narrow_chart: &'static str,
    pub insights: &'static str,
    pub top_uses: &'static str,
    pub fast_mode: &'static str,
    pub reasoning: &'static str,
    pub skills_explored: &'static str,
    pub skills_used: &'static str,
    pub total_chats: &'static str,
    reasoning_value: Msg<2>,
    run: Msg<1>,
    runs: Msg<1>,
    as_of: Msg<3>,
    pub profile_unavailable: &'static str,
    pub history_unavailable: &'static str,
    pub empty_range: &'static str,
    pub no_top: &'static str,
    pub top_unavailable: &'static str,
    pub stats_error: &'static str,
    pub dash: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    blank: " ",
    title: "/usage dashboard",
    heading: "Usage",
    heading_plan: msg!("Usage · {0}"),
    account_suffix: msg!(" · {0}"),
    tab_left: "\u{2039}",
    tab_right: "\u{203a}",
    tab_numbered: msg!("{0} {1}"),
    tab_overview: "Overview",
    tab_usage: "Usage",
    tab_tokens: "Tokens",
    tab_credits: "Credits",
    tab_messages: "Messages",
    tab_plugins: "Plugins",
    tab_skills: "Skills",
    tab_chats: "Chats",
    tab_plan: "Plan",
    range_key: "r",
    range_week: "7d",
    range_month: "30d",
    range_slash: "/",
    group_key: "g",
    model_key: "m",
    all_models: "All models",
    control_gap: " · ",
    interval_same_year: msg!("{0} {1}\u{2013}{2} {3}, {4}"),
    interval_years: msg!("{0} {1}, {2}\u{2013}{3} {4}, {5}"),
    interval_numeric: msg!("{0}/{1}/{2}\u{2013}{3}/{4}/{5}"),
    months: [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ],
    back: ("esc", "back"),
    hint_report: msg!("tab/1\u{2013}{0}"),
    hints: [("z", "dashboard"), ("?", "help"), ("R", "refresh")],
    not_available: "Analytics is not available for this account type.",
    lifetime_tokens: "Lifetime tokens",
    peak_tokens: "Peak tokens",
    longest_chat: "Longest chat",
    current_streak: "Current streak",
    longest_streak: "Longest streak",
    days: msg!("{0} days"),
    activity: "Token activity",
    last_year: "Last 12 months",
    weekdays: ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"],
    less: "Less",
    more: "More",
    cell_empty: "□",
    cell_active: "■",
    cell_bar: "█",
    cell_gap: " ",
    caption_indent: "   ",
    axis_max: "max ",
    axis_zero: "  0 ",
    axis_none: "    ",
    no_activity: "No token activity in the last 12 months",
    narrow_chart: "Widen terminal to show activity graph",
    insights: "Activity insights",
    top_uses: "Most used plugins and skills",
    fast_mode: "Fast Mode",
    reasoning: "Most used reasoning",
    skills_explored: "Skills explored",
    skills_used: "Total skills used",
    total_chats: "Total chats",
    reasoning_value: msg!("{0} · {1}"),
    run: msg!("{0} run"),
    runs: msg!("{0} runs"),
    as_of: msg!("Statistics as of {0} {1}, {2}"),
    profile_unavailable: "Profile statistics unavailable.",
    history_unavailable: "Token activity history unavailable",
    empty_range: "No data reported for this range.",
    no_top: "No reported plugins or skills.",
    top_unavailable: "Plugin and skill usage unavailable.",
    stats_error: "Some profile statistics are unavailable.",
    dash: "\u{2014}",
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

    pub fn account_suffix(&self, account: &str, num: &NumSyms) -> String {
        Fmt::new(self.account_suffix, [Arg::Text(account)], num).to_string()
    }

    pub fn tab_numbered(&self, number: u64, name: &str, num: &NumSyms) -> String {
        Fmt::new(self.tab_numbered, [Arg::Int(number), Arg::Text(name)], num).to_string()
    }

    pub fn hint_report(&self, count: u64, num: &NumSyms) -> String {
        Fmt::new(self.hint_report, [Arg::Int(count)], num).to_string()
    }

    pub fn days(&self, count: u64, num: &NumSyms) -> String {
        Fmt::new(self.days, [Arg::Int(count)], num).to_string()
    }

    pub fn reasoning_value(&self, effort: &str, percent: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.reasoning_value,
            [Arg::Text(effort), Arg::Text(percent)],
            num,
        )
        .to_string()
    }

    /// `1,142 runs`, `1 run`.
    pub fn runs(&self, count: u64, num: &NumSyms) -> String {
        let msg = if count == 1 { self.run } else { self.runs };
        Fmt::new(msg, [Arg::Grouped(count)], num).to_string()
    }

    /// `Statistics as of Sep 9, 2026`.
    pub fn as_of(&self, (year, month, day): (i32, u32, u32), num: &NumSyms) -> String {
        Fmt::new(
            self.as_of,
            [
                Arg::Text(self.month(month)),
                Arg::Int(u64::from(day)),
                Arg::Int(year.max(0) as u64),
            ],
            num,
        )
        .to_string()
    }

    fn month(&self, month: u32) -> &'static str {
        self.months[(month as usize).saturating_sub(1).min(11)]
    }

    /// The dates of a report's range: `Sep 3–Sep 9, 2026` inside a year,
    /// with both years across two, numeric (`08/01/2025–09/09/2026`) when
    /// the room is under 50 cells (oracle: `controls.rs`).
    pub fn interval(
        &self,
        start: (i32, u32, u32),
        end: (i32, u32, u32),
        width: usize,
        num: &NumSyms,
    ) -> String {
        let year = |year: i32| Arg::Int(year.max(0) as u64);
        let day = |day: u32| Arg::Int(u64::from(day));
        if start.0 != end.0 && width < 50 {
            let two = |value: u32| format!("{value:02}");
            let (sm, sd, em, ed) = (two(start.1), two(start.2), two(end.1), two(end.2));
            let (sy, ey) = (format!("{:04}", start.0), format!("{:04}", end.0));
            return Fmt::new(
                self.interval_numeric,
                [
                    Arg::Text(&sm),
                    Arg::Text(&sd),
                    Arg::Text(&sy),
                    Arg::Text(&em),
                    Arg::Text(&ed),
                    Arg::Text(&ey),
                ],
                num,
            )
            .to_string();
        }
        if start.0 == end.0 {
            Fmt::new(
                self.interval_same_year,
                [
                    Arg::Text(self.month(start.1)),
                    day(start.2),
                    Arg::Text(self.month(end.1)),
                    day(end.2),
                    year(end.0),
                ],
                num,
            )
            .to_string()
        } else {
            Fmt::new(
                self.interval_years,
                [
                    Arg::Text(self.month(start.1)),
                    day(start.2),
                    year(start.0),
                    Arg::Text(self.month(end.1)),
                    day(end.2),
                    year(end.0),
                ],
                num,
            )
            .to_string()
        }
    }
}
