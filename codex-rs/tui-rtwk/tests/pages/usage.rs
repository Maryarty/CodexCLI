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
const DASHBOARD: &str = "analytics/snapshots/codex_tui__analytics__tests__dashboard__dashboard_cards_align_and_keep_stable_summary_heights.snap";
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

/// `?` shows the page's shortcuts in the report's place, the footer reads
/// `esc back` and `? close help`, Esc closes the help before the page and
/// `?` closes it too (oracle: `help_lines`, `footer_lines`, `handle_key`).
#[test]
fn question_mark_opens_the_help_and_escape_closes_it_before_the_page() {
    let mut h = open("credits_refund", 100, 32);
    assert!(!h.t.screen().contains("Usage shortcuts"));
    h.press("?");
    let screen = h.t.screen();
    assert!(screen.contains("Usage shortcuts"), "{screen}");
    assert!(screen.contains("next / previous report"), "{screen}");
    assert!(
        screen.contains("1\u{2013}6 \u{b7} open a report directly"),
        "{screen}"
    );
    assert!(
        screen.contains("r \u{b7} switch between 7 and 30 days"),
        "{screen}"
    );
    assert!(screen.contains("R \u{b7} refresh all reports"), "{screen}");
    assert!(screen.contains("? \u{b7} toggle this help"), "{screen}");
    assert!(
        !screen.contains("Product"),
        "the help replaces the report: {screen}"
    );
    assert!(screen.contains("? close help"), "{screen}");
    // Esc closes the help first, the page stays.
    h.press("escape");
    let screen = h.t.screen();
    assert!(!screen.contains("Usage shortcuts"), "{screen}");
    assert!(screen.contains("Product"), "{screen}");
    // `?` toggles it, a digit goes to its report and closes it.
    h.press("?");
    h.press("?");
    assert!(!h.t.screen().contains("Usage shortcuts"));
    h.press("?");
    h.press("2");
    assert!(
        !h.t.screen().contains("Usage shortcuts"),
        "{}",
        h.t.screen()
    );
}

/// `q` closes the page, as Esc does on a report without an open detail
/// (oracle: `is_done`).
#[test]
fn q_closes_the_page() {
    let mut h = open("credits_refund", 100, 32);
    h.press("q");
    assert!(!h.t.screen().contains("Product"), "{}", h.t.screen());
}

/// A report taller than its room scrolls with the page keys and the footer
/// offers them only then (oracle: `footer_lines(.., scrollable)`,
/// `summary_action`).
#[test]
fn the_page_keys_scroll_a_report_taller_than_its_room() {
    let mut h = open("overview", 140, 18);
    let screen = h.t.screen();
    assert!(screen.contains("pgup/pgdn scroll"), "{screen}");
    assert!(screen.contains("Example User"), "{screen}");
    assert!(!screen.contains("Statistics as of"), "{screen}");
    h.press("pagedown");
    let screen = h.t.screen();
    assert!(!screen.contains("Example User"), "{screen}");
    h.press("end");
    assert!(
        h.t.screen().contains("Statistics as of"),
        "{}",
        h.t.screen()
    );
    h.press("pageup");
    h.press("home");
    assert!(h.t.screen().contains("Example User"), "{}", h.t.screen());
    // A tall terminal has nothing to scroll: no hint.
    let h = open("overview", 140, 44);
    assert!(!h.t.screen().contains("pgup/pgdn"), "{}", h.t.screen());
}

/// Up and Down scroll the Overview by a row (oracle: `summary_action`).
#[test]
fn the_arrows_scroll_the_overview_a_row() {
    let mut h = open("overview", 140, 18);
    let before = h.t.screen();
    h.press("down");
    let after = h.t.screen();
    assert_ne!(before, after);
    h.press("up");
    assert_eq!(before, h.t.screen());
}

/// The cell of the first row that holds `label`.
fn find(h: &Harness, label: &str) -> (u16, u16) {
    let screen = h.t.screen();
    let (y, row) = screen
        .lines()
        .enumerate()
        .find(|(_, row)| row.contains(label))
        .unwrap_or_else(|| panic!("no `{label}` on\n{screen}"));
    let x = row[..row.find(label).unwrap_or(0)].chars().count();
    (x as u16 + 1, y as u16)
}

/// A click on `label`, then the time a double tap would have taken.
fn tap(h: &mut Harness, label: &str) {
    let (x, y) = find(h, label);
    h.t.click(x, y);
    h.advance(500);
}

/// Two clicks on `label` in a row.
fn double_tap(h: &mut Harness, label: &str) {
    let (x, y) = find(h, label);
    h.t.click(x, y);
    h.t.click(x, y);
    h.advance(500);
}

/// A click on a tab picks its report (oracle: `mouse.rs`, the tab hits).
#[test]
fn clicking_a_tab_picks_the_report() {
    let mut h = open("overview", 140, 44);
    assert!(h.t.screen().contains("Lifetime tokens"));
    tap(&mut h, "2 Usage");
    assert!(
        !h.t.screen().contains("Lifetime tokens"),
        "{}",
        h.t.screen()
    );
    tap(&mut h, "1 Overview");
    assert!(h.t.screen().contains("Lifetime tokens"), "{}", h.t.screen());
}

