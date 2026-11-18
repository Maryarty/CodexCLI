//! The command center's widgets, composed of rtwk primitives (`Row`,
//! `Column`, `Expanded`, `Padding`, `Divider`, `Text`, `SelectionList`,
//! `GestureDetector`): the header with its tab strip, the grouped rows, the
//! details pane, the footer. Every function takes what it draws (the core
//! `Center`, the strings, the look) and the handlers for the mouse; none
//! reads the model by itself and none counts a width: `Text` cuts and wraps,
//! the constraints give the room.
//! Oracle: `app/agent_center/{render,rows,hints}.rs`,
//! `app/agents_overview_view.rs` (`status`, `render_details`).

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::BuildCx;
use rtwk::StatelessWidget;
use rtwk::impl_widget;
use rtwk::Handler;
use rtwk::LayoutBuilder;
use rtwk::NotificationListener;
use rtwk::controls::{SelectionController, SelectionList};
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{ExcludeFocus, FocusNode};
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::l10n::NumSyms;
use rtwk::layout::{
    BorderType, BoxWidget, Center as Middle, Column, CrossAxisAlignment, Divider, EdgeInsets,
    Expanded, MainAxisSize, Padding, Row, SizedBox, VerticalDivider,
};
use rtwk::overlay::Dialog;
use rtwk::scroll::ScrollNotification;
use rtwk::style::{Attrs, Span, Style, ThemeData};
use rtwk::text::{Text, TextAlign, TextWrap};

use codex_tui_core::screens::Shared;
use codex_tui_core::screens::agents::{
    Action, Center, CenterRow, DetailLine, Failure, Group, GroupLabel, Grouping, Model,
    Notice, RowData, Sel, display_directory,
};

use super::help::{self, Ink};
use super::path;
use super::soft;
use super::strings::Strings;
use super::tabs::Tabs;
use super::usage;
use crate::theme::ThreadPalette;
/// What the screen's own state decides the view shows (the model holds none
/// of it): the status tab, the help, and the editing line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ui {
    pub filter: usize,
    pub help: bool,
    pub searching: bool,
    /// A task's name is being edited.
    pub renaming: bool,
}

/// The number of status tabs.
pub const FILTERS: usize = 5;

impl Ui {
    pub fn editing(&self) -> bool {
        self.searching || self.renaming
    }
}

/// A removal waiting for the user's answer (oracle:
/// `confirm_agents_overview_action`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirm {
    pub thread: String,
    pub action: Action,
}

/// The styles of the screen, taken from the theme.
#[derive(Clone, PartialEq)]
pub struct Look {
    pub plain: Style,
    pub dim: Style,
    pub muted: Style,
    pub bold: Style,
    pub red: Style,
    pub green: Style,
    pub cyan: Style,
    pub blue: Style,
    pub selection: Style,
    pub tab: Style,
    pub accent: Style,
    /// The theme's accents for the per-task colours; `None` with
    /// `tui.status_line_use_colors` off.
    pub threads: Option<ThreadPalette>,
}

impl Look {
    pub fn new(data: &ThemeData, thread_colors: bool) -> Self {
        Self {
            plain: Style::new(),
            dim: data.dim(),
            muted: data.muted.clone(),
            bold: Style::new().attrs(Attrs::BOLD),
            red: data.error.clone(),
            green: data.success.clone(),
            cyan: data.info.clone(),
            blue: data.primary.clone(),
            selection: data.selection.clone(),
            tab: data.selection.clone().attrs(Attrs::BOLD),
            accent: data.accent.clone(),
            threads: thread_colors.then(|| ThreadPalette::of(data)),
        }
    }

    /// `base` in the colour of the thread `id`, when tasks have one.
    fn thread(&self, base: &Style, id: &str) -> Style {
        match self.threads.as_ref().and_then(|palette| palette.color(id)) {
            Some(color) => base.clone().fg(color),
            None => base.clone(),
        }
    }
}

type Line = Vec<Span>;

fn span(text: impl Into<String>, style: &Style) -> Span {
    Span::styled(text.into(), style.clone())
}

/// One row of spans, cut off with an ellipsis where the room ends.
fn one(spans: Line) -> AnyWidget {
    Text::rich_line(spans).into()
}

fn fixed(width: i32, child: impl Into<AnyWidget>) -> AnyWidget {
    SizedBox::new(Some(width), Some(1), Some(child.into())).into()
}

fn gap(width: i32) -> AnyWidget {
    SizedBox::fixed(width, 1).into()
}

fn inset(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::symmetric(2, 0), child).into()
}

/// A row that leaves its last cell free.
fn short(child: impl Into<AnyWidget>) -> AnyWidget {
    Padding::new(EdgeInsets::only(0, 0, 1, 0), child).into()
}

fn top(child: impl Into<AnyWidget>) -> AnyWidget {
    Column::new()
        .overflow_intended(true)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(child)
        .into()
}

fn stacked(rows: Vec<AnyWidget>) -> AnyWidget {
    let mut column = Column::new()
        .overflow_intended(true)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    for row in rows {
        column = column.child(row);
    }
    column.into()
}

