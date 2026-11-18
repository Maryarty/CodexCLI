//! `usage` (S20): the view half of the `/usage` dashboard. One `PageRoute`
//! over the model (`ScreenStates.usage`): the heading, the report tabs, a rule,
//! the period control, the report's body and the key hints. The overview is
//! the oracle's summary: the profile block, the five numbers, the activity
//! heat map and the insights, centred in the body. Its model, events and
//! intents are core's `codex_tui_core::screens::usage`.
//! Oracle: `tui/src/analytics/` (`render.rs`, `summary.rs`, `chrome.rs`,
//! `hints.rs`, `data.rs`, `chart.rs`).

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, watch};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk::{Cx, l10n::NumSyms};
use rtwk_core::style::Attrs;

use codex_tui_core::screens::usage::{
    Failure, Intent as UsageIntent, Model, Overview, Panel, Report,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::theme;
use crate::ui::seeded::Seeded;
use crate::ui::view::Hs;

pub mod strings;

use strings::Strings;

/// What the debug menu lists for this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "overview",
            description: "the summary of a Plus account at 140 cells",
            script: include_str!("fixtures/overview.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { panels_only: false }.into()),
        },
        DebugEntry {
            name: "panel_states",
            description: "four panels: loading, unavailable, an error, an empty range",
            script: include_str!("fixtures/panel_states.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { panels_only: true }.into()),
        },
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen { panels_only: false }.into())
}

/// Opens the dashboard on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen { panels_only: false }.into())
}

/// The dashboard; `panels_only` shows just the four panels' messages (the
/// oracle's test renders those without chrome).
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub panels_only: bool,
}

/// Widget state: none (R14); the report is the model's.
#[derive(Debug)]
pub struct ScreenState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Report(Report),
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    Plain,
    Dim,
    Bold,
    Accent,
    Selected,
}

type Line = Vec<(String, Look)>;

fn text(text: impl Into<String>, look: Look) -> Line {
    vec![(text.into(), look)]
}

fn pad(mut line: Line, left: usize) -> Line {
    if left > 0 && !line.is_empty() {
        line.insert(0, (" ".repeat(left), Look::Plain));
    }
    line
}

fn centered(line: Line, width: usize) -> Line {
    let used: usize = line.iter().map(|(t, _)| t.chars().count()).sum();
    pad(line, width.saturating_sub(used) / 2)
}

/// A figure with a suffix: 142_300_000_000 as `142.3B` (oracle: `tokens.rs`).
fn compact(value: u64, units: &[&str; 4]) -> String {
    let mut scaled = value;
    let mut unit = 0;
    let mut divisor = 1u64;
    while scaled >= 1000 && unit < units.len() {
        scaled /= 1000;
        divisor *= 1000;
        unit += 1;
    }
    if unit == 0 {
        return value.to_string();
    }
    let tenths = value * 10 / divisor;
    let (whole, tenth) = (tenths / 10, tenths % 10);
    if tenth == 0 {
        format!("{whole}{}", units[unit - 1])
    } else {
        format!("{whole}.{tenth}{}", units[unit - 1])
    }
}

fn message<T>(panel: &Panel<T>, strings: &Strings) -> Option<&'static str> {
    match panel {
        Panel::Loading => Some(strings.loading),
        Panel::Unavailable => Some(strings.unavailable),
        Panel::Failed(Failure::Analytics) => Some(strings.failed_analytics),
        Panel::Failed(Failure::Credits) => Some(strings.failed_credits),
        Panel::Ready(_) => None,
    }
}

/// The four panels, one message each, a blank row between (oracle:
/// `Remote::message` and the chart's empty range).
fn panel_lines(model: &Model, strings: &Strings) -> Vec<Line> {
    let list_message = |panel: &Panel<Vec<_>>| {
        message(panel, strings).or(match panel {
            Panel::Ready(days) if days.is_empty() => Some(strings.empty_range),
            _ => None,
        })
    };
    let mut lines = Vec::new();
    let overview = message(&model.overview, strings);
    for shown in [
        overview,
        list_message(&model.history),
        list_message(&model.credits),
        list_message(&model.usage),
    ]
    .into_iter()
    .flatten()
    {
        if !lines.is_empty() {
            lines.push(Vec::new());
        }
        lines.push(text(shown, Look::Plain));
    }
    lines
}

