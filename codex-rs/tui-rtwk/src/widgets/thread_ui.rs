//! Per-thread UI controllers (R2, R14): the composer's text controller of
//! each thread and its draft's elements (`widgets::composer_draft`), and the
//! reader of its transcript (`Follow`). They are UI state, never seen by
//! `apply`; `ingest` creates a thread's controllers after the thread appears.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use rtwk::editing::TextEditingController;
use rtwk::foundation::{Listenable, ValueNotifier};
use rtwk::gestures::SelectionAreaController;
use rtwk::scroll::ListController;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::kit::backtrack::{Backtrack, Chosen as Selection, EscStep, Targets};

use crate::widgets::composer_draft::ComposerDraft;
use crate::widgets::finder::Finder;
use crate::widgets::composer_sparkle::SparkleController;

/// The controllers of one thread.
#[derive(Clone, Default, PartialEq)]
pub struct ThreadUi {
    pub composer: TextEditingController,
    /// The composer's paste placeholders and kill ring, kept in step with
    /// `composer`'s text.
    pub draft: ComposerDraft,
    /// Where the reader of the transcript is, and what it has not seen.
    pub follow: Follow,
    /// The modes of the transcript sheet (browsing prompts).
    pub modes: Modes,
    /// The composer's sparkle: what the draft did to it, what shows.
    pub sparkle: SparkleController,
    /// What the reader has selected in the transcript (Flutter's `SelectableRegion` controller):
    /// the transcript's selection area is served by it, and the page's keys (copy, Esc) and a
    /// right click read it from outside the area.
    pub selection: TextSelection,
    /// Find over the transcript (F3): its query, its match and the position it came from. Its
    /// mode is the truth of `Input::Find`.
    pub find: Finder,
}

impl ThreadUi {
    /// How the transcript takes keys now: Find while it is open (editing the query or reading a
    /// match), else the mode the reader holds (`Modes::input`).
    pub fn input(&self) -> Input {
        if self.find.active() {
            Input::Find
        } else {
            self.modes.input()
        }
    }
}

/// The controller of the transcript's selection, shared by the area it serves, the keys of the
/// page and the footer that hints at them; equal to its own clones. It lives as long as the
/// thread's controllers do (its listener holds it), so the footer can listen to whether
/// something is selected: Flutter's `ValueNotifier<bool>` over `SelectableRegion`'s controller.
#[derive(Clone)]
pub struct TextSelection(Rc<Selected>);

struct Selected {
    controller: SelectionAreaController,
    held: Rc<ValueNotifier<bool>>,
}

impl Default for TextSelection {
    fn default() -> TextSelection {
        let controller = SelectionAreaController::new();
        let held = Rc::new(ValueNotifier::new(false));
        let (watched, notifier) = (controller.clone(), held.clone());
        controller.add_listener(Rc::new(move || {
            notifier.set_value(watched.has_selection());
        }));
        TextSelection(Rc::new(Selected { controller, held }))
    }
}

impl TextSelection {
    /// The controller the selection area is given.
    pub fn controller(&self) -> SelectionAreaController {
        self.0.controller.clone()
    }

    /// Whether something is selected.
    pub fn held(&self) -> bool {
        self.0.held.value()
    }

    /// Notifies when something comes to be selected and when nothing is any more.
    pub fn listenable(&self) -> Rc<dyn Listenable> {
        self.0.held.clone()
    }
}

impl PartialEq for TextSelection {
    fn eq(&self, other: &TextSelection) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The mode of one thread's transcript sheet, as Flutter's
/// `ValueNotifier<Mode>` a controller owns: whether the detailed transcript
/// is open (Ctrl+T), the backtrack machine of core (primed by the first Esc,
/// which prompt is highlighted; never thread data) and a notifier that ticks
/// when either moves. Whoever draws what depends on the mode listens to
/// `listenable()`; the keys that move it call the methods.
#[derive(Clone)]
pub struct Modes {
    machine: Rc<RefCell<Backtrack>>,
    /// The mode of input the reader holds the transcript in (select, copy).
    input: Rc<Cell<Input>>,
    /// The transcript shows its details (Ctrl+T).
    detailed: Rc<Cell<bool>>,
    /// Whether it showed them when browsing began: browsing shows the plain
    /// transcript, and ending it puts the presentation back
    /// (`remember_browsing_origin`, `set_browsing_details`).
    origin: Rc<Cell<Option<bool>>>,
    ticks: Rc<ValueNotifier<u32>>,
}

/// How the transcript takes keys over the chat's: the plain chat, the cursor that selects text
/// (oracle `select_mode`), the picker that copies a block (`copy_mode`), Find (F3: the query's
/// field, then the match being read; `ThreadUi::input` says it while the finder is open). The
/// state each mode keeps is its own (its page or sheet, the finder); this is only which one is
/// held, so that the keys, the footer and the area that selects read the same answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Input {
    #[default]
    Plain,
    Select,
    Copy,
    Find,
}

