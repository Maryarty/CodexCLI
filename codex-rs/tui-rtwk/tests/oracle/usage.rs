//! Shard `usage` (screens S20, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: analytics/account_layout_tests.rs (1 screen-level tests here; 11 state tests left to routes-lists)
// ORACLE analytics/account_layout_tests.rs:423 navigation_keeps_its_row_when_sections_change kind=ui owner=routes-lists
#[test]
fn navigation_keeps_its_row_when_sections_change() {
    kit::pending(
        Owner::RoutesLists,
        "the sections-change test needs an account that changes plan under an open page (no entry with that data)",
    );
}

// ---- oracle: analytics/activity_chart/palette_tests.rs (1 screen-level tests here; 5 state tests left to routes-lists)
// ORACLE analytics/activity_chart/palette_tests.rs:7 current_palette_uses_effective_terminal_color_level kind=snap owner=routes-lists snap=current_palette_uses_effective_terminal_color_level
#[test]
fn current_palette_uses_effective_terminal_color_level() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's activity data (the chart is drawn from the test's own history)",
    );
}

// ---- oracle: analytics/activity_chart_tests.rs (4 screen-level tests here; 4 state tests left to routes-lists)
// ORACLE analytics/activity_chart_tests.rs:65 daily_graph_snapshot_uses_distinct_empty_and_active_cells kind=snap owner=routes-lists
#[test]
fn daily_graph_snapshot_uses_distinct_empty_and_active_cells() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's activity data (the chart is drawn from the test's own history)",
    );
}

// ORACLE analytics/activity_chart_tests.rs:100 daily_graph_snapshot_stays_left_aligned_in_wide_terminal kind=snap owner=routes-lists
#[test]
fn daily_graph_snapshot_stays_left_aligned_in_wide_terminal() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's activity data (the chart is drawn from the test's own history)",
    );
}

// ORACLE analytics/activity_chart_tests.rs:121 weekly_graph_snapshot_renders_bar_chart_and_caption kind=snap owner=routes-lists
#[test]
fn weekly_graph_snapshot_renders_bar_chart_and_caption() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's activity data (the chart is drawn from the test's own history)",
    );
}

// ORACLE analytics/activity_chart_tests.rs:165 cumulative_graph_snapshot_renders_running_total_bar_chart_and_caption kind=snap owner=routes-lists
#[test]
fn cumulative_graph_snapshot_renders_running_total_bar_chart_and_caption() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's activity data (the chart is drawn from the test's own history)",
    );
}

// ---- oracle: analytics/chat_panel_tests.rs (2 screen-level tests here; 5 state tests left to routes-lists)
// ORACLE analytics/chat_panel_tests.rs:55 chats_breakdown_ranks_nonzero_groups_and_reflows kind=snap owner=routes-lists
#[test]
fn chats_breakdown_ranks_nonzero_groups_and_reflows() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the chats body at the test's width with its own listing; no entry has that listing",
    );
}

// ORACLE analytics/chat_panel_tests.rs:109 chats_long_title_keeps_detail_indent_and_counts_missing_estimates kind=snap owner=routes-lists
#[test]
fn chats_long_title_keeps_detail_indent_and_counts_missing_estimates() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the chats body at the test's width with its own listing; no entry has that listing",
    );
}

// ---- oracle: analytics/client_tests.rs (1 screen-level tests here; 19 state tests left to routes-lists)
// ORACLE analytics/client_tests.rs:800 live_account_identity_is_visible_and_closing_aborts_pending_loads kind=snap owner=routes-lists snap=live_account_identity_is_visible_and_closing_aborts_pending_loads
#[test]
fn live_account_identity_is_visible_and_closing_aborts_pending_loads() {
    kit::pending(
        Owner::RoutesLists,
        "the test drives the oracle's live client (identity check, aborting loads); the page's host side answers only with a real sign-in",
    );
}

// ---- oracle: analytics/compact_tests.rs (1 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE analytics/compact_tests.rs:5 report_headlines_and_axes_compact_large_values kind=ui owner=routes-lists
#[test]
fn report_headlines_and_axes_compact_large_values() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's large values",
    );
}

// ---- oracle: analytics/connection_tests.rs (1 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/connection_tests.rs:183 account_lookup_failure_recovers_on_refresh_with_the_server_plan kind=ui owner=routes-lists
#[test]
fn account_lookup_failure_recovers_on_refresh_with_the_server_plan() {
    kit::pending(
        Owner::RoutesLists,
        "the test drives a failing account lookup then a refresh; the fake answers no Usage command",
    );
}

// ---- oracle: analytics/consumer_refresh_tests.rs (1 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/consumer_refresh_tests.rs:9 consumer_chats_keep_missing_rows_and_only_offer_known_metrics kind=snap owner=routes-lists
#[test]
fn consumer_chats_keep_missing_rows_and_only_offer_known_metrics() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the consumer chats listing of the test",
    );
}

