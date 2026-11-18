//! `resume` (S18): the view half of the resume and fork picker. One
//! `PageRoute` over the model's rows (`ScreenStates.resume`): the header, the
//! filter/status/sort toolbar, the search line, the list of saved threads with
//! their meta lines and the footer with progress and key hints. Its model,
//! events and intents are core's `codex_tui_rtwk_core::screens::resume`.
//! Oracle: `tui/src/resume_picker.rs` and `resume_picker/layout.rs`; the pure
//! geometry and packing are in `layout.rs`.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment};
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::prelude::KeyCode;
use rtwk::provider::{read, watch};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::style::Attrs;

use codex_tui_rtwk_core::screens::resume::{Intent as ResumeIntent, Load, Model, Row};
use codex_tui_rtwk_core::screens::{ScreenIntent, ScreenStates};

use crate::l10n::strings as shared;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::theme;
use crate::ui::seeded::Seeded;
use crate::ui::view::Hs;

mod layout;
mod list;
pub mod strings;

use layout::{Hint, Part};
use strings::Strings;

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        DebugEntry {
            name: "fake_rollouts",
            description: "a resume list from three fake rollouts",
            script: include_str!("fixtures/fake_rollouts.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| list::List.into()),
        },
        DebugEntry {
            name: "table",
            description: "three rows: a cloud one, an unread one, a plain one",
            script: include_str!("fixtures/table.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen::default().into()),
        },
        DebugEntry {
            name: "more_indicators",
            description: "five rows, the third selected and the list scrolled",
            script: include_str!("fixtures/more_indicators.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| {
                Screen {
                    selected: 2,
                    scroll_top: 1,
                }
                .into()
            }),
        },
        DebugEntry {
            name: "search_error",
            description: "a listing that failed: the message on the search line",
            script: include_str!("fixtures/search_error.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen::default().into()),
        },
        DebugEntry {
            name: "footer_wide",
            description: "no rows at 220 cells: the footer's wide hints",
            script: include_str!("fixtures/footer_wide.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen::default().into()),
        },
        DebugEntry {
            name: "loading",
            description: "the first page is loading",
            script: include_str!("fixtures/loading.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen::default().into()),
        },
        DebugEntry {
            name: "empty",
            description: "no saved sessions",
            script: include_str!("fixtures/empty.jsonl"),
            open: EntryOpen::Route(|_: &Seeded| Screen::default().into()),
        },
    ]
}

/// The route of this screen, pushed by the widget that opens it
/// (`Navigator::of(cx).push`).
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen::default().into())
}

/// Opens the picker on the live model (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen::default().into())
}

/// The picker: where its selection and scroll start.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    pub selected: usize,
    pub scroll_top: usize,
}

/// Widget state: the selection and the list's first row (R14: view state, the
/// model holds none of it).
#[derive(Debug)]
pub struct ScreenState {
    selected: usize,
    scroll_top: Rc<Cell<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Up,
    Down,
    Accept,
    Close,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState {
            selected: self.selected,
            scroll_top: Rc::new(Cell::new(self.scroll_top)),
        }
    }
}

impl_widget!(stateful eq Screen);

/// How a run of a row is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    Plain,
    Dim,
    DimItalic,
    Bold,
    Accent,
    Error,
}

type Line = Vec<(String, Look)>;

fn blank() -> Line {
    Vec::new()
}

/// The meta lines of `row` at `width` cells (oracle: `render_footer_lines`).
fn meta_lines(row: &Row, model: &Model, strings: &Strings, num: &rtwk::l10n::NumSyms, width: usize) -> Vec<Line> {
    let date = strings.age(layout::age(model.clock, row.updated.or(row.created)), num);
    let parts = vec![
        Part::Date(date),
        Part::Cwd(row.cwd.clone(), strings.no_cwd),
        Part::Branch(row.branch.clone(), strings.no_branch),
    ];
    layout::pack(parts, width)
        .into_iter()
        .map(|segs| {
            segs.into_iter()
                .map(|seg| {
                    let look = if seg.italic { Look::DimItalic } else { Look::Dim };
                    (seg.text, look)
                })
                .collect()
        })
        .collect()
}

