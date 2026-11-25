//! `usage` (S20): the view half of the `/usage` dashboard. One `PageRoute`
//! over the model (`ScreenStates.usage`): the heading, the report tabs, a rule,
//! the report's controls, the report's body and the key hints (oracle:
//! `analytics/render.rs`, `chrome.rs`, `controls.rs`, `hints.rs`). The tabs are
//! the account kind's sections (`Model::visible_sections`); the Overview is
//! `overview.rs`, the other reports `report.rs`. The model, its events and
//! intents are core's `codex_tui_core::screens::usage`; what the page holds
//! itself is the widget's (R14).

use std::rc::Rc;

use rtwk::focus::{Focus, KeyBindings, KeyPress};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::l10n::NumSyms;
use rtwk::layout::{
    BorderType, BoxConstraints, Center, Column, ConstrainedBox, CrossAxisAlignment, Divider,
    EdgeInsets, Expanded, Padding, Row, SizedBox, UNBOUNDED,
};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, select};
use rtwk::scroll::{ListController, ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::{Keymap, fixed};
use codex_tui_core::screens::usage::chats::Pos as ChatPos;
use codex_tui_core::screens::usage::plan::{PlanStep, Pos as PlanPos};
use codex_tui_core::screens::usage::{
    Control, Intent as UsageIntent, Model, ReportKey, Section, TokenActivityView, UsageChoice, View,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use crate::app::theme;
use crate::app::view::Hs;
use crate::keymap::press_for;
use crate::l10n::strings as shared;

mod cards;
mod chats;
mod help;
mod hints;
mod keys;
mod list;
pub mod overview;
mod plan;
pub mod report;
pub mod strings;

use strings::Strings;

/// The most tabs an account kind shows.
const TABS: usize = 7;

/// What the debug menu lists for this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "overview",
            description: "the Overview of a Plus account at 140 cells",
            script: include_str!("fixtures/overview.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "credits_refund",
            description: "an Enterprise account's Credits report with a refund, the focused report",
            script: include_str!("fixtures/credits_refund.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "credits_days",
            description: "the Credits report with a day of one product and a day with no row",
            script: include_str!("fixtures/credits_days.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "credits_exact",
            description: "the Credits report with 12746.441206 credits on a day",
            script: include_str!("fixtures/credits_exact.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "dashboard",
            description: "a Plus account's four reports of the week, z for the dashboard of cards",
            script: include_str!("fixtures/dashboard.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "dashboard_denied",
            description: "the same four reports, the Plugins report refused (access denied)",
            script: include_str!("fixtures/dashboard_denied.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "updated",
            description: "the dashboard's Usage report with the time the host made it (help, card, controls)",
            script: include_str!("fixtures/updated.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "plan",
            description: "a Plus account's plan history: one 5-hour and one weekly period, the Plan report in front",
            script: include_str!("fixtures/plan.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "plan_twelve_hour",
            description: "the same history on a 12-hour clock",
            script: include_str!("fixtures/plan_twelve_hour.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "plan_unavailable",
            description: "the plan history was never loaded",
            script: include_str!("fixtures/plan_unavailable.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "chats",
            description: "an Enterprise account's Chats report, the first chat with a breakdown of four groups",
            script: include_str!("fixtures/chats.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "chats_long",
            description: "a long first chat title and two chats without an estimate",
            script: include_str!("fixtures/chats_long.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "chats_usd",
            description: "chats whose estimates carry dollars",
            script: include_str!("fixtures/chats_usd.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "tasks",
            description: "a Plus account's Chats report with all three metrics",
            script: include_str!("fixtures/tasks.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "activity_daily",
            description: "the Overview's daily graph: one token on May 25 and four on May 29",
            script: include_str!("fixtures/activity_daily.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "activity_empty",
            description: "the Overview of an account with no daily buckets",
            script: include_str!("fixtures/activity_empty.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "activity_weekly",
            description: "the Overview's weekly graph: 3, 6 and 9 tokens on three Mondays",
            script: include_str!("fixtures/activity_weekly.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "activity_cumulative",
            description: "the same weeks as a running total",
            script: include_str!("fixtures/activity_cumulative.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "chats_balance",
            description: "a Plus account's Chats report with only the balance known: three chats, one a refund",
            script: include_str!("fixtures/chats_balance.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "chats_many",
            description: "thirty chats, more than the window shows",
            script: include_str!("fixtures/chats_many.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "plan_empty",
            description: "the plan history with complete coverage and no periods",
            script: include_str!("fixtures/plan_empty.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "plan_zero",
            description: "a 5-hour period that used nothing",
            script: include_str!("fixtures/plan_zero.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "plan_future",
            description: "a 5-hour period that starts after the snapshot",
            script: include_str!("fixtures/plan_future.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "plan_many",
            description: "fourteen 5-hour periods and one weekly one",
            script: include_str!("fixtures/plan_many.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(false, seeded).into()),
        },
        DebugEntry {
            name: "panel_states",
            description: "four reports: loading, unavailable, an error, an empty range",
            script: include_str!("fixtures/panel_states.jsonl"),
            open: EntryOpen::Route(|seeded: &Seeded| Screen::seeded(true, seeded).into()),
        },
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`): it opens a session of the model.
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen::opening(None).into())
}

/// The route of `/usage daily|weekly|cumulative`.
pub fn route_with(view: TokenActivityView) -> PageRoute<()> {
    PageRoute::new(move |_| Screen::opening(Some(view)).into())
}

/// Opens the dashboard on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen::opening(None).into())
}

/// The dashboard. `open` starts a session of the model when the widget is
/// created (a seeded fixture does not); `panels` shows four reports' bodies
/// stacked, without chrome (the oracle's test renders those alone).
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    open: bool,
    /// What the page opens on: the Overview's view of `/usage daily`, or what a fixture chose.
    choice: UsageChoice,
    panels: bool,
}

impl Screen {
    fn opening(view: Option<TokenActivityView>) -> Self {
        Self {
            open: true,
            choice: UsageChoice::opening(view),
            panels: false,
        }
    }

    /// The page over a model a fixture script seeded: its choice is the script's.
    fn seeded(panels: bool, seeded: &Seeded<'_>) -> Self {
        let model = &seeded.core.screens().usage;
        let choice = model
            .seeded
            .as_ref()
            .map_or_else(UsageChoice::default, |seed| {
                UsageChoice::seeded(seed, model)
            });
        Self {
            open: false,
            choice,
            panels,
        }
    }
}

/// The handlers of the clicks, made once.
#[derive(Clone, Default)]
struct Taps {
    tabs: Vec<Handler<PointerEvent>>,
    /// Two taps on a card: its report.
    open: Vec<Handler<PointerEvent>>,
    dashboard: Option<Handler<PointerEvent>>,
    range: Option<Handler<PointerEvent>>,
    group: Option<Handler<PointerEvent>>,
    model: Option<Handler<PointerEvent>>,
    plan_window: Option<Handler<PointerEvent>>,
    sort: Option<Handler<PointerEvent>>,
}

/// The reports of an account (the model's `Section`s).
const SECTIONS: usize = 8;

/// The days of the shortest range: the selected day starts at the last.
const LAST_DAY: usize = 6;

/// Widget state: the handlers of its clicks, the selected day and the open
/// detail of each report, whether the report is the focused one (R14: indexes
/// and flags, never what the model holds).
pub struct ScreenState {
    taps: Taps,
    /// What the person chose: the tab, the ranges, the groups, the model, the chats' sort. The
    /// model answers for what it is asked, it keeps none of this.
    sel: UsageChoice,
    /// The session whose reports the choice has started: a session of the model that is past it
    /// and has its account starts them.
    started: u32,
    cursors: [usize; SECTIONS],
    details: [Option<usize>; SECTIONS],
    /// The plan report's table in front, its rows and open details (core's).
    plan: PlanPos,
    /// A click on each period of the plan report, remade when the history
    /// has other counts.
    plan_taps: Option<([usize; 2], plan::RowTaps)>,
    /// The chats report's cursor and open row (core's), and a click on
    /// each row, remade when the report has another count.
    chat: ChatPos,
    chat_taps: Option<(usize, chats::RowTaps)>,
    zoomed: bool,
    /// `?` is open: the help replaces the report.
    help: bool,
    /// One scroll controller per surface: each report, then the help and
    /// the dashboard (oracle: the offset of each retained surface).
    scrolls: Vec<ScrollController>,
    /// The dashboard's list, the controller `scrolls` holds for it.
    dashboard: ListController,
}

/// The surface the controllers are indexed by: a report, the help or the
/// dashboard.
const HELP_SCROLL: usize = SECTIONS;
const DASHBOARD_SCROLL: usize = SECTIONS + 1;

impl Default for ScreenState {
    fn default() -> Self {
        let dashboard = ListController::new();
        let mut scrolls = Vec::with_capacity(SECTIONS + 2);
        for at in 0..SECTIONS + 2 {
            scrolls.push(if at == DASHBOARD_SCROLL {
                dashboard.as_scroll_controller().clone()
            } else {
                ScrollController::new()
            });
        }
        Self {
            taps: Taps::default(),
            sel: UsageChoice::default(),
            started: 0,
            cursors: [LAST_DAY; SECTIONS],
            details: [None; SECTIONS],
            plan: PlanPos::default(),
            plan_taps: None,
            chat: ChatPos::default(),
            chat_taps: None,
            zoomed: true,
            help: false,
            scrolls,
            dashboard,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The `index`-th tab (a digit, a click).
    Pick(usize),
    /// Tab and Shift+Tab: the next or previous tab, wrapping.
    Step(isize),
    Range,
    Group,
    Model,
    /// The other table of the Plan report (a click on its control).
    PlanWindow,
    /// A click on the `index`-th period of table `window`.
    PlanRow(usize, usize),
    /// A click on the `index`-th chat.
    ChatRow(usize),
    /// `a`: the zero-credit groups of the open chat.
    ZeroGroups,
    /// `s`: the next sort metric of the chats.
    SortMetric,
    Refresh,
    Close,
    /// What the list keys ask of the surface in front (`keys::ListAct`).
    List(keys::ListAct),
    /// `z`: the dashboard of cards and the focused report, by turns.
    Zoom,
    /// Two clicks on the `index`-th card: its report.
    Open(usize),
    /// `?`: the help.
    Help,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            sel: self.choice.clone(),
            ..ScreenState::default()
        }
    }
}

impl_widget!(stateful eq Screen);

/// The styles the dashboard draws with.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub dim: Style,
    /// Labels and instructions (oracle: `secondary_style`, a blend of the
    /// terminal's colours where it has them).
    pub secondary: Style,
    pub bold: Style,
    pub accent: Style,
    pub selected: Style,
    /// The plot's series: the theme's info, accent and success hues (the
    /// oracle: the terminal's cyan, magenta and green), the terminal's own after.
    pub series: [Style; 4],
}

/// An empty row.
pub fn gap() -> AnyWidget {
    SizedBox::new(None, Some(1), None).into()
}

pub fn line(shown: impl Into<String>, style: Style) -> Text {
    Text::line(shown.into()).style(style).wrap(TextWrap::None)
}

pub fn nothing() -> AnyWidget {
    SizedBox::new(Some(0), Some(0), None).into()
}

/// The name of a tab (oracle: `chrome.rs` `tab_label`).
fn tab_name(strings: &Strings, model: &Model, section: Section) -> &'static str {
    match section {
        Section::Summary => strings.tab_overview,
        Section::Usage if model.business() => strings.tab_tokens,
        Section::Usage => strings.tab_usage,
        Section::Credits => strings.tab_credits,
        Section::Activity => strings.tab_messages,
        Section::Plugins => strings.tab_plugins,
        Section::Skills => strings.tab_skills,
        Section::Chats => strings.tab_chats,
        Section::Plan => strings.tab_plan,
    }
}

/// The heading: the plan and the account (oracle: `render.rs`).
fn heading(model: &Model, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let title = match model.account.ready() {
        Some(plan) => strings.heading_with(strings.plan_name(*plan), num),
        None => strings.heading.to_string(),
    };
    let mut spans = vec![Span::styled(title, look.bold.clone())];
    if let Some(account) = &model.account_label {
        spans.push(Span::styled(
            strings.account_suffix(account, num),
            look.dim.clone(),
        ));
    }
    Text::rich(spans).wrap(TextWrap::None).max_lines(1).into()
}

/// The tab strip (oracle: `render_filled_tab_bar`, the labels numbered from
/// 80 cells).
fn tabs(
    model: &View<'_>,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    taps: &Taps,
    inner: i32,
) -> AnyWidget {
    let visible = model.visible_sections();
    let labels: Rc<[String]> = visible
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let name = tab_name(strings, model, *section);
            if inner >= 80 {
                strings.tab_numbered(index as u64 + 1, name, num)
            } else {
                name.to_owned()
            }
        })
        .collect();
    crate::widgets::tab_strip::Tabs {
        labels,
        active: visible
            .iter()
            .position(|section| *section == model.section())
            .unwrap_or(0),
        marks: (strings.tab_left, strings.tab_right),
        styles: (look.selected.clone(), look.dim.clone()),
        taps: taps.tabs.clone(),
    }
    .widget()
}

/// One control: its key, a blank and its label, a click on it does its
/// action (oracle: `controls.rs`).
fn control(spans: Vec<Span>, tap: &Option<Handler<PointerEvent>>) -> AnyWidget {
    let text: AnyWidget = Text::rich(spans).wrap(TextWrap::None).max_lines(1).into();
    match tap {
        Some(tap) => GestureDetector::new()
            .on_tap(tap.clone())
            .child(text)
            .into(),
        None => text,
    }
}

/// What the dashboard is drawn from: the model and the widget's own state
/// (R14), borrowed for one build.
struct Page<'a> {
    model: &'a View<'a>,
    strings: &'a Strings,
    num: &'a NumSyms,
    look: &'a Look,
    taps: &'a Taps,
    keymap: &'a Rc<Keymap>,
    shown: report::Shown,
    help: bool,
    /// An expanded detail is open (the help lists `a` then).
    detail_open: bool,
    scrolls: &'a [ScrollController],
    /// The selected day of each report (the cards show them).
    cursors: &'a [usize; SECTIONS],
    list: &'a ListController,
    plan: &'a PlanPos,
    plan_taps: Option<plan::RowTaps>,
    chat: &'a ChatPos,
    chat_taps: Option<chats::RowTaps>,
}

impl Page<'_> {
    /// The scroll controller of the surface in front.
    fn scroll(&self) -> &ScrollController {
        let at = if self.help {
            HELP_SCROLL
        } else if self.shown.zoomed {
            self.model.section() as usize
        } else {
            DASHBOARD_SCROLL
        };
        &self.scrolls[at]
    }

    /// The report below the controls, scrolled by the surface's controller.
    fn body(&self, width: i32) -> AnyWidget {
        let Page {
            model,
            strings,
            num,
            look,
            ..
        } = self;
        if self.help {
            let lines = help::lines(model, strings, num, self.keymap, self.detail_open);
            return SingleChildScrollView::new(help::body(lines))
                .controller(self.scroll().clone())
                .into();
        }
        if model.visible_sections().is_empty() {
            let message = model
                .account
                .note()
                .map_or(strings.not_available.to_owned(), |note| {
                    strings.note(&note).to_owned()
                });
            return line(message, Style::default()).wrap(TextWrap::Word).into();
        }
        if !self.shown.zoomed {
            return cards::dashboard(cards::Deck {
                model: model.chosen(),
                strings: **strings,
                num: **num,
                look: (*look).clone(),
                cursors: *self.cursors,
                list: self.list.clone(),
                plan: self.plan.clone(),
                chat: *self.chat,
                taps: self.taps.clone(),
            });
        }
        match model.section() {
            Section::Summary => {
                // The profile block is centred in the room (oracle: the
                // padding above it) and scrolls once it is taller.
                let content = overview::body(model, strings, num, look, width);
                let centred = model.profile.ready().is_some();
                let controller = self.scroll().clone();
                LayoutBuilder::new(move |_, constraints| {
                    let room = if centred { constraints.max_height } else { 0 };
                    SingleChildScrollView::new(ConstrainedBox::new(
                        BoxConstraints::new(0, UNBOUNDED, room, UNBOUNDED),
                        Center::new(content.clone()),
                    ))
                    .controller(controller.clone())
                    .into()
                })
                .into()
            }
            Section::Plan => plan::body(
                model,
                self.plan,
                strings,
                num,
                look,
                self.plan_taps.clone(),
                self.scroll().clone(),
            ),
            Section::Chats => chats::body(
                model,
                self.chat,
                strings,
                num,
                look,
                self.chat_taps.clone(),
                self.scroll().clone(),
            ),
            section => report::body(
                model,
                section,
                strings,
                num,
                look,
                self.shown,
                self.scroll().clone(),
            ),
        }
    }

    /// The controls' place: the controls, or the help's heading.
    fn controls(&self, inner: i32) -> AnyWidget {
        if self.help {
            return SizedBox::new(
                None,
                Some(if inner >= 100 { 2 } else { 3 }),
                Some(line(self.strings.help_heading, self.look.bold.clone()).into()),
            )
            .into();
        }
        self.report_controls(inner)
    }

    /// The report's controls: the dashboard's, the range with its dates, the
    /// grouping and the model filter; from 100 cells on one row, below on
    /// two; the last row says when the report was updated (oracle:
    /// `controls.rs`).
    fn report_controls(&self, inner: i32) -> AnyWidget {
        let Page {
            model,
            strings,
            num,
            look,
            taps,
            keymap,
            shown,
            ..
        } = self;
        let key = |text: &'static str| Span::styled(text, look.dim.clone());
        let blank = || Span::styled(strings.cell_gap, look.dim.clone());
        let mut rows: [Vec<AnyWidget>; 2] = [Vec::new(), Vec::new()];
        let report = model.control_available(Control::Range, false);
        if report {
            let range = model.range_of(model.section());
            let on = look.accent.clone().attrs(Attrs::BOLD);
            let pick = |active: bool| if active { on.clone() } else { look.dim.clone() };
            let dates = model.section_date_range(model.section());
            let interval = strings.interval(
                dates.start().ymd(),
                dates.end().ymd(),
                inner.max(0) as usize,
                num,
            );
            rows[0].push(control(
                vec![
                    key(strings.range_key),
                    blank(),
                    Span::styled(strings.range_week, pick(range == 0)),
                    Span::styled(strings.range_slash, look.dim.clone()),
                    Span::styled(strings.range_month, pick(range == 1)),
                    Span::styled(
                        format!("{}{interval}", strings.control_gap),
                        Style::default(),
                    ),
                ],
                &taps.range,
            ));
        }
        let option_row = usize::from(inner < 100 && report);
        if model.control_available(Control::Group, false) {
            let group = model.group_of(model.section());
            rows[option_row].push(control(
                vec![
                    key(strings.group_key),
                    blank(),
                    Span::styled(
                        strings.group_name(model.group_name(model.section(), group)),
                        Style::default(),
                    ),
                ],
                &taps.group,
            ));
        }
        if model.control_available(Control::Model, false) {
            let name = model
                .sel
                .token_model
                .as_deref()
                .map_or(strings.all_models, |name| model.model_name(name));
            rows[option_row].push(control(
                vec![
                    key(strings.model_key),
                    blank(),
                    Span::styled(name.to_owned(), Style::default()),
                ],
                &taps.model,
            ));
        }
        if model.control_available(Control::TaskMetric, false) {
            rows[0].push(control(
                vec![
                    key(strings.sort_key),
                    Span::styled(strings.task_sort, look.dim.clone()),
                    Span::styled(
                        strings.task_metric(model.task_metric().index()),
                        Style::default(),
                    ),
                ],
                &taps.sort,
            ));
        }
        if model.control_available(Control::PlanWindow, false) {
            let hints = [keys::LEFT, keys::RIGHT]
                .into_iter()
                .map(|id| keys::hint(keymap, id))
                .filter(|hint| !hint.is_empty())
                .collect::<Vec<_>>()
                .join(strings.plan_key_join);
            let on = look.accent.clone().attrs(Attrs::BOLD);
            let pick = |active: bool| if active { on.clone() } else { look.dim.clone() };
            let mut spans = Vec::new();
            if !hints.is_empty() {
                spans.push(Span::styled(hints, look.dim.clone()));
                spans.push(blank());
            }
            spans.extend([
                Span::styled(strings.plan_window_five, pick(self.plan.window == 0)),
                Span::styled(strings.plan_window_slash, look.dim.clone()),
                Span::styled(strings.plan_window_week, pick(self.plan.window == 1)),
            ]);
            rows[usize::from(inner < 100)].push(control(spans, &taps.plan_window));
        }
        if !shown.zoomed {
            let mut spans = vec![Span::styled(strings.dashboard, look.bold.clone())];
            let accept = keys::hint(keymap, keys::ACCEPT);
            if !accept.is_empty() {
                spans.push(Span::styled(
                    strings.dashboard_focus(&accept, num),
                    look.dim.clone(),
                ));
            }
            rows[0].insert(0, control(spans, &taps.dashboard));
        }
        let height = if inner >= 100 { 2 } else { 3 };
        let mut column = Column::new().cross_axis_alignment(CrossAxisAlignment::Start);
        // The rows above the last, the last says when the report was updated.
        for row in rows.into_iter().take(height as usize - 1) {
            let mut shown = Row::new();
            for (at, widget) in row.into_iter().enumerate() {
                if at > 0 {
                    shown = shown.child(line(strings.control_gap, look.dim.clone()));
                }
                shown = shown.child(widget);
            }
            column = column.child(SizedBox::new(None, Some(1), Some(shown.into())));
        }
        let updated = model
            .history(model.section())
            .ready()
            .and_then(|history| history.updated_at)
            .map(|at| strings.updated_controls(at, model.hour24, num));
        let column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(SizedBox::new(None, Some(height - 1), Some(column.into())))
            .child(
                Text::line(updated.unwrap_or_default())
                    .style(look.secondary.clone())
                    .wrap(TextWrap::None)
                    .max_lines(1),
            );
        SizedBox::new(None, Some(height), Some(column.into())).into()
    }

    /// The whole dashboard (oracle: `render.rs`).
    fn dashboard(&self, width: i32) -> AnyWidget {
        let Page {
            model,
            strings,
            num,
            look,
            taps,
            ..
        } = self;
        let inset = i32::from(width > 2);
        let inner = (width - 2 * inset).max(1);
        let rule = Divider::new()
            .kind(BorderType::Single)
            .style(look.dim.clone())
            .merge(false);
        let column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(heading(model, strings, num, look))
            .child(SizedBox::new(
                None,
                Some(1),
                Some(tabs(model, strings, num, look, taps, inner)),
            ))
            .child(rule)
            .child(self.controls(inner))
            .child(Expanded::new(1, self.body(inner)))
            .child(hints::footer(hints::Footer {
                model: model.chosen(),
                strings: **strings,
                num: **num,
                keymap: self.keymap.clone(),
                shown: self.shown,
                help: self.help,
                row_details: match model.section() {
                    Section::Chats => model.chat_has_details(self.chat),
                    _ => self.plan.has_row(model.plan.ready().map(|r| &**r)),
                },
                controller: self.scroll().clone(),
                key: look.dim.clone(),
                hint: look.dim.clone(),
            }));
        Padding::new(EdgeInsets::only(inset, 0, inset, 0), column).into()
    }
}

/// Four reports' bodies, one under the other with a blank row between
/// (the oracle's test of the loading, unavailable, error and empty states).
fn panels(model: &View<'_>, strings: &Strings, look: &Look) -> AnyWidget {
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(1)
        .children(
            [
                Section::Usage,
                Section::Plugins,
                Section::Credits,
                Section::Activity,
            ]
            .into_iter()
            .map(|section| report::message(model, section, strings, look)),
        )
        .into()
}

impl ScreenState {
    /// The handlers of the clicks on the Plan report's periods, as many as
    /// each table holds (made here, not in `build`: a handler made there
    /// would not compare equal and rebuild every row).
    fn retap(&mut self, cx: &mut Cx<'_, Self>) {
        let model = read::<ScreenStates, _>(cx).usage.clone();
        let counts = model.plan.ready().map_or([0, 0], |report| {
            [0, 1].map(|window| report.periods[window].len())
        });
        if self
            .plan_taps
            .as_ref()
            .is_some_and(|(was, _)| *was == counts)
        {
            return;
        }
        let taps = [0, 1].map(|window| {
            (0..counts[window])
                .map(|index| cx.map(move |_: PointerEvent| Msg::PlanRow(window, index)))
                .collect::<Vec<_>>()
        });
        self.plan_taps = Some((counts, Rc::new(taps)));
        let rows = model.chat_row_count();
        if self.chat_taps.as_ref().is_none_or(|(was, _)| *was != rows) {
            let taps = (0..rows)
                .map(|index| cx.map(move |_: PointerEvent| Msg::ChatRow(index)))
                .collect::<Vec<_>>();
            self.chat_taps = Some((rows, Rc::new(taps)));
        }
    }
}

impl ScreenState {
    /// The model changed: a session that has its account starts the reports of the tabs the
    /// choice asks for (oracle: `start_reports`), and the choice follows what the answers
    /// taught (`UsageChoice::after_settle`).
    fn model_changed(&mut self, cx: &mut Cx<'_, Self>) {
        let model = read::<ScreenStates, _>(cx).usage.clone();
        let mut loads = Vec::new();
        if model.session != self.started && model.account.ready().is_some() {
            self.started = model.session;
            self.sel.settled(&model);
            loads = self.sel.keys(&model);
        }
        self.sel.after_settle(&model);
        Self::load(cx, loads);
    }

    /// Asks the model for the reports `keys`, in order.
    fn load(cx: &mut Cx<'_, Self>, keys: Vec<ReportKey>) {
        if keys.is_empty() {
            return;
        }
        let hs = read::<Hs, _>(cx);
        for key in keys {
            cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Load(key)));
        }
    }
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn init_state(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        self.taps = Taps {
            tabs: (0..TABS)
                .map(|index| cx.map(move |_: PointerEvent| Msg::Pick(index)))
                .collect(),
            open: (0..TABS)
                .map(|index| cx.map(move |_: PointerEvent| Msg::Open(index)))
                .collect(),
            dashboard: Some(cx.map(|_: PointerEvent| Msg::Zoom)),
            range: Some(cx.map(|_: PointerEvent| Msg::Range)),
            group: Some(cx.map(|_: PointerEvent| Msg::Group)),
            model: Some(cx.map(|_: PointerEvent| Msg::Model)),
            plan_window: Some(cx.map(|_: PointerEvent| Msg::PlanWindow)),
            sort: Some(cx.map(|_: PointerEvent| Msg::SortMetric)),
        };
        self.retap(cx);
        self.started = read::<ScreenStates, _>(cx).usage.session;
        if w.open {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Open));
        }
    }

    fn did_change_dependencies(&mut self, _: &Screen, cx: &mut Cx<'_, Self>) {
        self.retap(cx);
        self.model_changed(cx);
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let model = read::<ScreenStates, _>(cx).usage.clone();
        // the choice as the message found it; what the message changes is made on `self.sel`
        let chosen = self.sel.clone();
        let view = View::new(&model, &chosen);
        let visible = model.visible_sections();
        let section = chosen.section;
        let mut rebuild = Rebuild::No;
        let mut loads: Vec<ReportKey> = Vec::new();
        let intent = match msg {
            Msg::List(act) => {
                let intent = self.list(act, &view, section, cx);
                rebuild = Rebuild::Yes;
                intent
            }
            Msg::Zoom => {
                self.zoomed = !self.zoomed;
                rebuild = Rebuild::Yes;
                None
            }
            Msg::Open(index) => {
                self.help = false;
                self.zoomed = true;
                rebuild = Rebuild::Yes;
                if let Some(section) = visible.get(index) {
                    self.sel.pick(&model, *section);
                }
                None
            }
            Msg::Help => {
                self.help = !self.help;
                rebuild = Rebuild::Yes;
                None
            }
            Msg::Pick(index) => {
                self.help = false;
                // The help closes whether or not the section changes.
                rebuild = Rebuild::Yes;
                if let Some(section) = visible.get(index) {
                    self.sel.pick(&model, *section);
                }
                None
            }
            Msg::Step(delta) if !visible.is_empty() => {
                let at = visible
                    .iter()
                    .position(|section| *section == chosen.section)
                    .unwrap_or(0);
                let next = (at as isize + delta).rem_euclid(visible.len() as isize) as usize;
                self.help = false;
                rebuild = Rebuild::Yes;
                self.sel.pick(&model, visible[next]);
                None
            }
            Msg::Step(_) => None,
            Msg::Range => {
                // The days keep their dates: the 30 days hold the 7 after
                // 23 older ones (oracle: `change_range`).
                if view.control_available(Control::Range, false) {
                    let group = model.range_group(section);
                    let longer = chosen.ranges[group as usize] == 0;
                    for other in visible.iter().filter(|other| {
                        !matches!(other, Section::Chats | Section::Plan | Section::Summary)
                            && model.range_group(**other) == group
                    }) {
                        let index = *other as usize;
                        self.cursors[index] = if longer {
                            self.cursors[index] + 23
                        } else {
                            self.cursors[index].saturating_sub(23)
                        };
                        self.details[index] = self.details[index].and_then(|day| {
                            if longer {
                                Some(day + 23)
                            } else {
                                day.checked_sub(23)
                            }
                        });
                    }
                    loads = self.sel.next_range(&model);
                    rebuild = Rebuild::Yes;
                }
                None
            }
            Msg::Group => {
                loads = self.sel.next_group(&model);
                rebuild = Rebuild::Yes;
                None
            }
            Msg::Model => {
                loads = self.sel.next_model(&model);
                rebuild = Rebuild::Yes;
                None
            }
            Msg::PlanRow(window, index) => {
                self.plan
                    .pick(window, index, model.plan.ready().map(|r| &**r));
                rebuild = Rebuild::Yes;
                None
            }
            Msg::PlanWindow => {
                self.plan
                    .step(PlanStep::Switch, model.plan.ready().map(|r| &**r));
                rebuild = Rebuild::Yes;
                None
            }
            Msg::ChatRow(index) => {
                let rows = model.chat_row_count();
                let probe = ChatPos {
                    cursor: index,
                    ..ChatPos::default()
                };
                self.chat.pick(index, rows, view.chat_has_details(&probe));
                rebuild = Rebuild::Yes;
                None
            }
            Msg::ZeroGroups => {
                // Only drawn: the details list the free groups or not.
                if view.control_available(Control::ZeroCreditGroups, self.chat.detail.is_some()) {
                    self.chat.zero_groups = !self.chat.zero_groups;
                    rebuild = Rebuild::Yes;
                }
                None
            }
            Msg::SortMetric => {
                if view.control_available(Control::TaskMetric, false) {
                    self.chat.detail = None;
                    rebuild = Rebuild::Yes;
                    self.sel.next_metric(&model);
                }
                None
            }
            Msg::Refresh => Some(UsageIntent::Refresh),
            Msg::Close => {
                cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Leave));
                Navigator::maybe_pop(cx);
                None
            }
        };
        if let Some(intent) = intent {
            cx.emit(&hs.screen, ScreenIntent::Usage(intent));
        }
        Self::load(cx, loads);
        rebuild
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let keymap = crate::keymap::keymap(cx);
        let usage = select::<ScreenStates, Model, _>(cx, |screens| screens.usage.clone());
        let sel = self.sel.clone();
        let panels_only = w.panels;
        let taps = self.taps.clone();
        let section = sel.section as usize;
        let shown = report::Shown {
            cursor: self.cursors[section],
            detail: self.details[section],
            zoomed: self.zoomed,
        };
        let (help, scrolls) = (self.help, self.scrolls.clone());
        let (cursors, list) = (self.cursors, self.dashboard.clone());
        let plan = self.plan.clone();
        let plan_taps = self.plan_taps.as_ref().map(|(_, taps)| taps.clone());
        let chat = self.chat;
        let chat_taps = self.chat_taps.as_ref().map(|(_, taps)| taps.clone());
        let look = Look {
            dim: tokens.dim(),
            secondary: tokens.dim(),
            series: [
                tokens.info(),
                tokens.accent(),
                tokens.success(),
                Style::new(),
            ],
            bold: tokens.bold(),
            accent: tokens.accent(),
            selected: tokens.selected().attrs(Attrs::REVERSE),
        };
        let shown_keymap = keymap.clone();
        let body = LayoutBuilder::new(move |_, constraints| {
            let model = &View::new(&usage, &sel);
            if panels_only {
                panels(model, strings, &look)
            } else {
                Page {
                    model,
                    strings,
                    num: &num,
                    look: &look,
                    taps: &taps,
                    keymap: &shown_keymap,
                    shown,
                    help,
                    detail_open: shown.detail.is_some(),
                    scrolls: &scrolls,
                    cursors: &cursors,
                    list: &list,
                    plan: &plan,
                    plan_taps: plan_taps.clone(),
                    chat: &chat,
                    chat_taps: chat_taps.clone(),
                }
                .dashboard(constraints.max_width)
            }
        });
        // The page's own keys yield to a key a list action has taken.
        let mut focus = Focus::new(body).autofocus();
        let mut own: Vec<(&str, KeyPress, Msg)> = [
            ("tab", fixed::USAGE_NEXT_TAB, Msg::Step(1)),
            ("shift+tab", fixed::USAGE_PREVIOUS_TAB, Msg::Step(-1)),
        ]
        .into_iter()
        .filter_map(|(name, id, msg)| Some((name, press_for(id)?, msg)))
        .collect();
        own.extend([
            ("r", 'r'.into(), Msg::Range),
            ("g", 'g'.into(), Msg::Group),
            ("m", 'm'.into(), Msg::Model),
            ("shift+r", 'R'.into(), Msg::Refresh),
            ("z", 'z'.into(), Msg::Zoom),
            ("q", 'q'.into(), Msg::Close),
            ("a", 'a'.into(), Msg::ZeroGroups),
            ("s", 's'.into(), Msg::SortMetric),
        ]);
        for (index, name) in ["1", "2", "3", "4", "5", "6", "7"].into_iter().enumerate() {
            let digit = char::from_digit(index as u32 + 1, 10).unwrap_or('1');
            own.push((name, digit.into(), Msg::Pick(index)));
        }
        if keys::help_available(&keymap) {
            own.push(("?", '?'.into(), Msg::Help));
        }
        for (name, key, msg) in own {
            if keys::free(&keymap, name) {
                focus = focus.bind(key, cx.send(msg));
            }
        }
        let list = keys::messages(&keymap)
            .into_iter()
            .map(|(name, msg)| (name, cx.send(msg)))
            .collect::<Vec<_>>();
        KeyBindings::new(list, focus).into()
    }
}