/// The details pane's width and the gap before it; the two panes show from
/// `TWO_PANES` cells of body (oracle: `center_layout`).
const DETAILS_WIDTH: usize = 38;
const GAP_WIDTH: i32 = 3;
const TWO_PANES: i32 = 90;
/// The hint of the filter strip shows from this width of header.
const KEYS_FROM: i32 = 80;
/// The list shows the status and age columns from this width.
const METADATA: i32 = 57;
const STATUS_COLUMN: i32 = 11;
const AGE_COLUMN: i32 = 9;
/// The width of the voice badge that ends a title.
const BADGE: i32 = 8;

/// How a row of the list is told apart: the list keeps its cursor by it.
/// Headings and gaps are never the cursor's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RowKey {
    Gap(usize),
    Group(usize),
    Task(String),
    ShowMore,
}

impl From<Sel> for RowKey {
    fn from(sel: Sel) -> Self {
        match sel {
            Sel::Task(id) => Self::Task(id),
            Sel::ShowMore => Self::ShowMore,
        }
    }
}

impl RowKey {
    /// The highlight this key stands for; none for a heading or a gap.
    pub fn into_sel(self) -> Option<Sel> {
        match self {
            Self::Task(id) => Some(Sel::Task(id)),
            Self::ShowMore => Some(Sel::ShowMore),
            Self::Gap(_) | Self::Group(_) => None,
        }
    }

    fn selectable(&self) -> bool {
        matches!(self, Self::Task(_) | Self::ShowMore)
    }
}

/// The key of entry `index` of the list.
pub fn key_of(center: &Center, index: usize) -> RowKey {
    match &center.entries[index] {
        CenterRow::Gap => RowKey::Gap(index),
        CenterRow::Group { .. } => RowKey::Group(index),
        CenterRow::Task(at) => RowKey::Task(center.rows[*at].id().to_owned()),
        CenterRow::ShowMore => RowKey::ShowMore,
    }
}

/// The task or "Show more" row next to entry `from` the way `forward` goes.
pub fn neighbour(center: &Center, from: usize, forward: bool) -> Option<Sel> {
    let keys = (0..center.entries.len()).map(|index| key_of(center, index));
    let found = if forward {
        keys.skip(from + 1).find(RowKey::selectable)
    } else {
        keys.take(from).rev().find(RowKey::selectable)
    };
    found.and_then(RowKey::into_sel)
}

/// Where the list is scrolled to: whether rows lie above and below the
/// window (the overflow arrows).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scrolled {
    pub above: bool,
    pub below: bool,
}

/// What the widget keeps for rtwk's widgets: the controllers and the
/// handlers of the lines they tell.
#[derive(Clone)]
pub struct Controls {
    pub selection: SelectionController<RowKey>,
    pub edit: TextEditingController,
    pub focus: FocusNode,
    pub edit_focus: FocusNode,
    pub scroll: Rc<ValueNotifier<Scrolled>>,
    pub open: Handler<RowKey>,
    pub search: Handler<Rc<str>>,
    pub submit: Handler<Rc<str>>,
}

/// The part of the controls the list takes.
#[derive(Clone)]
struct ListControls {
    selection: SelectionController<RowKey>,
    focus: FocusNode,
    scroll: Rc<ValueNotifier<Scrolled>>,
    open: Handler<RowKey>,
}

/// What the screen is told by a click: the handlers the widget made.
pub struct Taps {
    pub rows: Rc<Vec<Handler<PointerEvent>>>,
    pub tabs: Vec<Handler<PointerEvent>>,
}

fn tappable(handler: Option<&Handler<PointerEvent>>, child: AnyWidget) -> AnyWidget {
    match handler {
        Some(handler) => GestureDetector::new()
            .on_tap(handler.clone())
            .child(child)
            .into(),
        None => child,
    }
}

/// The status word and dot of a row (oracle: `status`).
fn status<'a>(row: &RowData, strings: &'a Strings, look: &Look) -> (&'a str, Span) {
    match row.group {
        Group::NeedsYou if row.is_error() => (strings.error, span("!", &look.red)),
        Group::NeedsYou => (strings.needs_input, span("\u{25cf}", &look.red)),
        Group::Working => (strings.working, span("\u{25cf}", &look.green)),
        Group::Ready => (strings.ready, span("\u{25cb}", &look.cyan)),
        Group::Finished => (strings.inactive, span("\u{25cb}", &look.dim)),
    }
}

fn grouping_name(strings: &Strings, grouping: Grouping) -> &str {
    match grouping {
        Grouping::Project => strings.grouping[0],
        Grouping::Directory => strings.grouping[1],
        Grouping::Status => strings.grouping[2],
        Grouping::Model => strings.grouping[3],
    }
}


/// The age column (oracle: `format_relative_time`).
fn age(strings: &Strings, now: i64, updated: i64) -> String {
    let seconds = (now - updated).max(0);
    if seconds == 0 {
        return strings.now.to_owned();
    }
    let (value, unit) = if seconds < 60 {
        (seconds, strings.ago[0])
    } else if seconds < 3600 {
        (seconds / 60, strings.ago[1])
    } else if seconds < 86_400 {
        (seconds / 3600, strings.ago[2])
    } else {
        (seconds / 86_400, strings.ago[3])
    };
    format!("{value}{unit}")
}