impl Default for Modes {
    fn default() -> Self {
        Modes {
            machine: Rc::default(),
            input: Rc::default(),
            detailed: Rc::default(),
            origin: Rc::default(),
            ticks: Rc::new(ValueNotifier::new(0)),
        }
    }
}

impl PartialEq for Modes {
    fn eq(&self, other: &Modes) -> bool {
        Rc::ptr_eq(&self.machine, &other.machine)
    }
}

impl Modes {
    fn moved<R>(&self, f: impl FnOnce(&mut Backtrack) -> R) -> R {
        let result = f(&mut self.machine.borrow_mut());
        self.ticks.set_value(self.ticks.value().wrapping_add(1));
        result
    }

    /// How the transcript takes keys now.
    pub fn input(&self) -> Input {
        self.input.get()
    }

    /// The transcript takes keys as `input` says; leaving a mode is `Input::Plain`.
    pub fn hold(&self, input: Input) {
        if self.input.get() != input {
            self.moved(|_| self.input.set(input));
        }
    }

    /// The first Esc has been pressed: the next starts browsing.
    pub fn primed(&self) -> bool {
        self.machine.borrow().is_primed()
    }

    /// The transcript shows its details.
    pub fn detailed(&self) -> bool {
        self.detailed.get()
    }

    /// Ctrl+T: the details show, or hide. While prompts are browsed it only
    /// changes what the browsed transcript shows (`handle_backtrack_preview_key`).
    pub fn toggle_detailed(&self) {
        if self.browsing() {
            self.moved(|_| self.detailed.set(!self.detailed()));
        } else if self.detailed() {
            self.close();
        } else {
            self.open();
        }
    }

    /// Shows the details (F3 and Ctrl+T), whatever mode the chat was in.
    pub fn open(&self) {
        self.moved(|machine| {
            machine.reset();
            self.origin.set(None);
            self.detailed.set(true);
        });
    }

    /// Back to the chat: no mode, no details (`close_transcript_overlay`).
    pub fn close(&self) {
        self.moved(|machine| {
            machine.reset();
            self.origin.set(None);
            self.input.set(Input::Plain);
            self.detailed.set(false);
        });
    }

    /// Esc in the chat (`handle_backtrack_esc_key`): the first primes, the
    /// second begins browsing the prompts, the newest marked.
    pub fn chat_esc(
        &self,
        thread: Option<&ThreadId>,
        composer_empty: bool,
        targets: Targets,
    ) -> EscStep {
        self.moved(|machine| {
            let step = machine.esc(thread, composer_empty, targets);
            if matches!(step, EscStep::Browse { .. }) {
                self.begin_browsing();
            }
            step
        })
    }

    /// Browsing shows the plain transcript, and remembers how it looked.
    fn begin_browsing(&self) {
        self.origin.set(Some(self.detailed()));
        self.detailed.set(false);
    }

    /// Any other input between the two Escs ends the pair
    /// (`cancel_primed_browsing_for_event`).
    pub fn interrupt(&self) {
        if self.primed() {
            self.moved(Backtrack::interrupt);
        }
    }

    /// Prompts are being browsed.
    pub fn browsing(&self) -> bool {
        matches!(*self.machine.borrow(), Backtrack::Browsing { .. })
    }

    /// The prompt highlighted, the nth from the oldest.
    pub fn selected(&self) -> Option<usize> {
        self.machine.borrow().selected()
    }

