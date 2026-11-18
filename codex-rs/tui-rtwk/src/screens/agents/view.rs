//! The command center's widgets, composed of rtwk primitives (`Row`,
//! `Column`, `SizedBox`, `Text`, `ListView`, `GestureDetector`): the header
//! with its tab strip, the grouped rows, the details pane, the footer. Every
//! function takes what it draws (the core `Center`, the strings, the look)
//! and the handlers for the mouse; none reads the model by itself.
//! Oracle: `app/agent_center/{render,rows,hints}.rs`,
//! `app/agents_overview_view.rs` (`status`, `render_details`).

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::Handler;
use rtwk::NotificationListener;
use rtwk::controls::{SelectionController, SelectionList};
use rtwk::editing::{TextEditingController, TextField};
use rtwk::focus::{ExcludeFocus, FocusNode};
use rtwk::foundation::{ListenableBuilder, ValueListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::l10n::NumSyms;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, Expanded, MainAxisSize, Positioned, Row, SizedBox,
};
use rtwk::scroll::ScrollNotification;
use rtwk::style::{Span, Style, ThemeData};
use rtwk::text::{Text, TextAlign};
use rtwk_core::style::Attrs;

use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::agents::{
    Action, Center, CenterRow, Confirm, DetailLine, Group, GroupLabel, Grouping, Model, RowData,
    Sel, display_directory,
};

use super::layout;
use super::strings::Strings;
use super::usage;

/// The styles of the screen, taken from the theme.
#[derive(Clone)]
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
}

impl Look {
    pub fn new(data: &ThemeData) -> Self {
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
        }
    }
}

type Line = Vec<Span>;

fn span(text: impl Into<String>, style: &Style) -> Span {
    Span::styled(text.into(), style.clone())
}

fn one(spans: Line) -> AnyWidget {
    Text::rich_line(spans).into()
}

fn blank() -> AnyWidget {
    SizedBox::fixed(1, 1).into()
}

fn cell(width: i32, child: impl Into<AnyWidget>) -> AnyWidget {
    SizedBox::new(Some(width), Some(1), Some(child.into())).into()
}

fn pad(width: usize) -> AnyWidget {
    SizedBox::fixed(width as i32, 1).into()
}