/// The text shown when no row is (oracle: the `indices.is_empty()` arm).
pub fn empty_text<'a>(strings: &'a Strings, model: &Model, no_rows: bool) -> &'a str {
    use codex_tui_core::screens::agents::Notice;
    if matches!(
        model.notice,
        Some(Notice::Reconnecting | Notice::ReconnectFailed)
    ) {
        strings.reconnecting_empty
    } else if model.history.is_some() {
        strings.no_match
    } else if model.paging.loading && no_rows {
        strings.loading
    } else if model.paging.failed {
        strings.load_failed_empty
    } else if no_rows {
        strings.no_tasks
    } else {
        strings.no_match
    }
}

/// The three header rows (oracle: `render`): title and grouping, the tab
/// strip with the filter hint, the rule.
pub fn header(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    model: &Model,
    ui: &Ui,
    center: &Center,
    taps: &Taps,
) -> AnyWidget {
    let title = one(vec![
        span(strings.header, &look.bold),
        span(
            strings.group(grouping_name(strings, model.grouping), strings.grouping_key, num),
            &look.dim,
        ),
    ]);
    let labels: Rc<[String]> = strings
        .filters
        .iter()
        .zip(center.tab_counts)
        .map(|(label, count)| strings.filter_count(label, count, num))
        .collect();
    let strip = Tabs {
        labels,
        active: ui.filter,
        marks: (strings.tab_left, strings.tab_right),
        styles: (look.tab.clone(), look.dim.clone()),
        taps: taps.tabs.clone(),
    }
    .widget();
    let hint = (!ui.editing()).then(|| {
        one(vec![
            span(strings.filter_keys, &look.plain),
            span(format!(" {}", strings.filter_label), &look.muted),
        ])
    });
    let tabs = LayoutBuilder::new(move |_, constraints| {
        let mut row = Row::new().child(Expanded::new(1, strip.clone()));
        if let Some(hint) = hint.clone().filter(|_| constraints.max_width >= KEYS_FROM) {
            row = row.child(gap(2)).child(hint);
        }
        row.into()
    });
    let rule = Divider::new()
        .kind(BorderType::Single)
        .style(look.dim.clone())
        .merge(false);
    inset(stacked(vec![
        title,
        SizedBox::new(None, Some(1), Some(tabs.into())).into(),
        rule.into(),
    ]))
}

/// A heading of the list (oracle: `render_center_rows`, the `Group` arm);
/// `width` is the room the row has.
fn heading(
    look: &Look,
    strings: &Strings,
    model: &Model,
    label: &GroupLabel,
    count: &str,
    width: usize,
) -> AnyWidget {
    let text = match label {
        GroupLabel::Status(group) => match group {
            Group::NeedsYou => strings.needs_input,
            Group::Working => strings.working,
            Group::Ready => strings.ready,
            Group::Finished => strings.inactive,
        }
        .to_owned(),
        GroupLabel::Pinned => strings.pinned.to_owned(),
        GroupLabel::Model(name) => name.clone(),
        GroupLabel::Path(path) => {
            path::fit_heading(&display_directory(path, model.home.as_deref()), count, width)
        }
    };
    one(vec![span(format!("{text}  {count}"), &look.dim)])
}

/// The label of the "Show more" row (oracle: `render_center_rows`).
fn show_more<'a>(strings: &'a Strings, model: &Model) -> &'a str {
    if let Some(history) = &model.history {
        if history.loading {
            strings.searching_history
        } else if history.failed {
            strings.retry
        } else {
            strings.show_more_matches
        }
    } else if model.paging.loading {
        strings.loading_more
    } else if model.paging.failed {
        strings.retry
    } else {
        strings.show_more
    }
}

/// The status and age columns (oracle: `rows.rs` `columns`).
fn metadata(cells: Row, status: (&str, &Style), age: (String, &Style)) -> Row {
    cells
        .child(gap(2))
        .child(fixed(STATUS_COLUMN, Text::line(status.0.to_owned()).style(status.1.clone())))
        .child(gap(2))
        .child(fixed(
            AGE_COLUMN,
            Text::line(age.0).style(age.1.clone()).text_align(TextAlign::Right),
        ))
}

