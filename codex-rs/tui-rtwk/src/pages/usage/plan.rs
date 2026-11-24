//! The Plan report (oracle: `plan_panel.rs`): the history of the 5-hour and
//! the weekly limits as two tables of periods, side by side from 100 cells and
//! one under the other below, the notes under them. Which rows the tables
//! hold and the cursor in them are core's (`Model::plan_view`, `plan::Pos`);
//! the words are `strings.rs`'s. A table is a `Column` of rows that wrap by
//! word at its width (`report::words`); a period's row puts its share at the
//! right when the two fit one row, and wraps as one text when they do not
//! (the oracle's `columns` then the wrapping of the joined row). The row under
//! the cursor of the table in front is filled with the selection style.

use std::rc::Rc;

use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::l10n::NumSyms;
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::Style;
use rtwk::text::{Text, TextOverflow, TextWrap, segmenter_of};
use rtwk::{AnyWidget, Handler, LayoutBuilder};

use codex_tui_core::screens::usage::View;
use codex_tui_core::screens::usage::plan::{self, Line, Note, PlanView, Pos, TABLE_GAP};

use super::cards::{Fit, Item};
use super::report::words;
use super::strings::Strings;
use super::{Look, gap};

/// A click on each period of each table (made by the widget that owns the
/// state, as many as the history holds).
pub type RowTaps = Rc<[Vec<Handler<PointerEvent>>; 2]>;

/// Everything a table's rows are written from.
struct Say<'a> {
    strings: &'a Strings,
    num: &'a NumSyms,
    look: &'a Look,
    hour24: bool,
    taps: Option<&'a RowTaps>,
    /// The room one table has.
    room: Fit,
}

impl Say<'_> {
    /// A label at the left and its value at the right (oracle: `columns`):
    /// one row when both fit the table, else the two as words that wrap.
    /// A click on the row does what the handler does.
    fn tapped(&self, item: Item, tap: Option<Handler<PointerEvent>>) -> Item {
        match tap {
            Some(tap) => Item::new(
                GestureDetector::new().on_tap(tap).child(item.widget),
                item.rows,
            ),
            None => item,
        }
    }

    fn pair(&self, label: String, value: String, style: Option<&Style>, fill: bool) -> Item {
        let blank = self.strings.blank;
        let joined = format!("{label}{blank}{value}");
        let rows = self.room.rows(&joined);
        let plain = Style::new();
        let style = style.unwrap_or(&plain);
        if rows > 1 {
            return words(joined, style.clone(), Some(self.room));
        }
        let one = |shown: String| {
            Text::new(shown)
                .style(style.clone())
                .wrap(TextWrap::None)
                .overflow(TextOverflow::Clip)
                .max_lines(1)
        };
        let row = Row::new()
            .child(Expanded::new(1, one(label)))
            .child(one(value));
        if fill {
            Item::one(BoxWidget::new().style(style.clone()).child(row))
        } else {
            Item::one(row)
        }
    }

    fn line(&self, line: &Line) -> Item {
        let Say {
            strings,
            num,
            look,
            hour24,
            taps,
            room,
        } = self;
        let room = Some(*room);
        match line {
            Line::Heading { window, front } => {
                let title = if *window == 0 {
                    strings.plan_five
                } else {
                    strings.plan_week
                };
                let mark = if *front {
                    strings.plan_mark
                } else {
                    strings.blank
                };
                words(
                    format!("{mark}{}{title}", strings.blank),
                    look.bold.clone(),
                    room,
                )
            }
            Line::Blank => Item::one(gap()),
            Line::Empty { complete } => words(
                if *complete {
                    strings.plan_empty
                } else {
                    strings.plan_unavailable
                }
                .to_owned(),
                look.secondary.clone(),
                room,
            ),
            Line::Unavailable => words(
                strings.plan_unavailable.to_owned(),
                look.secondary.clone(),
                room,
            ),
            Line::Notice(note) => {
                words(strings.note(note).to_owned(), look.secondary.clone(), room)
            }
            Line::Period {
                window,
                index,
                start,
                end,
                current,
                used,
                selected,
                lit,
            } => {
                let label = strings.plan_period(*selected, *start, *end, *current, *hour24, num);
                let value = used.as_ref().map_or_else(
                    || strings.plan_unknown.to_owned(),
                    |amount| strings.plan_percent(amount, num),
                );
                let tap = taps.and_then(|taps| taps[*window].get(*index)).cloned();
                self.tapped(
                    self.pair(label, value, lit.then_some(&look.selected), *lit),
                    tap,
                )
            }
            Line::Incomplete => words(strings.plan_incomplete.to_owned(), look.dim.clone(), room),
            Line::Share { label, percent } => self.pair(
                strings.plan_indented(label, num),
                strings.plan_percent(percent, num),
                None,
                false,
            ),
            Line::NoBreakdown => {
                words(strings.plan_no_breakdown.to_owned(), look.dim.clone(), room)
            }
        }
    }

    fn note(&self, note: &Note, fit: Fit) -> Item {
        let Say {
            strings,
            num,
            look,
            hour24,
            ..
        } = self;
        let shown = match note {
            Note::Turn => strings.plan_turn_note.to_owned(),
            Note::Gaps => strings.plan_gaps.to_owned(),
            Note::AsOf(at) => strings.plan_as_of(*at, *hour24, num),
            Note::Since(at) => strings.plan_since(*at, *hour24, num),
            Note::Approximate => strings.plan_approximate.to_owned(),
        };
        words(shown, look.dim.clone(), Some(fit))
    }
}

