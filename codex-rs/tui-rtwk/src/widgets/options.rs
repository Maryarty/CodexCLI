//! The option rows of a dialog (the oracle's `list_selection_view`: a number
//! and a label, a description column that wraps, the cursor row marked `›`),
//! one for the confirmation of the agents, the generic selection dialog and
//! the request dialog of approvals. In Flutter it is a
//! `Column(min)[Flexible(SingleChildScrollView(Grid[content, fr]))]` with a
//! cursor in a notifier and the cursor kept in view by `ScrollController`
//! (G39: not a gap; `Scrollable.ensureVisible` has no public counterpart from
//! a handler, so the rows are measured with `TextPainter`'s counterpart,
//! `measure::rows`, and the controller jumps).
//!
//! The mouse (ADR-011) is the widget's own, the controller stays the only
//! holder of the cursor: each row's cells are a `GestureDetector` whose tap
//! moves the cursor there (told to `on_move`), and a tap on the row the
//! cursor is already on, or a double tap (its two taps), accepts it
//! (`on_accept(row)`: the caller turns it into what Enter does at that row).
//! The wheel is the `SingleChildScrollView`'s own.

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::controls::SelectionController;
use rtwk::foundation::ListenableBuilder;
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::SizedBox;
use rtwk::layout::{
    Column, CrossAxisAlignment, EdgeInsets, Flexible, Grid, GridPlacement, GridTrack, MainAxisSize,
    Padding,
};
use rtwk::prelude::HitTestBehavior;
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, segmenter_of};
use rtwk::{AnyWidget, Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use super::measure;

/// A row: the label (`1. name`) and the description beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionRow {
    pub label: String,
    pub description: Option<String>,
}

/// A row whose label hangs: the `prefix` (the number, `1. `, or blanks for a
/// row that has none) stays at the left, the label (spans: an underlined
/// hotkey, a coloured status) wraps to the right of it and its continuation
/// lines start under the label's first cell (the oracle's `wrap_indent`).
#[derive(Clone, Debug, PartialEq)]
pub struct RichRow {
    pub prefix: String,
    pub label: Vec<Span>,
    pub description: Option<String>,
}

/// A row of the stacked menu: the lead (the mark and the number, as the
/// cursor row has it and as the others do), the label and the description
/// under it. A row that is not `enabled` is shown and cannot be tapped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackedRow {
    pub lead_on: String,
    pub lead_off: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
}

/// The look of the stacked menu: the cursor row's lead and label and
/// description, the others' label and description, and the indent of the
/// description line.
#[derive(Clone, Debug)]
pub struct Stacked {
    pub indent: &'static str,
    pub cursor_lead: Style,
    pub cursor_label: Style,
    pub cursor_description: Style,
    pub label: Style,
    pub description: Style,
}

/// What the grid draws: a row of either kind.
struct Item {
    /// `Some`: a hanging label (the mark and the prefix in a column of their
    /// own); `None`: the mark and the label in one text.
    prefix: Option<String>,
    label: Vec<Span>,
    description: Option<String>,
}

impl Item {
    fn text(&self) -> String {
        self.label.iter().map(|span| span.text.as_ref()).collect()
    }
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
    /// Told the row a tap moved the cursor to.
    moved: Option<Handler<usize>>,
    /// Told the row a tap accepted (the cursor was on it) or a double tap.
    accept: Option<Handler<usize>>,
}

