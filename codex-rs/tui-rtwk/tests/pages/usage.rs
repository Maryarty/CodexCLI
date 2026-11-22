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

const PLAN_NAVIGATION: &str = "analytics/snapshots/codex_tui__analytics__plan__tests__period_navigation_preserves_unknowns_and_resets_on_new_snapshot.snap";
const PLAN_STATES: &str = "analytics/snapshots/codex_tui__analytics__plan__tests__unavailable_empty_and_zero_history_are_distinct.snap";
const PLAN_CLOCKS: &str =
    "analytics/snapshots/codex_tui__analytics__plan__tests__plan_clocks_twelve_hour.snap";

/// The oracle's snapshot of two screens, 120x30 and 58x30, joined with a blank
/// row.
fn plan_halves() -> (Vec<String>, Vec<String>) {
    let mut wide = oracle_snap::rows(PLAN_NAVIGATION);
    let narrow = wide.split_off(31);
    assert_eq!(wide.pop().as_deref(), Some(""), "the separating row");
    (wide, narrow)
}

/// The Plan report in front: both tables open to their first period with
/// the details of each (a 5-hour period with an incomplete day and its
/// breakdown, a weekly one without one), the tables side by side from 100
/// cells and one under the other below (oracle: `period_navigation_preserves_
/// unknowns_and_resets_on_new_snapshot`, both screens).
#[test]
fn period_details_open_in_both_tables_wide_and_narrow() {
    let mut h = open("plan", 120, 30);
    for key in ["enter", "right", "enter"] {
        h.press(key);
    }
    let (wide, narrow) = plan_halves();
    oracle_snap::assert_same("plan_wide", &oracle_snap::screen(&h.t), &wide);
    h.t.resize(58, 30);
    oracle_snap::assert_same("plan_narrow", &oracle_snap::screen(&h.t), &narrow);
}

/// The body without the page's inset, from the row that starts `from`.
fn plan_body(h: &Harness, from: &str, rows: usize) -> Vec<String> {
    let screen = oracle_snap::screen(&h.t);
    let at = screen
        .iter()
        .position(|row| row.trim_start().starts_with(from))
        .unwrap_or_else(|| panic!("no `{from}` on\n{}", screen.join("\n")));
    screen
        .iter()
        .skip(at)
        .take(rows)
        .map(|row| row.strip_prefix(' ').unwrap_or(row).trim_end().to_owned())
        .collect()
}

/// The history is not loaded, has no periods, used nothing, or the period
/// has not started: four different bodies (oracle: `unavailable_empty_and_
/// zero_history_are_distinct`, at a body 100 cells wide).
#[test]
fn unavailable_empty_and_zero_history_are_distinct() {
    let rows = oracle_snap::rows(PLAN_STATES);
    let mut states: Vec<Vec<String>> = Vec::new();
    for row in rows {
        if row.starts_with("\u{203a} 5-hour limits") {
            states.push(Vec::new());
        }
        if let Some(state) = states.last_mut() {
            state.push(row);
        }
    }
    assert_eq!(states.len(), 4);
    for (state, entry) in
        states
            .iter_mut()
            .zip(["plan_unavailable", "plan_empty", "plan_zero", "plan_future"])
    {
        if state.last().is_some_and(String::is_empty) {
            state.pop();
        }
        let h = open(entry, 102, 30);
        let got = plan_body(&h, "\u{203a} 5-hour limits", state.len());
        oracle_snap::assert_same(entry, &got, state);
    }
}

/// Times follow the clock preference and stay UTC (oracle: `plan_clocks_follow_
/// preference_and_keep_utc`, a body 80 cells wide).
#[test]
fn plan_clocks_follow_preference_and_keep_utc() {
    let h = open("plan_twelve_hour", 82, 30);
    let expected = oracle_snap::rows(PLAN_CLOCKS);
    let got = plan_body(&h, "\u{203a} 5-hour limits", expected.len());
    oracle_snap::assert_same("plan_clocks_twelve_hour", &got, &expected);
}

/// Esc closes the details of the period before it closes the page (oracle:
/// `plan_escape_collapses_details_before_closing`).
#[test]
fn plan_escape_collapses_details_before_closing() {
    let mut h = open("plan", 120, 30);
    h.press("enter");
    assert!(h.t.screen().contains("Tasks"), "{}", h.t.screen());
    h.press("escape");
    assert!(!h.t.screen().contains("Tasks"), "{}", h.t.screen());
    assert!(h.t.screen().contains("5-hour limits"), "the page stays");
    h.press("escape");
    assert!(!h.t.screen().contains("5-hour limits"), "{}", h.t.screen());
}

