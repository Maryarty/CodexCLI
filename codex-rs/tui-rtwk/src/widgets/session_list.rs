//! The list of saved sessions shared by the agents command center and the
//! resume picker, as Flutter writes a list with a cursor of its own: a
//! `ListView.builder` with a `ListController` (scroll to an index), a
//! `SelectionController` (the cursor, a `ValueNotifier`), a row in a
//! `ListenableBuilder` on it (the row reads `controller.isSelected`) and a
//! `NotificationListener` that tells where the list is scrolled to (G35: not
//! a gap, the keys are the page's `Shortcuts`/`Actions`, here `Move` and
//! `target`). Each page keeps its row, its keys and where it draws the
//! overflow marks.

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::Handler;
use rtwk::NotificationListener;
use rtwk::controls::Pressable;
use rtwk::controls::SelectionController;
use rtwk::focus::FocusNode;
use rtwk::foundation::{ListenableBuilder, ValueNotifier};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::scroll::{ListController, ListView, ScrollNotification};

/// Where the list is scrolled to: the pixels above the window and the
/// extent that can be scrolled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scrolled {
    pub pixels: i64,
    pub max: i64,
}

impl Scrolled {
    /// Whether rows lie above the window.
    pub fn above(&self) -> bool {
        self.pixels > 0
    }

    /// Whether rows lie below the window.
    pub fn below(&self) -> bool {
        self.pixels < self.max
    }
}

/// A move of the cursor (the keys of the oracle's list keymap).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// The next row the cursor may rest on, round the ends.
    Step(bool),
    /// A window of rows further, stopping at the end.
    Page(bool),
    /// The first or last row the cursor may rest on.
    Edge(bool),
}

/// Where the cursor goes: from the row `from` over `len` rows of which
/// `selectable` ones take it, `rows` being what a page shows.
pub fn target(
    len: usize,
    from: Option<usize>,
    selectable: &dyn Fn(usize) -> bool,
    mv: Move,
    rows: usize,
) -> Option<usize> {
    let ok: Vec<usize> = (0..len).filter(|at| selectable(*at)).collect();
    let (first, last) = (*ok.first()?, *ok.last()?);
    let at = |forward: bool, wrap: bool| match (from, forward) {
        (None, true) => Some(first),
        (None, false) => Some(last),
        (Some(from), true) => ok
            .iter()
            .copied()
            .find(|at| *at > from)
            .or(wrap.then_some(first)),
        (Some(from), false) => ok
            .iter()
            .rev()
            .copied()
            .find(|at| *at < from)
            .or(wrap.then_some(last)),
    };
    match mv {
        Move::Step(forward) => at(forward, true),
        Move::Edge(end) => Some(if end { last } else { first }),
        Move::Page(forward) => {
            let from = from.unwrap_or(if forward { first } else { last });
            let rows = rows.max(1);
            let want = if forward {
                (from + rows).min(len - 1)
            } else {
                from.saturating_sub(rows)
            };
            let (before, after) = (
                ok.iter().rev().copied().find(|at| *at <= want),
                ok.iter().copied().find(|at| *at >= want),
            );
            if forward {
                after.or(before)
            } else {
                before.or(after)
            }
        }
    }
}

/// What a page tells the list.
pub struct SessionList<K> {
    pub len: usize,
    pub key: Rc<dyn Fn(usize) -> K>,
    /// Which rows the cursor may rest on (all, when `None`).
    pub selectable: Option<Rc<dyn Fn(usize) -> bool>>,
    /// The row at an index, told whether the cursor is on it.
    pub row: Rc<dyn Fn(usize, bool) -> AnyWidget>,
    pub selection: SelectionController<K>,
    pub list: ListController,
    pub scroll: Rc<ValueNotifier<Scrolled>>,
    pub focus: Option<FocusNode>,
    /// The rows a row takes, when every row takes the same.
    pub extent: Option<i32>,
    /// A tap on the row at an index (the page's handler: a tap on a row the
    /// cursor is not on moves it there, a tap on the row it is on opens it;
    /// the wheel is the `ListView`'s own). Made by the page, not in `build`.
    pub taps: Option<Rc<Vec<Handler<PointerEvent>>>>,
}

impl<K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static> SessionList<K> {
    pub fn build(self) -> AnyWidget {
        let SessionList {
            len,
            key,
            selectable,
            row,
            selection,
            list,
            scroll,
            focus,
            extent,
            taps,
        } = self;
        let builder = move |index: usize| -> AnyWidget {
            if selectable
                .as_ref()
                .is_some_and(|selectable| !selectable(index))
            {
                return row(index, false);
            }
            let (row, selection, at) = (row.clone(), selection.clone(), key(index));
            let tap = taps.as_ref().and_then(|taps| taps.get(index)).cloned();
            ListenableBuilder::new(selection.listenable(), move |_, _| {
                let row = row(index, selection.cursor().as_ref() == Some(&at));
                match &tap {
                    Some(tap) => GestureDetector::new().on_tap(tap.clone()).child(row).into(),
                    None => row,
                }
            })
            .into()
        };
        let mut view = ListView::builder(len, builder).controller(list);
        if let Some(rows) = extent {
            view = view.item_extent(rows);
        }
        let scrolled =
            NotificationListener::<ScrollNotification>::new(view).on_notification(move |note| {
                scroll.set_value(Scrolled {
                    pixels: note.metrics.pixels,
                    max: note.metrics.max_scroll_extent,
                });
                false
            });
        let scrolled: AnyWidget = scrolled.into();
        // The list is a focus stop (a click on a row focuses it) whose keys
        // are the page's: they bubble to its `KeyBindings`.
        Pressable::new(move |_, _| scrolled.clone())
            .focus_node(focus)
            .autofocus(true)
            .into()
    }
}

/// The rows a page of the list shows (one, before the list is laid out).
pub fn page_rows(list: &ListController) -> usize {
    if list.has_clients() {
        usize::try_from(list.position().viewport_dimension()).unwrap_or(1)
    } else {
        1
    }
}
