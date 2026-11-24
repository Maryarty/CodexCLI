// test root: pages
//! Tests of the clicks on the usage surfaces (oracle `analytics/mouse_tests.rs` and the usage
//! menu's picker rows): what the oracle's shards do not say. Each click is a person's: a cell of
//! the label drawn, as the label's own text says.

use crate::kit::{Owner, Screen};

fn dashboard() -> Screen {
    Screen::build(Owner::RoutesLists)
        .size(100, 38)
        .entry("usage", "dashboard")
        .open()
}

fn plan() -> Screen {
    Screen::build(Owner::RoutesLists)
        .size(100, 38)
        .entry("usage", "plan")
        .open()
}

fn menu() -> Screen {
    let mut s = Screen::build(Owner::UiPane).size(100, 30).live().open();
    s.type_text("/usage");
    s.keys("enter");
    s.assert_contains("Account usage and resets.");
    s
}

/// A click on a tab is that report (the digit's key does the same).
#[test]
fn a_click_on_a_tab_opens_its_report() {
    let mut s = dashboard();
    s.assert_contains("Total usage");
    s.click_text("4 Plugins");
    s.assert_contains("186 calls");
    s.assert_absent("Total usage");
}

/// A click on the range control switches between the seven and the thirty days, as `r` does.
#[test]
fn a_click_on_the_range_switches_between_7_and_30_days() {
    let mut s = dashboard();
    s.assert_contains("r 7d/30d · Aug 27–Sep 2, 2026");
    s.click_text("r 7d/30d");
    s.assert_contains("r 7d/30d · Aug 4–Sep 2, 2026");
    s.click_text("r 7d/30d");
    s.assert_contains("r 7d/30d · Aug 27–Sep 2, 2026");
}

/// A click on the group control is the next grouping of the report, as `g` does.
#[test]
fn a_click_on_the_group_control_steps_the_grouping() {
    let mut s = dashboard();
    s.assert_contains("g Feature");
    s.click_text("g Feature");
    s.assert_contains("g Model");
    s.assert_absent("g Feature");
}

/// A click on the Plan's window control shows the other table; a click on a period opens its
/// details.
#[test]
fn a_click_on_the_plan_window_and_a_period_work_like_the_keys() {
    let mut s = plan();
    let active = |s: &Screen, header: &str| {
        s.rows()
            .iter()
            .any(|row| row.trim_start().starts_with(&format!("› {header}")))
    };
    assert!(active(&s, "5-hour limits"), "{}", s.text());
    s.click_text("←/→ 5-hour / Weekly");
    assert!(active(&s, "Weekly limits"), "{}", s.text());
    assert!(!active(&s, "5-hour limits"), "{}", s.text());
    s.assert_absent("Breakdown isn't available for this period");
    s.click_text("Sep 1 00:00 – Sep 8 00:00");
    s.assert_contains("Breakdown isn't available for this period");
}

/// A click on the menu's first row opens the analytics page, as Enter on it does.
#[test]
fn a_click_on_view_analytics_opens_the_usage_page() {
    let mut s = menu();
    s.click_text("View analytics");
    s.assert_absent("Account usage and resets.");
    s.assert_contains("Loading…");
}

/// A click on a row that cannot be opened (no reset to redeem) moves the cursor to it and keeps
/// the menu.
#[test]
fn a_click_on_redeem_reset_with_none_available_keeps_the_menu() {
    let mut s = menu();
    s.click_text("Redeem reset");
    s.assert_contains("Account usage and resets.");
    s.assert_contains("None available");
    assert!(
        s.rows()
            .iter()
            .any(|row| row.trim_start().starts_with("›") && row.contains("Redeem reset")),
        "{}",
        s.text()
    );
}