/// One entry of the list as a one-row widget `width` cells wide.
#[allow(clippy::too_many_arguments)]
fn entry(
    look: &Look,
    strings: &Strings,
    model: &Model,
    center: &Center,
    index: usize,
    width: i32,
    selected: bool,
    tap: Option<&Handler<PointerEvent>>,
) -> AnyWidget {
    let fill = if selected { look.selection.clone() } else { look.plain.clone() };
    let marker = if selected { "\u{203a}" } else { " " };
    let boxed = |row: Row| BoxWidget::new().style(fill.clone()).child(row);
    match &center.entries[index] {
        CenterRow::Gap => gap(1),
        CenterRow::Group { label, count } => short(heading(
            look,
            strings,
            model,
            label,
            count,
            usize::try_from(width - 1).unwrap_or(0),
        )),
        CenterRow::ShowMore => {
            let label = format!("{marker}   {}", show_more(strings, model));
            tappable(tap, boxed(Row::new().child(Expanded::new(1, Text::line(label).style(fill.clone())))).into())
        }
        CenterRow::Task(at) => {
            let row = &center.rows[*at];
            let (status_text, mut dot) = status(row, strings, look);
            let idle = matches!(row.group, Group::Ready | Group::Finished);
            if row.unread || idle {
                let color = if row.unread { &look.blue } else { &look.muted };
                dot = span(
                    if row.unread { "\u{25cf}" } else { "\u{25cb}" },
                    &fill.merge(color),
                );
            } else if selected {
                dot = span(dot.text.to_string(), &fill);
            }
            let wide = width >= METADATA;
            let mut title = row.title().to_owned();
            if row.unread && !idle && !wide {
                title = format!("{status_text}: {title}");
            }
            if row.pinned && model.grouping == Grouping::Status {
                title = format!("\u{1f4cc} {title}");
            }
            let title_style = if selected {
                fill.clone()
            } else {
                look.thread(&look.plain, row.id())
            };
            let title = Text::line(title).style(title_style);
            // The row of the voice session ends its title column with a
            // badge (oracle: `render_rows`, `task.has_voice && title.width >= 8`).
            let title: AnyWidget = if row.has_voice {
                Row::new()
                    .child(Expanded::new(1, title))
                    .child(fixed(BADGE, Text::line("  voice").style(fill.clone())))
                    .into()
            } else {
                title.into()
            };
            let mut cells = Row::new()
                .child(fixed(1, Text::line(marker).style(fill.clone())))
                .child(gap(1))
                .child(fixed(1, one(vec![dot])))
                .child(gap(1))
                .child(Expanded::new(1, title));
            if wide {
                let age_style = if selected { &fill } else { &look.dim };
                let age = age(strings, model.now, row.updated_at);
                cells = metadata(cells, (status_text, &fill), (age, age_style));
            }
            tappable(tap, boxed(cells.child(gap(1))).into())
        }
    }
}

/// The list pane (oracle: `render_center_rows`): the column labels, the
/// rows in a `SelectionList` (it keeps the cursor and the scroll, and takes
/// the keys and the mouse), the overflow arrows.
pub fn list(
    look: &Look,
    strings: &'static Strings,
    agents: &Shared<Model>,
    center: &Rc<Center>,
    controls: &Controls,
    taps: &Rc<Vec<Handler<PointerEvent>>>,
) -> AnyWidget {
    RowsList {
        inputs: ListInputs {
            look: look.clone(),
            strings,
            agents: agents.clone(),
            center: center.clone(),
        },
        selection: controls.selection.clone(),
        focus: controls.focus.clone(),
        scroll: controls.scroll.clone(),
        open: controls.open.clone(),
        taps: taps.clone(),
    }
    .into()
}

/// What the list's rows are made of: the center's rows and entries, the model
/// they read and the look. The cursor is not among them (the list's own
/// controller keeps it and each row paints from it), so a cursor that moves
/// leaves the inputs equal and rtwk does not build the list again.
#[derive(Clone)]
pub struct ListInputs {
    pub look: Look,
    pub strings: &'static Strings,
    pub agents: Shared<Model>,
    pub center: Rc<Center>,
}

impl PartialEq for ListInputs {
    fn eq(&self, other: &Self) -> bool {
        self.look == other.look
            && self.strings == other.strings
            && self.agents == other.agents
            && self.center.rows == other.center.rows
            && self.center.entries == other.center.entries
    }
}

/// The list pane: its inputs, the list's controller, focus and scroll
/// notifier, and the handlers of the rows.
#[derive(Clone)]
pub struct RowsList {
    inputs: ListInputs,
    selection: SelectionController<RowKey>,
    focus: FocusNode,
    scroll: Rc<ValueNotifier<Scrolled>>,
    open: Handler<RowKey>,
    taps: Rc<Vec<Handler<PointerEvent>>>,
}

impl PartialEq for RowsList {
    fn eq(&self, other: &Self) -> bool {
        self.inputs == other.inputs
            && self.selection == other.selection
            && self.focus == other.focus
            && Rc::ptr_eq(&self.scroll, &other.scroll)
            && self.open == other.open
            && self.taps == other.taps
    }
}

impl StatelessWidget for RowsList {
    fn build(&self, _cx: &mut BuildCx<'_>) -> AnyWidget {
        let ListInputs { look, strings, agents, center } = self.inputs.clone();
        let (strings, taps) = (*strings, self.taps.clone());
        let controls = ListControls {
            selection: self.selection.clone(),
            focus: self.focus.clone(),
            scroll: self.scroll.clone(),
            open: self.open.clone(),
        };
        rows_in(look, strings, agents, center, controls, taps)
    }
}

impl_widget!(stateless eq RowsList);