    /// Esc on the detailed transcript with an empty composer: browsing
    /// starts (`begin_overlay_backtrack_preview`).
    pub fn esc(&self, thread: Option<&ThreadId>, targets: Targets) -> EscStep {
        self.moved(|machine| {
            if matches!(machine, Backtrack::Browsing { .. }) {
                machine.reset();
                self.end_browsing();
                EscStep::Cancelled
            } else {
                let step = machine.begin_from_detailed(thread, targets);
                if matches!(step, EscStep::Browse { .. }) {
                    self.begin_browsing();
                }
                step
            }
        })
    }

    /// Esc while browsing, or a key that is the composer's: browsing ends and
    /// the transcript is as it was (`cancel_transcript_browsing`).
    pub fn leave_browsing(&self) {
        if self.browsing() {
            self.moved(|machine| {
                machine.reset();
                self.end_browsing();
            });
        }
    }

    fn end_browsing(&self) {
        if let Some(detailed) = self.origin.take() {
            self.detailed.set(detailed);
        }
    }

    /// Left: the next older prompt.
    pub fn older(&self, count: usize) -> Option<usize> {
        self.moved(|machine| machine.older(count))
    }

    /// Right: the next newer prompt.
    pub fn newer(&self, count: usize) -> Option<usize> {
        self.moved(|machine| machine.newer(count))
    }

    /// Enter: the prompt chosen, when there is one.
    pub fn confirm(&self, thread: Option<&ThreadId>, count: usize) -> Option<Selection> {
        self.moved(|machine| machine.confirm(thread, count))
    }

    /// Back to no mode (the sheet closed).
    pub fn reset(&self) {
        self.origin.set(None);
        self.moved(Backtrack::reset);
    }

    /// Notifies when the mode moves.
    pub fn listenable(&self) -> Rc<ValueNotifier<u32>> {
        self.ticks.clone()
    }
}

/// The reader of one thread's transcript (oracle `transcript_view`'s
/// `position`, `tail_visible` and `unseen_activity`): the list's scroll
/// controller, which says whether the last row is in view, and whether the
/// transcript changed at its end while it was not. Flutter's
/// `ScrollController` plus a `ValueNotifier<bool>`; whoever draws what depends
/// on them listens to `listenable()`.
#[derive(Clone)]
pub struct Follow {
    scroll: ListController,
    unseen: Rc<ValueNotifier<bool>>,
}

impl Default for Follow {
    fn default() -> Self {
        Follow {
            scroll: ListController::new(),
            unseen: Rc::new(ValueNotifier::new(false)),
        }
    }
}

impl PartialEq for Follow {
    fn eq(&self, other: &Follow) -> bool {
        self.scroll == other.scroll && Rc::ptr_eq(&self.unseen, &other.unseen)
    }
}

impl std::fmt::Debug for Follow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Follow")
            .field("away", &self.away())
            .field("unseen", &self.unseen())
            .finish()
    }
}

impl Follow {
    /// The controller of the transcript's list.
    pub fn scroll(&self) -> ListController {
        self.scroll.clone()
    }

    /// The last row is in view (a list not laid out yet, or with no row of
    /// room, follows: `area.is_empty()` in the oracle's `follow_control_label`).
    pub fn at_tail(&self) -> bool {
        let Ok(position) = self.scroll.as_scroll_controller().try_position() else {
            return true;
        };
        if position.viewport_dimension() <= 0 {
            return true;
        }
        if position.reverse() {
            position.pixels() <= 0
        } else {
            position.pixels() >= position.max_scroll_extent()
        }
    }

    /// The reader has paused above the last row.
    pub fn away(&self) -> bool {
        !self.at_tail()
    }

    /// The transcript changed at its end while the reader was away.
    pub fn unseen(&self) -> bool {
        self.unseen.value() && self.away()
    }

    /// The end of the transcript changed: a reader that is away has news.
    pub fn note_activity(&self) {
        if self.away() {
            self.unseen.set_value(true);
        }
    }

    /// The reader is at the end: it has seen what there is.
    pub fn clear_unseen(&self) {
        self.unseen.set_value(false);
    }

