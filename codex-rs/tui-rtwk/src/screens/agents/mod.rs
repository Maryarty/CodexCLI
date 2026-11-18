//! `agents` (S21): the view half of the agents command center. One
//! `PageRoute` over the model (`ScreenStates.agents`): the header, the filter
//! strip, a rule, the grouped task list beside the details of the highlighted
//! task, and the key hints. Its model, events and intents are core's
//! `codex_tui_rtwk_core::screens::agents`.
//! Oracle: `tui/src/app/agent_center/` (`render.rs`, `rows.rs`, `hints.rs`) and
//! `app/agents_overview_view.rs` (`status`, `render_details`).

use std::cell::Cell;
use std::rc::Rc;

use rtwk::focus::{Focus, KeyPress};
use rtwk::l10n::NumSyms;
use rtwk::layout::{Column, CrossAxisAlignment};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::{KeyCode, Mods};
use rtwk::provider::{read, watch};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::style::Attrs;

use codex_tui_rtwk_core::ids::ThreadId;
use codex_tui_rtwk_core::screens::agents::{
    Attention, Filter, Intent as AgentsIntent, Model, Task, Wait,
};
use codex_tui_rtwk_core::screens::{ScreenIntent, ScreenStates};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::theme;
use crate::ui::seeded::Seeded;
use crate::ui::view::Hs;

mod layout;
pub mod strings;

use strings::Strings;

/// What the debug menu lists for this directory.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "overview",
            description: "three tasks of one project; the first one needs you",
            script: include_str!("fixtures/overview.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { can_return: true }.into()),
        },
        DebugEntry {
            name: "attention",
            description: "a task whose child waits for an approval and a response",
            script: include_str!("fixtures/attention.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { can_return: true }.into()),
        },
        DebugEntry {
            name: "empty_narrow",
            description: "no tasks at 48 cells, with nowhere to go back to",
            script: include_str!("fixtures/empty_narrow.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen { can_return: false }.into()),
        },
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen { can_return: true }.into())
}

/// Opens the command center on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen { can_return: true }.into())
}

/// The command center; `can_return` is whether a chat lies under it (the
/// oracle's `can_return_to_chat`: it adds the back hint).
#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    pub can_return: bool,
}

/// Widget state: the list's first row (view state, R14).
#[derive(Debug)]
pub struct ScreenState {
    scroll: Rc<Cell<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Move(isize),
    Filter(isize),
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState { scroll: Rc::new(Cell::new(0)) }
    }
}

impl_widget!(stateful eq Screen);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    Plain,
    Dim,
    Bold,
    Red,
    Green,
    Cyan,
    Selected,
}

type Line = Vec<(String, Look)>;

fn text(text: impl Into<String>, look: Look) -> Line {
    vec![(text.into(), look)]
}

const FILTERS: [Filter; 5] = [
    Filter::All,
    Filter::NeedsYou,
    Filter::Working,
    Filter::Ready,
    Filter::Inactive,
];

fn count(model: &Model, filter: Filter) -> usize {
    match filter {
        Filter::All => model.tasks.len(),
        Filter::NeedsYou => model.count(Attention::NeedsYou),
        Filter::Working => model.count(Attention::Working),
        Filter::Ready => model.count(Attention::Ready),
        Filter::Inactive => model.count(Attention::Inactive),
    }
}

/// The status word and dot of a task (oracle: `status`).
fn status(task: &Task, strings: &Strings) -> (&'static str, (&'static str, Look)) {
    match task.attention() {
        Attention::NeedsYou => (strings.needs_input, ("●", Look::Red)),
        Attention::Working => (strings.working, ("●", Look::Green)),
        Attention::Ready => (strings.ready, ("○", Look::Cyan)),
        Attention::Inactive => (strings.inactive, ("○", Look::Dim)),
    }
}

/// One entry of the list: a project heading, a task or a gap.
enum Entry<'a> {
    Group(String),
    Task(&'a Task),
}

