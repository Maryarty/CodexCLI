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
    /// The words after a disabled row's name.
    pub disabled_mark: &'static str,
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
                grid = grid.child(
                    GridPlacement::new(Text::new(format!("{}{}", row.prefix, row.name))).at(at, 0),
                );
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
        let mark = if row.disabled_mark {
            self.disabled_mark
        } else {
            ""
        };
        // The gutter stays put; a name that wraps hangs after it.
        let label: AnyWidget = if self.single_line {
            self.text(format!("{}{}{mark}", row.prefix, row.name), label_style)
                .into()
        } else {
            Line::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(Text::line(row.prefix.clone()).style(label_style.clone()))
                .child(Expanded::new(
                    1,
                    self.text(format!("{}{mark}", row.name), label_style),
                ))
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
        match row.description.as_deref().filter(|text| !text.is_empty()) {
            Some(text) => grid.child(GridPlacement::new(tap(label)).at(at, 0)).child(
                GridPlacement::new(tap(self.text(text.to_string(), description_style).into()))
                    .at(at, 1),
            ),
            None => grid.child(GridPlacement::new(tap(label)).at(at, 0).span(1, 2)),
        }
    }
}
