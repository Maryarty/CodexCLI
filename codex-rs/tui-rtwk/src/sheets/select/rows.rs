//! The rows of the selection list: the window of items the cursor leaves in
//! view, as a `Grid` with a label column as wide as its widest label (at most
//! 70% of the room, the oracle's `compute_desc_col`) and a description column
//! that wraps. In Flutter it is a `LayoutBuilder` over a `Table`; the window is
//! the items the list's `ScrollState` shows (`top`), not a scroll position,
//! because the oracle moves by items and marks what is hidden with arrows
//! (`render_scroll_indicators`).

use std::rc::Rc;

use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{CrossAxisAlignment, Expanded, Grid, GridPlacement, GridTrack, Row as Line};
use rtwk::style::Style;
use rtwk::text::{Text, TextOverflow, TextWrap};
use rtwk::{AnyWidget, Handler, LayoutBuilder};

use codex_tui_core::screens::select::list::Row;

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

    /// The description, the reason of a disabled row in it.
    fn description(&self, row: &Row) -> Option<String> {
        let text = row.description.as_deref().filter(|text| !text.is_empty());
        match (text, row.reason.as_deref()) {
            (Some(text), Some(reason)) => {
                Some([text, self.reason_after, reason, self.reason_close].concat())
            }
            (None, Some(reason)) => Some([self.reason_alone, reason].concat()),
            (text, None) => text.map(str::to_string),
        }
    }
}

/// What the grid is made of.
pub struct Rows {
    /// The rows of the window, in order (`Browse::rows` sliced).
    pub window: Rc<Vec<Row>>,
    /// The rows outside the window that still count for the label column
    /// (`ColumnWidthMode::AutoAllRows`); they take no height.
    pub measured: Rc<Vec<Row>>,
    /// The position of the cursor inside `window`.
    pub cursor: Option<usize>,
    pub styles: RowStyles,
    /// One line per row, cut with an ellipsis (`SelectionRowDisplay::SingleLine`).
    pub single_line: bool,
    /// The label column is at least this many cells (`name_column_width`).
    pub name_width: Option<usize>,
    /// The words around the rows' names and descriptions.
    pub words: Words,
    /// What a tap on each row of `window` sends (empty: rows do not react).
    pub taps: Vec<Handler<PointerEvent>>,
}

impl Rows {
    pub fn view(self) -> AnyWidget {
        LayoutBuilder::new(move |_, constraints| {
            // The label column keeps 30% of the room (less the gap) for the
            // descriptions: a longer label wraps inside it (G39b).
            let room = usize::try_from(constraints.max_width).unwrap_or(0);
            let cap = i32::try_from((room * 70 / 100).saturating_sub(2).max(1)).unwrap_or(1);
            let floor = self
                .name_width
                .map_or(0, |width| i32::try_from(width + 2).unwrap_or(0));
            let mut grid = Grid::new()
                .columns([
                    GridTrack::content().within(floor.min(cap), cap),
                    GridTrack::fr(1),
                ])
                .column_gap(2);
            let mut at = 0;
            for (index, row) in self.window.iter().enumerate() {
                grid = self.add(grid, index, row, Some(index) == self.cursor);
                at += 1;
            }
            for row in self.measured.iter() {
                let label = [row.prefix.as_str(), self.words.name(row).as_str()].concat();
                grid = grid.child(GridPlacement::new(Text::new(label)).at(at, 0));
                at += 1;
            }
            let hidden = self.measured.len();
            let tracks = (0..self.window.len())
                .map(|_| GridTrack::content())
                .chain((0..hidden).map(|_| GridTrack::fixed(0)));
            grid.rows(tracks).into()
        })
        .into()
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

    fn add(&self, grid: Grid, index: usize, row: &Row, cursor: bool) -> Grid {
        let at = i32::try_from(index).unwrap_or(0);
        let styles = &self.styles;
        let (label_style, description_style) = if row.disabled {
            (styles.disabled.clone(), styles.disabled.clone())
        } else if cursor {
            (styles.cursor.clone(), styles.cursor.clone())
        } else {
            (styles.label.clone(), styles.description.clone())
        };
        let name = self.words.name(row);
        // The gutter stays put; a name that wraps hangs after it.
        let label: AnyWidget = if self.single_line {
            self.text([row.prefix.as_str(), name.as_str()].concat(), label_style)
                .into()
        } else {
            Line::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Text::line(row.prefix.clone()).style(label_style.clone()))
                .child(Expanded::new(1, self.text(name, label_style)))
                .into()
        };
        let tap = |child: AnyWidget| -> AnyWidget {
            match self.taps.get(index) {
                Some(on_tap) => GestureDetector::new()
                    .on_tap(on_tap.clone())
                    .child(child)
                    .into(),
                None => child,
            }
        };
        match self.words.description(row) {
            Some(text) => grid.child(GridPlacement::new(tap(label)).at(at, 0)).child(
                GridPlacement::new(tap(self.text(text, description_style).into())).at(at, 1),
            ),
            None => grid.child(GridPlacement::new(tap(label)).at(at, 0).span(1, 2)),
        }
    }
}