fn stacked(items: Vec<Item>) -> AnyWidget {
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .children(items.into_iter().map(|item| item.widget))
        .into()
}

/// The tables and the notes in the room `fit` gives, with the rows each
/// part takes (the card cuts at them).
pub fn items(
    view: &PlanView,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    hour24: bool,
    taps: Option<&RowTaps>,
    fit: Fit,
) -> Vec<Item> {
    let width = plan::table_width(fit.width);
    let say = Say {
        strings,
        num,
        look,
        hour24,
        taps,
        room: Fit { width, ..fit },
    };
    let [left, right] = view
        .tables
        .each_ref()
        .map(|lines| lines.iter().map(|line| say.line(line)).collect::<Vec<_>>());
    let rows = |items: &[Item]| items.iter().map(|item| item.rows).sum::<usize>();
    let mut out = Vec::new();
    if plan::side_by_side(fit.width) {
        let (left_rows, right_rows) = (rows(&left), rows(&right));
        let table =
            |items: Vec<Item>| SizedBox::new(Some(width as i32), None, Some(stacked(items)));
        out.push(Item::new(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(table(left))
                .child(SizedBox::fixed(TABLE_GAP as i32, 1))
                .child(table(right)),
            left_rows.max(right_rows),
        ));
    } else {
        out.extend(left);
        out.push(Item::one(gap()));
        out.extend(right);
    }
    out.extend(view.notes.iter().map(|note| say.note(note, fit)));
    out
}

/// What the Plan card shows: the first period of each table and the notes.
pub fn card(
    model: &View<'_>,
    pos: &Pos,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    fit: Fit,
) -> Vec<Item> {
    items(
        &model.plan_view(pos, false),
        strings,
        num,
        look,
        model.hour24,
        None,
        fit,
    )
}

/// The focused Plan report: every period, the details of the open one, under
/// one scroll.
pub fn body(
    model: &View<'_>,
    pos: &Pos,
    strings: &Strings,
    num: &NumSyms,
    look: &Look,
    taps: Option<RowTaps>,
    scroll: ScrollController,
) -> AnyWidget {
    let view = model.plan_view(pos, true);
    let (strings, num, look, hour24) = (*strings, *num, look.clone(), model.hour24);
    LayoutBuilder::new(move |cx, constraints| {
        let fit = Fit {
            seg: segmenter_of(cx),
            width: usize::try_from(constraints.max_width).unwrap_or(0),
            plot_rows: 0,
        };
        let items = items(&view, &strings, &num, &look, hour24, taps.as_ref(), fit);
        SingleChildScrollView::new(stacked(items))
            .controller(scroll.clone())
            .into()
    })
    .into()
}
