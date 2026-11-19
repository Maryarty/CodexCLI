//! The agent rows: entries, the scrolling `RowsList`.

use super::header::{heading, show_more};
use super::*;

/// The status word and dot of a row (oracle: `status`).
pub(super) fn status<'a>(row: &RowData, strings: &'a Strings, look: &Look) -> (&'a str, Span) {
    match row.group {
        Group::NeedsYou if row.is_error() => (strings.error, span("!", &look.red)),
        Group::NeedsYou => (strings.needs_input, span("\u{25cf}", &look.red)),
        Group::Working => (strings.working, span("\u{25cf}", &look.green)),
        Group::Ready => (strings.ready, span("\u{25cb}", &look.cyan)),
        Group::Finished => (strings.inactive, span("\u{25cb}", &look.dim)),
    }
}

pub(super) fn grouping_name(strings: &Strings, grouping: Grouping) -> &str {
    match grouping {
        Grouping::Project => strings.grouping[0],
        Grouping::Directory => strings.grouping[1],
        Grouping::Status => strings.grouping[2],
        Grouping::Model => strings.grouping[3],
    }
}

/// The age column (oracle: `format_relative_time`): the shared age, worded
/// with this page's strings.
fn age(strings: &Strings, now: i64, updated: i64) -> String {
    use crate::widgets::age::{Age, age};
    match age(now, Some(updated)) {
        Age::Missing | Age::Now => strings.now.to_owned(),
        Age::Seconds(n) => format!("{n}{}", strings.ago[0]),
        Age::Minutes(n) => format!("{n}{}", strings.ago[1]),
        Age::Hours(n) => format!("{n}{}", strings.ago[2]),
        Age::Days(n) => format!("{n}{}", strings.ago[3]),
    }
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

/// The status and age columns (oracle: `rows.rs` `columns`).
fn metadata(cells: Row, status: (&str, &Style), age: (String, &Style)) -> Row {
    cells
        .child(gap(2))
        .child(fixed(
            STATUS_COLUMN,
            Text::line(status.0.to_owned()).style(status.1.clone()),
        ))
        .child(gap(2))
        .child(fixed(
            AGE_COLUMN,
            Text::line(age.0)
                .style(age.1.clone())
                .text_align(TextAlign::Right),
        ))
}

/// One entry of the list as a one-row widget `width` cells wide.
#[allow(clippy::too_many_arguments)]
fn entry(
    seg: Measure,
    look: &Look,
    strings: &Strings,
    model: &Model,
    center: &Center,
    index: usize,
    width: i32,
    selected: bool,
) -> AnyWidget {
    let fill = if selected {
        look.selection.clone()
    } else {
        look.plain.clone()
    };
    let marker = if selected { "\u{203a}" } else { " " };
    let boxed = |row: Row| BoxWidget::new().style(fill.clone()).child(row);
    match &center.entries[index] {
        CenterRow::Gap => gap(1),
        CenterRow::Group { label, count } => short(heading(
            seg,
            look,
            strings,
            model,
            label,
            count,
            usize::try_from(width - 1).unwrap_or(0),
        )),
        CenterRow::ShowMore => {
            let label = format!("{marker}   {}", show_more(strings, model));
            boxed(Row::new().child(Expanded::new(1, Text::line(label).style(fill.clone())))).into()
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
            boxed(cells.child(gap(1))).into()
        }
    }
}

/// The list pane (oracle: `render_center_rows`): the column labels, the
/// rows in a `ListView` (a `ListController` scrolls it, the `SelectionController`
/// keeps the cursor; the keys are the screen's), the overflow arrows.
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
        list: controls.list.clone(),
        focus: controls.focus.clone(),
        empty_focus: controls.empty_focus.clone(),
        scroll: controls.scroll.clone(),
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
    list: ListController,
    focus: FocusNode,
    empty_focus: FocusNode,
    scroll: Rc<ValueNotifier<Scrolled>>,
    taps: Rc<Vec<Handler<PointerEvent>>>,
}

impl PartialEq for RowsList {
    fn eq(&self, other: &Self) -> bool {
        self.inputs == other.inputs
            && self.selection == other.selection
            && self.list == other.list
            && self.focus == other.focus
            && self.empty_focus == other.empty_focus
            && Rc::ptr_eq(&self.scroll, &other.scroll)
            && self.taps == other.taps
    }
}

impl StatelessWidget for RowsList {
    fn build(&self, _cx: &mut BuildCx<'_>) -> AnyWidget {
        let ListInputs {
            look,
            strings,
            agents,
            center,
        } = self.inputs.clone();
        let (strings, taps) = (*strings, self.taps.clone());
        let controls = ListControls {
            selection: self.selection.clone(),
            list: self.list.clone(),
            focus: self.focus.clone(),
            empty_focus: self.empty_focus.clone(),
            scroll: self.scroll.clone(),
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
    LayoutBuilder::new(move |cx, constraints| {
        let width = constraints.max_width;
        let seg = segmenter_of(cx);
        let model = &*agents;
        if center.entries.is_empty() {
            let text = empty_text(&strings, model, model.threads.is_empty());
            // The line is a focus stop of its own: with no row the keys
            // have nowhere else to go.
            let line = top(short(Text::line(text.to_owned()).style(look.dim.clone())));
            return Focus::new(line)
                .focus_node(controls.empty_focus.clone())
                .autofocus()
                .into();
        }
        let keys: Rc<Vec<RowKey>> = Rc::new(
            (0..center.entries.len())
                .map(|at| key_of(&center, at))
                .collect(),
        );
        let rows = SessionList {
            len: keys.len(),
            key: {
                let keys = keys.clone();
                Rc::new(move |index| keys[index].clone())
            },
            selectable: Some({
                let keys = keys.clone();
                Rc::new(move |index| keys[index].selectable())
            }),
            row: {
                let (look, agents, center) = (look.clone(), agents.clone(), center.clone());
                Rc::new(move |index, selected| {
                    entry(
                        seg, &look, &strings, &agents, &center, index, width, selected,
                    )
                })
            },
            selection: controls.selection.clone(),
            list: controls.list.clone(),
            scroll: controls.scroll.clone(),
            focus: Some(controls.focus.clone()),
            extent: Some(1),
            taps: Some(taps.clone()),
        }
        .build();
        let mut column = Column::new()
            .overflow_intended(true)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        let tall = constraints.max_height >= 3;
        if tall {
            let mut labels = Row::new()
                .child(arrow(&look, &controls.scroll, false))
                .child(gap(3))
                .child(Expanded::new(
                    1,
                    Text::line(strings.tasks).style(look.dim.clone()),
                ));
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
        let show = if down {
            scrolled.below()
        } else {
            scrolled.above()
        };
        let glyph = match (show, down) {
            (false, _) => " ",
            (true, true) => "\u{2193}",
            (true, false) => "\u{2191}",
        };
        fixed(1, Text::line(glyph).style(style.clone()))
    })
    .into()
}