// ---- oracle: analytics/dashboard_tests.rs (1 screen-level tests here; 2 state tests left to routes-lists)
// ORACLE analytics/dashboard_tests.rs:33 dashboard_cards_align_and_keep_stable_summary_heights kind=snap owner=routes-lists
#[test]
fn dashboard_cards_align_and_keep_stable_summary_heights() {
    kit::pending(
        Owner::RoutesLists,
        "the snapshot joins the wide and the narrow screen with no label; the kit cannot cut it (tests/pages/usage.rs compares the halves)",
    );
}

// ---- oracle: analytics/navigation_tests.rs (2 screen-level tests here; 7 state tests left to routes-lists)
// ORACLE analytics/navigation_tests.rs:237 configured_question_mark_keeps_its_action_and_hides_help_hint kind=ui owner=routes-lists
#[test]
fn configured_question_mark_keeps_its_action_and_hides_help_hint() {
    kit::pending(
        Owner::RoutesLists,
        "needs a keymap with a configured `?` / key events with modifiers and release; the kit has no release events",
    );
}

// ORACLE analytics/navigation_tests.rs:301 horizontal_aliases_ignore_modifiers_and_release_but_accept_repeat kind=ui owner=routes-lists
#[test]
fn horizontal_aliases_ignore_modifiers_and_release_but_accept_repeat() {
    kit::pending(
        Owner::RoutesLists,
        "needs a keymap with a configured `?` / key events with modifiers and release; the kit has no release events",
    );
}

// ---- oracle: analytics/plan_tests.rs (6 screen-level tests here; 2 state tests left to routes-lists)
// ORACLE analytics/plan_tests.rs:33 resize_keeps_visible_weekly_selection_and_respects_manual_scroll kind=ui owner=routes-lists
#[test]
fn resize_keeps_visible_weekly_selection_and_respects_manual_scroll() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the plan body lines at the test's width, not a screen with chrome; no entry holds the test's history",
    );
}

// ORACLE analytics/plan_tests.rs:120 period_navigation_preserves_unknowns_and_resets_on_new_snapshot kind=snap owner=routes-lists snap=period_navigation_preserves_unknowns_and_resets_on_new_snapshot
#[test]
fn period_navigation_preserves_unknowns_and_resets_on_new_snapshot() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the plan body lines at the test's width, not a screen with chrome; no entry holds the test's history",
    );
}

// ORACLE analytics/plan_tests.rs:156 unavailable_empty_and_zero_history_are_distinct kind=snap owner=routes-lists snap=unavailable_empty_and_zero_history_are_distinct
#[test]
fn unavailable_empty_and_zero_history_are_distinct() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the plan body lines at the test's width, not a screen with chrome; no entry holds the test's history",
    );
}

// ORACLE analytics/plan_tests.rs:277 plan_escape_collapses_details_before_closing kind=ui owner=routes-lists
#[test]
fn plan_escape_collapses_details_before_closing() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the plan body lines at the test's width, not a screen with chrome; no entry holds the test's history",
    );
}

// ORACLE analytics/plan_tests.rs:291 moving_period_selection_collapses_its_previous_details kind=ui owner=routes-lists
#[test]
fn moving_period_selection_collapses_its_previous_details() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the plan body lines at the test's width, not a screen with chrome; no entry holds the test's history",
    );
}

// ORACLE analytics/plan_tests.rs:348 plan_clocks_follow_preference_and_keep_utc kind=snap owner=routes-lists snap=plan_clocks_twelve_hour
#[test]
fn plan_clocks_follow_preference_and_keep_utc() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle snapshot is the plan body lines at the test's width, not a screen with chrome; no entry holds the test's history",
    );
}

