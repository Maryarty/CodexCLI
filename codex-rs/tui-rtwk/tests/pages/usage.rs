//! Screen tests of S20, the `/usage` dashboard. The oracle's wide summary
//! snapshot (140x44) is compared whole; the panel snapshot is the four
//! panels' messages. Oracle: `codex-rs/tui/src/analytics/`.

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "../oracle_snap/mod.rs"]
mod oracle_snap;

use crate::common::{Harness, start_sized};

const SUMMARY: &str =
    "analytics/snapshots/codex_tui__analytics__tests__summary__summary_daily_wide.snap";
const REFUNDS: &str = "snapshots/codex_tui__analytics__tests__analytics_maximized_refunds_leave_room_for_both_bands.snap";
const PANELS: &str = "snapshots/codex_tui__analytics__tests__analytics_loading_errors_and_empty_reports_stay_independent.snap";

fn open(entry: &str, width: u16, height: u16) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("usage", Some(entry));
    h
}

#[test]
fn summary_daily_wide_header_numbers_and_footer() {
    let h = open("overview", 140, 44);
    // The whole screen: header, tabs, rule, the profile block, the five
    // numbers, the heat map, the insights and the footer.
    oracle_snap::assert_same(
        "summary_daily_wide",
        &oracle_snap::screen(&h.t),
        &oracle_snap::rows(SUMMARY),
    );
}

#[test]
fn analytics_loading_errors_and_empty_reports_stay_independent() {
    let h = open("panel_states", 60, 12);
    oracle_snap::assert_fragment(
        "analytics_loading_errors_and_empty_reports_stay_independent",
        &oracle_snap::screen(&h.t),
        0,
        &oracle_snap::rows(PANELS),
    );
}

/// An Enterprise account's Credits report with a refund, 100x32: the focused
/// report whole (the plot with both bands, the table, the footer).
#[test]
fn analytics_maximized_refunds_leave_room_for_both_bands() {
    let h = open("credits_refund", 100, 32);
    oracle_snap::assert_same(
        "analytics_maximized_refunds_leave_room_for_both_bands",
        &oracle_snap::screen(&h.t),
        &oracle_snap::rows(REFUNDS),
    );
}

/// The tab row's digit keys pick the report (oracle: `navigation_tests`).
#[test]
fn digit_keys_pick_the_report() {
    let mut h = open("overview", 140, 44);
    h.press("2");
    let screen = h.t.screen();
    assert!(!screen.contains("Lifetime tokens"), "{screen}");
    h.press("1");
    assert!(h.t.screen().contains("Lifetime tokens"), "{}", h.t.screen());
}

/// The arrows, `h`, `l`, Home and End move the selected day of a chart
/// report (oracle: `handle_key`, `MoveLeft`/`MoveRight`/`JumpTop`/`JumpBottom`).
#[test]
fn the_day_keys_move_the_selected_day() {
    let mut h = open("credits_refund", 100, 32);
    assert!(h.t.screen().contains("Sep 2 \u{b7} -0.000004 credits"));
    h.press("left");
    assert!(
        h.t.screen().contains("Sep 1 \u{b7} 109.00 credits"),
        "{}",
        h.t.screen()
    );
    h.press("h");
    assert!(h.t.screen().contains("Aug 31 \u{b7} 75.00 credits"));
    h.press("l");
    h.press("right");
    assert!(h.t.screen().contains("Sep 2 \u{b7} -0.000004 credits"));
    h.press("right");
    assert!(
        h.t.screen().contains("Sep 2 \u{b7} -0.000004 credits"),
        "the last day stays"
    );
    h.press("home");
    let screen = h.t.screen();
    assert!(screen.contains("Aug 27 \u{b7} No activity (0)"), "{screen}");
    // A day without numbers has no details to open.
    assert!(!screen.contains("enter details"), "{screen}");
    h.press("end");
    assert!(h.t.screen().contains("enter details"));
}

/// Enter shows the whole numbers of the day, Esc closes them before it
/// closes the page, and the open detail follows the selected day.
#[test]
fn enter_opens_the_whole_numbers_and_escape_closes_them_first() {
    let mut h = open("credits_refund", 100, 32);
    h.press("left");
    h.press("enter");
    let screen = h.t.screen();
    assert!(screen.contains("Sep 1 \u{b7} 109 credits"), "{screen}");
    h.press("left");
    assert!(
        h.t.screen().contains("Aug 31 \u{b7} 75 credits"),
        "{}",
        h.t.screen()
    );
    h.press("escape");
    let screen = h.t.screen();
    assert!(screen.contains("Aug 31 \u{b7} 75.00 credits"), "{screen}");
    assert!(screen.contains("Product"), "the page stays: {screen}");
    h.press("escape");
    assert!(!h.t.screen().contains("Product"), "{}", h.t.screen());
}
