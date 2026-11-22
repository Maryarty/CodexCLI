//! Per-thread UI controllers (R2, R14): the composer's text controller of
//! each thread and its draft's elements (`widgets::composer_draft`), and the
//! reader of its transcript (`Follow`). They are UI state, never seen by
//! `apply`; `ingest` creates a thread's controllers after the thread appears.

use std::collections::BTreeMap;
use std::rc::Rc;

use rtwk::editing::TextEditingController;
use rtwk::foundation::{Listenable, ValueNotifier};
use rtwk::scroll::ListController;

use codex_tui_core::ids::ThreadId;

use crate::widgets::composer_draft::ComposerDraft;

/// The controllers of one thread.
#[derive(Clone, Default, PartialEq)]
pub struct ThreadUi {
    pub composer: TextEditingController,
    /// The composer's paste placeholders and kill ring, kept in step with
    /// `composer`'s text.
    pub draft: ComposerDraft,
    /// Where the reader of the transcript is, and what it has not seen.
    pub follow: Follow,
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
    /// before the thread existed moves to the first one.
    pub fn ensure(&mut self, thread: &ThreadId) {
        if self.threads.contains_key(thread) {
            return;
        }
        let ui = if self.threads.is_empty() {
            std::mem::take(&mut self.blank)
        } else {
            Rc::new(ThreadUi::default())
        };
        self.threads.insert(thread.clone(), ui);
    }
}