// ---- oracle: analytics/plot_tests.rs (10 screen-level tests here; 4 state tests left to routes-lists)
// ORACLE analytics/plot_tests.rs:178 missing_zero_and_narrow_charts_keep_the_selection_visible kind=snap owner=routes-lists snap=missing_zero_and_narrow_charts_keep_the_selection_visible
#[test]
fn missing_zero_and_narrow_charts_keep_the_selection_visible() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:243 zero_credit_history_uses_a_readable_scale kind=snap owner=routes-lists snap=zero_credit_history_uses_a_readable_scale
#[test]
fn zero_credit_history_uses_a_readable_scale() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:271 partial_cells_use_the_midpoint_and_descend_from_the_baseline kind=snap owner=routes-lists snap=partial_cells_use_the_midpoint_and_descend_from_the_baseline
#[test]
fn partial_cells_use_the_midpoint_and_descend_from_the_baseline() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:313 oversized_axes_are_hidden_instead_of_truncated kind=snap owner=routes-lists snap=oversized_axes_are_hidden_instead_of_truncated
#[test]
fn oversized_axes_are_hidden_instead_of_truncated() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:348 selected_callout_keeps_the_authoritative_daily_total kind=snap owner=routes-lists snap=selected_callout_keeps_the_authoritative_daily_total
#[test]
fn selected_callout_keeps_the_authoritative_daily_total() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:369 narrow_long_range_charts_never_truncate_dates kind=snap owner=routes-lists snap=narrow_long_range_charts_never_truncate_dates
#[test]
fn narrow_long_range_charts_never_truncate_dates() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:401 tiny_segments_keep_their_series_color_beside_a_large_day kind=snap owner=routes-lists snap=tiny_segments_keep_their_series_color_beside_a_large_day
#[test]
fn tiny_segments_keep_their_series_color_beside_a_large_day() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:442 duplicate_message_dates_render_the_full_other_remainder kind=snap owner=routes-lists snap=duplicate_message_dates_render_the_full_other_remainder
#[test]
fn duplicate_message_dates_render_the_full_other_remainder() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:472 seven_day_totals_render_without_selecting_each_bar kind=snap owner=routes-lists snap=seven_day_totals
#[test]
fn seven_day_totals_render_without_selecting_each_bar() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ORACLE analytics/plot_tests.rs:501 narrow_equal_height_totals_remain_separate_numbers kind=snap owner=routes-lists snap=narrow_equal_height_totals_remain_separate_numbers
#[test]
fn narrow_equal_height_totals_remain_separate_numbers() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's series; the snapshot is the plot alone",
    );
}

// ---- oracle: analytics/recovery_tests.rs (2 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/recovery_tests.rs:19 twelve_hour_metadata_keeps_utc_across_renderers kind=snap owner=routes-lists
#[test]
fn twelve_hour_metadata_keeps_utc_across_renderers() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's report data and clock preference",
    );
}

// ORACLE analytics/recovery_tests.rs:55 tall_reports_use_available_height_and_wide_reports_put_details_beside_plot kind=snap owner=routes-lists
#[test]
fn tall_reports_use_available_height_and_wide_reports_put_details_beside_plot() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's report data and clock preference",
    );
}

// ---- oracle: analytics/redesign_tests.rs (2 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/redesign_tests.rs:6 reports_keep_tabs_fixed_and_expose_data_on_small_screens kind=snap owner=routes-lists
#[test]
fn reports_keep_tabs_fixed_and_expose_data_on_small_screens() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's report; the help snapshot is not an entry",
    );
}

// ORACLE analytics/redesign_tests.rs:25 keyboard_tabs_and_help_preserve_report_reading_position kind=snap owner=routes-lists
#[test]
fn keyboard_tabs_and_help_preserve_report_reading_position() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's report; the help snapshot is not an entry",
    );
}

// ---- oracle: analytics/styles_tests.rs (2 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE analytics/styles_tests.rs:27 analytics_sections_adapt_to_terminal_colors kind=ui owner=routes-lists
#[test]
fn analytics_sections_adapt_to_terminal_colors() {
    kit::pending(
        Owner::RoutesLists,
        "asserts terminal colour mapping of the test's palette, needs a fixture of every section",
    );
}

// ORACLE analytics/styles_tests.rs:131 chat_selection_keeps_a_highlighted_trailing_blank_after_wrapping kind=ui owner=routes-lists
#[test]
fn chat_selection_keeps_a_highlighted_trailing_blank_after_wrapping() {
    kit::pending(
        Owner::RoutesLists,
        "asserts terminal colour mapping of the test's palette, needs a fixture of every section",
    );
}

// ---- oracle: analytics/summary_tests.rs (2 screen-level tests here; 3 state tests left to routes-lists)
// ORACLE analytics/summary_tests.rs:20 summary_modes_reflow_and_keep_selection_across_tabs kind=snap owner=routes-lists
#[test]
fn summary_modes_reflow_and_keep_selection_across_tabs() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(140, 44)
        .entry("usage", "overview")
        .open();
    s.assert_snap("summary_daily_wide");
    s.keys("g tab shift+tab g");
    s.resize(64, 60);
    s.assert_snap("summary_cumulative_narrow");
    // The oracle's later checks (scroll offset, `is_done`) are state, not drawn here.
}

// ORACLE analytics/summary_tests.rs:55 partial_summary_keeps_missing_values_and_zero kind=snap owner=routes-lists
#[test]
fn partial_summary_keeps_missing_values_and_zero() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the partial profile of the test",
    );
}

// ---- oracle: analytics_tests.rs (7 screen-level tests here; 7 state tests left to routes-lists)
// ORACLE analytics_tests.rs:65 analytics_narrow_charts_and_small_terminals_keep_valid_cursors kind=ui owner=routes-lists
#[test]
fn analytics_narrow_charts_and_small_terminals_keep_valid_cursors() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's data",
    );
}