/// Moving to another period closes the open one; the other table's key
/// moves to its table, leaving the cursor of this one where it was (oracle:
/// `moving_period_selection_collapses_its_previous_details`).
#[test]
fn moving_period_selection_collapses_its_previous_details() {
    let mut h = open("plan_many", 120, 30);
    h.press("enter");
    assert!(h.t.screen().contains("Tasks"), "{}", h.t.screen());
    h.press("down");
    let screen = h.t.screen();
    assert!(!screen.contains("Tasks"), "{screen}");
    assert!(
        screen.contains("\u{203a} Sep 2 05:00"),
        "the second row: {screen}"
    );
    h.press("right");
    let screen = h.t.screen();
    assert!(screen.contains("\u{203a} Weekly limits"), "{screen}");
    assert!(!screen.contains("  \u{203a} 5-hour limits"), "{screen}");
    // The tab leaves the report and the shift tab comes back to it with
    // the cursors where they were.
    h.press("tab");
    assert!(!h.t.screen().contains("5-hour limits"), "{}", h.t.screen());
    h.press("shift+tab");
    let screen = h.t.screen();
    assert!(screen.contains("\u{203a} Weekly limits"), "{screen}");
    assert!(screen.contains("\u{203a} Sep 2 05:00"), "{screen}");
}

/// The oracle follows the cursor: the rows of the period under it stay in
/// view as it moves down a long table and to the other table (`follow_
/// selection_on_focus`). Flutter: `Scrollable.ensureVisible(context)` of the
/// selected row.
#[test]
#[ignore = "pending(G-new-usage-1): no ensure_visible of a row from a State; the Plan report does not follow its cursor"]
fn the_plan_report_follows_the_cursor_row() {
    let mut h = open("plan_many", 80, 24);
    for _ in 0..13 {
        h.press("down");
    }
    assert!(
        h.t.screen().contains("\u{203a} Sep 2 05:00"),
        "{}",
        h.t.screen()
    );
    h.press("right");
    assert!(
        h.t.screen()
            .contains("\u{203a} Sep 1 00:00 \u{2013} Sep 8 00:00"),
        "{}",
        h.t.screen()
    );
}

/// A click on the control that names the tables puts the other one in front,
/// where it is painted and not beside it (oracle: `mouse_tests`, the
/// `PlanWindow` hit).
#[test]
fn clicking_the_plan_window_control_puts_the_other_table_in_front() {
    let mut h = open("plan", 120, 30);
    assert!(
        h.t.screen().contains("\u{203a} 5-hour limits"),
        "{}",
        h.t.screen()
    );
    let (x, y) = find(&h, "5-hour /");
    // Right of the control's last cell: no table changes.
    let width = "\u{2190}/\u{2192} 5-hour / Weekly".chars().count() as u16;
    let (start, _) = find(&h, "\u{2190}/\u{2192}");
    h.t.click(start + width, y);
    h.advance(500);
    assert!(
        h.t.screen().contains("\u{203a} 5-hour limits"),
        "{}",
        h.t.screen()
    );
    h.t.click(x, y);
    h.advance(500);
    let screen = h.t.screen();
    assert!(screen.contains("\u{203a} Weekly limits"), "{screen}");
    assert!(!screen.contains("\u{203a} 5-hour limits"), "{screen}");
    // The cursor of the table left stays.
    h.press("left");
    assert!(
        h.t.screen().contains("\u{203a} 5-hour limits"),
        "{}",
        h.t.screen()
    );
}

/// A click on a period of the report puts the cursor on it, in its table;
/// a second click opens its details and a third closes them.
#[test]
fn clicking_a_period_selects_it_and_a_second_click_opens_it() {
    let mut h = open("plan_many", 100, 30);
    // The second 5-hour period; the first is the one in progress.
    tap(&mut h, "Sep 2 05:00 \u{2013} Sep 2 10:00");
    let screen = h.t.screen();
    assert!(screen.contains("\u{203a} Sep 2 05:00"), "{screen}");
    assert!(!screen.contains("Tasks"), "{screen}");
    tap(&mut h, "Sep 2 05:00 \u{2013} Sep 2 10:00");
    assert!(h.t.screen().contains("Tasks"), "{}", h.t.screen());
    tap(&mut h, "Sep 2 05:00 \u{2013} Sep 2 10:00");
    assert!(!h.t.screen().contains("Tasks"), "{}", h.t.screen());
    // A period of the other table takes the front.
    tap(&mut h, "Sep 1 00:00 \u{2013} Sep 8 00:00");
    let screen = h.t.screen();
    assert!(screen.contains("\u{203a} Weekly limits"), "{screen}");
}

/// `enter details` is offered where a period is under the cursor (oracle:
/// `footer_lines`, `plan_details`).
#[test]
fn the_details_hint_needs_a_period_under_the_cursor() {
    let h = open("plan", 120, 30);
    assert!(h.t.screen().contains("enter details"), "{}", h.t.screen());
    let h = open("plan_empty", 120, 30);
    assert!(!h.t.screen().contains("details"), "{}", h.t.screen());
}

/// The dashboard's Plan card shows the first period of each table and the
/// notes, no details (oracle: `plan_lines` of an unzoomed view).
#[test]
fn the_plan_card_shows_the_first_period_of_each_table() {
    let mut h = open("plan_many", 144, 64);
    h.press("z");
    let screen = h.t.screen();
    assert!(screen.contains("Plan \u{2500}"), "{screen}");
    assert_eq!(screen.matches("Sep 2 05:00").count(), 0, "{screen}");
    assert!(
        screen.contains("Sep 2 10:00 \u{2013} Sep 2 15:00 *"),
        "{screen}"
    );
    assert!(screen.contains("Usage as of Sep 2 12:00 UTC"), "{screen}");
}