/// How a row of the list is told apart: the list keeps its cursor by it.
/// Headings and gaps are never the cursor's.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RowKey {
    Gap(usize),
    Group(usize),
    Task(String),
    ShowMore,
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
pub struct Controls {
    pub selection: SelectionController<RowKey>,
    pub edit: TextEditingController,
    pub focus: FocusNode,
    pub edit_focus: FocusNode,
    pub scroll: Rc<ValueNotifier<Scrolled>>,
    pub open: Handler<RowKey>,
    pub search: Handler<Rc<str>>,
    pub submit: Handler<Rc<str>>,
    pub confirm_selection: SelectionController<usize>,
    pub confirm_focus: FocusNode,
    pub confirm_pick: Handler<usize>,
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

/// The three header rows (oracle: `render`): title and grouping, the tab
/// strip with the filter hint, the rule.
pub fn header(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    model: &Model,
    center: &Center,
    width: usize,
    taps: &Taps,
) -> AnyWidget {
    let inner = width.saturating_sub(4);
    let title = one(vec![
        span(strings.header, &look.bold),
        span(
            strings.group(
                grouping_name(strings, model.grouping),
                strings.grouping_key,
                num,
            ),
            &look.dim,
        ),
    ]);
    let labels: Vec<String> = strings
        .filters
        .iter()
        .zip(center.tab_counts)
        .map(|(label, count)| strings.filter_count(label, count, num))
        .collect();
    let editing = model.searching || model.rename.is_some();
    let hint_text = format!("{} {}", strings.filter_keys, strings.filter_label);
    let hint_width = layout::columns(&hint_text);
    let show_hint = inner >= 80 && !editing;
    let tabs_width = if show_hint {
        inner.saturating_sub(hint_width + 2)
    } else {
        inner
    };
    let strip = layout::strip(&labels, model.filter, tabs_width);
    let mut tabs = Row::new();
    let mut used = 0;
    if strip.left {
        tabs = tabs
            .child(cell(1, one(vec![span(strings.tab_left, &look.dim)])))
            .child(pad(1));
        used += 2;
    }
    for (offset, shown) in strip.widths.iter().enumerate() {
        let index = strip.start + offset;
        let text = layout::ellipsize(&format!(" {} ", labels[index]), *shown);
        let style = if index == model.filter {
            &look.tab
        } else {
            &look.dim
        };
        let tab = cell(*shown as i32, one(vec![span(text, style)]));
        if offset > 0 {
            tabs = tabs.child(pad(1));
            used += 1;
        }
        tabs = tabs.child(tappable(taps.tabs.get(index), tab));
        used += shown;
    }
    if strip.right {
        let gap = (tabs_width - 1).saturating_sub(used);
        tabs = tabs
            .child(pad(gap))
            .child(cell(1, one(vec![span(strings.tab_right, &look.dim)])));
        used = tabs_width;
    }
    if show_hint {
        tabs = tabs
            .child(pad(tabs_width.saturating_sub(used) + 2))
            .child(cell(
                hint_width as i32,
                one(vec![
                    span(strings.filter_keys, &look.plain),
                    span(format!(" {}", strings.filter_label), &look.muted),
                ]),
            ));
    }
    let rule = one(vec![span("\u{2500}".repeat(inner), &look.dim)]);
    let indent = |child: AnyWidget| -> AnyWidget {
        Row::new()
            .child(pad(2))
            .child(cell(inner as i32, child))
            .into()
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(indent(title))
        .child(indent(
            SizedBox::new(Some(inner as i32), Some(1), Some(tabs.into())).into(),
        ))
        .child(indent(rule))
        .into()
}

/// A heading of the list (oracle: `render_center_rows`, the `Group` arm).
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
            let shown = display_directory(path, model.home.as_deref());
            let room = width.saturating_sub(layout::columns(count) + 2).min(64);
            layout::center_truncate_path(&shown, room)
        }
    };
    one(vec![span(format!("{text}  {count}"), &look.dim)])
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

/// One entry of the list as a one-row widget `width` cells wide.
#[allow(clippy::too_many_arguments)]
fn entry(
    look: &Look,
    strings: &Strings,
    model: &Model,
    center: &Center,
    index: usize,
    width: usize,
    selected: bool,
    tap: Option<&Handler<PointerEvent>>,
) -> AnyWidget {
    let inner = width.saturating_sub(1);
    match &center.entries[index] {
        CenterRow::Gap => blank(),
        CenterRow::Group { label, count } => cell(
            inner as i32,
            heading(look, strings, model, label, count, inner),
        ),
        CenterRow::ShowMore => {
            let fill = if selected {
                look.selection.clone()
            } else {
                look.plain.clone()
            };
            let label = if let Some(history) = &model.history {
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
            };
            let marker = if selected { "\u{203a}" } else { " " };
            let row = BoxWidget::new().style(fill.clone()).child(cell(
                width as i32,
                one(vec![span(format!("{marker}   {label}"), &fill)]),
            ));
            tappable(tap, row.into())
        }
        CenterRow::Task(at) => {
            let row = &center.rows[*at];
            let fill = if selected {
                look.selection.clone()
            } else {
                look.plain.clone()
            };
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
            let metadata = inner >= 56;
            let title_width = inner - 4 - if metadata { 24 } else { 0 };
            let mut title = row.title().to_owned();
            if row.unread && !idle && !metadata {
                title = format!("{status_text}: {title}");
            }
            if row.pinned && model.grouping == Grouping::Status {
                title = format!("\u{1f4cc} {title}");
            }
            let title_style = if selected {
                fill.clone()
            } else {
                look.plain.clone()
            };
            let mut cells = Row::new()
                .child(cell(
                    1,
                    one(vec![span(if selected { "\u{203a}" } else { " " }, &fill)]),
                ))
                .child(pad(1))
                .child(cell(1, one(vec![dot])))
                .child(pad(1))
                .child(cell(
                    title_width as i32,
                    one(vec![span(title, &title_style)]),
                ));
            if metadata {
                let age_style = if selected {
                    fill.clone()
                } else {
                    look.dim.clone()
                };
                cells = cells
                    .child(pad(2))
                    .child(cell(11, one(vec![span(status_text, &fill)])))
                    .child(pad(2))
                    .child(cell(
                        9,
                        Text::rich_line(vec![span(
                            age(strings, model.now, row.thread.updated_at),
                            &age_style,
                        )])
                        .text_align(TextAlign::Right),
                    ));
            }
            cells = cells.child(pad(1));
            let boxed = BoxWidget::new()
                .style(fill)
                .child(cell(width as i32, cells));
            tappable(tap, boxed.into())
        }
    }
}