/// Two `Options` are the same when they share the controllers and tell the
/// same handlers (what a row's tap widget compares).
impl PartialEq for Options {
    fn eq(&self, other: &Options) -> bool {
        Rc::ptr_eq(&self.heights, &other.heights)
            && self.moved == other.moved
            && self.accept == other.accept
    }
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
            moved: None,
            accept: None,
        }
    }

    /// The same controllers, telling `handler` the row a tap accepts: a tap
    /// on the row the cursor is on, or a double tap. Made by the state that
    /// builds the rows (`cx.map(|row: usize| Msg::Accept(row))`); the cursor
    /// is on the row already when it is told.
    #[must_use]
    pub fn on_accept(&self, handler: Handler<usize>) -> Options {
        Options {
            accept: Some(handler),
            ..self.clone()
        }
    }

    /// The same controllers, telling `handler` the row a tap moved the
    /// cursor to (for a screen that shows something that follows the cursor).
    #[must_use]
    pub fn on_move(&self, handler: Handler<usize>) -> Options {
        Options {
            moved: Some(handler),
            ..self.clone()
        }
    }

    /// The row the cursor is on.
    pub fn cursor(&self) -> usize {
        self.selection.cursor().unwrap_or(0)
    }

    /// No row has the cursor (the oracle's `selected_idx: None`): no row is marked until a key
    /// or a tap sets one. `cursor()` still answers row 0, where a step starts.
    pub fn clear(&self) {
        self.selection.set_cursor(None);
    }

    /// The cursor goes to `row` and the rows scroll to show it. The window
    /// moves by whole rows, as the oracle's list does: the first row shown
    /// goes on one by one until the cursor row fits whole, so a short last
    /// window leaves blank lines under it (the content is padded for it).
    pub fn set(&self, row: usize) {
        self.selection.set_cursor(Some(row));
        let heights = self.heights.borrow();
        if row >= heights.len() || !self.scroll.has_clients() {
            return;
        }
        let position = self.scroll.position();
        let (pixels, view) = (position.pixels(), position.viewport_dimension());
        let mut tops = Vec::with_capacity(heights.len());
        let mut top = 0;
        for height in heights.iter() {
            tops.push(top);
            top += height;
        }
        // The row at the top of the window now.
        let mut first = tops.iter().position(|top| *top >= pixels).unwrap_or(row);
        if row < first {
            first = row;
        } else {
            while first < row && !fits(&heights[first..], row - first, view) {
                first += 1;
            }
        }
        if tops[first] != pixels {
            self.scroll.jump_to(tops[first]);
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
        let items = rows
            .iter()
            .map(|row| Item {
                prefix: None,
                label: vec![Span::new(row.label.clone())],
                description: row.description.clone(),
            })
            .collect();
        self.items_view(Rc::new(items), styles, indent)
    }

    /// The same with hanging, styled labels (`RichRow`).
    pub fn view_rich(
        &self,
        rows: Rc<Vec<RichRow>>,
        styles: OptionStyles,
        indent: i32,
    ) -> AnyWidget {
        let items = rows
            .iter()
            .map(|row| Item {
                prefix: Some(row.prefix.clone()),
                label: row.label.clone(),
                description: row.description.clone(),
            })
            .collect();
        self.items_view(Rc::new(items), styles, indent)
    }

    /// The rows as a menu whose descriptions stand under the labels, a blank
    /// row after each (the oracle's `render_pick_mode`: one paragraph, the
    /// lines wrap where they stand). When the room the menu is given is less
    /// than that, the descriptions and gaps give way and the labels stay (a
    /// `LayoutBuilder` over the room, which the `Column` it stands in gives
    /// after the lines around it). A `Flexible` for that `Column`.
    pub fn view_stacked(&self, rows: Rc<Vec<StackedRow>>, look: Stacked) -> AnyWidget {
        let me = self.clone();
        Flexible::new(
            1,
            LayoutBuilder::new(move |cx, constraints| {
                let seg = segmenter_of(cx);
                let width = usize::try_from(constraints.max_width).unwrap_or(0).max(1);
                let lines = |text: &str| -> i64 {
                    i64::try_from(measure::rows(seg, text, width).len().max(1)).unwrap_or(1)
                };
                let heads: Vec<i64> = rows
                    .iter()
                    .map(|row| lines(&[row.lead_off.as_str(), row.label.as_str()].concat()))
                    .collect();
                let bodies: Vec<i64> = rows
                    .iter()
                    .map(|row| lines(&[look.indent, row.description.as_str()].concat()))
                    .collect();
                let full: i64 = heads.iter().zip(&bodies).map(|(a, b)| a + b + 1).sum();
                let compact = full > i64::from(constraints.max_height);
                *me.heights.borrow_mut() = if compact {
                    heads
                } else {
                    heads.iter().zip(&bodies).map(|(a, b)| a + b + 1).collect()
                };
                let mut column = Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch);
                for (index, row) in rows.iter().enumerate() {
                    let (row, look, watch) = (row.clone(), look.clone(), me.selection.clone());
                    let lines = ListenableBuilder::new(watch.clone().listenable(), move |_, _| {
                        let on = watch.cursor() == Some(index) && row.enabled;
                        let (head, body) = if on {
                            (
                                vec![
                                    Span::styled(row.lead_on.clone(), look.cursor_lead.clone()),
                                    Span::styled(row.label.clone(), look.cursor_label.clone()),
                                ],
                                look.cursor_description.clone(),
                            )
                        } else {
                            (
                                vec![Span::styled(
                                    [row.lead_off.as_str(), row.label.as_str()].concat(),
                                    look.label.clone(),
                                )],
                                look.description.clone(),
                            )
                        };
                        let mut lines = Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .child(Text::rich(head));
                        if !compact {
                            lines = lines.child(Text::rich(vec![Span::styled(
                                [look.indent, row.description.as_str()].concat(),
                                body,
                            )]));
                        }
                        lines.into()
                    });
                    column = column.child(if rows[index].enabled {
                        me.tap(index, lines)
                    } else {
                        lines.into()
                    });
                    if !compact {
                        column = column.child(SizedBox::new(None, Some(1), None));
                    }
                }
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(Flexible::new(
                        1,
                        SingleChildScrollView::new(column).controller(me.scroll.clone()),
                    ))
                    .into()
            }),
        )
        .into()
    }

    /// A row high, `mark` where rows are scrolled out of view above (or
    /// below), in `style`: the oracle's `↑` and `↓` of a picker. It follows
    /// the scroll of the rows.
    pub fn arrow(&self, above: bool, mark: &'static str, style: Style) -> AnyWidget {
        let (scroll, heights) = (self.scroll.clone(), self.heights.clone());
        SizedBox::new(
            None,
            Some(1),
            Some(
                ListenableBuilder::new(Rc::new(scroll.clone()), move |_, _| {
                    let shown = hidden(&scroll, &heights.borrow(), above);
                    Text::new(if shown { mark } else { "" })
                        .style(style.clone())
                        .into()
                })
                .into(),
            ),
        )
        .into()
    }

    /// `child` as the row `index`'s: a tap on it moves the cursor there or
    /// accepts (see the module's note).
    fn tap(&self, index: usize, child: impl Into<AnyWidget>) -> AnyWidget {
        TapRow {
            options: self.clone(),
            index,
            child: child.into(),
        }
        .into()
    }

    fn items_view(&self, items: Rc<Vec<Item>>, styles: OptionStyles, indent: i32) -> AnyWidget {
        let me = self.clone();
        let (selection, scroll, heights) = (
            self.selection.clone(),
            self.scroll.clone(),
            self.heights.clone(),
        );
        Flexible::new(
            1,
            LayoutBuilder::new(move |cx, constraints| {
                let seg = segmenter_of(cx);
                let hang = items.iter().any(|item| item.prefix.is_some());
                // The mark and the prefix: the column of a hanging label.
                let lead = |item: &Item| {
                    seg.width(&format!(
                        "\u{203a} {}",
                        item.prefix.as_deref().unwrap_or("")
                    ))
                };
                let lead_width = if hang {
                    items.iter().map(lead).max().unwrap_or(0)
                } else {
                    0
                };
                let wide = |item: &Item| {
                    if hang {
                        seg.width(&item.text())
                    } else {
                        seg.width(&format!("\u{203a} {}", item.text()))
                    }
                };
                let room = usize::try_from(constraints.max_width - 2 * indent).unwrap_or(0);
                // The label column is the widest label, at most 70% of the
                // room less the gap (the oracle's `compute_desc_col`) when
                // there are descriptions: a longer label wraps inside it
                // (`GridTrack::content().within(0, cap)`, G39b).
                let described = items.iter().any(|item| {
                    item.description
                        .as_deref()
                        .is_some_and(|text| !text.is_empty())
                });
                let cap = if described {
                    (room * 70 / 100).saturating_sub(2)
                } else {
                    room
                }
                .saturating_sub(lead_width)
                .max(1);
                let label_width = items.iter().map(wide).max().unwrap_or(0).min(cap);
                let description_width = room.saturating_sub(lead_width + label_width + 2).max(1);
                *heights.borrow_mut() = items
                    .iter()
                    .map(|item| {
                        let lines = item
                            .description
                            .as_deref()
                            .map_or(1, |text| measure::rows(seg, text, description_width).len());
                        let text = if hang {
                            item.text()
                        } else {
                            ["\u{203a} ", item.text().as_str()].concat()
                        };
                        let label = measure::rows(seg, &text, label_width.max(1)).len();
                        i64::try_from(lines.max(label).max(1)).unwrap_or(1)
                    })
                    .collect();
                // Windows go by whole rows, so the last row may stand at the
                // top: room for it under the content, where it scrolls.
                let total: i64 = heights.borrow().iter().sum();
                let last = heights.borrow().last().copied().unwrap_or(0);
                let max_height = i64::from(constraints.max_height);
                let tail = if total > max_height {
                    i32::try_from((max_height - last).max(0)).unwrap_or(0)
                } else {
                    0
                };
                let cap = i32::try_from(cap).unwrap_or(1);
                let mut grid = if hang {
                    Grid::new()
                        .columns([
                            // Fixed: a text's own width drops the blank the
                            // prefix ends with.
                            GridTrack::fixed(i32::try_from(lead_width).unwrap_or(0)),
                            GridTrack::content().within(0, cap),
                            GridTrack::fr(1),
                        ])
                        .column_gap(0)
                } else {
                    // Rows without descriptions give the labels the whole
                    // room: no gap before an empty column.
                    Grid::new()
                        .columns([GridTrack::content().within(0, cap), GridTrack::fr(1)])
                        .column_gap(if described { 2 } else { 0 })
                };
                for (index, item) in items.iter().enumerate() {
                    let at = i32::try_from(index).unwrap_or(0);
                    let (lead_text, label, text) = (
                        item.prefix.clone().unwrap_or_default(),
                        item.label.clone(),
                        item.description.clone().unwrap_or_default(),
                    );
                    let (watch, st) = (selection.clone(), styles.clone());
                    let mark = move |on: bool| if on { "\u{203a} " } else { "  " };
                    let hung = hang;
                    // The mark (and the prefix of a hanging label) or the
                    // whole label of a plain row.
                    let first = {
                        let (watch, st, label, lead_text) =
                            (watch.clone(), st.clone(), label.clone(), lead_text.clone());
                        ListenableBuilder::new(watch.clone().listenable(), move |_, _| {
                            let on = watch.cursor() == Some(index);
                            let style = if on {
                                st.cursor.clone()
                            } else {
                                st.label.clone()
                            };
                            let mut spans =
                                vec![Span::new([mark(on), lead_text.as_str()].concat())];
                            if !hung {
                                spans.extend(label.iter().cloned());
                            }
                            Text::rich(spans).style(style).into()
                        })
                    };
                    let column = if hang { 1 } else { 0 };
                    grid = grid.child(GridPlacement::new(me.tap(index, first)).at(at, 0));
                    if hang {
                        let (watch, st, label) = (watch.clone(), st.clone(), label.clone());
                        let label =
                            ListenableBuilder::new(watch.clone().listenable(), move |_, _| {
                                let style = if watch.cursor() == Some(index) {
                                    st.cursor.clone()
                                } else {
                                    st.label.clone()
                                };
                                Text::rich(label.clone()).style(style).into()
                            });
                        grid = grid.child(GridPlacement::new(me.tap(index, label)).at(at, 1));
                    }
                    let (watch, st) = (watch, st);
                    let description: AnyWidget =
                        ListenableBuilder::new(watch.clone().listenable(), move |_, _| {
                            let style = if watch.cursor() == Some(index) {
                                st.cursor.clone()
                            } else {
                                st.description.clone()
                            };
                            Text::new(text.clone()).style(style).into()
                        })
                        .into();
                    // After the label column, `2` cells off it.
                    let description = if hang && described {
                        Padding::new(EdgeInsets::only(2, 0, 0, 0), description).into()
                    } else {
                        description
                    };
                    let description = me.tap(index, description);
                    grid = grid.child(GridPlacement::new(description).at(at, column + 1));
                }
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(Flexible::new(
                        1,
                        SingleChildScrollView::new(Padding::new(
                            EdgeInsets::only(indent, 0, indent, tail),
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

/// Whether the row `at` of `heights` (the first of them the top of the
/// window) is whole in a window `view` lines high (the oracle's
/// `is_selected_visible_in_wrapped_viewport`).
fn fits(heights: &[i64], at: usize, view: i64) -> bool {
    let mut used = 0;
    for (index, height) in heights.iter().enumerate() {
        if used > 0 && used + height > view {
            return false;
        }
        if index == at {
            return true;
        }
        used += height;
        if used >= view {
            return false;
        }
    }
    false
}

/// Whether rows are scrolled out of view above (or below).
fn hidden(scroll: &ScrollController, heights: &[i64], above: bool) -> bool {
    let Ok(position) = scroll.try_position() else {
        return false;
    };
    if above {
        position.pixels() > 0
    } else {
        position.pixels() + position.viewport_dimension() < heights.iter().sum::<i64>()
    }
}

/// A row's cell under a tap: the first tap of a row moves the cursor to it
/// (`GestureDetector::on_tap`), a tap on the row the cursor is on accepts it,
/// so a double tap (two taps) selects and accepts.
#[derive(Clone, Debug, PartialEq)]
struct TapRow {
    options: Options,
    index: usize,
    child: AnyWidget,
}

impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("cursor", &self.cursor())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tap {
    On,
}

struct TapRowState;

impl StatefulWidget for TapRow {
    type State = TapRowState;

    fn create_state(&self) -> TapRowState {
        TapRowState
    }
}

impl_widget!(stateful eq TapRow);

impl State for TapRowState {
    type Widget = TapRow;
    type Msg = Tap;

    fn update(&mut self, Tap::On: Tap, w: &TapRow, cx: &mut Cx<'_, Self>) -> Rebuild {
        let options = &w.options;
        if options.cursor() == w.index {
            if let Some(accept) = &options.accept {
                cx.emit(accept, w.index);
            }
            return Rebuild::No;
        }
        options.set(w.index);
        if let Some(moved) = &options.moved {
            cx.emit(moved, w.index);
        }
        Rebuild::No
    }

    fn build(&self, w: &TapRow, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let tap: Handler<PointerEvent> = cx.map(|_: PointerEvent| Tap::On);
        GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .on_tap(tap)
            .child(w.child.clone())
            .into()
    }
}
