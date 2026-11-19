//! `usage` (S20): the view half of the `/usage` dashboard. One `PageRoute`
//! over the model (`ScreenStates.usage`): the heading, the report tabs, a rule,
//! the period control, the report's body and the key hints. The overview is
//! the oracle's summary: the profile block, the five numbers, the activity
//! heat map and the insights, centred in the body. Its model, events and
//! intents are core's `codex_tui_core::screens::usage`.
//! Oracle: `tui/src/analytics/` (`render.rs`, `summary.rs`, `chrome.rs`,
//! `hints.rs`, `data.rs`, `chart.rs`).

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{
    BorderType, Center, Column, CrossAxisAlignment, Divider, EdgeInsets, Expanded, MainAxisSize,
    Padding, Positioned, Row, SizedBox, Stack, StackFit,
};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, watch};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextAlign, TextWrap};
use rtwk::{AnyWidget, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk::{Cx, l10n::NumSyms};

use codex_tui_core::screens::usage::{
    Failure, Intent as UsageIntent, Model, Overview, Panel, Report,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as shared;

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
    /// Tab: the next report, the first after the last.
    NextReport,
    /// A key of the footer whose action is not built yet: the hint's index.
    NotYet(usize),
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

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

/// The styles the dashboard draws with.
#[derive(Clone)]
struct Look {
    dim: Style,
    bold: Style,
    accent: Style,
    selected: Style,
}

/// An empty row.
fn gap() -> AnyWidget {
    SizedBox::new(None, Some(1), None).into()
}

fn line(shown: impl Into<String>, style: Style) -> Text {
    Text::line(shown.into()).style(style).wrap(TextWrap::None)
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
fn panels(model: &Model, strings: &Strings) -> AnyWidget {
    let list_message = |panel: &Panel<Vec<_>>| {
        message(panel, strings).or(match panel {
            Panel::Ready(days) if days.is_empty() => Some(strings.empty_range),
            _ => None,
        })
    };
    let shown = [
        message(&model.overview, strings),
        list_message(&model.history),
        list_message(&model.credits),
        list_message(&model.usage),
    ];
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .children(shown.into_iter().flatten().map(|m| Text::new(m).into()))
        .into()
}

/// The five figures, a column each (oracle: `summary.rs`).
fn figures(overview: &Overview, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let figures = [
        (
            compact(overview.lifetime_tokens, &strings.units),
            strings.lifetime_tokens,
        ),
        (
            compact(overview.peak_tokens, &strings.units),
            strings.peak_tokens,
        ),
        (
            strings.duration(
                overview.longest_chat_secs / 3600,
                overview.longest_chat_secs % 3600 / 60,
                num,
            ),
            strings.longest_chat,
        ),
        (
            strings.days(u64::from(overview.current_streak_days), num),
            strings.current_streak,
        ),
        (
            strings.days(u64::from(overview.longest_streak_days), num),
            strings.longest_streak,
        ),
    ];
    let row = |style: &Style, pick: fn(&(String, &'static str)) -> String| -> AnyWidget {
        let cells = figures.iter().map(|figure| {
            Expanded::new(
                1,
                line(pick(figure), style.clone()).text_align(TextAlign::Center),
            )
            .into()
        });
        Row::new().children(cells).into()
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(row(&look.bold, |(value, _)| value.clone()))
        .child(row(&look.dim, |(_, label)| (*label).to_string()))
        .into()
}

/// The activity heat map: month marks over seven weekday rows of cells
/// (`□` none, `■` some), then the legend.
fn heat_map(overview: &Overview, strings: &Strings, look: &Look) -> AnyWidget {
    let activity = &overview.activity;
    let mut months = Stack::new().fit(StackFit::Expand);
    for mark in activity.months.iter().filter(|m| m.week < activity.weeks) {
        let at = i32::try_from(3 + 2 * mark.week).unwrap_or(i32::MAX);
        months = months.child(
            Positioned::new(line(mark.label.clone(), look.dim.clone()))
                .left(at)
                .top(0),
        );
    }
    let mut rows = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(SizedBox::new(None, Some(1), Some(months.into())));
    for (day, weekday) in strings.weekdays.iter().enumerate() {
        let mut spans = vec![Span::styled((*weekday).to_string(), look.dim.clone())];
        for week in 0..activity.weeks {
            if week + 1 == activity.weeks && day > activity.last_day {
                break;
            }
            let level = activity
                .cells
                .iter()
                .find(|(w, d, _)| *w == week && *d == day)
                .map_or(0, |(_, _, level)| *level);
            spans.push(if level == 0 {
                Span::styled(strings.cell_none, look.dim.clone())
            } else {
                Span::styled(strings.cell_some, look.accent.clone())
            });
        }
        rows = rows.child(Text::rich(spans).wrap(TextWrap::None).max_lines(1));
    }
    let legend = Text::rich([
        Span::styled(strings.less, look.dim.clone()),
        Span::styled(strings.legend_none, look.dim.clone()),
        Span::styled(strings.legend_scale, look.accent.clone()),
        Span::styled(strings.legend_gap, look.dim.clone()),
        Span::styled(strings.more, look.dim.clone()),
    ])
    .wrap(TextWrap::None);
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Padding::new(EdgeInsets::only(1, 0, 0, 0), rows))
        .child(gap())
        .child(Padding::new(EdgeInsets::only(3, 0, 0, 0), legend))
        .into()
}

/// A label at the left and its value at the right of the room it gets.
fn entry(label: &str, value: String) -> AnyWidget {
    Row::new()
        .child(Expanded::new(1, line(label, Style::default())))
        .child(line(value, Style::default()))
        .into()
}

/// Two columns with a four-cell gap between.
fn pair(left: AnyWidget, right: AnyWidget) -> AnyWidget {
    Row::new()
        .child(Expanded::new(1, left))
        .child(SizedBox::new(Some(4), None, None))
        .child(Expanded::new(1, right))
        .into()
}

fn nothing() -> AnyWidget {
    SizedBox::new(Some(0), Some(0), None).into()
}

/// Insights on the left, the most used on the right.
fn insights(overview: &Overview, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(pair(
            line(strings.insights, look.bold.clone()).into(),
            line(strings.top_uses, look.bold.clone()).into(),
        ))
        .child(gap());
    for index in 0..overview.insights.len().max(overview.top.len()) {
        let left = overview
            .insights
            .get(index)
            .map_or_else(nothing, |i| entry(&i.label, i.value.clone()));
        let right = overview
            .top
            .get(index)
            .map_or_else(nothing, |t| entry(&t.name, strings.runs(t.runs, num)));
        column = column.child(pair(left, right));
    }
    column.into()
}

/// The overview's body: profile, numbers, heat map, insights (oracle:
/// `summary.rs`); the block is at most 110 cells wide, centred.
fn summary(
    model: &Model,
    overview: &Overview,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    width: i32,
) -> AnyWidget {
    const BLOCK: i32 = 110;
    let mut block = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(figures(overview, strings, num, look))
        .child(gap())
        .child(line(strings.activity, look.bold.clone()))
        .child(line(strings.last_year, look.dim.clone()))
        .child(gap())
        .child(heat_map(overview, strings, look))
        .child(gap())
        .child(insights(overview, strings, num, look))
        .child(gap());
    if let Some(date) = &model.as_of {
        block = block.child(line(strings.as_of(date, num), look.dim.clone()));
    }
    let block = Center::new(SizedBox::new(
        Some(width.min(BLOCK)),
        None,
        Some(block.into()),
    ))
    .height_factor(1);
    let mut whole = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    if let Some(account) = &model.account {
        let centred =
            |shown: &str, style: &Style| line(shown, style.clone()).text_align(TextAlign::Center);
        whole = whole
            .child(centred(&account.name, &look.bold))
            .child(centred(&account.handle, &look.dim))
            .child(centred(&account.plan, &look.dim))
            .child(gap());
    }
    let whole = whole.child(block);
    if model.account.is_some() {
        Center::new(whole).into()
    } else {
        whole.into()
    }
}

fn body(model: &Model, strings: &Strings, num: &NumSyms, look: &Look, width: i32) -> AnyWidget {
    let text = |shown: &'static str| -> AnyWidget { Text::new(shown).into() };
    match model.report {
        Report::Overview => match &model.overview {
            Panel::Ready(overview) => summary(model, overview, strings, num, look, width),
            other => message(other, strings).map_or_else(nothing, text),
        },
        Report::Usage => match &model.usage {
            Panel::Ready(days) if !days.is_empty() => Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children(days.iter().map(|day| {
                    Row::new()
                        .spacing(2)
                        .child(Text::new(day.date.clone()))
                        .child(Text::new(compact(day.total, &strings.units)))
                        .into()
                }))
                .into(),
            other => text(message(other, strings).unwrap_or(strings.empty_range)),
        },
        _ => text(message(&model.history, strings).unwrap_or(strings.empty_range)),
    }
}

/// The whole dashboard (oracle: `render.rs`): heading, tabs, rule, the
/// period control, the body, the key hints.
fn dashboard(
    model: &Model,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    width: i32,
) -> AnyWidget {
    let inset = i32::from(width > 2);
    let inner = width - 2 * inset;
    let controls = if inner >= 100 { 2 } else { 3 };
    let heading = match &model.account {
        Some(account) => strings.heading_with(&account.plan, num),
        None => strings.heading.to_string(),
    };
    let tabs = Row::new()
        .spacing(1)
        .children(Report::ALL.iter().enumerate().map(|(index, report)| {
            let label = if inner >= 80 {
                format!(" {} {} ", index + 1, strings.tabs[index])
            } else {
                format!(" {} ", strings.tabs[index])
            };
            let style = if *report == model.report {
                look.selected.clone()
            } else {
                Style::default()
            };
            line(label, style).into()
        }));
    let rule = Divider::new()
        .kind(BorderType::Single)
        .style(look.dim.clone())
        .merge(false);
    let column = Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(line(heading, look.bold.clone()))
        .child(tabs)
        .child(rule)
        .child(SizedBox::new(
            None,
            Some(controls),
            Some(line(strings.control_daily, look.dim.clone()).into()),
        ))
        .child(Expanded::new(1, body(model, strings, num, look, inner)))
        .child(crate::widgets::page_keys::footer(
            [strings.back],
            look.dim.clone(),
            look.dim.clone(),
        ))
        .child(crate::widgets::page_keys::footer_with(
            strings.hints,
            look.dim.clone(),
            look.dim.clone(),
            " · ",
        ));
    Padding::new(EdgeInsets::only(inset, 0, inset, 0), column).into()
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
            Msg::NextReport => {
                let model = &watch::<ScreenStates, _>(cx).usage;
                let at = Report::ALL
                    .iter()
                    .position(|r| *r == model.report)
                    .unwrap_or(0);
                let next = Report::ALL[(at + 1) % Report::ALL.len()];
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Report(next)));
            }
            Msg::NotYet(index) => {
                let action = strings::of(cx).hints[index].1;
                drop(crate::widgets::page_keys::not_yet(cx, action));
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
        let look = Look {
            dim: tokens.dim(),
            bold: tokens.bold(),
            accent: tokens.accent(),
            selected: tokens.selected().attrs(Attrs::REVERSE),
        };
        let body = LayoutBuilder::new(move |_, constraints| {
            let model = &screens.usage;
            if panels_only {
                panels(model, strings)
            } else {
                dashboard(model, strings, &num, &look, constraints.max_width)
            }
        });
        let mut focus = Focus::new(body)
            .autofocus()
            .bind(KeyPress::ESC, cx.send(Msg::Close));
        for (index, report) in Report::ALL.iter().enumerate() {
            let digit = char::from_digit(index as u32 + 1, 10).unwrap_or('1');
            focus = focus.bind(digit, cx.send(Msg::Report(*report)));
        }
        // The footer's other keys: tab cycles the reports; the dashboard, the
        // help and the refresh are not built yet and say so.
        focus = focus.bind(KeyPress::TAB, cx.send(Msg::NextReport));
        for (index, key) in [(1, 'z'), (2, '?'), (3, 'R')] {
            focus = focus.bind(key, cx.send(Msg::NotYet(index)));
        }
        focus.into()
    }
}