fn entries<'a>(model: &'a Model, strings: &Strings, num: &NumSyms) -> Vec<Entry<'a>> {
    let visible = model.visible();
    let mut order: Vec<&str> = Vec::new();
    for task in &visible {
        if !order.contains(&task.cwd.as_str()) {
            order.push(&task.cwd);
        }
    }
    let mut list = Vec::new();
    for cwd in order {
        let shown = visible.iter().filter(|t| t.cwd == cwd).count();
        let total = model.tasks.iter().filter(|t| t.cwd == cwd).count();
        list.push(Entry::Group(strings.group_heading(cwd, shown, total, num)));
        list.extend(visible.iter().filter(|t| t.cwd == cwd).map(|t| Entry::Task(t)));
    }
    list
}

/// The details lines of a task (oracle: `render_details`).
fn details(task: &Task, strings: &Strings, num: &NumSyms, width: usize, height: usize) -> Vec<Line> {
    let (status, (dot, dot_look)) = status(task, strings);
    let clip = |s: &str| layout::ellipsize(s, width);
    let mut lines: Vec<Line> = vec![
        text(strings.details, Look::Bold),
        Vec::new(),
        text(clip(&task.title), Look::Bold),
        vec![(dot.to_string(), dot_look), (" ".to_string(), Look::Plain), (status.to_string(), Look::Plain)],
        Vec::new(),
        vec![
            (strings.model.to_string(), Look::Dim),
            (clip(task.model.as_deref().unwrap_or(strings.unknown)), Look::Plain),
        ],
        vec![
            (strings.reasoning.to_string(), Look::Dim),
            (clip(task.reasoning.as_deref().unwrap_or(strings.unknown)), Look::Plain),
        ],
        Vec::new(),
        text(strings.project, Look::Dim),
        text(clip(&task.cwd), Look::Plain),
    ];
    if let Some(child) = &task.child {
        lines.push(Vec::new());
        lines.push(text(clip(&strings.agent(&child.name, num)), Look::Plain));
        if let Some(message) = &child.last_message {
            for row in layout::wrap(message, width) {
                lines.push(text(row, Look::Plain));
            }
        }
        lines.push(text(strings.open_task, Look::Plain));
        for wait in &child.waits {
            let shown = match wait {
                Wait::Approval => strings.wait_approval,
                Wait::Response => strings.wait_response,
            };
            lines.push(text(shown, Look::Plain));
        }
    }
    lines.push(Vec::new());
    lines.push(text(strings.prompt, Look::Dim));
    let prompt = if task.prompt.trim().is_empty() { strings.no_prompt } else { task.prompt.trim() };
    let mut rows: Vec<String> = layout::wrap(prompt, width);
    if rows.len() > 2 {
        rows.truncate(2);
        rows[1] = "…".to_string();
    }
    lines.extend(rows.into_iter().map(|r| text(r, Look::Plain)));
    lines.truncate(height);
    lines
}

/// The footer hints (oracle: `center_footer_hints`).
fn hints(strings: &Strings, can_return: bool) -> Line {
    let mut items = vec![(strings.key_help, strings.help)];
    if can_return {
        items.push((strings.key_back, strings.back));
    }
    items.extend([
        (strings.key_move, strings.move_),
        (strings.key_open, strings.open),
        (strings.key_new, strings.new),
    ]);
    let mut line = Line::new();
    for (index, (key, label)) in items.into_iter().enumerate() {
        if index > 0 {
            line.push(("  ".to_string(), Look::Dim));
        }
        line.push((key.to_string(), Look::Dim));
        line.push((format!(" {label}"), Look::Dim));
    }
    line
}

/// A cell-exact canvas of `height` rows: text placed at columns, later
/// writes over earlier ones.
struct Canvas {
    rows: Vec<Vec<(char, Look)>>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self { rows: vec![vec![(' ', Look::Plain); width]; height] }
    }

    fn put(&mut self, row: usize, col: usize, limit: usize, line: &Line) {
        let Some(cells) = self.rows.get_mut(row) else { return };
        let mut x = col;
        for (text, look) in line {
            for ch in text.chars() {
                if x >= col + limit || x >= cells.len() {
                    return;
                }
                cells[x] = (ch, *look);
                x += 1;
            }
        }
    }

    fn fill(&mut self, row: usize, col: usize, width: usize, look: Look) {
        if let Some(cells) = self.rows.get_mut(row) {
            for cell in cells.iter_mut().skip(col).take(width) {
                cell.1 = look;
            }
        }
    }

    fn lines(self) -> Vec<Line> {
        self.rows
            .into_iter()
            .map(|cells| {
                let mut line: Line = Vec::new();
                for (ch, look) in cells {
                    match line.last_mut() {
                        Some((text, last)) if *last == look => text.push(ch),
                        _ => line.push((ch.to_string(), look)),
                    }
                }
                line
            })
            .collect()
    }
}

