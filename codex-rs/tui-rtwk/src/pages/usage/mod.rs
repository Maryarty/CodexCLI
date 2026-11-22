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
use rtwk::provider::{read, watch};
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::Keymap;
use codex_tui_core::screens::usage::models::plan_label;
use codex_tui_core::screens::usage::{
    Control, Intent as UsageIntent, Model, Section, TokenActivityView,
};
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use crate::app::theme;
use crate::app::view::Hs;
use crate::l10n::strings as shared;

mod help;
mod hints;
mod keys;
mod list;
pub mod overview;
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
            open: EntryOpen::Route(|_: &Seeded| Screen::seeded(false).into()),
        },
        DebugEntry {
            name: "credits_refund",
            description: "an Enterprise account's Credits report with a refund, the focused report",
            script: include_str!("fixtures/credits_refund.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen::seeded(false).into()),
        },
        DebugEntry {
            name: "panel_states",
            description: "four reports: loading, unavailable, an error, an empty range",
            script: include_str!("fixtures/panel_states.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen::seeded(true).into()),
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
    open: Option<Option<TokenActivityView>>,
    panels: bool,
}

impl Screen {
    fn opening(view: Option<TokenActivityView>) -> Self {
        Self {
            open: Some(view),
            panels: false,
        }
    }

    fn seeded(panels: bool) -> Self {
        Self { open: None, panels }
    }
}

/// The handlers of the clicks, made once.
#[derive(Clone, Default)]
struct Taps {
    tabs: Vec<Handler<PointerEvent>>,
    range: Option<Handler<PointerEvent>>,
    group: Option<Handler<PointerEvent>>,
    model: Option<Handler<PointerEvent>>,
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
    cursors: [usize; SECTIONS],
    details: [Option<usize>; SECTIONS],
    zoomed: bool,
    /// `?` is open: the help replaces the report.
    help: bool,
    /// One scroll controller per surface: each report, then the help and
    /// the dashboard (oracle: the offset of each retained surface).
    scrolls: Vec<ScrollController>,
}

/// The surface the controllers are indexed by: a report, the help or the
/// dashboard.
const HELP_SCROLL: usize = SECTIONS;
const DASHBOARD_SCROLL: usize = SECTIONS + 1;

impl Default for ScreenState {
    fn default() -> Self {
        Self {
            taps: Taps::default(),
            cursors: [LAST_DAY; SECTIONS],
            details: [None; SECTIONS],
            zoomed: true,
            help: false,
            scrolls: (0..SECTIONS + 2).map(|_| ScrollController::new()).collect(),
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
    Refresh,
    Close,
    /// What the list keys ask of the surface in front (`keys::ListAct`).
    List(keys::ListAct),
    /// `z`: the dashboard of cards and the focused report, by turns.
    Zoom,
    /// `?`: the help.
    Help,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState::default()
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
        Some(plan) => strings.heading_with(plan_label(*plan), num),
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
    model: &Model,
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
            .position(|section| *section == model.section)
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

/// The report's controls: the range with its dates, the grouping and the
/// model filter; from 100 cells on one row, below on two.
fn controls(
    model: &Model,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    taps: &Taps,
    inner: i32,
) -> AnyWidget {
    let key = |text: &'static str| Span::styled(text, look.dim.clone());
    let blank = || Span::styled(strings.cell_gap, look.dim.clone());
    let mut rows: [Vec<AnyWidget>; 2] = [Vec::new(), Vec::new()];
    let report = model.control_available(Control::Range, false);
    if report {
        let range = model.range_of(model.section);
        let on = look.accent.clone().attrs(Attrs::BOLD);
        let pick = |active: bool| if active { on.clone() } else { look.dim.clone() };
        let dates = model.section_date_range(model.section);
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
        let group = model.groups[model.section as usize];
        rows[option_row].push(control(
            vec![
                key(strings.group_key),
                blank(),
                Span::styled(model.group_label(model.section, group), Style::default()),
            ],
            &taps.group,
        ));
    }
    if model.control_available(Control::Model, false) {
        let name = model
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
    let height = if inner >= 100 { 2 } else { 3 };
    let mut column = Column::new().cross_axis_alignment(CrossAxisAlignment::Start);
    for row in rows {
        let mut shown = Row::new();
        for (at, widget) in row.into_iter().enumerate() {
            if at > 0 {
                shown = shown.child(line(strings.control_gap, look.dim.clone()));
            }
            shown = shown.child(widget);
        }
        column = column.child(SizedBox::new(None, Some(1), Some(shown.into())));
    }
    SizedBox::new(None, Some(height), Some(column.into())).into()
}

/// What the dashboard is drawn from: the model and the widget's own state
/// (R14), borrowed for one build.
struct Page<'a> {
    model: &'a Model,
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
}

impl Page<'_> {
    /// The scroll controller of the surface in front.
    fn scroll(&self) -> &ScrollController {
        let at = if self.help {
            HELP_SCROLL
        } else if self.shown.zoomed {
            self.model.section as usize
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
            let message = model.account.message().unwrap_or(strings.not_available);
            return line(message, Style::default()).wrap(TextWrap::Word).into();
        }
        match model.section {
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
        controls(
            self.model,
            self.strings,
            self.num,
            self.look,
            self.taps,
            inner,
        )
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
                model: (*model).clone(),
                strings: **strings,
                num: **num,
                keymap: self.keymap.clone(),
                shown: self.shown,
                help: self.help,
                controller: self.scroll().clone(),
                key: look.dim.clone(),
                hint: look.dim.clone(),
            }));
        Padding::new(EdgeInsets::only(inset, 0, inset, 0), column).into()
    }
}

/// Four reports' bodies, one under the other with a blank row between
/// (the oracle's test of the loading, unavailable, error and empty states).
fn panels(model: &Model, strings: &Strings) -> AnyWidget {
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
            .map(|section| report::message(model, section, strings)),
        )
        .into()
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn init_state(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        self.taps = Taps {
            tabs: (0..TABS)
                .map(|index| cx.map(move |_: PointerEvent| Msg::Pick(index)))
                .collect(),
            range: Some(cx.map(|_: PointerEvent| Msg::Range)),
            group: Some(cx.map(|_: PointerEvent| Msg::Group)),
            model: Some(cx.map(|_: PointerEvent| Msg::Model)),
        };
        if let Some(view) = w.open {
            let hs = read::<Hs, _>(cx);
            cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Open { view }));
        }
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let model = watch::<ScreenStates, _>(cx).usage.clone();
        let visible = model.visible_sections();
        let section = model.section;
        let mut rebuild = Rebuild::No;
        let intent = match msg {
            Msg::List(act) => {
                let intent = self.list(act, &model, section, cx);
                rebuild = Rebuild::Yes;
                intent
            }
            Msg::Zoom => {
                // The dashboard of cards is not built yet and says so.
                let action = strings::of(cx).hint_dashboard;
                drop(crate::widgets::page_keys::not_yet(cx, action));
                None
            }
            Msg::Help => {
                self.help = !self.help;
                rebuild = Rebuild::Yes;
                None
            }
            Msg::Pick(index) => {
                self.help = false;
                visible
                    .get(index)
                    .map(|section| UsageIntent::Select(*section))
            }
            Msg::Step(delta) if !visible.is_empty() => {
                let at = visible
                    .iter()
                    .position(|section| *section == model.section)
                    .unwrap_or(0);
                let next = (at as isize + delta).rem_euclid(visible.len() as isize) as usize;
                self.help = false;
                Some(UsageIntent::Select(visible[next]))
            }
            Msg::Step(_) => None,
            Msg::Range => {
                // The days keep their dates: the 30 days hold the 7 after
                // 23 older ones (oracle: `change_range`).
                if model.control_available(Control::Range, false) {
                    let group = model.range_group(section);
                    let longer = model.ranges[group as usize] == 0;
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
                    rebuild = Rebuild::Yes;
                }
                Some(UsageIntent::Range)
            }
            Msg::Group => Some(UsageIntent::Group),
            Msg::Model => Some(UsageIntent::Model),
            Msg::Refresh => Some(UsageIntent::Refresh),
            Msg::Close => {
                cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Close));
                Navigator::maybe_pop(cx);
                None
            }
        };
        if let Some(intent) = intent {
            cx.emit(&hs.screen, ScreenIntent::Usage(intent));
        }
        rebuild
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let keymap = crate::keymap::keymap(cx);
        let screens = watch::<ScreenStates, _>(cx);
        let panels_only = w.panels;
        let taps = self.taps.clone();
        let section = screens.usage.section as usize;
        let shown = report::Shown {
            cursor: self.cursors[section],
            detail: self.details[section],
            zoomed: self.zoomed,
        };
        let (help, scrolls) = (self.help, self.scrolls.clone());
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
            let model = &screens.usage;
            if panels_only {
                panels(model, strings)
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
                }
                .dashboard(constraints.max_width)
            }
        });
        // The page's own keys yield to a key a list action has taken.
        let mut focus = Focus::new(body).autofocus();
        let mut own: Vec<(&str, KeyPress, Msg)> = vec![
            ("tab", KeyPress::TAB, Msg::Step(1)),
            ("shift+tab", KeyPress::BACKTAB, Msg::Step(-1)),
            ("r", 'r'.into(), Msg::Range),
            ("g", 'g'.into(), Msg::Group),
            ("m", 'm'.into(), Msg::Model),
            ("shift+r", 'R'.into(), Msg::Refresh),
            ("z", 'z'.into(), Msg::Zoom),
            ("q", 'q'.into(), Msg::Close),
        ];
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