/// The overview's body: profile, numbers, heat map, insights (oracle:
/// `summary.rs`).
fn summary_lines(model: &Model, overview: &Overview, strings: &Strings, num: &NumSyms, width: usize) -> Vec<Line> {
    const BLOCK: usize = 110;
    const COLUMN: usize = 22;
    let block = width.min(BLOCK);
    let margin = width.saturating_sub(block) / 2;
    let at = |line: Line| pad(line, margin);
    let mut lines: Vec<Line> = Vec::new();
    if let Some(account) = &model.account {
        lines.push(centered(text(account.name.clone(), Look::Bold), width));
        lines.push(centered(text(account.handle.clone(), Look::Dim), width));
        lines.push(centered(text(account.plan.clone(), Look::Dim), width));
        lines.push(Vec::new());
    }
    let figures = [
        (compact(overview.lifetime_tokens, &strings.units), strings.lifetime_tokens),
        (compact(overview.peak_tokens, &strings.units), strings.peak_tokens),
        (
            strings.duration(
                overview.longest_chat_secs / 3600,
                overview.longest_chat_secs % 3600 / 60,
                num,
            ),
            strings.longest_chat,
        ),
        (strings.days(u64::from(overview.current_streak_days), num), strings.current_streak),
        (strings.days(u64::from(overview.longest_streak_days), num), strings.longest_streak),
    ];
    let cell = |shown: &str, look: Look| -> Line {
        let used = shown.chars().count();
        let left = COLUMN.saturating_sub(used) / 2;
        text(format!("{}{shown}{}", " ".repeat(left), " ".repeat(COLUMN - left - used.min(COLUMN - left))), look)
    };
    let mut numbers = Line::new();
    let mut labels = Line::new();
    for (value, label) in &figures {
        numbers.extend(cell(value, Look::Bold));
        labels.extend(cell(label, Look::Dim));
    }
    lines.push(at(numbers));
    lines.push(at(labels));
    lines.push(Vec::new());
    lines.push(at(text(strings.activity, Look::Bold)));
    lines.push(at(text(strings.last_year, Look::Dim)));
    lines.push(Vec::new());

    let weeks = overview.activity.weeks.min(block.saturating_sub(2) / 2);
    let mut months = " ".repeat(3);
    for mark in &overview.activity.months {
        if mark.week >= weeks {
            continue;
        }
        let column = 3 + 2 * mark.week;
        let have = months.chars().count();
        if column >= have {
            months.push_str(&" ".repeat(column - have));
            months.push_str(&mark.label);
        }
    }
    lines.push(at(pad(text(months, Look::Dim), 1)));
    for (day, weekday) in strings.weekdays.iter().enumerate() {
        let mut row = text(weekday.to_string(), Look::Dim);
        for week in 0..weeks {
            if week + 1 == overview.activity.weeks && day > overview.activity.last_day {
                break;
            }
            let level = overview
                .activity
                .cells
                .iter()
                .find(|(w, d, _)| *w == week && *d == day)
                .map_or(0, |(_, _, level)| *level);
            row.push((" ".to_string(), Look::Plain));
            if level == 0 {
                row.push(("□".to_string(), Look::Dim));
            } else {
                row.push(("■".to_string(), Look::Accent));
            }
        }
        lines.push(at(pad(row, 1)));
    }
    lines.push(Vec::new());
    let mut legend = text(format!("{} □ ", strings.less), Look::Dim);
    legend.push(("■ ■ ■ ■".to_string(), Look::Accent));
    legend.push((format!(" {}", strings.more), Look::Dim));
    lines.push(at(pad(legend, 3)));
    lines.push(Vec::new());

    // Insights on the left, the most used on the right, each half the block
    // less a four-cell gap.
    let half = block.saturating_sub(4) / 2;
    let two = |left: Line, right: Line| -> Line {
        let used: usize = left.iter().map(|(t, _)| t.chars().count()).sum();
        let mut line = left;
        line.push((" ".repeat((half + 4).saturating_sub(used)), Look::Plain));
        line.extend(right);
        line
    };
    let row = |label: &str, value: &str, look: Look| -> Line {
        let used = label.chars().count() + value.chars().count();
        vec![
            (label.to_string(), look),
            (" ".repeat(half.saturating_sub(used)), Look::Plain),
            (value.to_string(), Look::Plain),
        ]
    };
    lines.push(at(two(text(strings.insights, Look::Bold), text(strings.top_uses, Look::Bold))));
    lines.push(Vec::new());
    for index in 0..overview.insights.len().max(overview.top.len()) {
        let left = overview
            .insights
            .get(index)
            .map(|i| row(&i.label, &i.value, Look::Plain))
            .unwrap_or_default();
        let right = overview
            .top
            .get(index)
            .map(|t| row(&t.name, &strings.runs(t.runs, num), Look::Plain))
            .unwrap_or_default();
        lines.push(at(two(left, right)));
    }
    lines.push(Vec::new());
    if let Some(date) = &model.as_of {
        lines.push(at(text(strings.as_of(date, num), Look::Dim)));
    }
    lines
}