#[allow(clippy::too_many_arguments)]
fn screen_lines(
    model: &Model,
    can_return: bool,
    scroll: &Cell<usize>,
    strings: &Strings,
    num: &NumSyms,
    width: usize,
    height: usize,
) -> Vec<Line> {
    let areas = layout::areas(width, height);
    let mut canvas = Canvas::new(width, height);
    let inner = width.saturating_sub(4);
    if areas.header > 0 {
        let group = strings.group(strings.grouping_project, strings.grouping_key, num);
        canvas.put(
            0,
            2,
            inner,
            &vec![(strings.header.to_string(), Look::Bold), (group, Look::Dim)],
        );
        let labels: Vec<String> = FILTERS
            .iter()
            .enumerate()
            .map(|(i, f)| strings.filter_count(strings.filters[i], count(model, *f), num))
            .collect();
        let mut tabs_width = inner;
        let hint = format!("{} {}", strings.filter_keys, strings.filter_label);
        if inner >= 80 {
            let hint_width = hint.chars().count();
            canvas.put(1, 2 + inner - hint_width, hint_width, &text(hint, Look::Dim));
            tabs_width = inner.saturating_sub(hint_width + 2);
        }
        let active = FILTERS.iter().position(|f| *f == model.filter).unwrap_or(0);
        let strip = layout::strip(&labels, active, tabs_width);
        let mut x = 2;
        if strip.left {
            canvas.put(1, x, 1, &text("‹", Look::Dim));
            x += 2;
        }
        for (offset, shown) in strip.widths.iter().enumerate() {
            let index = strip.start + offset;
            let label = layout::ellipsize(&format!(" {} ", labels[index]), *shown);
            let look = if index == active { Look::Selected } else { Look::Dim };
            canvas.put(1, x, *shown, &text(label, look));
            canvas.fill(1, x, *shown, look);
            x += shown + 1;
        }
        if strip.right {
            canvas.put(1, 2 + tabs_width - 1, 1, &text("›", Look::Dim));
        }
        canvas.put(2, 2, inner, &text("─".repeat(inner), Look::Dim));
    }

    let list_width = areas.list_width.saturating_sub(1);
    let top = areas.body_top;
    let list = entries(model, strings, num);
    if list.is_empty() {
        let message = if model.tasks.is_empty() { strings.no_tasks } else { strings.no_match };
        canvas.put(top, areas.list_x, list_width, &text(message, Look::Dim));
    } else {
        let highlighted = model.highlighted().map(|t| t.id.as_str());
        let selected = list
            .iter()
            .position(|e| matches!(e, Entry::Task(t) if Some(t.id.as_str()) == highlighted))
            .unwrap_or(0);
        let padding = usize::from(areas.body_height >= 3);
        let viewport = areas.body_height.saturating_sub(2 * padding);
        if padding > 0 {
            canvas.put(top, areas.list_x + 4, list_width.saturating_sub(4), &text(strings.tasks, Look::Dim));
        }
        let group_above = selected > 0 && matches!(list[selected - 1], Entry::Group(_));
        let start = layout::scroll(scroll.get(), selected, list.len(), viewport, group_above);
        scroll.set(start);
        for (offset, entry) in list.iter().skip(start).take(viewport).enumerate() {
            let row = top + padding + offset;
            match entry {
                Entry::Group(heading) => {
                    canvas.put(row, areas.list_x, list_width, &text(heading.clone(), Look::Dim));
                }
                Entry::Task(task) => {
                    let is_selected = Some(task.id.as_str()) == highlighted;
                    let look = if is_selected { Look::Selected } else { Look::Plain };
                    if is_selected {
                        canvas.fill(row, areas.list_x, areas.list_width, Look::Selected);
                    }
                    let marker = if is_selected { "›" } else { " " };
                    canvas.put(row, areas.list_x, 1, &text(marker, look));
                    let (_, (dot, dot_look)) = status(task, strings);
                    let dot_look = if is_selected { Look::Selected } else { dot_look };
                    canvas.put(row, areas.list_x + 2, 1, &text(dot, dot_look));
                    let title = layout::ellipsize(&task.title, list_width.saturating_sub(4));
                    canvas.put(row, areas.list_x + 4, list_width.saturating_sub(4), &text(title, look));
                }
            }
        }
        if padding > 0 && start > 0 {
            canvas.put(top, areas.list_x, 1, &text("↑", Look::Dim));
        }
        if padding > 0 && start + viewport < list.len() {
            canvas.put(top + areas.body_height - 1, areas.list_x, 1, &text("↓", Look::Dim));
        }
    }
    if let (Some(gap), Some(details_x)) = (areas.gap_x, areas.details_x) {
        for row in top..top + areas.body_height {
            canvas.put(row, gap + 1, 1, &text("│", Look::Dim));
        }
        if let Some(task) = model.highlighted() {
            for (offset, line) in details(task, strings, num, areas.details_width, areas.body_height)
                .iter()
                .enumerate()
            {
                canvas.put(top + offset, details_x, areas.details_width, line);
            }
        }
    }
    if areas.footer_row < height {
        canvas.put(areas.footer_row, 2, inner, &hints(strings, can_return));
    }
    canvas.lines()
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let screens = watch::<ScreenStates, _>(cx);
        let model = &screens.agents;
        let hs = read::<Hs, _>(cx);
        match msg {
            Msg::Move(step) => {
                let visible = model.visible();
                let at = model
                    .highlighted()
                    .and_then(|h| visible.iter().position(|t| t.id == h.id))
                    .unwrap_or(0);
                let next = at.saturating_add_signed(step).min(visible.len().saturating_sub(1));
                if let Some(task) = visible.get(next) {
                    cx.emit(&hs.screen, ScreenIntent::Agents(AgentsIntent::Select(ThreadId::new(task.id.clone()))));
                }
            }
            Msg::Filter(step) => {
                let at = FILTERS.iter().position(|f| *f == model.filter).unwrap_or(0);
                let next = (at as isize + step).rem_euclid(FILTERS.len() as isize) as usize;
                cx.emit(&hs.screen, ScreenIntent::Agents(AgentsIntent::SetFilter(FILTERS[next])));
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
        let can_return = w.can_return;
        let scroll = self.scroll.clone();
        let body = LayoutBuilder::new(move |_, constraints| {
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let height = usize::try_from(constraints.max_height).unwrap_or(0);
            let lines = screen_lines(&screens.agents, can_return, &scroll, strings, &num, width, height);
            let style = |look: Look| -> Style {
                match look {
                    Look::Plain => Style::default(),
                    Look::Dim => tokens.dim(),
                    Look::Bold => tokens.bold(),
                    Look::Red => tokens.error(),
                    Look::Green | Look::Cyan => tokens.accent(),
                    Look::Selected => tokens.selected().attrs(Attrs::REVERSE),
                }
            };
            let mut column = Column::new().cross_axis_alignment(CrossAxisAlignment::Start);
            for line in lines {
                let spans: Vec<Span> =
                    line.into_iter().map(|(t, look)| Span::styled(t, style(look))).collect();
                column = column.child(Text::rich(spans).wrap(TextWrap::None).max_lines(1));
            }
            column.into()
        });
        Focus::new(body)
            .autofocus()
            .bind(KeyPress::code(KeyCode::Up), cx.send(Msg::Move(-1)))
            .bind(KeyPress::code(KeyCode::Down), cx.send(Msg::Move(1)))
            .bind(KeyPress::code(KeyCode::Tab), cx.send(Msg::Filter(1)))
            .bind(KeyPress::code(KeyCode::Tab).with_mods(Mods::SHIFT), cx.send(Msg::Filter(-1)))
            .bind(KeyPress::ESC, cx.send(Msg::Close))
            .into()
    }
}