/// The footer's two hint rows (oracle: `footer_hint_lines`).
fn hints(strings: &Strings, width: usize) -> [String; 2] {
    let hint = |key: &str, wide: &str, compact: &str, priority| Hint {
        key: key.to_string(),
        wide: wide.to_string(),
        compact: compact.to_string(),
        priority,
    };
    let first = [
        hint(strings.key_accept, strings.accept, strings.accept, 0),
        hint(strings.key_cancel, strings.cancel_wide, strings.cancel_compact, 1),
        hint(strings.key_quit, strings.quit, strings.quit, 2),
        hint(strings.key_focus, strings.focus_wide, strings.focus_compact, 7),
        hint(strings.key_option, strings.option_wide, strings.option_compact, 8),
    ];
    let second = [
        hint(strings.key_density, strings.density_wide, strings.density_compact, 3),
        hint(strings.key_transcript, strings.transcript_wide, strings.transcript_compact, 4),
        hint(strings.key_expand, strings.expand_wide, strings.expand_compact, 6),
        hint(strings.key_browse, strings.browse, strings.browse, 5),
    ];
    [layout::hint_row(&first, width), layout::hint_row(&second, width)]
}

/// The toolbar (oracle: `toolbar_for_width`): the full strip, else the compact
/// one.
fn toolbar(strings: &Strings, width: usize) -> Line {
    let full = |compact: bool| -> Line {
        let value = |text: &str, active: bool| {
            let look = if active { Look::Bold } else { Look::Dim };
            (format!(" {text} "), look)
        };
        let label = |text: &str, gap: &str| (format!("{text}{gap}"), Look::Dim);
        let sep = || ("   ".to_string(), Look::Dim);
        let gap = if compact { "" } else { " " };
        let mut line = vec![label(strings.filter, gap), value(strings.filter_all, true), sep()];
        line.push(label(strings.status, gap));
        line.push(value(strings.status_active, true));
        if !compact {
            line.push(value(strings.status_archived, false));
        }
        line.push(sep());
        line.push(label(strings.sort, gap));
        line.push(value(strings.sort_updated, true));
        if !compact {
            line.push(value(strings.sort_created, false));
        }
        line
    };
    let measure = |line: &Line| line.iter().map(|(text, _)| text.chars().count()).sum::<usize>();
    let wide = full(false);
    if measure(&wide) <= width {
        return wide;
    }
    full(true)
}

/// The whole picker as `height` lines of `width` cells (oracle:
/// `resume_picker/layout.rs::render`).
fn screen_lines(
    model: &Model,
    selected: usize,
    scroll_top: &Cell<usize>,
    strings: &Strings,
    num: &rtwk::l10n::NumSyms,
    width: usize,
    height: u16,
) -> Vec<Line> {
    let areas = layout::areas(height);
    let inset = |mut line: Line, by: usize| {
        if !line.is_empty() {
            line.insert(0, (" ".repeat(by), Look::Plain));
        }
        line
    };
    let mut lines: Vec<Line> = vec![blank(); usize::from(height)];
    let chrome_width = width.saturating_sub(2);
    lines[0] = inset(vec![(strings.header.to_string(), Look::Bold)], 1);
    lines[usize::from(areas.toolbar)] = inset(toolbar(strings, chrome_width), 1);
    lines[usize::from(areas.search)] = match &model.load {
        Load::Failed(message) => inset(vec![(layout::truncate(message, chrome_width), Look::Error)], 1),
        _ => inset(vec![(strings.search_hint.to_string(), Look::Dim)], 1),
    };

    let list_width = width.saturating_sub(usize::from(layout::LIST_INSET));
    let list_at = usize::from(areas.list);
    let viewport = usize::from(areas.list_height);
    let rows = &model.rows;
    let metas: Vec<Vec<Line>> = rows
        .iter()
        .map(|row| meta_lines(row, model, strings, num, list_width))
        .collect();
    let heights: Vec<usize> = metas.iter().map(|meta| layout::row_height(meta.len())).collect();
    let selected = selected.min(rows.len().saturating_sub(1));
    let top = layout::ensure_visible(scroll_top.get(), selected, &heights, viewport);
    scroll_top.set(top);

    let mut list: Vec<Line> = Vec::new();
    if rows.is_empty() {
        let text = match model.load {
            Load::Loading => strings.loading,
            _ => strings.empty,
        };
        list.push(vec![(text.to_string(), Look::DimItalic)]);
    } else {
        let above = top > 0;
        let below = layout::more_below(top, selected, &heights, viewport);
        let content = viewport - usize::from(above) - usize::from(below).min(viewport - usize::from(above));
        if above {
            list.push(vec![(strings.more_above.to_string(), Look::Accent)]);
        }
        let mut body: Vec<Line> = Vec::new();
        for (index, row) in rows.iter().enumerate().skip(top) {
            if body.len() >= content {
                break;
            }
            let is_selected = index == selected;
            let marker = if is_selected { "› " } else { "  " };
            let mut title: Line = vec![(marker.to_string(), if is_selected { Look::Accent } else { Look::Plain })];
            if row.unread {
                title.push(("● ".to_string(), Look::Accent));
            }
            let mark = if row.cloud { strings.cloud_mark } else { "" };
            let room = list_width.saturating_sub(marker.chars().count() + 2 * usize::from(row.unread));
            let text = format!("{mark}{}", layout::truncate(row.display_preview(), room));
            title.push((text, if is_selected { Look::Bold } else { Look::Plain }));
            body.push(title);
            body.extend(metas[index].iter().cloned());
            if index + 1 < rows.len() {
                body.push(blank());
            }
        }
        body.truncate(content);
        list.extend(body);
        while list.len() < viewport - usize::from(below).min(viewport) {
            list.push(blank());
        }
        if below {
            list.push(vec![(strings.more_below.to_string(), Look::Accent)]);
        }
    }
    for (offset, line) in list.into_iter().enumerate().take(viewport) {
        lines[list_at + offset] = inset(line, usize::from(layout::LIST_INSET));
    }

    let footer = usize::from(areas.footer);
    let position = if rows.is_empty() { 0 } else { selected + 1 };
    let total = rows.len().to_string();
    let total = if matches!(model.load, Load::Loading) { format!("{total}…") } else { total };
    let percent = if rows.is_empty() {
        100
    } else {
        layout::scroll_percent(top, &heights, viewport, selected)
    };
    let label = layout::progress_label(position, &total, percent, width);
    lines[footer] = vec![(layout::rule(width, &label), Look::Dim)];
    let [first, second] = hints(strings, width);
    if footer + 1 < lines.len() {
        lines[footer + 1] = vec![(first, Look::Dim)];
    }
    if footer + 2 < lines.len() {
        lines[footer + 2] = vec![(second, Look::Dim)];
    }
    lines
}

