//! Screen tests of S19 (the `/status` card). Expected text: the oracle's
//! snapshot (`dialogs::oracle`). Oracle tests: `status/tests.rs`
//! (`status_snapshot_includes_reasoning_details`), `status/helpers.rs`
//! (`format_tokens_compact`).

use crate::common::{Harness, ms, start_sized};
use crate::dialogs::oracle::assert_rows;

fn open(entry: &str) -> Harness {
    let mut h = start_sized(true, 100, 24);
    h.open_debug_row("status", Some(entry));
    h.t.pump_for(ms(800));
    h
}

#[test]
fn status_snapshot_includes_reasoning_details() {
    let h = open("reasoning_details");
    assert_rows(
        &h.t.screen(),
        "status/snapshots/codex_tui__status__tests__status_snapshot_includes_reasoning_details.snap",
    );
}

#[test]
fn esc_closes_the_card() {
    let mut h = open("reasoning_details");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("OpenAI Codex"), "{}", h.t.screen());
}

/// The limit bar (G40): the oracle draws `[██████░░░░░░░░░░░░░░]` for 28% left,
/// 20 whole cells of `█` and `░` between brackets (`render_limit_progress_bar`);
/// a `CustomPaint` in the card.
#[test]
fn limit_bar_is_a_custom_paint_of_whole_cells() {
    let h = open("reasoning_details");
    assert!(
        h.t.screen().contains("[██████░░░░░░░░░░░░░░] 28% left"),
        "{}",
        h.t.screen()
    );
}
