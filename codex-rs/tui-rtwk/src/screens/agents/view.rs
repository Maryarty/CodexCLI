//! The command center's widgets, composed of rtwk primitives (`Row`,
//! `Column`, `SizedBox`, `Text`, `ListView`, `GestureDetector`): the header
//! with its tab strip, the grouped rows, the details pane, the footer. Every
//! function takes what it draws (the core `Center`, the strings, the look)
//! and the handlers for the mouse; none reads the model by itself.
//! Oracle: `app/agent_center/{render,rows,hints}.rs`,
//! `app/agents_overview_view.rs` (`status`, `render_details`).

use rtwk::Handler;
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::l10n::NumSyms;
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, MainAxisSize, Row, SizedBox};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::{Span, Style, ThemeData};
use rtwk::text::{Text, TextAlign};
use rtwk::AnyWidget;
use rtwk_core::style::Attrs;

use codex_tui_core::screens::agents::{
    Center, CenterRow, DetailLine, GroupLabel, Grouping, Group, Model, RowData, display_directory,
    format_tokens_compact,
};

use super::layout;
use super::strings::Strings;

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

/// What the screen is told by a click: the handlers the widget made.
pub struct Taps {
    pub rows: Vec<Handler<PointerEvent>>,
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

fn grouping_name<'a>(strings: &'a Strings, grouping: Grouping) -> &'a str {
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
            strings.group(grouping_name(strings, model.grouping), strings.grouping_key, num),
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
    let hint_width = hint_text.chars().count();
    let show_hint = inner >= 80 && !editing;
    let tabs_width = if show_hint { inner.saturating_sub(hint_width + 2) } else { inner };
    let strip = layout::strip(&labels, model.filter, tabs_width);
    let mut tabs = Row::new();
    let mut used = 0;
    if strip.left {
        tabs = tabs.child(cell(1, one(vec![span(strings.tab_left, &look.dim)]))).child(pad(1));
        used += 2;
    }
    for (offset, shown) in strip.widths.iter().enumerate() {
        let index = strip.start + offset;
        let text = layout::ellipsize(&format!(" {} ", labels[index]), *shown);
        let style = if index == model.filter { &look.tab } else { &look.dim };
        let tab = cell(*shown as i32, one(vec![span(text, style)]));
        tabs = tabs.child(tappable(taps.tabs.get(index), tab)).child(pad(1));
        used += shown + 1;
    }
    if strip.right {
        let gap = (tabs_width - 1).saturating_sub(used);
        tabs = tabs.child(pad(gap)).child(cell(1, one(vec![span(strings.tab_right, &look.dim)])));
        used = tabs_width;
    }
    if show_hint {
        tabs = tabs
            .child(pad(tabs_width.saturating_sub(used) + 2))
            .child(cell(hint_width as i32, one(vec![
                span(strings.filter_keys, &look.plain),
                span(format!(" {}", strings.filter_label), &look.muted),
            ])));
    }
    let rule = one(vec![span("\u{2500}".repeat(inner), &look.dim)]);
    let indent = |child: AnyWidget| -> AnyWidget {
        Row::new().child(pad(2)).child(cell(inner as i32, child)).into()
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(indent(title))
        .child(indent(SizedBox::new(Some(inner as i32), Some(1), Some(tabs.into())).into()))
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
            let room = width.saturating_sub(count.chars().count() + 2).min(64);
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
    if matches!(model.notice, Some(Notice::Reconnecting | Notice::ReconnectFailed)) {
        strings.reconnecting_empty
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
    tap: Option<&Handler<PointerEvent>>,
) -> AnyWidget {
    let inner = width.saturating_sub(1);
    match &center.entries[index] {
        CenterRow::Gap => blank(),
        CenterRow::Group { label, count } => {
            cell(inner as i32, heading(look, strings, model, label, count, inner))
        }
        CenterRow::ShowMore => {
            let selected = center.show_more_selected;
            let fill = if selected { look.selection.clone() } else { look.plain.clone() };
            let label = if model.paging.loading {
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
            let selected = center.selected == Some(*at);
            let fill = if selected { look.selection.clone() } else { look.plain.clone() };
            let (status_text, mut dot) = status(row, strings, look);
            let idle = matches!(row.group, Group::Ready | Group::Finished);
            if row.unread || idle {
                let color = if row.unread { &look.blue } else { &look.muted };
                dot = span(if row.unread { "\u{25cf}" } else { "\u{25cb}" }, &fill.merge(color));
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
            let title_style = if selected { fill.clone() } else { look.plain.clone() };
            let mut cells = Row::new()
                .child(cell(1, one(vec![span(if selected { "\u{203a}" } else { " " }, &fill)])))
                .child(pad(1))
                .child(cell(1, one(vec![dot])))
                .child(pad(1))
                .child(cell(title_width as i32, one(vec![span(title, &title_style)])));
            if metadata {
                let age_style = if selected { fill.clone() } else { look.dim.clone() };
                cells = cells
                    .child(pad(2))
                    .child(cell(11, one(vec![span(status_text, &fill)])))
                    .child(pad(2))
                    .child(cell(
                        9,
                        Text::rich_line(vec![span(age(strings, model.now, row.thread.updated_at), &age_style)])
                            .text_align(TextAlign::Right),
                    ));
            }
            cells = cells.child(pad(1));
            let boxed = BoxWidget::new().style(fill).child(cell(width as i32, cells));
            tappable(tap, boxed.into())
        }
    }
}

/// The list pane (oracle: `render_center_rows`): the column labels, the
/// scrolling rows, the overflow arrows.
#[allow(clippy::too_many_arguments)]
pub fn list(
    look: &Look,
    strings: &Strings,
    model: &Model,
    center: &Center,
    width: usize,
    height: usize,
    controller: &ListController,
    taps: &Taps,
) -> AnyWidget {
    let inner = width.saturating_sub(1);
    if center.entries.is_empty() {
        let text = empty_text(strings, model, model.threads.is_empty());
        return Column::new()
            .child(cell(inner as i32, one(vec![span(text, &look.dim)])))
            .into();
    }
    let padding = usize::from(height >= 3);
    let viewport = height.saturating_sub(2 * padding);
    let rows: Vec<AnyWidget> = (0..center.entries.len())
        .map(|index| entry(look, strings, model, center, index, width, taps.rows.get(index)))
        .collect();
    let list = SizedBox::new(
        Some(width as i32),
        Some(viewport as i32),
        Some(ListView::new(rows).item_extent(1).controller(controller.clone()).into()),
    );
    let mut column = Column::new().main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start);
    if padding > 0 {
        let metadata = inner >= 56;
        let title_width = inner - 4 - if metadata { 24 } else { 0 };
        let mut labels = Row::new()
            .child(pad(4))
            .child(cell(title_width as i32, one(vec![span(strings.tasks, &look.dim)])));
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
    }
    column = column.child(list);
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
            row.thread.reasoning_effort.as_ref().map_or(strings.unknown, |e| e.as_str()),
            look,
            width,
        ),
        Vec::new(),
        vec![span(strings.project, &look.dim)],
    ];
    let mut head: Vec<Line> = lines.split_off(0);
    head.extend(wrapped(&row.thread.cwd.display().to_string(), &look.plain, width));
    if let Some((input, output)) = row.details.tokens {
        let value = strings.tokens_line(&format_tokens_compact(input), &format_tokens_compact(output), num);
        head.push(vec![span(strings.tokens, &look.dim), span(value, &look.plain)]);
    }
    if let Some(branch) = row.thread.git_info.as_ref().and_then(|git| git.branch.as_ref()) {
        head.push(Vec::new());
        head.push(vec![span(strings.branch, &look.dim)]);
        head.extend(wrapped(branch, &look.plain, width));
    }
    let prompt_start = head.len();
    head.push(Vec::new());
    head.push(vec![span(strings.prompt, &look.dim)]);
    let prompt_text = codex_tui_core::screens::agents::preview_markdown(
        row.details.latest_prompt.as_deref().unwrap_or(&row.thread.preview),
    );
    let prompt_text = if prompt_text.is_empty() { strings.no_prompt.to_owned() } else { prompt_text };
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
                activity.push(vec![span(strings.agent_label, &look.dim), span(name.clone(), &look.plain)]);
            }
            DetailLine::NeedsAttention => activity.push(vec![span(strings.needs_attention, &look.red)]),
            DetailLine::Request(text) => activity.extend(wrapped(text, &look.plain, width)),
            DetailLine::OpenToReview => activity.push(vec![span(strings.open_task, &look.dim)]),
            DetailLine::WaitingApproval => activity.push(vec![span(strings.wait_approval, &look.plain)]),
            DetailLine::WaitingResponse => activity.push(vec![span(strings.wait_response, &look.plain)]),
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
        Some(codex_tui_core::screens::agents::Notice::Reconnecting
            | codex_tui_core::screens::agents::Notice::ReconnectFailed)
    );
    if !offline && !activity.is_empty() {
        if row.details.tokens.is_some() && activity.len() > height.saturating_sub(lines.len()) {
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
    let mut column = Column::new().main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start);
    for line in lines {
        column = column.child(cell(width as i32, one(line)));
    }
    column.into()
}