fn body_lines(model: &Model, strings: &Strings, num: &NumSyms, width: usize, height: usize) -> Vec<Line> {
    match model.report {
        Report::Overview => match &model.overview {
            Panel::Ready(overview) => {
                let mut lines = summary_lines(model, overview, strings, num, width);
                if model.account.is_some() {
                    let padding = height.saturating_sub(lines.len()) / 2;
                    lines.splice(0..0, std::iter::repeat_n(Vec::new(), padding));
                }
                lines
            }
            other => message(other, strings).map(|m| vec![text(m, Look::Plain)]).unwrap_or_default(),
        },
        Report::Usage => match &model.usage {
            Panel::Ready(days) if !days.is_empty() => days
                .iter()
                .map(|day| text(format!("{}  {}", day.date, compact(day.total, &strings.units)), Look::Plain))
                .collect(),
            other => vec![text(
                message(other, strings).unwrap_or(strings.empty_range),
                Look::Plain,
            )],
        },
        _ => vec![text(
            message(&model.history, strings).unwrap_or(strings.empty_range),
            Look::Plain,
        )],
    }
}

/// The whole dashboard as `height` lines of `width` cells (oracle:
/// `render.rs`).
fn screen_lines(model: &Model, strings: &Strings, num: &NumSyms, width: usize, height: usize) -> Vec<Line> {
    let inset = usize::from(width > 2);
    let inner = width.saturating_sub(2 * inset);
    let controls = if inner >= 100 { 2 } else { 3 };
    let hints = 2;
    let body_height = height.saturating_sub(3 + controls + hints).max(1);
    let mut lines: Vec<Line> = vec![Vec::new(); height];
    let mut put = |row: usize, line: Line| {
        if row < height {
            lines[row] = pad(line, inset);
        }
    };
    let heading = match &model.account {
        Some(account) => strings.heading_with(&account.plan, num),
        None => strings.heading.to_string(),
    };
    put(0, text(heading, Look::Bold));
    let mut tabs: Line = Vec::new();
    for (index, report) in Report::ALL.iter().enumerate() {
        let label = if inner >= 80 {
            format!(" {} {} ", index + 1, strings.tabs[index])
        } else {
            format!(" {} ", strings.tabs[index])
        };
        let look = if *report == model.report { Look::Selected } else { Look::Plain };
        tabs.push((label, look));
        tabs.push((" ".to_string(), Look::Plain));
    }
    put(1, tabs);
    put(2, text("─".repeat(inner), Look::Dim));
    put(3, text(strings.control_daily, Look::Dim));
    let body = body_lines(model, strings, num, inner, body_height);
    for (offset, line) in body.into_iter().take(body_height).enumerate() {
        put(3 + controls + offset, line);
    }
    put(height.saturating_sub(2), text(strings.back, Look::Dim));
    put(height.saturating_sub(1), text(strings.hints, Look::Dim));
    lines
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Report(report) => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Report(report)));
            }
            Msg::Close => {
                Navigator::maybe_pop(cx);
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let screens = watch::<ScreenStates, _>(cx);
        let panels_only = w.panels_only;
        let body = LayoutBuilder::new(move |_, constraints| {
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let height = usize::try_from(constraints.max_height).unwrap_or(0);
            let model = &screens.usage;
            let lines = if panels_only {
                panel_lines(model, strings)
            } else {
                screen_lines(model, strings, &num, width, height)
            };
            let style = |look: Look| -> Style {
                match look {
                    Look::Plain => Style::default(),
                    Look::Dim => tokens.dim(),
                    Look::Bold => tokens.bold(),
                    Look::Accent => tokens.accent(),
                    Look::Selected => tokens.selected().attrs(Attrs::REVERSE),
                }
            };
            let mut column = Column::new().cross_axis_alignment(CrossAxisAlignment::Start);
            for line in lines {
                let spans: Vec<Span> = if line.is_empty() {
                    vec![Span::new(strings.blank)]
                } else {
                    line.into_iter().map(|(t, look)| Span::styled(t, style(look))).collect()
                };
                column = column.child(Text::rich(spans).wrap(TextWrap::None).max_lines(1));
            }
            column.into()
        });
        let mut focus = Focus::new(body).autofocus().bind(KeyPress::ESC, cx.send(Msg::Close));
        for (index, report) in Report::ALL.iter().enumerate() {
            let digit = char::from_digit(index as u32 + 1, 10).unwrap_or('1');
            focus = focus.bind(digit, cx.send(Msg::Report(*report)));
        }
        focus.into()
    }
}
