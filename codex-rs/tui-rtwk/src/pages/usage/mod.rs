//! `usage` (S20): the view half of the `/usage` dashboard. One `PageRoute`
//! over the model (`ScreenStates.usage`): the heading, the report tabs, a rule,
//! the report's controls, the report's body and the key hints (oracle:
//! `analytics/render.rs`, `chrome.rs`, `controls.rs`, `hints.rs`). The tabs are
//! the account kind's sections (`Model::visible_sections`); the Overview is
//! `overview.rs`, the other reports `report.rs`. The model, its events and
//! intents are core's `codex_tui_core::screens::usage`; what the page holds
//! itself is the widget's (R14).

use std::rc::Rc;

use rtwk::focus::{Focus, KeyPress};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{
    BorderType, Column, CrossAxisAlignment, Divider, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
    SizedBox,
};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::{read, watch};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk::l10n::NumSyms;

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

/// Widget state: the handlers of its clicks.
#[derive(Default)]
pub struct ScreenState {
    taps: Taps,
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
    /// A key whose action is not built yet: the hint's index.
    NotYet(usize),
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState::default()
    }
}

impl_widget!(stateful eq Screen);

/// The styles the dashboard draws with.
#[derive(Clone)]
pub struct Look {
    pub dim: Style,
    pub bold: Style,
    pub accent: Style,
    pub selected: Style,
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
fn control(
    spans: Vec<Span>,
    tap: &Option<Handler<PointerEvent>>,
) -> AnyWidget {
    let text: AnyWidget = Text::rich(spans).wrap(TextWrap::None).max_lines(1).into();
    match tap {
        Some(tap) => GestureDetector::new().on_tap(tap.clone()).child(text).into(),
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
            vec![key(strings.model_key), blank(), Span::styled(name.to_owned(), Style::default())],
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

/// The footer's two rows: what the page does, then the keys that go to
/// other reports (oracle: `hints.rs`, wide spelling).
fn footer(model: &Model, strings: &Strings, num: &NumSyms, look: &Look) -> AnyWidget {
    let report = if model.visible_sections().is_empty() {
        vec![("R".to_owned(), "retry".to_owned())]
    } else {
        let mut hints = vec![(
            strings.hint_report(model.visible_sections().len() as u64, num),
            "report".to_owned(),
        )];
        hints.extend(
            strings
                .hints
                .iter()
                .map(|(key, hint)| ((*key).to_owned(), (*hint).to_owned())),
        );
        hints
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(crate::widgets::page_keys::footer(
            [strings.back],
            look.dim.clone(),
            look.dim.clone(),
        ))
        .child(crate::widgets::page_keys::footer_with(
            report,
            look.dim.clone(),
            look.dim.clone(),
            " · ",
        ))
        .into()
}

/// The report below the controls.
fn body(model: &Model, strings: &Strings, num: &NumSyms, look: &Look, width: i32) -> AnyWidget {
    if model.visible_sections().is_empty() {
        let message = model.account.message().unwrap_or(strings.not_available);
        return line(message, Style::default()).into();
    }
    match model.section {
        Section::Summary => overview::body(model, strings, num, look, width),
        section => report::body(model, section, strings),
    }
}

/// The whole dashboard (oracle: `render.rs`).
fn dashboard(
    model: &Model,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    taps: &Taps,
    width: i32,
) -> AnyWidget {
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
        .child(controls(model, strings, num, look, taps, inner))
        .child(Expanded::new(1, body(model, strings, num, look, inner)))
        .child(footer(model, strings, num, look));
    Padding::new(EdgeInsets::only(inset, 0, inset, 0), column).into()
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
            .map(|section| report::body(model, section, strings)),
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
            cx.emit(
                &hs.screen,
                ScreenIntent::Usage(UsageIntent::Open { view }),
            );
        }
    }

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let hs = read::<Hs, _>(cx);
        let model = watch::<ScreenStates, _>(cx).usage.clone();
        let visible = model.visible_sections();
        let intent = match msg {
            Msg::Pick(index) => visible.get(index).map(|section| UsageIntent::Select(*section)),
            Msg::Step(delta) if !visible.is_empty() => {
                let at = visible
                    .iter()
                    .position(|section| *section == model.section)
                    .unwrap_or(0);
                let next = (at as isize + delta).rem_euclid(visible.len() as isize) as usize;
                Some(UsageIntent::Select(visible[next]))
            }
            Msg::Step(_) => None,
            Msg::Range => Some(UsageIntent::Range),
            Msg::Group => Some(UsageIntent::Group),
            Msg::Model => Some(UsageIntent::Model),
            Msg::Refresh => Some(UsageIntent::Refresh),
            Msg::NotYet(index) => {
                let action = strings::of(cx).hints[index].1;
                drop(crate::widgets::page_keys::not_yet(cx, action));
                None
            }
            Msg::Close => {
                cx.emit(&hs.screen, ScreenIntent::Usage(UsageIntent::Close));
                Navigator::maybe_pop(cx);
                None
            }
        };
        if let Some(intent) = intent {
            cx.emit(&hs.screen, ScreenIntent::Usage(intent));
        }
        Rebuild::No
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let screens = watch::<ScreenStates, _>(cx);
        let panels_only = w.panels;
        let taps = self.taps.clone();
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
                dashboard(model, strings, &num, &look, &taps, constraints.max_width)
            }
        });
        let mut focus = Focus::new(body)
            .autofocus()
            .bind(KeyPress::ESC, cx.send(Msg::Close))
            .bind(KeyPress::TAB, cx.send(Msg::Step(1)))
            .bind(KeyPress::BACKTAB, cx.send(Msg::Step(-1)))
            .bind('r', cx.send(Msg::Range))
            .bind('g', cx.send(Msg::Group))
            .bind('m', cx.send(Msg::Model))
            .bind('R', cx.send(Msg::Refresh));
        for index in 0..TABS {
            let digit = char::from_digit(index as u32 + 1, 10).unwrap_or('1');
            focus = focus.bind(digit, cx.send(Msg::Pick(index)));
        }
        // The dashboard and the help are not built yet and say so.
        for (index, key) in [(0, 'z'), (1, '?')] {
            focus = focus.bind(key, cx.send(Msg::NotYet(index)));
        }
        focus.into()
    }
}
