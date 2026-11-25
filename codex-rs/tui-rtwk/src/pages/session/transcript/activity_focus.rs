//! The focus on a fold of the transcript (`focus_activity`, F4; oracle `transcript_view/disclosure.rs`
//! `handle_disclosure_key`): the reader walks the disclosure rows of the transcript with the
//! keys of a list (`↑ ↓`, a page, the ends), `Enter` opens or closes the one in focus, `←` closes
//! and `→` opens it, `Esc` gives the keys back; any other key does that too and goes on to the
//! composer. A stop of the walk is a fold of the turn details or a cell that discloses its
//! details (a shell call, a patch: `+ Show details`), in the order of the rows.
//!
//! What the reader opened is this controller's, not the list state's (Flutter: the
//! `ExpansionTileController` of the transcript, held for the thread like its scroll controller):
//! the list reads `is_open` as it plans its rows, and a tap on a fold, `Enter` and `→` all come
//! through `toggle`, which also puts the focus on the fold, as a click on a control does; a cell
//! opens and closes through the thread's `Disclosed` (`toggle_cell`), which Find shares. The list
//! tells which stops its rows have (`set_folds`, oldest first) and scrolls to the one the
//! controller asks to be shown (`take_reveal`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::focus::KeyPress;
use rtwk::foundation::{Listenable, ValueNotifier};

use codex_tui_core::keymap::actions::{
    GLOBAL_FOCUS_ACTIVITY, LIST_ACCEPT, LIST_CANCEL, LIST_JUMP_BOTTOM, LIST_JUMP_TOP,
    LIST_MOVE_DOWN, LIST_MOVE_LEFT, LIST_MOVE_RIGHT, LIST_MOVE_UP, LIST_PAGE_DOWN, LIST_PAGE_UP,
};
use codex_tui_core::keymap::{ActionId, Keymap};
use codex_tui_core::threads::ItemKey;

use super::strings::Strings;
use super::turn_details::Unfolded;
use crate::keymap::{names, pressed};
use crate::widgets::finder::Finder;
use crate::widgets::thread_ui::Disclosed;

/// How many folds a page moves (oracle: `steps` of `PageUp`/`PageDown`).
const PAGE: usize = 5;

/// What a key did while a fold has the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The key is the mode's: nothing goes on to the composer.
    Taken,
    /// The key is none of the mode's: the focus is let go and the key goes on.
    Left,
}

/// A stop of the walk: a fold of the turn details (the ids of the items it hides) or a cell that
/// discloses its details (the key of its first item).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stop {
    pub ids: Vec<ItemKey>,
    pub cell: bool,
}

impl Stop {
    /// The fold of `ids`.
    pub fn fold(ids: Vec<ItemKey>) -> Stop {
        Stop { ids, cell: false }
    }

    /// The cell of `key`.
    pub fn cell(key: ItemKey) -> Stop {
        Stop {
            ids: vec![key],
            cell: true,
        }
    }

    fn overlaps(&self, ids: &[ItemKey]) -> bool {
        ids.iter().any(|id| self.ids.contains(id))
    }
}

/// The folds and the cells that disclose in the thread's transcript: which are open, which has
/// the focus.
#[derive(Clone)]
pub struct Activity(Rc<Shared>);

struct Shared {
    /// The stop in focus.
    focused: RefCell<Option<Stop>>,
    /// The stops the list has rows for, oldest first.
    folds: RefCell<Vec<Stop>>,
    /// The cells that disclose and which of them are open (Find opens one for its match).
    disclosed: Disclosed,
    open: RefCell<Unfolded>,
    /// The revision of what is open, for the list's cache of its rows.
    revision: Cell<u64>,
    /// The fold in focus is to be scrolled to.
    reveal: Cell<bool>,
    ticks: Rc<ValueNotifier<u32>>,
}

