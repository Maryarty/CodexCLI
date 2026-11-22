//! The rows of the selection list, as Flutter writes a list with a cursor of
//! its own: a `ListView.builder` over every row, a `ListController` for the
//! scroll (the dialog asks it to show the cursor row) and a
//! `SelectionController` for the cursor, each row in a `ListenableBuilder` on
//! it. The label column is as wide as the widest label of the list (at most
//! 70% of the room, the oracle's `compute_desc_col`, `ColumnWidthMode::
//! AutoAllRows`), measured under a `LayoutBuilder` with the tree's segmenter; a
//! description column wraps beside it. The list is as tall as the `max_rows`
//! rows that take most lines (the oracle shows that many items at once).
//!
//! The oracle also measures the column over the rows in view only
//! (`AutoVisible`); rtwk has no hook for a list's window at layout time, so
//! that is gap G39b (`parity/select-dialog.md`), not a window of ours.

use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{CrossAxisAlignment, Expanded, Row as Line, SizedBox};
use rtwk::scroll::{ListController, ListView};
use rtwk::style::Style;
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, Handler};

use codex_tui_core::screens::select::list::Row;

use crate::widgets::measure::{self, Measure};

/// The styles of the rows.
#[derive(Clone, Debug)]
pub struct RowStyles {
    pub cursor: Style,
    pub label: Style,
    pub description: Style,
    pub disabled: Style,
}

/// The words a row adds to what the model holds (strings.rs).
#[derive(Clone, Copy, Debug)]
pub struct Words {
    pub current: &'static str,
    pub disabled: &'static str,
    pub reason_after: &'static str,
    pub reason_alone: &'static str,
    pub reason_close: &'static str,
    /// The cell before a row's prefix: the cursor's mark, or blank.
    pub mark_on: &'static str,
    pub mark_off: &'static str,
    /// The rule row under a row with a section break.
    pub section_break: &'static str,
}

impl Words {
    /// The name with its marks.
    fn name(&self, row: &Row) -> String {
        let mut name = row.name.clone();
        if row.current {
            name.push_str(self.current);
        }
        if row.reason.is_some() {
            name.push_str(self.disabled);
        }
        name
    }

    /// The description, the reason of a disabled row in it; `selected`: the
    /// one the cursor row shows.
    fn description(&self, row: &Row, selected: bool) -> Option<String> {
        let shown = if selected {
            row.selected_description
                .as_ref()
                .or(row.description.as_ref())
        } else {
            row.description.as_ref()
        };
        let text = shown.map(String::as_str).filter(|text| !text.is_empty());
        match (text, row.reason.as_deref()) {
            (Some(text), Some(reason)) => {
                Some([text, self.reason_after, reason, self.reason_close].concat())
            }
            (None, Some(reason)) => Some([self.reason_alone, reason].concat()),
            (text, None) => text.map(str::to_string),
        }
    }
}

/// What the list is made of.
pub struct Rows {
    /// One row per visible item, in order (`Browse::rows`).
    pub rows: Rc<Vec<Row>>,
    /// The item each row stands for: what the cursor holds.
    pub items: Rc<Vec<usize>>,
    pub selection: SelectionController<usize>,
    pub list: ListController,
    /// The items the list shows at once (`List::max_rows`).
    pub max_rows: usize,
    pub styles: RowStyles,
    /// One line per row, cut with an ellipsis (`SelectionRowDisplay::SingleLine`).
    pub single_line: bool,
    /// The label column is at least this many cells (`name_column_width`).
    pub name_width: Option<usize>,
    /// The description column hides when it would be narrower (cells).
    pub hide_description_below: Option<usize>,
    /// The words around the rows' names and descriptions.
    pub words: Words,
    /// What a tap on each row sends (empty: rows do not react).
    pub taps: Rc<Vec<Handler<PointerEvent>>>,
}

/// Where the cells of the rows go, measured at one width.
pub struct Layout {
    /// The label column, none when no row has a description.
    label: Option<i32>,
    /// The lines each row takes.
    heights: Vec<i32>,
}

impl Rows {
    /// Measures the rows at `room` cells with the tree's segmenter.
    pub fn measure(&self, seg: Measure, room: usize) -> Layout {
        self.layout(
            &|text| seg.width(text),
            &|text, width| measure::rows(seg, text, width.max(1)).len().max(1),
            room,
        )
    }

    /// The most lines the list wants: the `max_rows` consecutive rows that
    /// take most (the oracle shows that many items at once).
    pub fn wanted(&self, layout: &Layout) -> i32 {
        self.tallest(&layout.heights)
    }

    /// The list, `height` lines high: a `ListView` over every entry, which
    /// scrolls under the `ListController` when the room is less than it wants.
    pub fn list(self: &Rc<Self>, layout: &Layout, height: i32) -> AnyWidget {
        let rows = self.clone();
        let label = layout.label;
        // A rule is an entry of its own: it takes one of the list's rows.
        let entries = Rc::new(self.entries());
        let list = ListView::builder(entries.len(), move |index| {
            match entries.get(index).copied().flatten() {
                Some(row) => rows.row(row, label),
                None => rows.rule(),
            }
        })
        .controller(self.list.clone())
        .can_request_focus(false);
        SizedBox::new(None, Some(height), Some(list.into())).into()
    }

