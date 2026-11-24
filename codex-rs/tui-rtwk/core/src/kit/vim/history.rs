//! Bounded undo and redo of whole drafts (`chat_composer/vim_history.rs`): a
//! snapshot holds what the text area does not own (placeholders, images,
//! shell mode) with the text, so one step restores them together. Committed
//! snapshots share a byte budget; the one pending edit has the same cap, so a
//! canceled command cannot push committed history out.

use std::collections::VecDeque;

/// Steps kept.
pub const MAX_STEPS: usize = 64;
/// Bytes kept, committed snapshots together.
pub const MAX_BYTES: usize = 1024 * 1024;

/// The undo and redo stacks of one draft, and the snapshot of the command
/// being typed (`S`: the caller's snapshot of its whole draft).
#[derive(Debug)]
pub struct UndoHistory<S> {
    undo: VecDeque<(S, usize)>,
    redo: VecDeque<(S, usize)>,
    pending: Option<(S, usize)>,
}

impl<S> Default for UndoHistory<S> {
    fn default() -> UndoHistory<S> {
        UndoHistory {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            pending: None,
        }
    }
}

impl<S: Clone> UndoHistory<S> {
    /// A command or an insert session is under way (its snapshot is held).
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Everything forgotten (Vim turned on or off, a draft put in whole).
    pub fn reset(&mut self) {
        *self = UndoHistory::default();
    }

    fn trim(&mut self) {
        while self.undo.len() > MAX_STEPS
            || self
                .undo
                .iter()
                .chain(self.redo.iter())
                .map(|(_, bytes)| *bytes)
                .sum::<usize>()
                > MAX_BYTES
        {
            if self.undo.pop_front().is_none() {
                self.redo.pop_front();
            }
        }
    }

    /// A transaction starts from `snapshot`, `bytes` large. A draft over the
    /// budget has no history at all.
    pub fn begin(&mut self, snapshot: S, bytes: usize) {
        if bytes > MAX_BYTES {
            self.reset();
            return;
        }
        self.pending = Some((snapshot, bytes));
        self.trim();
    }

    /// The transaction ends: its snapshot becomes an undo step unless the
    /// draft is as it was (`unchanged`), and what could be redone is gone.
    pub fn commit(&mut self, unchanged: impl FnOnce(&S) -> bool) {
        let Some((snapshot, bytes)) = self.pending.take() else {
            return;
        };
        if unchanged(&snapshot) {
            return;
        }
        self.redo.clear();
        self.undo.push_back((snapshot, bytes));
        self.trim();
    }

    /// The snapshot `u` goes back to, `current` (`bytes` large) kept for redo.
    pub fn undo(&mut self, current: S, bytes: usize) -> Option<S> {
        let (snapshot, _) = self.undo.pop_back()?;
        self.redo.push_back((current, bytes));
        self.trim();
        Some(snapshot)
    }

    /// The snapshot redo goes forward to, `current` kept for undo.
    pub fn redo(&mut self, current: S, bytes: usize) -> Option<S> {
        let (snapshot, _) = self.redo.pop_back()?;
        self.undo.push_back((current, bytes));
        self.trim();
        Some(snapshot)
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(h: &mut UndoHistory<&'static str>, before: &'static str) {
        h.begin(before, before.len());
        h.commit(|s| *s == "same");
    }

    #[test]
    fn undo_and_redo_swap_the_draft() {
        let mut h = UndoHistory::default();
        step(&mut h, "a");
        step(&mut h, "ab");
        assert_eq!(h.undo("abc", 3), Some("ab"));
        assert_eq!(h.undo("ab", 2), Some("a"));
        assert_eq!(h.undo("a", 1), None);
        assert_eq!(h.redo("a", 1), Some("ab"));
        assert_eq!(h.redo("ab", 2), Some("abc"));
        assert_eq!(h.redo("abc", 3), None);
    }

    #[test]
    fn a_new_edit_ends_redo_and_an_unchanged_one_is_no_step() {
        let mut h = UndoHistory::default();
        step(&mut h, "a");
        assert!(h.undo("ab", 2).is_some());
        assert!(h.can_redo());
        step(&mut h, "same");
        assert!(h.can_redo(), "an unchanged command leaves redo");
        step(&mut h, "b");
        assert!(!h.can_redo());
    }

    #[test]
    fn steps_and_bytes_are_bounded() {
        let mut h: UndoHistory<usize> = UndoHistory::default();
        for i in 0..100 {
            h.begin(i, 1);
            h.commit(|_| false);
        }
        assert_eq!(h.undo.len(), MAX_STEPS);
        assert_eq!(h.undo.front().map(|(s, _)| *s), Some(36));
        let mut h: UndoHistory<u8> = UndoHistory::default();
        h.begin(0, MAX_BYTES + 1);
        assert!(!h.has_pending(), "a draft over the budget has no history");
        h.begin(0, MAX_BYTES);
        h.commit(|_| false);
        h.begin(1, 10);
        h.commit(|_| false);
        assert_eq!(h.undo.len(), 1, "the oldest step goes for the byte budget");
    }
}