fn rows_in(
    look: Look,
    strings: Strings,
    agents: Shared<Model>,
    center: Rc<Center>,
    controls: ListControls,
    taps: Rc<Vec<Handler<PointerEvent>>>,
) -> AnyWidget {
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width;
        let model = &*agents;
        if center.entries.is_empty() {
            let text = empty_text(&strings, model, model.threads.is_empty());
            return top(short(Text::line(text.to_owned()).style(look.dim.clone())));
        }
        let keys: Rc<Vec<RowKey>> = Rc::new(
            (0..center.entries.len())
                .map(|at| key_of(&center, at))
                .collect(),
        );
        let builder = {
            let (look, agents, center, taps, keys, selection) = (
                look.clone(),
                agents.clone(),
                center.clone(),
                taps.clone(),
                keys.clone(),
                controls.selection.clone(),
            );
            move |index: usize| -> AnyWidget {
                let paint = {
                    let (look, agents, center, taps) =
                        (look.clone(), agents.clone(), center.clone(), taps.clone());
                    move |selected: bool| {
                        entry(&look, &strings, &agents, &center, index, width, selected, taps.get(index))
                    }
                };
                if !keys[index].selectable() {
                    return paint(false);
                }
                let (selection, key) = (selection.clone(), keys[index].clone());
                ListenableBuilder::new(selection.listenable(), move |_, _| {
                    paint(selection.cursor().as_ref() == Some(&key))
                })
                .into()
            }
        };
        let rows = SelectionList::new(
            keys.len(),
            {
                let keys = keys.clone();
                move |index| keys[index].clone()
            },
            builder,
        )
        .controller(controls.selection.clone())
        .item_enabled({
            let keys = keys.clone();
            move |index| keys[index].selectable()
        })
        .on_submitted(controls.open.clone())
        .focus_node(controls.focus.clone())
        .autofocus(true)
        .item_extent(1)
        .bar(false)
        .idle_style(look.selection.clone())
        .selected_style(Style::new())
        .disabled_style(Style::new());
        let scroll = controls.scroll.clone();
        let rows = NotificationListener::<ScrollNotification>::new(rows).on_notification(
            move |note| {
                scroll.set_value(Scrolled {
                    above: note.metrics.pixels > 0,
                    below: note.metrics.pixels < note.metrics.max_scroll_extent,
                });
                false
            },
        );
        let mut column = Column::new()
            .overflow_intended(true)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let tall = constraints.max_height >= 3;
        if tall {
            let mut labels = Row::new()
                .child(arrow(&look, &controls.scroll, false))
                .child(gap(3))
                .child(Expanded::new(1, Text::line(strings.tasks).style(look.dim.clone())));
            if width >= METADATA {
                labels = metadata(
                    labels,
                    (strings.status_header, &look.dim),
                    (strings.updated_header.to_owned(), &look.dim),
                );
            }
            column = column.child(short(labels));
        }
        column = column.child(Expanded::new(1, rows));
        if tall {
            column = column.child(Row::new().child(arrow(&look, &controls.scroll, true)));
        }
        column.into()
    })
    .into()
}

/// The overflow arrow of an edge of the list: shown while rows lie beyond it.
fn arrow(look: &Look, scroll: &Rc<ValueNotifier<Scrolled>>, down: bool) -> AnyWidget {
    let style = look.dim.clone();
    ValueListenableBuilder::new(scroll.clone(), move |_, scrolled: &Scrolled, _| {
        let show = if down { scrolled.below } else { scrolled.above };
        let glyph = match (show, down) {
            (false, _) => " ",
            (true, true) => "\u{2193}",
            (true, false) => "\u{2191}",
        };
        fixed(1, Text::line(glyph).style(style.clone()))
    })
    .into()
}

/// The rows a text of the details takes at the pane's width.
fn flow(text: &str, style: &Style) -> Vec<Line> {
    soft::rows(text, DETAILS_WIDTH)
        .into_iter()
        .map(|row| vec![span(row, style)])
        .collect()
}

/// Whether the account's plan is billed directly in dollars (oracle:
/// `PlanType::Ent26`), which hides the credits and shows the native cost.
fn bills_in_dollars(model: &Model) -> bool {
    model.plan_bills_in_dollars()
}

/// The details of the highlighted row (oracle: `render_details`): one row
/// per line, the prompt kept to two rows and the activity to the rows left.
pub fn details(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    center: &Rc<Center>,
    agents: &Shared<Model>,
) -> AnyWidget {
    let (look, strings, num, center, agents) =
        (look.clone(), *strings, *num, center.clone(), agents.clone());
    LayoutBuilder::new(move |_, constraints| {
        let Some(row) = center.selected_row() else {
            return AnyWidget::default();
        };
        let height = usize::try_from(constraints.max_height).unwrap_or(0);
        let lines = detail_lines(&look, &strings, &num, row, &agents, height);
        top(stacked(lines.into_iter().map(one).collect()))
    })
    .into()
}

