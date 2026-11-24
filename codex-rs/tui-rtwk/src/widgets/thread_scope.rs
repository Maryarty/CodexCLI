//! `thread_scope` (R2, ADR-013): the one place a subtree is keyed by the thread it belongs to.
//! Switching the thread remounts exactly that subtree, so what an element keeps of its own (the
//! scroll position of a list, a focus) is the thread's, never the one before it. The rule
//! `r2_thread_subtree_is_keyed` lets no other file build a key from a `ThreadId`.

use rtwk::{AnyWidget, Key};

use codex_tui_core::ids::ThreadId;

/// `child`, in the scope of `thread`: another thread's is another element.
pub fn thread_scope(thread: &ThreadId, child: impl Into<AnyWidget>) -> AnyWidget {
    child.into().with_key(Key::value(thread.clone()))
}