fn clip_pair(label: &str, value: &str, look: &Look, width: usize) -> Line {
    let text = layout::ellipsize(&format!("{label}{value}"), width);
    let split = label.chars().count().min(text.chars().count());
    let head: String = text.chars().take(split).collect();
    let tail: String = text.chars().skip(split).collect();
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
        Some(Notice::LoadFailed) => Some(strings.load_failed.to_owned()),
        Some(Notice::ServerVersion(text) | Notice::Error(text)) => Some(text.clone()),
        None => (model.paging.failed && !model.paging.loading).then(|| strings.load_failed.to_owned()),
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
    items.push((strings.key_open, if model.rename.is_some() { strings.rename } else { strings.open }));
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

/// The search or rename row above the list.
pub fn editor(look: &Look, strings: &Strings, model: &Model, width: usize) -> AnyWidget {
    let (label, text) = match &model.rename {
        Some(rename) => (strings.rename_label, rename.text.as_str()),
        None => (strings.search_label, model.search.as_str()),
    };
    let available = width.saturating_sub(label.chars().count() + 1);
    let skip = text.chars().count().saturating_sub(available);
    let shown: String = text.chars().skip(skip).collect();
    let label_style = look.cyan.clone().attrs(Attrs::BOLD);
    BoxWidget::new()
        .style(look.tab.clone())
        .child(cell(
            width as i32,
            one(vec![span(label, &label_style), span(shown, &look.tab)]),
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
    model: &Model,
    center: &Center,
    areas: &layout::Areas,
    controller: &ListController,
    taps: &Taps,
) -> AnyWidget {
    let editing = model.searching || model.rename.is_some();
    let search_rows = usize::from(editing && areas.body_height > 0);
    let list_height = areas.body_height - search_rows;
    let mut pane = Column::new().main_axis_size(MainAxisSize::Min).cross_axis_alignment(CrossAxisAlignment::Start);
    if search_rows > 0 {
        pane = pane.child(editor(look, strings, model, areas.list_width));
    }
    pane = pane.child(list(look, strings, model, center, areas.list_width, list_height, controller, taps));
    let mut row = Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(pad(areas.list_x))
        .child(SizedBox::new(Some(areas.list_width as i32), Some(areas.body_height as i32), Some(pane.into())));
    if let Some(_gap) = areas.gap_x {
        let mut gap = Column::new().main_axis_size(MainAxisSize::Min);
        for _ in 0..areas.body_height {
            gap = gap.child(cell(3, Row::new().child(pad(1)).child(cell(1, one(vec![span("\u{2502}", &look.dim)])))));
        }
        row = row.child(SizedBox::new(Some(3), Some(areas.body_height as i32), Some(gap.into())));
        if let Some(selected) = center.selected_row() {
            row = row.child(SizedBox::new(
                Some(areas.details_width as i32),
                Some(areas.body_height as i32),
                Some(details(look, strings, num, selected, model, areas.details_width, areas.body_height)),
            ));
        }
    }
    SizedBox::new(None, Some(areas.body_height as i32), Some(row.into())).into()
}

/// The footer row, inset two cells.
pub fn footer_row(look: &Look, strings: &Strings, model: &Model, can_return: bool, width: usize) -> AnyWidget {
    let inner = width.saturating_sub(4);
    Row::new()
        .child(pad(2))
        .child(cell(inner as i32, one(footer(look, strings, model, can_return))))
        .into()
}

/// The shortcut help in place of the list and the details (oracle:
/// `center_help_lines`).
pub fn help(look: &Look, strings: &Strings, areas: &layout::Areas, width: usize) -> AnyWidget {
    let _ = (look, strings, width);
    SizedBox::new(None, Some(areas.body_height as i32), None).into()
}