fn detail_lines(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    row: &RowData,
    agents: &Shared<Model>,
    height: usize,
) -> Vec<Line> {
    let model = &**agents;
    let (status_text, dot) = status(row, strings, look);
    let pair = |label: &'static str, value: &str| {
        vec![span(label, &look.dim), span(value, &look.plain)]
    };
    let mut head: Vec<Line> = vec![
        vec![span(strings.details, &look.bold)],
        Vec::new(),
        vec![span(row.title(), &look.thread(&look.bold, row.id()))],
        vec![dot, span(" ", &look.plain), span(status_text, &look.plain)],
        Vec::new(),
        pair(strings.model, row.model()),
        pair(strings.reasoning, row.reasoning_effort.as_deref().unwrap_or(strings.unknown)),
        Vec::new(),
        vec![span(strings.project, &look.dim)],
    ];
    head.extend(flow(&row.cwd, &look.plain));
    let usage_lines = usage::lines(strings, num, &row.details.usage, bills_in_dollars(model));
    for line in &usage_lines {
        head.push(pair(line.label, &line.value));
    }
    if let Some(branch) = &row.branch {
        head.push(Vec::new());
        head.push(vec![span(strings.branch, &look.dim)]);
        head.extend(flow(branch, &look.plain));
    }
    let prompt_start = head.len();
    head.push(Vec::new());
    head.push(vec![span(strings.prompt, &look.dim)]);
    let prompt_text = codex_tui_core::screens::agents::preview_markdown(
        row.details.latest_prompt.as_deref().unwrap_or(&row.preview),
    );
    let prompt_text = if prompt_text.is_empty() {
        strings.no_prompt.to_owned()
    } else {
        prompt_text
    };
    let mut prompt = flow(&prompt_text, &look.plain);
    if prompt.len() > 2 {
        prompt.truncate(2);
        prompt[1] = vec![span(strings.elision, &look.dim)];
    }
    let mut lines = head;
    lines.extend(prompt);
    let mut activity: Vec<Line> = Vec::new();
    for line in &row.details.lines {
        match line {
            DetailLine::Blank => activity.push(Vec::new()),
            DetailLine::Agent(name) => activity.push(pair(strings.agent_label, name)),
            DetailLine::NeedsAttention => {
                activity.push(vec![span(strings.needs_attention, &look.red)]);
            }
            DetailLine::Request(text) => activity.extend(flow(text, &look.plain)),
            DetailLine::OpenToReview => activity.push(vec![span(strings.open_task, &look.dim)]),
            DetailLine::WaitingApproval => {
                activity.push(vec![span(strings.wait_approval, &look.plain)]);
            }
            DetailLine::WaitingResponse => {
                activity.push(vec![span(strings.wait_response, &look.plain)]);
            }
            DetailLine::TaskError => activity.push(vec![span(strings.task_error, &look.plain)]),
            DetailLine::LatestActivity(header) => {
                activity.push(vec![span(strings.latest_activity, &look.dim)]);
                activity.extend(flow(header, &look.plain));
            }
        }
    }
    if let Some(message) = &row.details.last_message {
        activity.push(Vec::new());
        activity.push(vec![span(strings.last_message, &look.dim)]);
        activity.extend(flow(message, &look.plain));
    }
    let offline = matches!(model.notice, Some(Notice::Reconnecting | Notice::ReconnectFailed));
    if !offline && !activity.is_empty() {
        if !usage_lines.is_empty() && activity.len() > height.saturating_sub(lines.len()) {
            lines.truncate(prompt_start);
        }
        let available = height.saturating_sub(lines.len());
        if activity.len() > available {
            activity.truncate(available);
            if let Some(last) = activity.last_mut() {
                *last = vec![span(strings.elision, &look.dim)];
            }
        }
        let tail = lines.split_off(prompt_start);
        lines.extend(activity);
        lines.extend(tail);
    }
    lines.truncate(height);
    lines
}

/// The footer line: the hints, or the notice that replaces them.
pub fn footer(look: &Look, strings: &Strings, model: &Model, ui: &Ui, can_return: bool) -> Line {
        let notice = match &model.notice {
        Some(Notice::Reconnecting) => Some(strings.reconnecting.to_owned()),
        Some(Notice::ReconnectFailed) => Some(strings.reconnect_failed.to_owned()),
        Some(Notice::Removing(Action::Archive)) => Some(strings.archiving.to_owned()),
        Some(Notice::Removing(Action::Delete)) => Some(strings.deleting.to_owned()),
        Some(Notice::CreatingWorktree) => Some(strings.creating_worktree.to_owned()),
        Some(Notice::LoadFailed) => Some(strings.load_failed.to_owned()),
        Some(Notice::ServerVersion(text) | Notice::Error(text)) => Some(text.clone()),
        None => {
            let failed =
                model.paging.failed || model.history.as_ref().is_some_and(|history| history.failed);
            (failed && !model.paging.loading).then(|| strings.load_failed.to_owned())
        }
    };
    if let Some(text) = notice.filter(|_| !ui.help) {
        return vec![span(text, &look.dim)];
    }
    let editing = ui.editing();
    let mut items: Vec<(&str, &str)> = Vec::new();
    if !ui.help && !editing {
        items.push((strings.key_help, strings.help));
    }
    if ui.help || editing || can_return {
        items.push((strings.key_back, strings.back));
    }
    if ui.help {
        return hint_line(look, &items);
    }
    if !editing {
        items.push((strings.key_move, strings.move_));
    }
    items.push((
        strings.key_open,
        if ui.renaming {
            strings.rename
        } else {
            strings.open
        },
    ));
    if !editing {
        items.push((strings.key_new, strings.new));
    }
    hint_line(look, &items)
}

fn hint_line(look: &Look, items: &[(&str, &str)]) -> Line {
    let mut line = Line::new();
    for (key, label) in items {
        if !line.is_empty() {
            line.push(span("  ", &look.dim));
        }
        line.push(span(*key, &look.plain));
        line.push(span(format!(" {label}"), &look.muted));
    }
    line
}