// ORACLE analytics_tests.rs:128 analytics_loading_errors_and_empty_reports_stay_independent kind=snap owner=routes-lists snap=analytics_loading_errors_and_empty_reports_stay_independent
#[test]
fn analytics_loading_errors_and_empty_reports_stay_independent() {
    let s = Screen::build(Owner::RoutesLists)
        .size(60, 12)
        .entry("usage", "panel_states")
        .open();
    s.assert_snap("analytics_loading_errors_and_empty_reports_stay_independent");
}

// ORACLE analytics_tests.rs:169 analytics_preserves_full_denominators_and_signed_small_credits kind=snap owner=routes-lists snap=analytics_preserves_full_denominators_and_signed_small_credits
#[test]
fn analytics_preserves_full_denominators_and_signed_small_credits() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's data",
    );
}

// ORACLE analytics_tests.rs:267 analytics_dates_distinguish_zero_missing_and_exact_details kind=snap owner=routes-lists snap=analytics_dates_distinguish_zero_missing_and_exact_details
#[test]
fn analytics_dates_distinguish_zero_missing_and_exact_details() {
    kit::pending(
        Owner::RoutesLists,
        "the oracle's snapshot is five `panel(Section, 54, 3)` bodies joined (a function of the model, \
         no screen of the page draws one: the chart is not at that size), and its last two panels \
         are the same report emptied by a test's own edit; the three `contains` of the first \
         three panels are ported below",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(100, 38)
        .entry("usage", "credits_exact")
        .open();
    s.keys("home");
    s.assert_contains("No activity (0)");
    s.keys("right enter");
    s.assert_contains("12746.441206 credits");
    s.keys("right");
    s.assert_contains("Aug 29 · Not reported");
}

// ORACLE analytics_tests.rs:345 analytics_maximized_refunds_leave_room_for_both_bands kind=snap owner=routes-lists snap=analytics_maximized_refunds_leave_room_for_both_bands
#[test]
fn analytics_maximized_refunds_leave_room_for_both_bands() {
    let s = Screen::build(Owner::RoutesLists)
        .size(100, 32)
        .entry("usage", "credits_refund")
        .open();
    s.assert_snap("analytics_maximized_refunds_leave_room_for_both_bands");
}

// ORACLE analytics_tests.rs:367 analytics_day_navigation_keeps_geometry_and_legend kind=ui owner=routes-lists
#[test]
fn analytics_day_navigation_keeps_geometry_and_legend() {
    // The oracle's frame: the bars (cell and style), the marker row and the table row.
    type Geometry = (Vec<(usize, usize, String, String)>, usize, usize);
    fn geometry(s: &Screen) -> Geometry {
        let grid = s.grid();
        let mut bars = Vec::new();
        for y in 0..grid.height {
            for x in 0..grid.width {
                let cell = grid.cell(x, y);
                if matches!(
                    cell.text.as_str(),
                    "█" | "▁" | "▂" | "▃" | "▄" | "▅" | "▆" | "▇"
                ) {
                    bars.push((x, y, cell.text.clone(), format!("{:?}", grid.style(x, y))));
                }
            }
        }
        let rows = s.rows();
        let row_of = |text: &str| {
            rows.iter()
                .position(|row| row.contains(text))
                .unwrap_or_else(|| panic!("no `{text}` row on the screen:\n{}", s.text()))
        };
        (bars, row_of("▲"), row_of("Product"))
    }
    let mut s = Screen::build(Owner::RoutesLists)
        .size(58, 38)
        .entry("usage", "credits_days")
        .open();
    for width in [58, 160] {
        s.resize(width, 38);
        for expanded in [false, true] {
            s.keys("home");
            if expanded {
                s.keys("enter");
            }
            let mut previous: Option<Geometry> = None;
            for _ in 0..7 {
                let now = geometry(&s);
                assert!(!now.0.is_empty(), "no bars drawn:\n{}", s.text());
                if let Some(previous) = &previous {
                    assert!(
                        *previous == now,
                        "the geometry moved at width {width} (expanded {expanded}):\n{}",
                        s.text()
                    );
                }
                previous = Some(now);
                s.assert_contains("● CLI");
                s.assert_contains("● Desktop app");
                s.keys("right");
            }
            if expanded {
                s.keys("enter");
            }
        }
    }
}

// ORACLE analytics_tests.rs:419 analytics_series_rank_by_range_and_keep_calendar_ticks_stable kind=snap owner=routes-lists snap=analytics_series_rank_by_range_and_keep_calendar_ticks_stable
#[test]
fn analytics_series_rank_by_range_and_keep_calendar_ticks_stable() {
    kit::pending(
        Owner::RoutesLists,
        "feature usage debug entry (w2-status, src/pages/usage): the page is built, its slice registers entry missing with the oracle test's data",
    );
}
