//! The list of saved sessions shared by the agents command center and the
//! resume picker: a `SelectionList` whose rows know whether the cursor is on
//! them (the item builder holds the controller and the row sits in a
//! `ListenableBuilder` on it, as a Flutter `ListView.itemBuilder` row reads
//! `controller.isSelected`), and a `NotificationListener` that tells where the
//! list is scrolled to. Each page keeps only its row and where it draws the
//! overflow marks.

use std::rc::Rc;

use rtwk::NotificationListener;
use rtwk::controls::{SelectionController, SelectionList};
use rtwk::focus::FocusNode;
use rtwk::foundation::{ListenableBuilder, ValueNotifier};
use rtwk::scroll::ScrollNotification;
use rtwk::style::Style;
use rtwk::{AnyWidget, Handler};

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

/// What a page tells the list.
pub struct SessionList<K> {
    pub len: usize,
    pub key: Rc<dyn Fn(usize) -> K>,
    /// Which rows the cursor may rest on (all, when `None`).
    pub selectable: Option<Rc<dyn Fn(usize) -> bool>>,
    /// The row at an index, told whether the cursor is on it.
    pub row: Rc<dyn Fn(usize, bool) -> AnyWidget>,
    pub selection: SelectionController<K>,
    pub scroll: Rc<ValueNotifier<Scrolled>>,
    pub submit: Handler<K>,
    pub focus: Option<FocusNode>,
    /// The rows a row takes, when every row takes the same.
    pub extent: Option<i32>,
    /// The style of a row at rest; the list paints no cursor of its own.
    pub idle: Style,
}

impl<K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static> SessionList<K> {
    pub fn build(self) -> AnyWidget {
        let SessionList {
            len,
            key,
            selectable,
            row,
            selection,
            scroll,
            submit,
            focus,
            extent,
            idle,
        } = self;
        let builder = {
            let (key, selectable, selection) = (key.clone(), selectable.clone(), selection.clone());
            move |index: usize| -> AnyWidget {
                if selectable
                    .as_ref()
                    .is_some_and(|selectable| !selectable(index))
                {
                    return row(index, false);
                }
                let (row, selection, at) = (row.clone(), selection.clone(), key(index));
                ListenableBuilder::new(selection.listenable(), move |_, _| {
                    row(index, selection.cursor().as_ref() == Some(&at))
                })
                .into()
            }
        };
        let mut list = SelectionList::new(len, move |index| key(index), builder)
            .controller(selection)
            .on_submitted(submit)
            .autofocus(true)
            .bar(false)
            .idle_style(idle)
            .selected_style(Style::new())
            .disabled_style(Style::new())
            .cursor_style(Style::new());
        if let Some(selectable) = selectable {
            list = list.item_enabled_rc(selectable);
        }
        if let Some(node) = focus {
            list = list.focus_node(node);
        }
        if let Some(rows) = extent {
            list = list.item_extent(rows);
        }
        NotificationListener::<ScrollNotification>::new(list)
            .on_notification(move |note| {
                scroll.set_value(Scrolled {
                    pixels: note.metrics.pixels,
                    max: note.metrics.max_scroll_extent,
                });
                false
            })
            .into()
    }
}