/// The search or rename row above the list: the label and a `TextField`
/// that edits the controller's text (a search tells the model each change, a
/// rename its text on Enter).
pub fn editor(look: &Look, strings: &Strings, ui: &Ui, controls: &Controls) -> AnyWidget {
    let label = if ui.renaming { strings.rename_label } else { strings.search_label };
    let label_style = look.cyan.clone().attrs(Attrs::BOLD);
    let mut field = TextField::new()
        .controller(controls.edit.clone())
        .style(look.tab.clone())
        .focused_style(look.tab.clone())
        .key_hints(false)
        .focus_node(controls.edit_focus.clone())
        .autofocus(true)
        .on_submitted(controls.submit.clone());
    if !ui.renaming {
        field = field.on_changed(controls.search.clone());
    }
    let row = Row::new()
        .child(Text::line(label).style(label_style))
        .child(Expanded::new(1, field));
    BoxWidget::new()
        .style(look.tab.clone())
        .child(SizedBox::new(None, Some(1), Some(row.into())))
        .into()
}

/// Everything below the header and above the footer, for the unfiltered
/// frame: the list, the gap and the details.
#[allow(clippy::too_many_arguments)]
pub fn body(
    look: &Look,
    strings: &'static Strings,
    num: &NumSyms,
    agents: &Shared<Model>,
    ui: &Ui,
    center: &Rc<Center>,
    controls: &Controls,
    taps: &Taps,
) -> AnyWidget {
    let editing = ui.editing();
    let mut pane = Column::new()
        .overflow_intended(true)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    if editing {
        pane = pane.child(editor(look, strings, ui, controls));
    }
    // The list gives the focus to the editing line while one is open.
    let rows = list(look, strings, agents, center, controls, &taps.rows);
    let pane: AnyWidget = pane
        .child(Expanded::new(1, ExcludeFocus::new(rows).excluding(editing)))
        .into();
    let details = details(look, strings, num, center, agents);
    let rule = VerticalDivider::new()
        .kind(BorderType::Single)
        .style(look.dim.clone())
        .merge(false);
    let rule = SizedBox::new(
        Some(GAP_WIDTH),
        None,
        Some(Padding::new(EdgeInsets::only(1, 0, 1, 0), rule).into()),
    );
    inset(LayoutBuilder::new(move |_, constraints| {
        if constraints.max_width < TWO_PANES {
            return pane.clone();
        }
        Row::new()
            .overflow_intended(true)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Expanded::new(1, pane.clone()))
            .child(rule.clone())
            .child(SizedBox::new(Some(DETAILS_WIDTH as i32), None, Some(details.clone())))
            .into()
    }))
}

/// The footer row, inset two cells.
pub fn footer_row(
    look: &Look,
    strings: &Strings,
    model: &Model,
    ui: &Ui,
    can_return: bool,
) -> AnyWidget {
    inset(one(footer(look, strings, model, ui, can_return)))
}

/// The shortcut help in place of the list and the details (oracle:
/// `center_help_lines` and the `help` arm of `render`): three groups flowed
/// into as many columns as the width holds, cut to the room with a note.
pub fn help(look: &Look, strings: &Strings, model: &Model, center: &Center) -> AnyWidget {
    let pair = |entries: &[(&str, &str)]| -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(key, action)| ((*key).to_owned(), (*action).to_owned()))
            .collect()
    };
    // Pinning is offered while the server has pins and a task is selected.
    let can_pin = model.pinned.is_some() && center.selected_row().is_some();
    let tasks: Vec<(&str, &str)> = strings
        .help_entries_tasks
        .iter()
        .copied()
        .filter(|(_, action)| {
            (can_pin || *action != "Pin/unpin") && (model.worktrees || *action != "New worktree")
        })
        .collect();
    let mut view = vec![(strings.filter_keys.to_owned(), strings.help_filter.to_owned())];
    view.extend(pair(&strings.help_entries_view));
    let group = |title: &str, entries| help::Group { title: title.to_owned(), entries };
    let groups = [
        group(strings.help_navigate, pair(&strings.help_entries_navigate)),
        group(strings.help_tasks, pair(&tasks)),
        group(strings.help_view, view),
    ];
    let ink = Ink {
        title: look.bold.clone(),
        key: look.accent.clone(),
        plain: look.plain.clone(),
        dim: look.dim.clone(),
    };
    let (title, resize) = (strings.help_title, strings.help_resize);
    inset(LayoutBuilder::new(move |_, constraints| {
        let room = |cells: i32| usize::try_from(cells).unwrap_or(0);
        let size = (room(constraints.max_width), room(constraints.max_height));
        help::help(title, &groups, size, resize, &ink)
    }))
}

/// The name a confirmation asks about: the task's name or its preview, the
/// first non-empty line (oracle: `confirm_agents_overview_action`).
pub fn confirm_name(strings: &Strings, model: &Model, confirm: &Confirm) -> String {
    let row = model.row(&confirm.thread);
    let name = row
        .as_ref()
        .map(|row| row.name.as_deref().unwrap_or(&row.preview))
        .unwrap_or_default();
    name.trim()
        .lines()
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(strings.untitled_task)
        .to_owned()
}

/// A line of a dialog, two cells in from its sides, wrapped by word.
fn note(text: &str, style: &Style) -> AnyWidget {
    inset(Text::new(text.to_owned()).style(style.clone()).wrap(TextWrap::Word))
}

