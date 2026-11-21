//! The option rows of a dialog (the oracle's `list_selection_view`: a number
//! and a label, a description column that wraps, the cursor row marked `›`),
//! one for the confirmation of the agents, the generic selection dialog and
//! the request dialog of approvals. In Flutter it is a
//! `Column(min)[Flexible(SingleChildScrollView(Grid[content, fr]))]` with a
//! cursor in a notifier and the cursor kept in view by `ScrollController`
//! (G39: not a gap; `Scrollable.ensureVisible` has no public counterpart from
//! a handler, so the rows are measured with `TextPainter`'s counterpart,
//! `measure::rows`, and the controller jumps).

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, Flexible, Grid, GridPlacement, GridTrack, MainAxisSize,
    Padding,
};
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::Style;
use rtwk::text::{Text, segmenter_of};
use rtwk::{AnyWidget, LayoutBuilder};

use super::measure;

/// A row: the label (`1. name`) and the description beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionRow {
    pub label: String,
    pub description: Option<String>,
}

/// The styles of a row: the cursor row, the label and the description of
/// the others.
#[derive(Clone, Debug)]
pub struct OptionStyles {
    pub cursor: Style,
    pub label: Style,
    pub description: Style,
}

/// The cursor and the scroll of the rows, and the rows as the last layout
/// measured them (what a key needs to keep the cursor in view).
#[derive(Clone)]
pub struct Options {
    selection: SelectionController<usize>,
    scroll: ScrollController,
    heights: Rc<RefCell<Vec<i64>>>,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    pub fn new() -> Self {
        let selection = SelectionController::new();
        selection.set_cursor(Some(0));
        Self {
            selection,
            scroll: ScrollController::default(),
            heights: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// The row the cursor is on.
    pub fn cursor(&self) -> usize {
        self.selection.cursor().unwrap_or(0)
    }

    /// The cursor goes to `row` and the rows scroll to show it.
    pub fn set(&self, row: usize) {
        self.selection.set_cursor(Some(row));
        let heights = self.heights.borrow();
        if row >= heights.len() || !self.scroll.has_clients() {
            return;
        }
        let top: i64 = heights[..row].iter().sum();
        let bottom = top + heights[row];
        let position = self.scroll.position();
        let (pixels, view) = (position.pixels(), position.viewport_dimension());
        if top < pixels {
            self.scroll.jump_to(top);
        } else if bottom > pixels + view {
            self.scroll.jump_to(bottom - view);
        }
    }

    /// The cursor one row on (`delta` 1) or back, round the ends.
    pub fn step(&self, delta: i64, len: usize) {
        if len == 0 {
            return;
        }
        let len = i64::try_from(len).unwrap_or(1);
        let from = i64::try_from(self.cursor()).unwrap_or(0);
        self.set(usize::try_from((from + delta).rem_euclid(len)).unwrap_or(0));
    }

    /// The scroll of the rows, for what shows whether rows are hidden above
    /// or below (the oracle's picker arrows).
    pub fn scroll(&self) -> ScrollController {
        self.scroll.clone()
    }

    pub fn dispose(&self) {
        self.selection.dispose();
    }

    /// The rows: as tall as they are, scrolled by the room the dialog leaves
    /// (a `Flexible` for the `Column` of the dialog).
    pub fn view(&self, rows: Rc<Vec<OptionRow>>, styles: OptionStyles, indent: i32) -> AnyWidget {
        let (selection, scroll, heights) = (
            self.selection.clone(),
            self.scroll.clone(),
            self.heights.clone(),
        );
        Flexible::new(
            1,
            LayoutBuilder::new(move |cx, constraints| {
                let seg = segmenter_of(cx);
                let wide = |row: &OptionRow| seg.width(&format!("\u{203a} {}", row.label));
                let room = usize::try_from(constraints.max_width - 2 * indent).unwrap_or(0);
                // The label column is the widest label, at most 70% of the
                // room less the gap (the oracle's `compute_desc_col`) when
                // there are descriptions: a longer label wraps inside it
                // (`GridTrack::content().within(0, cap)`, G39b).
                let described = rows.iter().any(|row| {
                    row.description
                        .as_deref()
                        .is_some_and(|text| !text.is_empty())
                });
                let cap = if described {
                    (room * 70 / 100).saturating_sub(2)
                } else {
                    room
                }
                .max(1);
                let label_width = rows.iter().map(wide).max().unwrap_or(0).min(cap);
                let description_width = room.saturating_sub(label_width + 2).max(1);
                *heights.borrow_mut() =
                    rows.iter()
                        .map(|row| {
                            let lines = row.description.as_deref().map_or(1, |text| {
                                measure::rows(seg, text, description_width).len()
                            });
                            let label = measure::rows(
                                seg,
                                &["\u{203a} ", row.label.as_str()].concat(),
                                label_width.max(1),
                            )
                            .len();
                            i64::try_from(lines.max(label).max(1)).unwrap_or(1)
                        })
                        .collect();
                let mut grid = Grid::new()
                    .columns([
                        GridTrack::content().within(0, i32::try_from(cap).unwrap_or(1)),
                        GridTrack::fr(1),
                    ])
                    // Rows without descriptions give the labels the whole
                    // room: no gap before an empty column.
                    .column_gap(if described { 2 } else { 0 });
                for (index, row) in rows.iter().enumerate() {
                    let at = i32::try_from(index).unwrap_or(0);
                    let (label, description) = (row.label.clone(), row.description.clone());
                    let (watch, styles_a, styles_b) =
                        (selection.clone(), styles.clone(), styles.clone());
                    let (watch_b, text) = (selection.clone(), description.unwrap_or_default());
                    grid = grid
                        .child(
                            GridPlacement::new(ListenableBuilder::new(
                                watch.listenable(),
                                move |_, _| {
                                    let on = watch.cursor() == Some(index);
                                    let (mark, style) = if on {
                                        ("\u{203a} ", styles_a.cursor.clone())
                                    } else {
                                        ("  ", styles_a.label.clone())
                                    };
                                    Text::new([mark, label.as_str()].concat())
                                        .style(style)
                                        .into()
                                },
                            ))
                            .at(at, 0),
                        )
                        .child(
                            GridPlacement::new(ListenableBuilder::new(
                                watch_b.listenable(),
                                move |_, _| {
                                    let on = watch_b.cursor() == Some(index);
                                    let style = if on {
                                        styles_b.cursor.clone()
                                    } else {
                                        styles_b.description.clone()
                                    };
                                    Text::new(text.clone()).style(style).into()
                                },
                            ))
                            .at(at, 1),
                        );
                }
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(Flexible::new(
                        1,
                        SingleChildScrollView::new(Padding::new(
                            EdgeInsets::symmetric(indent, 0),
                            grid,
                        ))
                        .controller(scroll.clone()),
                    ))
                    .into()
            }),
        )
        .into()
    }
}
