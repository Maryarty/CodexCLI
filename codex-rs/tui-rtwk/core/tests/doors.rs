#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The doors of the W2 round-1 "needs first" task (doors-r1) that sit in the core: one test per
//! door, each the smallest use a package makes of it.

use std::rc::Rc;

use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::agents::CenterThreads;
use codex_tui_core::threads::ThreadPreviews;

#[test]
fn the_center_names_the_live_thread() {
    assert_eq!(CenterThreads::default().live(), None);
    let id = ThreadId::new("thread-1");
    let threads = CenterThreads::new(Some(id.clone()), ThreadPreviews::default(), Rc::default());
    assert_eq!(threads.live(), Some(&id));
    assert!(threads.preview("thread-1").is_none());
}