    /// The label column and the height of each row at `room` cells.
    fn layout(
        &self,
        width: &dyn Fn(&str) -> usize,
        lines: &dyn Fn(&str, usize) -> usize,
        room: usize,
    ) -> Layout {
        let mark = width(self.words.mark_off);
        let described = self
            .rows
            .iter()
            .any(|row| self.words.description(row, false).is_some());
        let label = described.then(|| {
            // The label column keeps 30% of the room (less the gap) for the
            // descriptions: a longer label wraps inside it.
            let cap = (room * 70 / 100).saturating_sub(2).max(1);
            let widest = self
                .rows
                .iter()
                .map(|row| mark + width(&row.prefix) + width(&self.words.name(row)))
                .max()
                .unwrap_or(0);
            self.name_width
                .map_or(widest, |own| own.max(widest))
                .min(cap)
        });
        // A column left narrower than the oracle's minimum hides, and every
        // row has the whole room (`HideWhenNarrow`).
        let label = label.filter(|label| {
            self.hide_description_below
                .is_none_or(|least| room.saturating_sub(label + 2) >= least)
        });
        let heights = self
            .rows
            .iter()
            .flat_map(|row| {
                let rule = row.section_break.then_some(1);
                let height = if self.single_line {
                    1
                } else {
                    self.row_height(row, label, mark, width, lines, room)
                };
                [Some(height), rule].into_iter().flatten()
            })
            .collect();
        Layout {
            label: label.and_then(|label| i32::try_from(label).ok()),
            heights,
        }
    }

    /// The list's entries: a row, or `None` for the rule under it.
    fn entries(&self) -> Vec<Option<usize>> {
        let mut entries = Vec::new();
        for (index, row) in self.rows.iter().enumerate() {
            entries.push(Some(index));
            if row.section_break {
                entries.push(None);
            }
        }
        entries
    }

    /// The rule row of a section break: dim, one line.
    fn rule(&self) -> AnyWidget {
        Text::line(self.words.section_break.to_string())
            .style(self.styles.disabled.clone())
            .into()
    }

    /// The lines one row takes at `room` cells, its label column at `label`.
    fn row_height(
        &self,
        row: &Row,
        label: Option<usize>,
        mark: usize,
        width: &dyn Fn(&str) -> usize,
        lines: &dyn Fn(&str, usize) -> usize,
        room: usize,
    ) -> i32 {
        let hung = mark + width(&row.prefix);
        let name = self.words.name(row);
        let taken = match label {
            Some(label) => {
                let text = |selected| self.words.description(row, selected);
                let beside = room.saturating_sub(label + 2);
                let described = [text(false), text(true)]
                    .iter()
                    .flatten()
                    .map(|text| lines(text, beside))
                    .max()
                    .unwrap_or(1);
                lines(&name, label.saturating_sub(hung)).max(described)
            }
            None => lines(&name, room.saturating_sub(hung)),
        };
        i32::try_from(taken).unwrap_or(1)
    }

    /// The lines of the `max_rows` consecutive rows that take most.
    fn tallest(&self, heights: &[i32]) -> i32 {
        let count = self.max_rows.min(heights.len());
        (0..=heights.len() - count)
            .map(|start| heights[start..start + count].iter().sum())
            .max()
            .unwrap_or(0)
    }

    /// A cell's text: wrapped by word, or one line cut with an ellipsis.
    fn text(&self, data: String, style: Style) -> Text {
        if self.single_line {
            Text::line(data)
                .style(style)
                .overflow(TextOverflow::Ellipsis)
        } else {
            Text::new(data).style(style).wrap(TextWrap::Word)
        }
    }

    /// The row at `index`, in a builder that redraws it when the cursor moves.
    fn row(self: &Rc<Self>, index: usize, label: Option<i32>) -> AnyWidget {
        let rows = self.clone();
        let item = rows.items.get(index).copied();
        let tap = rows.taps.get(index).cloned();
        let cursor = rows.selection.clone();
        let built = ListenableBuilder::new(cursor.listenable(), move |_, _| {
            let on = item.is_some() && cursor.cursor() == item;
            rows.cells(index, on, label)
        });
        match tap {
            Some(on_tap) => GestureDetector::new().on_tap(on_tap).child(built).into(),
            None => built.into(),
        }
    }

    fn cells(&self, index: usize, cursor: bool, label_width: Option<i32>) -> AnyWidget {
        let Some(row) = self.rows.get(index) else {
            return AnyWidget::default();
        };
        let styles = &self.styles;
        let (label_style, description_style) = if row.disabled {
            (styles.disabled.clone(), styles.disabled.clone())
        } else if cursor {
            (styles.cursor.clone(), styles.cursor.clone())
        } else {
            (styles.label.clone(), styles.description.clone())
        };
        let mark = if cursor {
            self.words.mark_on
        } else {
            self.words.mark_off
        };
        let gutter = [mark, row.prefix.as_str()].concat();
        let name = self.words.name(row);
        // The gutter stays put; a name that wraps hangs after it.
        let label: AnyWidget = if self.single_line {
            self.text([gutter.as_str(), name.as_str()].concat(), label_style)
                .into()
        } else {
            Line::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Text::line(gutter).style(label_style.clone()))
                .child(Expanded::new(1, self.text(name, label_style)))
                .into()
        };
        match (self.words.description(row, cursor), label_width) {
            (Some(text), Some(width)) => Line::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(SizedBox::new(Some(width), None, Some(label)))
                .child(SizedBox::new(Some(2), None, None))
                .child(Expanded::new(1, self.text(text, description_style)))
                .into(),
            _ => label,
        }
    }
}
