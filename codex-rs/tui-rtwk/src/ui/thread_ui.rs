//! Per-thread UI controllers (R2, R14): the composer's text controller of
//! each thread. They are UI state, never seen by `apply`; `ingest` creates a
//! thread's controllers after the thread appears.

use std::collections::BTreeMap;
use std::rc::Rc;

use rtwk::editing::TextEditingController;

use codex_tui_rtwk_core::ids::ThreadId;

/// The controllers of one thread.
#[derive(Clone, Default, PartialEq)]
pub struct ThreadUi {
    pub composer: TextEditingController,
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
