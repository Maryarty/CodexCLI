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

use codex_tui_core::screens::usage::NaiveDate;
use codex_tui_core::screens::usage::report::Unit;

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
    /// Oracle: `hints.rs`: the words of the footer rows (the keys are the keymap's).
    pub back: &'static str,
    /// The second row: the key as the oracle writes it, its hint.
    hint_report: Msg<1>,
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
    /// Oracle: `chart.rs`: the report's total line, notes, summary and table.
    pub unit_tokens: &'static str,
    pub unit_messages: &'static str,
    pub unit_calls: &'static str,
    pub unit_uses: &'static str,
    pub unit_credits: &'static str,
    pub reported: &'static str,
    total_line: Msg<3>,
    pub relative_head: &'static str,
    pub approximate: &'static str,
    pub turn_start: &'static str,
    no_unit_range: Msg<1>,
    pub no_activity_range: &'static str,
    pub summary_not_reported: &'static str,
    pub summary_no_activity: &'static str,
    summary_relative: Msg<1>,
    summary_amount: Msg<2>,
    day_not_reported: Msg<1>,
    day_no_unit: Msg<2>,
    pub head_plugin: &'static str,
    pub head_skill: &'static str,
    pub head_chat: &'static str,
    pub head_tokens: &'static str,
    pub head_share: &'static str,
    pub head_messages: &'static str,
    pub head_calls: &'static str,
    pub head_uses: &'static str,
    pub head_credits: &'static str,
    pub bullet: &'static str,
    pub fold_mark: &'static str,
    more_rows: Msg<1>,
    date_short: Msg<2>,
    pub hint_day: &'static str,
    pub hint_details: &'static str,
    /// Oracle: `chrome.rs` `help_lines`, `controls.rs`: the help's heading
    /// (in the controls' place) and its lines; `{n}` are key labels.
    pub help_heading: &'static str,
    help_tabs: Msg<1>,
    help_digits: Msg<1>,
    help_days: Msg<2>,
    help_rows: Msg<2>,
    help_details: Msg<1>,
    help_scroll: Msg<2>,
    pub help_mouse: &'static str,
    pub help_range: &'static str,
    pub help_group: &'static str,
    pub help_model: &'static str,
    pub help_metric: &'static str,
    pub help_zero: &'static str,
    pub help_refresh: &'static str,
    help_zoom: Msg<1>,
    pub help_toggle: &'static str,
    help_close: Msg<2>,
    /// Oracle: `hints.rs`: the words of the footer rows.
    pub hint_focus: &'static str,
    pub hint_card: &'static str,
    pub hint_period: &'static str,
    pub hint_row: &'static str,
    pub hint_scroll: &'static str,
    pub hint_close_help: &'static str,
    pub hint_report_word: &'static str,
    pub hint_all: &'static str,
    pub hint_dashboard: &'static str,
    pub hint_help: &'static str,
    pub hint_refresh: &'static str,
    pub hint_retry: &'static str,
    pub hint_tab_key: &'static str,
    /// Oracle: `controls.rs`, `chart.rs`, `chrome.rs`: the times (`Updated`,
    /// `Report updated`) on the user's clock, `ClockFormat::date_time_format`.
    date_time_24: Msg<4>,
    date_time_12: Msg<5>,
    pub am: &'static str,
    pub pm: &'static str,
    updated_card: Msg<1>,
    updated_controls: Msg<1>,
    help_updated: Msg<1>,
    /// Oracle: `controls.rs`: the dashboard's control and its key hint.
    pub dashboard: &'static str,
    dashboard_focus: Msg<1>,
    /// Oracle: `dashboard.rs`: a card's title, its range and grouping lines
    /// and the mark of a cut card.
    card_title: Msg<2>,
    card_range_model: Msg<2>,
    card_group: Msg<1>,
    pub card_maximize: &'static str,
    /// The mark before the focused card's name.
    pub card_mark: &'static str,
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
    back: "back",
    hint_report: msg!("tab/1\u{2013}{0}"),
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
    // chart.rs
    unit_tokens: "tokens",
    unit_messages: "messages",
    unit_calls: "calls",
    unit_uses: "uses",
    unit_credits: "credits",
    reported: "reported ",
    total_line: msg!("{0} {1}{2}"),
    relative_head: "Total usage \u{b7} relative units",
    approximate: "Approximate \u{b7} may be delayed up to 6 hours",
    turn_start: "All usage by how each turn started",
    no_unit_range: msg!("No {0} reported in this range."),
    no_activity_range: "No activity reported in this range.",
    summary_not_reported: " \u{b7} Not reported",
    summary_no_activity: " \u{b7} No activity (0)",
    summary_relative: msg!(" \u{b7} {0} usage units"),
    summary_amount: msg!(" \u{b7} {0} {1}"),
    day_not_reported: msg!("Data not reported for {0}."),
    day_no_unit: msg!("No {0} reported for {1}."),
    head_plugin: "Plugin",
    head_skill: "Skill",
    head_chat: "Chat",
    head_tokens: "Tokens",
    head_share: "Share",
    head_messages: "Messages",
    head_calls: "Calls",
    head_uses: "Uses",
    head_credits: "Credits",
    bullet: "\u{25cf} ",
    fold_mark: "\u{2026} ",
    more_rows: msg!("{0} more"),
    date_short: msg!("{0} {1}"),
    hint_day: "day",
    hint_details: "details",
    help_heading: "Usage shortcuts",
    help_tabs: msg!("Tab / {0} \u{b7} next / previous report"),
    help_digits: msg!("1\u{2013}{0} \u{b7} open a report directly"),
    help_days: msg!("{0} / {1} \u{b7} select chart day or plan window"),
    help_rows: msg!("{0} / {1} \u{b7} select chat or plan period; scroll Overview"),
    help_details: msg!("{0} \u{b7} expand or collapse details"),
    help_scroll: msg!("{0} / {1} \u{b7} scroll report"),
    help_mouse: "In alternate screen: mouse wheel \u{b7} scroll; click tabs or report controls",
    help_range: "r \u{b7} switch between 7 and 30 days",
    help_group: "g \u{b7} change grouping / Overview aggregation",
    help_model: "m \u{b7} cycle model filter",
    help_metric: "s \u{b7} change chat sort metric",
    help_zero: "a \u{b7} show zero-credit groups in expanded details",
    help_refresh: "R \u{b7} refresh all reports",
    help_zoom: msg!("z \u{b7} dashboard / focused report; {0} \u{b7} focus dashboard card"),
    help_toggle: "? \u{b7} toggle this help",
    help_close: msg!("{0} \u{b7} back; q / {1} \u{b7} close usage"),
    hint_focus: "focus",
    hint_card: "card",
    hint_period: "period",
    hint_row: "row",
    hint_scroll: "scroll",
    hint_close_help: "close help",
    hint_report_word: "report",
    hint_all: "all",
    hint_dashboard: "dashboard",
    hint_help: "help",
    hint_refresh: "refresh",
    hint_retry: "retry",
    hint_tab_key: "tab",
    date_time_24: msg!("{0} {1} {2}:{3}"),
    date_time_12: msg!("{0} {1} {2}:{3} {4}"),
    am: "AM",
    pm: "PM",
    updated_card: msg!("Updated {0} UTC"),
    updated_controls: msg!("Updated \u{b7} {0} UTC"),
    help_updated: msg!("Report updated {0} UTC"),
    dashboard: "Dashboard",
    dashboard_focus: msg!(" \u{b7} {0} focus"),
    card_title: msg!(" {0} {1} "),
    card_range_model: msg!("{0} \u{b7} {1}"),
    card_group: msg!("By {0}"),
    card_maximize: "\u{2026} z to maximize",
    card_mark: "\u{25b8}",
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

    pub fn help_tabs(&self, shift_tab: &str, num: &NumSyms) -> String {
        Fmt::new(self.help_tabs, [Arg::Text(shift_tab)], num).to_string()
    }

    pub fn help_digits(&self, count: u64, num: &NumSyms) -> String {
        Fmt::new(self.help_digits, [Arg::Int(count)], num).to_string()
    }

    pub fn help_days(&self, left: &str, right: &str, num: &NumSyms) -> String {
        Fmt::new(self.help_days, [Arg::Text(left), Arg::Text(right)], num).to_string()
    }

    pub fn help_rows(&self, up: &str, down: &str, num: &NumSyms) -> String {
        Fmt::new(self.help_rows, [Arg::Text(up), Arg::Text(down)], num).to_string()
    }

    pub fn help_details(&self, accept: &str, num: &NumSyms) -> String {
        Fmt::new(self.help_details, [Arg::Text(accept)], num).to_string()
    }

    pub fn help_scroll(&self, up: &str, down: &str, num: &NumSyms) -> String {
        Fmt::new(self.help_scroll, [Arg::Text(up), Arg::Text(down)], num).to_string()
    }

    pub fn help_zoom(&self, accept: &str, num: &NumSyms) -> String {
        Fmt::new(self.help_zoom, [Arg::Text(accept)], num).to_string()
    }

    pub fn help_close(&self, cancel: &str, interrupt: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.help_close,
            [Arg::Text(cancel), Arg::Text(interrupt)],
            num,
        )
        .to_string()
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

    /// The word of a unit.
    pub fn unit(&self, unit: Unit) -> &'static str {
        match unit {
            Unit::Tokens => self.unit_tokens,
            Unit::Messages => self.unit_messages,
            Unit::Calls => self.unit_calls,
            Unit::Uses => self.unit_uses,
            Unit::Credits => self.unit_credits,
        }
    }

    /// `1.5K tokens`, `12 reported messages`.
    pub fn total_line(&self, amount: &str, unit: Unit, reported: bool, num: &NumSyms) -> String {
        Fmt::new(
            self.total_line,
            [
                Arg::Text(amount),
                Arg::Text(if reported { self.reported } else { "" }),
                Arg::Text(self.unit(unit)),
            ],
            num,
        )
        .to_string()
    }

    pub fn no_unit_range(&self, unit: Unit, num: &NumSyms) -> String {
        Fmt::new(self.no_unit_range, [Arg::Text(self.unit(unit))], num).to_string()
    }

    pub fn summary_relative(&self, amount: &str, num: &NumSyms) -> String {
        Fmt::new(self.summary_relative, [Arg::Text(amount)], num).to_string()
    }

    pub fn summary_amount(&self, amount: &str, unit: Unit, num: &NumSyms) -> String {
        Fmt::new(
            self.summary_amount,
            [Arg::Text(amount), Arg::Text(self.unit(unit))],
            num,
        )
        .to_string()
    }

    pub fn day_not_reported(&self, date: &str, num: &NumSyms) -> String {
        Fmt::new(self.day_not_reported, [Arg::Text(date)], num).to_string()
    }

    pub fn day_no_unit(&self, unit: Unit, date: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.day_no_unit,
            [Arg::Text(self.unit(unit)), Arg::Text(date)],
            num,
        )
        .to_string()
    }

    pub fn more_rows(&self, count: u64, num: &NumSyms) -> String {
        Fmt::new(self.more_rows, [Arg::Int(count)], num).to_string()
    }

    /// `Sep 2` (chrono's `%b %-d`), or `2` without the month.
    pub fn date_short(
        &self,
        (_, month, day): (i32, u32, u32),
        with_month: bool,
        num: &NumSyms,
    ) -> String {
        if with_month {
            Fmt::new(
                self.date_short,
                [Arg::Text(self.month(month)), Arg::Int(u64::from(day))],
                num,
            )
            .to_string()
        } else {
            day.to_string()
        }
    }

    /// `Sep 2 16:00`, or `Sep 2 4:00 PM` on a 12-hour clock (chrono's
    /// `%b %-d %H:%M` and `%b %-d %-I:%M %p`) of a Unix time in UTC.
    pub fn date_time(&self, seconds: i64, hour24: bool, num: &NumSyms) -> String {
        let (_, month, day) = NaiveDate::from_unix_secs(seconds).ymd();
        let of_day = seconds.rem_euclid(86_400);
        let (hour, minute) = ((of_day / 3_600) as u64, (of_day % 3_600 / 60) as u64);
        let (month, day) = (Arg::Text(self.month(month)), Arg::Int(u64::from(day)));
        let minute = format!("{minute:02}");
        if hour24 {
            let hour = format!("{hour:02}");
            return Fmt::new(
                self.date_time_24,
                [month, day, Arg::Text(&hour), Arg::Text(&minute)],
                num,
            )
            .to_string();
        }
        let shown = match hour % 12 {
            0 => 12,
            hour => hour,
        };
        let hour_text = shown.to_string();
        Fmt::new(
            self.date_time_12,
            [
                month,
                day,
                Arg::Text(&hour_text),
                Arg::Text(&minute),
                Arg::Text(if hour >= 12 { self.pm } else { self.am }),
            ],
            num,
        )
        .to_string()
    }

    /// `Updated Sep 2 16:00 UTC` (a card).
    pub fn updated_card(&self, seconds: i64, hour24: bool, num: &NumSyms) -> String {
        let time = self.date_time(seconds, hour24, num);
        Fmt::new(self.updated_card, [Arg::Text(&time)], num).to_string()
    }

    /// `Updated · Sep 2 16:00 UTC` (the controls' last row).
    pub fn updated_controls(&self, seconds: i64, hour24: bool, num: &NumSyms) -> String {
        let time = self.date_time(seconds, hour24, num);
        Fmt::new(self.updated_controls, [Arg::Text(&time)], num).to_string()
    }

    /// `Report updated Sep 2 16:00 UTC` (the help).
    pub fn help_updated(&self, seconds: i64, hour24: bool, num: &NumSyms) -> String {
        let time = self.date_time(seconds, hour24, num);
        Fmt::new(self.help_updated, [Arg::Text(&time)], num).to_string()
    }

    /// ` · enter focus` after `Dashboard`.
    pub fn dashboard_focus(&self, accept: &str, num: &NumSyms) -> String {
        Fmt::new(self.dashboard_focus, [Arg::Text(accept)], num).to_string()
    }

    /// ` 2 Usage `, the words of a card's title.
    pub fn card_title(&self, number: u64, name: &str, num: &NumSyms) -> String {
        Fmt::new(self.card_title, [Arg::Int(number), Arg::Text(name)], num).to_string()
    }

    /// `7d`, `30d`, with ` · <model>` after it on a business Usage card.
    pub fn card_range(&self, long: bool, model: Option<&str>, num: &NumSyms) -> String {
        let days = if long {
            self.range_month
        } else {
            self.range_week
        };
        match model {
            Some(model) => Fmt::new(
                self.card_range_model,
                [Arg::Text(days), Arg::Text(model)],
                num,
            )
            .to_string(),
            None => days.to_owned(),
        }
    }

    /// `By feature`: the grouping's name in lower case.
    pub fn card_group(&self, group: &str, num: &NumSyms) -> String {
        Fmt::new(self.card_group, [Arg::Text(&group.to_lowercase())], num).to_string()
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