/// A click on a control does what its key does: `r` switches the range,
/// `g` the grouping (oracle: `controls.rs` hits).
#[test]
fn clicking_the_controls_switches_the_range_and_the_grouping() {
    let mut h = open("credits_refund", 120, 32);
    assert!(h.t.screen().contains("r 7d/30d"), "{}", h.t.screen());
    let before = h.t.screen();
    tap(&mut h, "r 7d/30d");
    assert_ne!(before, h.t.screen());
    let grouped = h.t.screen();
    tap(&mut h, "g ");
    assert_ne!(grouped, h.t.screen());
}

/// The wheel scrolls the report in front by rows (oracle: `mouse.rs`).
#[test]
fn the_wheel_scrolls_the_overview() {
    let mut h = open("overview", 140, 18);
    assert!(h.t.screen().contains("Example User"));
    for _ in 0..3 {
        h.t.mouse(
            rtwk_core::input::MouseKind::WheelDown,
            40,
            8,
            0,
            rtwk_core::input::Mods::NONE,
        );
    }
    h.settle();
    assert!(!h.t.screen().contains("Example User"), "{}", h.t.screen());
    for _ in 0..12 {
        h.t.mouse(
            rtwk_core::input::MouseKind::WheelUp,
            40,
            8,
            0,
            rtwk_core::input::Mods::NONE,
        );
    }
    h.settle();
    assert!(h.t.screen().contains("Example User"), "{}", h.t.screen());
}

/// The oracle's snapshot joins two screens with a blank row: the wide one
/// (144x64) and the narrow one (72x48).
fn dashboard_halves() -> (Vec<String>, Vec<String>) {
    let mut wide = oracle_snap::rows(DASHBOARD);
    let narrow = wide.split_off(65);
    assert_eq!(wide.pop().as_deref(), Some(""), "the separating row");
    assert_eq!(wide.len(), 64);
    (wide, narrow)
}

/// `z` shows the cards (oracle: `dashboard_cards_align_and_keep_stable_
/// summary_heights`, the wide half): two columns, the report in front marked,
/// each card keeping its height whatever its report holds.
#[test]
fn dashboard_cards_align_in_two_columns_at_144() {
    let mut h = open("dashboard", 144, 64);
    h.press("z");
    oracle_snap::assert_same(
        "dashboard_wide",
        &oracle_snap::screen(&h.t),
        &dashboard_halves().0,
    );
}

/// The narrow half: one column at 72 cells, the day moved with the left arrow,
/// the Plugins report refused; Enter focuses the report again and keeps the
/// section, the day and the grouping.
#[test]
fn dashboard_cards_stack_in_one_column_at_72() {
    let mut h = open("dashboard_denied", 72, 48);
    h.press("z");
    h.press("left");
    oracle_snap::assert_same(
        "dashboard_narrow",
        &oracle_snap::screen(&h.t),
        &dashboard_halves().1,
    );
    h.press("enter");
    let screen = h.t.screen();
    assert!(!screen.contains("Dashboard \u{b7}"), "{screen}");
    assert!(screen.contains("Sep 1 \u{b7} 100 usage units"), "{screen}");
}

/// Esc closes the page from the dashboard (oracle: `overview_closes_on_escape_
/// with_retained_day_details`).
#[test]
fn escape_on_the_dashboard_closes_the_page() {
    let mut h = open("dashboard", 144, 64);
    h.press("z");
    assert!(
        h.t.screen().contains("Dashboard \u{b7}"),
        "{}",
        h.t.screen()
    );
    h.press("esc");
    assert!(
        !h.t.screen().contains("Usage \u{b7} Plus"),
        "{}",
        h.t.screen()
    );
}

/// Moving to a card scrolls it into view (oracle: the dashboard follows the
/// selected card on a section change, not on the wheel).
#[test]
fn the_dashboard_follows_the_selected_card() {
    let mut h = open("dashboard", 80, 24);
    h.press("z");
    h.press("6");
    let screen = h.t.screen();
    assert!(screen.contains("\u{25b8} 6 Chats"), "{screen}");
    assert!(!screen.contains("1 Overview"), "{screen}");
}

/// A tap on a card selects its report; a second tap opens it (ADR-011).
#[test]
fn a_tap_selects_a_card_and_a_double_tap_opens_it() {
    let mut h = open("dashboard", 144, 64);
    h.press("z");
    tap(&mut h, "3 Messages \u{2500}");
    let screen = h.t.screen();
    assert!(screen.contains("\u{25b8} 3 Messages"), "{screen}");
    assert!(screen.contains("Dashboard \u{b7}"), "{screen}");
    double_tap(&mut h, "3 Messages \u{2500}");
    let screen = h.t.screen();
    assert!(!screen.contains("Dashboard \u{b7}"), "{screen}");
}

/// The report's time shows in the help, on its card and in the controls
/// (oracle: `help_lines`, the card's footer, the controls' last row).
#[test]
fn the_report_time_shows_in_the_help_the_card_and_the_controls() {
    let mut h = open("updated", 144, 64);
    let focused = h.t.screen();
    assert!(
        focused.contains("Updated \u{b7} Sep 2 16:30 UTC"),
        "{focused}"
    );
    h.press("z");
    let cards = h.t.screen();
    assert!(cards.contains("Updated Sep 2 16:30 UTC"), "{cards}");
    h.press("?");
    let help = h.t.screen();
    assert!(help.contains("Report updated Sep 2 16:30 UTC"), "{help}");
}