impl PartialEq for Activity {
    fn eq(&self, other: &Activity) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The activity of the thread whose transcript `finder` serves, made on the first ask.
pub fn of(finder: &Finder) -> Activity {
    let disclosed = finder.held(Disclosed::default);
    finder.held(move || Activity::new(disclosed))
}

impl Activity {
    /// The activity over the cells `disclosed` holds: a cell opening or closing is heard by whoever
    /// listens here.
    pub fn new(disclosed: Disclosed) -> Activity {
        let ticks = Rc::new(ValueNotifier::new(0u32));
        let heard = ticks.clone();
        disclosed.listenable().add_listener(Rc::new(move || {
            heard.set_value(heard.value().wrapping_add(1));
        }));
        Activity(Rc::new(Shared {
            focused: RefCell::default(),
            folds: RefCell::default(),
            disclosed,
            open: RefCell::default(),
            revision: Cell::new(0),
            reveal: Cell::new(false),
            ticks,
        }))
    }

    /// The cells that disclose, shared with Find and the tap on a control.
    pub fn disclosed(&self) -> Disclosed {
        self.0.disclosed.clone()
    }

    /// Ticks when the focus or what is open changes.
    pub fn listenable(&self) -> Rc<ValueNotifier<u32>> {
        self.0.ticks.clone()
    }

    fn changed(&self) {
        self.0.ticks.set_value(self.0.ticks.value().wrapping_add(1));
    }

    /// Whether a fold is in focus.
    pub fn focused(&self) -> bool {
        self.0.focused.borrow().is_some()
    }

    /// Whether the stop of `ids` (a fold's, or a cell's key) is the one in focus.
    pub fn is_focused(&self, ids: &[ItemKey]) -> bool {
        self.0
            .focused
            .borrow()
            .as_ref()
            .is_some_and(|focused| focused.overlaps(ids))
    }

    /// Whether `stop` is open: the reader opened it, or Find shows its match in the cell.
    fn is_stop_open(&self, stop: &Stop) -> bool {
        if stop.cell {
            stop.ids
                .first()
                .is_some_and(|key| self.0.disclosed.is_open(key))
        } else {
            self.is_open(&stop.ids)
        }
    }

    /// Whether the fold of `ids` is open.
    pub fn is_open(&self, ids: &[ItemKey]) -> bool {
        self.0.open.borrow().is_open(ids)
    }

    /// The folds that are open, as the plan of the rows reads them.
    pub fn open(&self) -> Unfolded {
        self.0.open.borrow().clone()
    }

    /// Changes whenever a fold opens or closes.
    pub fn revision(&self) -> u64 {
        self.0.revision.get()
    }

    /// The stops the list has rows for, oldest first.
    pub fn set_folds(&self, folds: Vec<Stop>) {
        *self.0.folds.borrow_mut() = folds;
    }

    /// Opens a closed fold and closes an open one, and puts the focus on it (`toggle_activity`:
    /// a tap on the control and `Enter` both do).
    pub fn toggle(&self, ids: &[ItemKey]) {
        self.toggle_stop(&Stop::fold(ids.to_vec()));
    }

    /// The same for the cell of `key`: a tap on its control, or `Enter` while it has the focus.
    pub fn toggle_cell(&self, key: &ItemKey) {
        self.toggle_stop(&Stop::cell(key.clone()));
    }

    fn toggle_stop(&self, stop: &Stop) {
        if stop.ids.is_empty() {
            return;
        }
        if let (true, Some(key)) = (stop.cell, stop.ids.first()) {
            self.0.disclosed.toggle(key);
        } else {
            self.0.open.borrow_mut().toggle(&stop.ids);
            self.0.revision.set(self.0.revision.get() + 1);
        }
        *self.0.focused.borrow_mut() = Some(stop.clone());
        self.changed();
    }

    /// The focus key: a stop in focus is let go, else the last stop has the focus
    /// (`handle_disclosure_key`; the oracle takes the last one in view first, which the list does
    /// not tell: `ListController` has no visible range). Whether the focus began.
    pub fn begin(&self) -> bool {
        if self.focused() {
            self.leave();
            return false;
        }
        let last = self.0.folds.borrow().last().cloned();
        let Some(last) = last else {
            return false;
        };
        *self.0.focused.borrow_mut() = Some(last);
        self.0.reveal.set(true);
        self.changed();
        true
    }

    /// Lets go of the focus.
    pub fn leave(&self) {
        let had = self.0.focused.borrow_mut().take().is_some();
        if had {
            self.changed();
        }
    }

