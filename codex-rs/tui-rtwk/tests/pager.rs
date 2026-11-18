//! Screen tests of S16, the pager. Each one opens a debug entry (a fixture
//! script played through core `apply`) at the size of its oracle snapshot and
//! compares the rows with the oracle's `.snap` (`tests/oracle_snap`).
//! Oracle: `codex-rs/tui/src/pager_overlay.rs` and `pager_overlay/transcript_tests.rs`.

#[path = "oracle_snap/mod.rs"]
mod oracle_snap;

use crate::common::{Harness, slot, start_sized};

const STATIC_BASIC: &str =
    "snapshots/codex_tui__pager_overlay__tests__static_overlay_snapshot_basic.snap";
const STATIC_WRAPS: &str =
    "snapshots/codex_tui__pager_overlay__tests__static_overlay_wraps_long_lines.snap";
const PAGINATED: &str = "pager_overlay/snapshots/codex_tui__pager_overlay__transcript_tests__transcript_overlay_paginated_history_states.snap";
const COMPLETED: &str = "pager_overlay/snapshots/codex_tui__pager_overlay__transcript_tests__transcript_overlay_completed_stream.snap";

fn open(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("pager", Some(entry));
    h
}

/// Delta D1: the static pager does not fill the rows under its content with
/// `~` (rtwk's `ListView` and `SingleChildScrollView` end at their content).
fn without_tilde(rows: Vec<String>) -> Vec<String> {
    rows.into_iter()
        .map(|row| if row == "~" { String::new() } else { row })
        .collect()
}

#[test]
fn pager_slot_is_registered_and_empty() {
    slot("pager");
}

#[test]
fn static_overlay_snapshot_basic() {
    let h = open("static_basic", 40, 10);
    oracle_snap::assert_same(
        "static_overlay_snapshot_basic",
        &oracle_snap::screen(&h.t),
        &without_tilde(oracle_snap::rows(STATIC_BASIC)),
    );
}

#[test]
fn static_overlay_wraps_long_lines() {
    let h = open("static_wraps_long_lines", 24, 8);
    oracle_snap::assert_same(
        "static_overlay_wraps_long_lines",
        &oracle_snap::screen(&h.t),
        &without_tilde(oracle_snap::rows(STATIC_WRAPS)),
    );
}

#[test]
fn transcript_overlay_snapshots_paginated_history_states() {
    for (state, entry) in [
        ("loading", "transcript_loading"),
        ("partial", "transcript_partial"),
        ("failed", "transcript_failed"),
        ("complete", "transcript_complete"),
    ] {
        let h = open(entry, 72, 10);
        oracle_snap::assert_same(
            state,
            &oracle_snap::screen(&h.t),
            &oracle_snap::section(PAGINATED, state),
        );
    }
}

#[test]
fn transcript_overlay_completed_stream() {
    let h = open("transcript_completed_stream", 40, 10);
    oracle_snap::assert_same(
        "transcript_overlay_completed_stream",
        &oracle_snap::screen(&h.t),
        &oracle_snap::rows(COMPLETED),
    );
}

/// The static pager's keys: `q` closes (back on the menu), the arrows scroll
/// and the percentage follows (oracle: `PagerView::handle_key_event`,
/// `render_bottom_bar`).
#[test]
fn static_pager_scrolls_and_q_closes() {
    let mut h = open("static_long", 40, 10);
    let top = h.t.screen();
    assert!(top.contains("line 1"), "{top}");
    assert!(top.contains(" 0% "), "{top}");
    h.press("pagedown");
    let paged = h.t.screen();
    assert!(!paged.contains("line 1\n"), "{paged}");
    assert!(!paged.contains(" 0% "), "{paged}");
    h.press("end");
    assert!(h.t.screen().contains(" 100% "), "{}", h.t.screen());
    h.press("q");
    assert!(
        !h.t.screen().contains("S T A T I C"),
        "q closed the pager: {}",
        h.t.screen()
    );
}

#[test]
fn transcript_pager_closes_with_q() {
    let mut h = open("transcript_complete", 72, 10);
    assert!(h.t.screen().contains("T R A N S C R I P T"));
    h.press("q");
    assert!(!h.t.screen().contains("T R A N S C R I P T"));
}