/// The list pane (oracle: `render_center_rows`): the column labels, the
/// rows in a `SelectionList` (it keeps the cursor and the scroll, and takes
/// the keys and the mouse), the overflow arrows.
#[allow(clippy::too_many_arguments)]
pub fn list(
    look: &Look,
    strings: &Strings,
    screens: &Rc<ScreenStates>,
    center: &Rc<Center>,
    width: usize,
    height: usize,
    controls: &Controls,
    taps: &Rc<Vec<Handler<PointerEvent>>>,
) -> AnyWidget {
    let model = &screens.agents;
    let inner = width.saturating_sub(1);
    if center.entries.is_empty() {
        let text = empty_text(strings, model, model.threads.is_empty());
        return Column::new()
            .child(cell(inner as i32, one(vec![span(text, &look.dim)])))
            .into();
    }
    let padding = usize::from(height >= 3);
    let viewport = height.saturating_sub(2 * padding);
    let keys: Rc<Vec<RowKey>> = Rc::new(
        (0..center.entries.len())
            .map(|at| key_of(center, at))
            .collect(),
    );
    let builder = {
        let (look, screens, center, taps, keys) = (
            look.clone(),
            screens.clone(),
            center.clone(),
            taps.clone(),
            keys.clone(),
        );
        let strings = *strings;
        let selection = controls.selection.clone();
        move |index: usize| -> AnyWidget {
            let paint = {
                let (look, screens, center, taps) =
                    (look.clone(), screens.clone(), center.clone(), taps.clone());
                move |selected: bool| {
                    entry(
                        &look,
                        &strings,
                        &screens.agents,
                        &center,
                        index,
                        width,
                        selected,
                        taps.get(index),
                    )
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
    let mut rows = SelectionList::new(
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
    rows.enabled = true;
    let scroll = controls.scroll.clone();
    let rows = NotificationListener::<ScrollNotification>::new(SizedBox::new(
        Some(width as i32),
        Some(viewport as i32),
        Some(rows.into()),
    ))
    .on_notification(move |note| {
        scroll.set_value(Scrolled {
            above: note.metrics.pixels > 0,
            below: note.metrics.pixels < note.metrics.max_scroll_extent,
        });
        false
    });
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    if padding > 0 {
        let metadata = inner >= 56;
        let title_width = inner - 4 - if metadata { 24 } else { 0 };
        let strings = *strings;
        let arrow = {
            let (look, scroll) = (look.clone(), controls.scroll.clone());
            move |down: bool| {
                let look = look.clone();
                ValueListenableBuilder::new(scroll.clone(), move |_, scrolled: &Scrolled, _| {
                    let show = if down { scrolled.below } else { scrolled.above };
                    cell(
                        1,
                        one(vec![span(
                            if show {
                                if down { "\u{2193}" } else { "\u{2191}" }
                            } else {
                                " "
                            },
                            &look.dim,
                        )]),
                    )
                })
            }
        };
        let mut labels = Row::new().child(arrow(false)).child(pad(3)).child(cell(
            title_width as i32,
            one(vec![span(strings.tasks, &look.dim)]),
        ));
        if metadata {
            labels = labels
                .child(pad(2))
                .child(cell(11, one(vec![span(strings.status_header, &look.dim)])))
                .child(pad(2))
                .child(cell(
                    9,
                    Text::rich_line(vec![span(strings.updated_header, &look.dim)])
                        .text_align(TextAlign::Right),
                ));
        }
        column = column.child(cell(inner as i32, labels));
        column = column.child(rows);
        column = column.child(Row::new().child(arrow(true)));
    } else {
        column = column.child(rows);
    }
    column.into()
}

/// A text of the details, cut to `width` with an ellipsis.
fn clip(text: &str, style: &Style, width: usize) -> Line {
    vec![span(layout::ellipsize(text, width), style)]
}

fn wrapped(text: &str, style: &Style, width: usize) -> Vec<Line> {
    text.split('\n')
        .flat_map(|part| {
            if part.trim().is_empty() {
                vec![String::new()]
            } else {
                layout::wrap(part, width)
            }
        })
        .map(|row| vec![span(row, style)])
        .collect()
}

/// Whether the account's plan is billed directly in dollars (oracle:
/// `PlanType::Ent26`), which hides the credits and shows the native cost.
/// The plan is a protocol type a view never names: core has to answer this
/// (handover: `Model::plan_bills_in_dollars`); until it does, no plan is.
fn bills_in_dollars(_model: &Model) -> bool {
    false
}

/// The details of the highlighted row (oracle: `render_details`).
pub fn details(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    row: &RowData,
    model: &Model,
    width: usize,
    height: usize,
) -> AnyWidget {
    let (status_text, dot) = status(row, strings, look);
    let mut lines: Vec<Line> = vec![
        vec![span(strings.details, &look.bold)],
        Vec::new(),
        clip(row.title(), &look.bold, width),
        vec![dot, span(" ", &look.plain), span(status_text, &look.plain)],
        Vec::new(),
        clip_pair(strings.model, row.model(), look, width),
        clip_pair(
            strings.reasoning,
            row.thread
                .reasoning_effort
                .as_ref()
                .map_or(strings.unknown, |e| e.as_str()),
            look,
            width,
        ),
        Vec::new(),
        vec![span(strings.project, &look.dim)],
    ];
    let mut head: Vec<Line> = lines.split_off(0);
    head.extend(wrapped(
        &row.thread.cwd.display().to_string(),
        &look.plain,
        width,
    ));
    let usage_lines = usage::lines(strings, num, &row.details.usage, bills_in_dollars(model));
    for line in &usage_lines {
        head.push(vec![
            span(line.label, &look.dim),
            span(line.value.clone(), &look.plain),
        ]);
    }
    if let Some(branch) = row
        .thread
        .git_info
        .as_ref()
        .and_then(|git| git.branch.as_ref())
    {
        head.push(Vec::new());
        head.push(vec![span(strings.branch, &look.dim)]);
        head.extend(wrapped(branch, &look.plain, width));
    }
    let prompt_start = head.len();
    head.push(Vec::new());
    head.push(vec![span(strings.prompt, &look.dim)]);
    let prompt_text = codex_tui_core::screens::agents::preview_markdown(
        row.details
            .latest_prompt
            .as_deref()
            .unwrap_or(&row.thread.preview),
    );
    let prompt_text = if prompt_text.is_empty() {
        strings.no_prompt.to_owned()
    } else {
        prompt_text
    };
    let mut prompt = wrapped(&prompt_text, &look.plain, width);
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
            DetailLine::Agent(name) => {
                activity.push(vec![
                    span(strings.agent_label, &look.dim),
                    span(name.clone(), &look.plain),
                ]);
            }
            DetailLine::NeedsAttention => {
                activity.push(vec![span(strings.needs_attention, &look.red)])
            }
            DetailLine::Request(text) => activity.extend(wrapped(text, &look.plain, width)),
            DetailLine::OpenToReview => activity.push(vec![span(strings.open_task, &look.dim)]),
            DetailLine::WaitingApproval => {
                activity.push(vec![span(strings.wait_approval, &look.plain)])
            }
            DetailLine::WaitingResponse => {
                activity.push(vec![span(strings.wait_response, &look.plain)])
            }
            DetailLine::TaskError => activity.push(vec![span(strings.task_error, &look.plain)]),
            DetailLine::LatestActivity(header) => {
                activity.push(vec![span(strings.latest_activity, &look.dim)]);
                activity.extend(wrapped(header, &look.plain, width));
            }
        }
    }
    if let Some(message) = &row.details.last_message {
        activity.push(Vec::new());
        activity.push(vec![span(strings.last_message, &look.dim)]);
        activity.extend(wrapped(message, &look.plain, width));
    }
    let offline = matches!(
        model.notice,
        Some(
            codex_tui_core::screens::agents::Notice::Reconnecting
                | codex_tui_core::screens::agents::Notice::ReconnectFailed
        )
    );
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
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    for line in lines {
        column = column.child(cell(width as i32, one(line)));
    }
    column.into()
}

fn clip_pair(label: &str, value: &str, look: &Look, width: usize) -> Line {
    let text = layout::ellipsize(&format!("{label}{value}"), width);
    let split = if text.starts_with(label) {
        label.len()
    } else {
        text.len()
    };
    let (head, tail) = text.split_at(split);
    vec![span(head, &look.dim), span(tail, &look.plain)]
}

/// The footer line: the hints, or the notice that replaces them.
pub fn footer(look: &Look, strings: &Strings, model: &Model, can_return: bool) -> Line {
    use codex_tui_core::screens::agents::{Action, Notice};
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
    if let Some(text) = notice.filter(|_| !model.help) {
        return vec![span(text, &look.dim)];
    }
    let editing = model.searching || model.rename.is_some();
    let mut items: Vec<(&str, &str)> = Vec::new();
    if !model.help && !editing {
        items.push((strings.key_help, strings.help));
    }
    if model.help || editing || can_return {
        items.push((strings.key_back, strings.back));
    }
    if model.help {
        return hint_line(look, &items);
    }
    if !editing {
        items.push((strings.key_move, strings.move_));
    }
    items.push((
        strings.key_open,
        if model.rename.is_some() {
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
pub fn editor(
    look: &Look,
    strings: &Strings,
    model: &Model,
    width: usize,
    controls: &Controls,
) -> AnyWidget {
    let label = if model.rename.is_some() {
        strings.rename_label
    } else {
        strings.search_label
    };
    let label_style = look.cyan.clone().attrs(Attrs::BOLD);
    let mut field = TextField::new()
        .controller(controls.edit.clone())
        .style(look.tab.clone())
        .focused_style(look.tab.clone())
        .key_hints(false)
        .focus_node(controls.edit_focus.clone())
        .autofocus(true)
        .on_submitted(controls.submit.clone());
    if model.rename.is_none() {
        field = field.on_changed(controls.search.clone());
    }
    BoxWidget::new()
        .style(look.tab.clone())
        .child(SizedBox::new(
            Some(width as i32),
            Some(1),
            Some(
                Row::new()
                    .child(cell(
                        layout::columns(label) as i32,
                        one(vec![span(label, &label_style)]),
                    ))
                    .child(Expanded::new(1, field))
                    .into(),
            ),
        ))
        .into()
}

/// Everything below the header and above the footer, for the unfiltered
/// frame: the list, the gap and the details.
#[allow(clippy::too_many_arguments)]
pub fn body(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    screens: &Rc<ScreenStates>,
    center: &Rc<Center>,
    areas: &layout::Areas,
    controls: &Controls,
    taps: &Taps,
) -> AnyWidget {
    let model = &screens.agents;
    let editing = model.searching || model.rename.is_some();
    let search_rows = usize::from(editing && areas.body_height > 0);
    let list_height = areas.body_height - search_rows;
    let mut pane = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    if search_rows > 0 {
        pane = pane.child(editor(look, strings, model, areas.list_width, controls));
    }
    // The list gives the focus to the editing line while one is open.
    let rows = list(
        look,
        strings,
        screens,
        center,
        areas.list_width,
        list_height,
        controls,
        &taps.rows,
    );
    pane = pane.child(ExcludeFocus::new(rows).excluding(editing || model.confirm.is_some()));
    let mut row = Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(pad(areas.list_x))
        .child(SizedBox::new(
            Some(areas.list_width as i32),
            Some(areas.body_height as i32),
            Some(pane.into()),
        ));
    if let Some(_gap) = areas.gap_x {
        let mut gap = Column::new().main_axis_size(MainAxisSize::Min);
        for _ in 0..areas.body_height {
            gap = gap.child(cell(
                3,
                Row::new()
                    .child(pad(1))
                    .child(cell(1, one(vec![span("\u{2502}", &look.dim)]))),
            ));
        }
        row = row.child(SizedBox::new(
            Some(3),
            Some(areas.body_height as i32),
            Some(gap.into()),
        ));
        if let Some(selected) = center.selected_row() {
            row = row.child(SizedBox::new(
                Some(areas.details_width as i32),
                Some(areas.body_height as i32),
                Some(details(
                    look,
                    strings,
                    num,
                    selected,
                    model,
                    areas.details_width,
                    areas.body_height,
                )),
            ));
        }
    }
    SizedBox::new(None, Some(areas.body_height as i32), Some(row.into())).into()
}

/// The footer row, inset two cells.
pub fn footer_row(
    look: &Look,
    strings: &Strings,
    model: &Model,
    can_return: bool,
    width: usize,
) -> AnyWidget {
    let inner = width.saturating_sub(4);
    Row::new()
        .child(pad(2))
        .child(cell(
            inner as i32,
            one(footer(look, strings, model, can_return)),
        ))
        .into()
}

/// The shortcut help in place of the list and the details (oracle:
/// `center_help_lines` and the `help` arm of `render`): three groups flowed
/// into as many columns as the width holds, cut to the room with a note.
pub fn help(
    look: &Look,
    strings: &Strings,
    model: &Model,
    center: &Center,
    areas: &layout::Areas,
    width: usize,
) -> AnyWidget {
    let inner = width.saturating_sub(4);
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
    let mut view = vec![(
        strings.filter_keys.to_owned(),
        strings.help_filter.to_owned(),
    )];
    view.extend(pair(&strings.help_entries_view));
    let groups = [
        layout::HelpGroup {
            title: strings.help_navigate.to_owned(),
            entries: pair(&strings.help_entries_navigate),
        },
        layout::HelpGroup {
            title: strings.help_tasks.to_owned(),
            entries: pair(&tasks),
        },
        layout::HelpGroup {
            title: strings.help_view.to_owned(),
            entries: view,
        },
    ];
    let paint = |ink: layout::Ink| match ink {
        layout::Ink::Title => look.bold.clone(),
        layout::Ink::Key => look.accent.clone(),
        layout::Ink::Plain => look.plain.clone(),
    };
    let mut rows: Vec<Line> = vec![vec![span(strings.help_title, &look.bold)], Vec::new()];
    rows.extend(layout::help_rows(&groups, inner).into_iter().map(|row| {
        row.into_iter()
            .map(|(ink, text)| span(text, &paint(ink)))
            .collect()
    }));
    if rows.len() > areas.body_height {
        rows.truncate(areas.body_height.saturating_sub(1));
        rows.push(vec![span(strings.help_resize, &look.dim)]);
    }
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    for row in rows {
        column = column.child(cell(inner as i32, one(row)));
    }
    SizedBox::new(
        None,
        Some(areas.body_height as i32),
        Some(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(pad(2))
                .child(column)
                .into(),
        ),
    )
    .into()
}

/// The name a confirmation asks about: the task's name or its preview, the
/// first non-empty line (oracle: `confirm_agents_overview_action`).
fn confirm_name(strings: &Strings, center: &Center, confirm: &Confirm) -> String {
    let row = center
        .rows
        .iter()
        .find(|row| row.id() == confirm.thread.as_str());
    let name = row
        .map(|row| row.thread.name.as_deref().unwrap_or(&row.thread.preview))
        .unwrap_or_default();
    name.trim()
        .lines()
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(strings.untitled_task)
        .to_owned()
}

/// A row of the confirmation as wide as the dialog: its spans, then blanks
/// that clear what the screen shows below (oracle: the centered view clears
/// its area).
fn dialog_row(spans: Line, width: usize, fill: &Style) -> AnyWidget {
    let used: usize = spans.iter().map(|span| layout::columns(&span.text)).sum();
    let mut spans = spans;
    spans.push(span(" ".repeat(width.saturating_sub(used)), fill));
    cell(width as i32, one(spans))
}

/// The archive and delete confirmation: a centered panel over the screen
/// (oracle: `CenteredView` with `SelectionViewParams::confirmation()`): the
/// title and the description wrapped, the two rows, a hint.
#[allow(clippy::too_many_arguments)]
pub fn confirm_dialog(
    look: &Look,
    strings: &Strings,
    num: &NumSyms,
    model: &Model,
    center: &Center,
    confirm: &Confirm,
    area: (usize, usize),
    controls: &Controls,
) -> AnyWidget {
    let (width, height) = area;
    let name = confirm_name(strings, center, confirm);
    let (title, description, label) = match confirm.action {
        Action::Archive if model.cloud => (
            strings.archive_title(&name, num),
            strings.archive_description_cloud,
            strings.archive_label_cloud,
        ),
        Action::Archive => (
            strings.archive_title(&name, num),
            strings.archive_description,
            strings.archive_label,
        ),
        Action::Delete => (
            strings.delete_title(&name, num),
            strings.delete_description,
            strings.delete_label,
        ),
    };
    let panel = width.saturating_sub(4).max(width.min(8)).min(72);
    let inner = panel.saturating_sub(4);
    let bold = look.bold.clone();
    let mut header: Vec<(String, Style)> = layout::wrap(&title, inner)
        .into_iter()
        .map(|line| (line, bold.clone()))
        .collect();
    header.extend(
        layout::wrap(description, inner)
            .into_iter()
            .map(|line| (line, look.dim.clone())),
    );
    let rows_at = 1 + header.len() + 2;
    let total = (rows_at + 2 + 2).min(height);
    let plain = look.plain.clone();
    let mut column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start);
    column = column.child(dialog_row(Vec::new(), panel, &plain));
    for (line, style) in header {
        column = column.child(dialog_row(
            vec![span("  ", &plain), span(line, &style)],
            panel,
            &plain,
        ));
    }
    column = column.child(dialog_row(Vec::new(), panel, &plain));
    column = column.child(dialog_row(Vec::new(), panel, &plain));
    let labels = Rc::new([strings.cancel.to_owned(), label.to_owned()]);
    let accent = look.accent.clone().attrs(Attrs::BOLD);
    let builder = {
        let selection = controls.confirm_selection.clone();
        let plain = plain.clone();
        move |index: usize| -> AnyWidget {
            let (selection, labels, plain, accent) = (
                selection.clone(),
                labels.clone(),
                plain.clone(),
                accent.clone(),
            );
            ListenableBuilder::new(selection.listenable(), move |_, _| {
                let selected = selection.cursor() == Some(index);
                let style = if selected { &accent } else { &plain };
                let marker = if selected { "\u{203a} " } else { "  " };
                dialog_row(
                    vec![span(
                        format!("{marker}{}. {}", index + 1, labels[index]),
                        style,
                    )],
                    panel,
                    &plain,
                )
            })
            .into()
        }
    };
    let list = SelectionList::new(2, |index| index, builder)
        .controller(controls.confirm_selection.clone())
        .on_submitted(controls.confirm_pick.clone())
        .focus_node(controls.confirm_focus.clone())
        .autofocus(true)
        .item_extent(1)
        .bar(false)
        .idle_style(Style::new())
        .selected_style(Style::new())
        .disabled_style(Style::new());
    column = column.child(SizedBox::new(
        Some(panel as i32),
        Some(2),
        Some(list.into()),
    ));
    column = column.child(dialog_row(Vec::new(), panel, &plain));
    column = column.child(dialog_row(
        vec![span("  ", &plain), span(strings.select_hint, &look.dim)],
        panel,
        &plain,
    ));
    let left = (width - panel) / 2;
    let top = (height - total) / 2;
    Positioned::new(SizedBox::new(
        Some(panel as i32),
        Some(total as i32),
        Some(column.into()),
    ))
    .left(left as i32)
    .top(top as i32)
    .into()
}