    /// Back to the last row.
    pub fn jump_to_latest(&self) {
        let controller = self.scroll.as_scroll_controller();
        let reverse = controller
            .try_position()
            .map(|position| position.reverse())
            .unwrap_or(false);
        controller.jump_to(if reverse { 0 } else { i64::MAX });
        self.clear_unseen();
    }

    /// To the first row (`jump_top`).
    pub fn jump_to_first(&self) {
        let controller = self.scroll.as_scroll_controller();
        let reverse = controller
            .try_position()
            .map(|position| position.reverse())
            .unwrap_or(false);
        controller.jump_to(if reverse { i64::MAX } else { 0 });
    }

    /// Half the rows in view, rounded up, towards the last row (a negative
    /// count: towards the first) (`ctrl-d`, `ctrl-u`).
    pub fn half_page(&self, pages: i64) {
        let Ok(position) = self.scroll.as_scroll_controller().try_position() else {
            return;
        };
        let half = position.viewport_dimension().saturating_add(1) / 2;
        self.scroll_by(pages.saturating_mul(half.max(1)));
    }

    /// `rows` towards the last row (a negative count towards the first).
    pub fn scroll_by(&self, rows: i64) {
        let controller = self.scroll.as_scroll_controller();
        let Ok(position) = controller.try_position() else {
            return;
        };
        // a reverse list counts its offset from its end
        let rows = if position.reverse() { -rows } else { rows };
        controller.jump_to(position.pixels().saturating_add(rows));
    }

    /// A page towards the last row, less the one row kept in view (a negative
    /// count: towards the first).
    pub fn page(&self, pages: i64) {
        let Ok(position) = self.scroll.as_scroll_controller().try_position() else {
            return;
        };
        self.scroll_by(pages.saturating_mul((position.viewport_dimension() - 1).max(1)));
    }

    /// Notifies when the reader moves, the content changes or the news does.
    pub fn listenable(&self) -> Rc<dyn Listenable> {
        let scroll: Rc<dyn Listenable> = Rc::new(self.scroll.as_scroll_controller().clone());
        let unseen: Rc<dyn Listenable> = self.unseen.clone();
        <dyn Listenable>::merge([Some(scroll), Some(unseen)])
    }
}

/// The controllers of every thread, and of the blank "no thread yet" one.
#[derive(Clone, Default)]
pub struct ThreadUiMap {
    blank: Rc<ThreadUi>,
    threads: BTreeMap<ThreadId, Rc<ThreadUi>>,
}

impl ThreadUiMap {
    /// The controllers of `thread`, or the blank ones.
    pub fn get(&self, thread: Option<&ThreadId>) -> Rc<ThreadUi> {
        thread
            .and_then(|id| self.threads.get(id))
            .cloned()
            .unwrap_or_else(|| self.blank.clone())
    }

    /// Makes the controllers of `thread` if it has none. The draft typed
    /// before the thread existed moves to the first one. With `vim_default`
    /// (`tui.vim_mode_default`) a new draft is edited modally, in Insert mode.
    pub fn ensure(&mut self, thread: &ThreadId, vim_default: bool) {
        if self.threads.contains_key(thread) {
            return;
        }
        let ui = if self.threads.is_empty() {
            std::mem::take(&mut self.blank)
        } else {
            Rc::new(ThreadUi::default())
        };
        if vim_default && !ui.draft.vim_enabled() {
            ui.draft.start_in_vim_insert(&ui.composer);
        }
        self.threads.insert(thread.clone(), ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mode_of_input_is_held_until_the_transcript_closes() {
        let modes = Modes::default();
        assert_eq!(modes.input(), Input::Plain);
        let ticks = modes.listenable().value();
        modes.hold(Input::Select);
        assert_eq!(modes.input(), Input::Select);
        assert_ne!(modes.listenable().value(), ticks);
        // holding the same mode again is no news
        let ticks = modes.listenable().value();
        modes.hold(Input::Select);
        assert_eq!(modes.listenable().value(), ticks);
        modes.close();
        assert_eq!(modes.input(), Input::Plain);
    }

    #[test]
    fn the_selection_tells_whether_something_is_selected() {
        let selection = TextSelection::default();
        assert!(!selection.held());
        assert!(selection == selection.clone());
        assert!(selection != TextSelection::default());
    }
}