impl State for ScreenState {
    type Widget = Screen;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _: &Screen, cx: &mut Cx<'_, Self>) -> Rebuild {
        let screens = watch::<ScreenStates, _>(cx);
        let rows = &screens.resume.rows;
        match msg {
            Msg::Up => self.selected = self.selected.saturating_sub(1),
            Msg::Down => self.selected = (self.selected + 1).min(rows.len().saturating_sub(1)),
            Msg::Accept => {
                if let Some(row) = rows.get(self.selected) {
                    let hs = read::<Hs, _>(cx);
                    let intent = ScreenIntent::Resume(ResumeIntent::Resume(row.id.clone()));
                    cx.emit(&hs.screen, intent);
                }
                return Rebuild::No;
            }
            Msg::Close => {
                Navigator::maybe_pop(cx);
                return Rebuild::No;
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let strings = strings::of(cx);
        let num = shared::of(cx).num;
        let screens = watch::<ScreenStates, _>(cx);
        let selected = self.selected;
        let scroll_top = self.scroll_top.clone();
        let body = LayoutBuilder::new(move |_, constraints| {
            let width = usize::try_from(constraints.max_width).unwrap_or(0);
            let height = u16::try_from(constraints.max_height).unwrap_or(0);
            let lines = screen_lines(&screens.resume, selected, &scroll_top, strings, &num, width, height);
            let style = |look: Look| -> Style {
                match look {
                    Look::Plain => Style::default(),
                    Look::Dim => tokens.dim(),
                    Look::DimItalic => tokens.dim().attrs(Attrs::ITALIC),
                    Look::Bold => tokens.bold(),
                    Look::Accent => tokens.accent(),
                    Look::Error => tokens.error(),
                }
            };
            let mut column = Column::new().cross_axis_alignment(CrossAxisAlignment::Start);
            for line in lines {
                let spans: Vec<Span> = if line.is_empty() {
                    vec![Span::new(strings.blank)]
                } else {
                    line.into_iter()
                        .map(|(text, look)| Span::styled(text, style(look)))
                        .collect()
                };
                column = column.child(Text::rich(spans).wrap(TextWrap::None).max_lines(1));
            }
            column.into()
        });
        Focus::new(body)
            .autofocus()
            .bind(KeyPress::code(KeyCode::Up), cx.send(Msg::Up))
            .bind(KeyPress::code(KeyCode::Down), cx.send(Msg::Down))
            .bind(KeyPress::code(KeyCode::Enter), cx.send(Msg::Accept))
            .bind(KeyPress::ESC, cx.send(Msg::Close))
            .into()
    }
}