    /// The ids of the fold to scroll to, once.
    pub fn take_reveal(&self) -> Option<Vec<ItemKey>> {
        if !self.0.reveal.replace(false) {
            return None;
        }
        self.0
            .focused
            .borrow()
            .as_ref()
            .map(|stop| stop.ids.clone())
    }

    /// Where the stop in focus is among the stops; none lets go of the focus (one that went with
    /// its rows).
    fn at(&self) -> Option<usize> {
        let focused = self.0.focused.borrow().clone()?;
        let at = self
            .0
            .folds
            .borrow()
            .iter()
            .position(|stop| focused.overlaps(&stop.ids));
        if at.is_none() {
            self.leave();
        }
        at
    }

    fn focus_at(&self, at: usize) {
        let stop = self.0.folds.borrow().get(at).cloned();
        if let Some(stop) = stop {
            *self.0.focused.borrow_mut() = Some(stop);
            self.0.reveal.set(true);
            self.changed();
        }
    }

    /// What the key does while a fold has the focus: none while none has it, or for the focus
    /// key itself (its action lets go), or for a fold that is gone.
    pub fn key(&self, keymap: &Keymap, press: &KeyPress) -> Option<Step> {
        if !self.focused() || pressed(keymap, GLOBAL_FOCUS_ACTIVITY, press) {
            return None;
        }
        let at = self.at()?;
        let is = |id: ActionId| pressed(keymap, id, press);
        let last = self.0.folds.borrow().len().saturating_sub(1);
        let stop = self.0.folds.borrow().get(at).cloned()?;
        if is(LIST_CANCEL) {
            self.leave();
        } else if is(LIST_ACCEPT) {
            self.toggle_stop(&stop);
        } else if is(LIST_MOVE_LEFT) || is(LIST_MOVE_RIGHT) {
            // the key that does what the stop is not yet: right opens, left closes
            if self.is_stop_open(&stop) != is(LIST_MOVE_RIGHT) {
                self.toggle_stop(&stop);
            }
        } else if is(LIST_MOVE_UP) {
            self.focus_at(at.saturating_sub(1));
        } else if is(LIST_PAGE_UP) {
            self.focus_at(at.saturating_sub(PAGE));
        } else if is(LIST_JUMP_TOP) {
            self.focus_at(0);
        } else if is(LIST_MOVE_DOWN) {
            self.focus_at((at + 1).min(last));
        } else if is(LIST_PAGE_DOWN) {
            self.focus_at((at + PAGE).min(last));
        } else if is(LIST_JUMP_BOTTOM) {
            self.focus_at(last);
        } else {
            // ordinary text resumes composing: navigation never becomes a text editor
            self.leave();
            return Some(Step::Left);
        }
        Some(Step::Taken)
    }

    /// The hints of the footer while a fold has the focus (`disclosure_footer`): the key of each
    /// as the keymap has it, and what it does; the toggle says what the fold in focus would do.
    /// None while none has the focus.
    pub fn hints(
        &self,
        keymap: &Keymap,
        table: &'static Strings,
    ) -> Option<Vec<(String, &'static str)>> {
        let focused = self.0.focused.borrow().clone()?;
        let toggle = if self.is_stop_open(&focused) {
            table.activity_collapse
        } else {
            table.activity_expand
        };
        Some(
            [
                (LIST_MOVE_UP, table.activity_previous),
                (LIST_MOVE_DOWN, table.activity_next),
                (LIST_ACCEPT, toggle),
                (LIST_CANCEL, table.activity_back),
            ]
            .into_iter()
            .filter_map(|(id, label)| Some((spelled(names(keymap, id).first()?), label)))
            .collect(),
        )
    }
}

/// A key as a hint spells it: arrows as arrows, pages as `pgup`.
fn spelled(name: &str) -> String {
    match name {
        "up" => "↑".to_owned(),
        "down" => "↓".to_owned(),
        "left" => "←".to_owned(),
        "right" => "→".to_owned(),
        "escape" => "esc".to_owned(),
        "pageup" => "pgup".to_owned(),
        "pagedown" => "pgdn".to_owned(),
        name => name.to_owned(),
    }
}
