//! Screen tests of S20, the `/usage` dashboard. The oracle's wide summary
//! snapshot (140x44) is compared whole; the panel snapshot is the four
//! panels' messages. Oracle: `codex-rs/tui/src/analytics/`.

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "oracle_snap/mod.rs"]
mod oracle_snap;

use crate::common::{Harness, start_sized};

const SUMMARY: &str =
    "analytics/snapshots/codex_tui__analytics__tests__summary__summary_daily_wide.snap";
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