/// A dialog's panel: its rows over a background that clears what lies below
/// (oracle: the centered view clears its area).
fn panel(look: &Look, rows: Vec<AnyWidget>) -> AnyWidget {
    BoxWidget::new().style(look.plain.clone()).child(stacked(rows)).into()
}

/// What the confirmation asks: the removal, the task's name and whether the
/// archive is the cloud's. They are the question's own text, taken when it
/// is asked: the dialog is a route of the `Navigator`, above the providers
/// of a seeded debug screen, so it does not read the model by itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    pub confirm: Confirm,
    pub name: String,
    pub cloud: bool,
}

/// The archive and delete confirmation: a centered panel over the screen
/// (oracle: `CenteredView` with `SelectionViewParams::confirmation()`): the
/// title and the description wrapped, the two rows, a hint.
pub fn confirm_dialog(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    ask: &Ask,
    selection: &SelectionController<usize>,
    pick: Handler<usize>,
) -> AnyWidget {
    let name = &ask.name;
    let (title, description, label) = match ask.confirm.action {
        Action::Archive if ask.cloud => (
            strings.archive_title(name, num),
            strings.archive_description_cloud,
            strings.archive_label_cloud,
        ),
        Action::Archive => (
            strings.archive_title(name, num),
            strings.archive_description,
            strings.archive_label,
        ),
        Action::Delete => (
            strings.delete_title(name, num),
            strings.delete_description,
            strings.delete_label,
        ),
    };
    let labels = Rc::new([strings.cancel.to_owned(), label.to_owned()]);
    let (watch, accent, plain) = (
        selection.clone(),
        look.accent.clone().attrs(Attrs::BOLD),
        look.plain.clone(),
    );
    let choices = SelectionList::new(2, |index| index, move |index: usize| -> AnyWidget {
        let (selection, labels, accent, plain) =
            (watch.clone(), labels.clone(), accent.clone(), plain.clone());
        ListenableBuilder::new(selection.listenable(), move |_, _| {
            let selected = selection.cursor() == Some(index);
            let marker = if selected { "\u{203a} " } else { "  " };
            let style = if selected { &accent } else { &plain };
            Text::line(format!("{marker}{}. {}", index + 1, labels[index])).style(style.clone()).into()
        })
        .into()
    })
    .controller(selection.clone())
    .on_submitted(pick)
    .autofocus(true)
    .item_extent(1)
    .bar(false)
    .idle_style(Style::new())
    .selected_style(Style::new())
    .disabled_style(Style::new());
    let blank = || gap(1);
    let rows = vec![
        blank(),
        note(&title, &look.bold),
        note(description, &look.dim),
        blank(),
        blank(),
        SizedBox::new(None, Some(2), Some(choices.into())).into(),
        blank(),
        note(strings.select_hint, &look.dim),
    ];
    // The panel is 72 cells wide at most and two cells in from the screen.
    inset(Middle::new(Dialog::bare(panel(look, rows)).max_width(72)))
}

/// The dialog of a refused removal (oracle: the `LifecycleHeader` selection
/// view of `run_agents_overview_action`): the verb in bold red, the server's
/// error in red, the note that work may have stopped when the request was
/// sent, and one row, `Back to agents`. It is the screen's whole content.
pub fn failure_dialog(look: &Look, strings: &Strings, failure: &Failure) -> AnyWidget {
    let title = match failure.action {
        Action::Archive => strings.failure_archive,
        Action::Delete => strings.failure_delete,
    };
    let accent = look.accent.clone().attrs(Attrs::BOLD);
    let mut rows = vec![
        note(title, &look.red.clone().attrs(Attrs::BOLD)),
        note(&failure.error, &look.red),
    ];
    if failure.attempted {
        rows.push(note(strings.failure_note, &look.dim));
    }
    rows.extend([
        gap(1),
        gap(1),
        one(vec![span(format!("\u{203a} 1. {}", strings.back_to_agents), &accent)]),
        gap(1),
        note(strings.select_hint, &look.dim),
    ]);
    Dialog::bare(panel(look, rows)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::agents::strings::EN;
    use rtwk::style::ThemeData;

    fn inputs(center: Center, agents: &Shared<Model>) -> ListInputs {
        ListInputs {
            look: Look::new(&ThemeData::NORD, false),
            strings: &EN,
            agents: agents.clone(),
            center: Rc::new(center),
        }
    }

    fn center(entries: Vec<CenterRow>, show_more_selected: bool) -> Center {
        Center {
            rows: Vec::new(),
            entries,
            tab_counts: [0; 5],
            selected: None,
            show_more_selected,
            more: false,
        }
    }

    /// S5 (vs1-review): a cursor that moves changes the center's
    /// `show_more_selected` and `selected`, not what the rows are made of,
    /// so the list compares equal and is not built again; a changed entry
    /// or model does build it.
    #[test]
    fn the_list_inputs_ignore_the_cursor() {
        let agents: Shared<Model> = Shared::default();
        let entries = || vec![CenterRow::Gap, CenterRow::ShowMore];
        let base = inputs(center(entries(), false), &agents);
        assert!(base == inputs(center(entries(), true), &agents), "cursor on `Show more`");
        assert!(base != inputs(center(vec![CenterRow::Gap], false), &agents), "other rows");
        let mut later = agents;
        later.now += 1;
        assert!(base != inputs(center(entries(), false), &later), "other model");
    }
}
