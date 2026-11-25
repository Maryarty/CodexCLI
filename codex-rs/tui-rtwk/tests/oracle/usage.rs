//! Shard `usage` (screens S20, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen, snap_rows};

// ---- oracle: analytics/account_layout_tests.rs (1 screen-level tests here; 11 state tests left to routes-lists)
// ORACLE analytics/account_layout_tests.rs:423 navigation_keeps_its_row_when_sections_change kind=ui owner=routes-lists
#[test]
fn navigation_keeps_its_row_when_sections_change() {
    // the row of the tabs (the line holding `Overview` and `Usage`) is the same in every section,
    // at 64 and at 120 cells
    for width in [64, 120] {
        let mut s = Screen::build(Owner::RoutesLists)
            .size(width, 40)
            .entry("usage", "dashboard")
            .open();
        let mut rows = Vec::new();
        for keys in ["1", "2", "6", "1"] {
            s.keys(keys);
            rows.push(
                s.rows()
                    .iter()
                    .position(|row| row.contains("Overview") && row.contains("Usage"))
                    .unwrap_or_else(|| panic!("no tabs row at {width}:\n{}", s.text())),
            );
        }
        kit::assert_same(rows.clone(), vec![rows[0]; 4], "the tabs row");
    }
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
    // today 2026-05-29, one token on the 25th and four on the 29th: the empty and the active
    // cells differ (`□`, `■`), and the legend sits under the grid; a graph 22 cells wide
    let s = Screen::build(Owner::RoutesLists)
        .size(24, 70)
        .entry("usage", "activity_daily")
        .open();
    s.assert_chart(
        &[
            "     Apr     May",
            "Su □ □ □ □ □ □ □ □ □",
            "Mo □ □ □ □ □ □ □ □ ■",
            "Tu □ □ □ □ □ □ □ □ □",
            "We □ □ □ □ □ □ □ □ □",
            "Th □ □ □ □ □ □ □ □ □",
            "Fr □ □ □ □ □ □ □ □ ■",
            "Sa □ □ □ □ □ □ □ □",
            "",
            "  Less □ ■ ■ ■ ■ More",
        ],
        false,
    );
}

// ORACLE analytics/activity_chart_tests.rs:100 daily_graph_snapshot_stays_left_aligned_in_wide_terminal kind=snap owner=routes-lists
#[test]
fn daily_graph_snapshot_stays_left_aligned_in_wide_terminal() {
    // no history on a 160-cell terminal: the graph is 107 cells wide and sits at the left; the
    // months row, the first weekday row and the legend are the oracle's
    let s = Screen::build(Owner::RoutesLists)
        .size(162, 40)
        .entry("usage", "activity_empty")
        .open();
    s.assert_chart(
        &[
            "   Jun       Jul     Aug       Sep     Oct     Nov       Dec     Jan     Feb     Mar       Apr     May",
            "Su □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □ □",
        ],
        false,
    );
    s.assert_contains("Less □ ■ ■ ■ ■ More");
}

// ORACLE analytics/activity_chart_tests.rs:121 weekly_graph_snapshot_renders_bar_chart_and_caption kind=snap owner=routes-lists
#[test]
fn weekly_graph_snapshot_renders_bar_chart_and_caption() {
    // three Mondays with 3, 6 and 9 tokens: the weekly bars (one column a week) and the caption `Each column = 1 week · tallest 9`; a graph 22 cells wide
    let s = Screen::build(Owner::RoutesLists)
        .size(24, 70)
        .entry("usage", "activity_weekly")
        .open();
    s.assert_chart(
        &[
            "      Apr     May",
            "max                 █",
            "                    █",
            "                  █ █",
            "                  █ █",
            "                █ █ █",
            "                █ █ █",
            "  0             █ █ █",
            "",
            "   Each column = 1 week · tallest 9",
        ],
        true,
    );
}

// ORACLE analytics/activity_chart_tests.rs:165 cumulative_graph_snapshot_renders_running_total_bar_chart_and_caption kind=snap owner=routes-lists
#[test]
fn cumulative_graph_snapshot_renders_running_total_bar_chart_and_caption() {
    // three Mondays with 3, 6 and 9 tokens: the running total (the same weeks added up) and the caption `Running total · top 18`; a graph 22 cells wide
    let s = Screen::build(Owner::RoutesLists)
        .size(24, 70)
        .entry("usage", "activity_cumulative")
        .open();
    s.assert_chart(
        &[
            "      Apr     May",
            "max                 █",
            "                    █",
            "                    █",
            "                  █ █",
            "                  █ █",
            "                █ █ █",
            "  0             █ █ █",
            "",
            "   Running total · top 18",
        ],
        true,
    );
}

// ---- oracle: analytics/chat_panel_tests.rs (2 screen-level tests here; 5 state tests left to routes-lists)
// ORACLE analytics/chat_panel_tests.rs:55 chats_breakdown_ranks_nonzero_groups_and_reflows kind=snap owner=routes-lists
#[test]
fn chats_breakdown_ranks_nonzero_groups_and_reflows() {
    // an Enterprise account's first chat open: its groups ranked by credits (the zero one hidden,
    // the one without a speed `Not reported`, the refund exact) as a table at 110 cells, two rows
    // a group at 58, then with the zero-credit group shown by `a`; the oracle's snapshot joins the
    // three screens (32 rows each) with a blank row
    let name = "chats_breakdown_ranks_nonzero_groups_and_reflows";
    let mut s = Screen::build(Owner::RoutesLists)
        .size(110, 32)
        .entry("usage", "chats")
        .open();
    s.keys("enter");
    s.assert_snap_state_rows(name, 0, 32);
    s.resize(58, 32);
    s.assert_snap_state_rows(name, 33, 32);
    s.resize(110, 32);
    s.keys("a");
    s.assert_snap_state_rows(name, 66, 32);
}

// ORACLE analytics/chat_panel_tests.rs:109 chats_long_title_keeps_detail_indent_and_counts_missing_estimates kind=snap owner=routes-lists
#[test]
fn chats_long_title_keeps_detail_indent_and_counts_missing_estimates() {
    // a title the row cuts is written whole under the rule, and the chats without an estimate
    // are counted
    let mut s = Screen::build(Owner::RoutesLists)
        .size(110, 30)
        .entry("usage", "chats_long")
        .open();
    s.keys("enter");
    s.assert_snap_state_rows(
        "chats_long_title_keeps_detail_indent_and_counts_missing_estimates",
        0,
        30,
    );
}

// ---- oracle: analytics/client_tests.rs (1 screen-level tests here; 19 state tests left to routes-lists)
// ORACLE analytics/client_tests.rs:800 live_account_identity_is_visible_and_closing_aborts_pending_loads kind=snap owner=routes-lists snap=live_account_identity_is_visible_and_closing_aborts_pending_loads
#[test]
fn live_account_identity_is_visible_and_closing_aborts_pending_loads() {
    // the page names the signed-in account in its heading before the plan is known, the failed
    // Usage report is not drawn, and the page says how to leave and to retry; closing the page
    // aborts its pending loads: core/tests/usage/state.rs
    // `a_refresh_starts_a_session_and_a_close_drops_every_load`
    let s = Screen::build(Owner::RoutesLists)
        .size(100, 24)
        .entry("usage", "account_label")
        .open();
    s.assert_contains("Usage \u{b7} analytics@example.test");
    s.assert_contains("No history has been reported.");
    s.assert_contains("esc back");
    s.assert_contains("R retry");
    s.assert_snap("live_account_identity_is_visible_and_closing_aborts_pending_loads");
}

// ---- oracle: analytics/compact_tests.rs (1 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE analytics/compact_tests.rs:5 report_headlines_and_axes_compact_large_values kind=ui owner=routes-lists
#[test]
fn report_headlines_and_axes_compact_large_values() {
    // a day of 12,280,365,226 tokens, 9,749 calls and 2,901 uses: the headline compacts, the day's
    // detail keeps the full amount
    for (key, headline, full) in [
        ("3", "12.3B tokens", "12,280,365,226"),
        ("4", "9.7K calls", "9,749"),
        ("5", "2.9K uses", "2,901"),
    ] {
        let mut s = Screen::build(Owner::RoutesLists)
            .size(100, 30)
            .entry("usage", "large_values")
            .open();
        s.keys(key);
        s.assert_contains(headline);
        s.assert_contains(full);
    }
}

// ---- oracle: analytics/connection_tests.rs (1 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/connection_tests.rs:183 account_lookup_failure_recovers_on_refresh_with_the_server_plan kind=ui owner=routes-lists
#[test]
fn account_lookup_failure_recovers_on_refresh_with_the_server_plan() {
    // the lookup fails (the page says so, no tabs), `R` asks again and the server's plan (an
    // Enterprise account: Overview, Credits, Tokens, Plugins, Skills) gives the page its tabs and
    // every report its load (the page's own form: doors.rs `usage_host`)
    let mut s = Screen::build(Owner::RoutesLists)
        .size(100, 24)
        .dir("usage")
        .usage_accounts(&[
            Err("503 Service Unavailable"),
            Ok(r#"{"plan":"enterprise","today":"2026-09-09"}"#),
        ])
        .usage_report(Ok(r#"{"history":null}"#))
        .open();
    s.assert_contains("503 Service Unavailable");
    s.assert_absent("Credits");
    s.keys("R");
    s.wait_for("Credits");
    s.assert_absent("503 Service Unavailable");
    s.assert_in_order(&["Overview", "Credits", "Tokens", "Plugins", "Skills"]);
    s.assert_absent("Chats");
    // every report the account shows has its answer: none is still loading
    for key in ["2", "3", "4", "5"] {
        s.keys(key);
        s.assert_absent("Loading");
    }
}

// ---- oracle: analytics/consumer_refresh_tests.rs (1 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/consumer_refresh_tests.rs:9 consumer_chats_keep_missing_rows_and_only_offer_known_metrics kind=snap owner=routes-lists
#[test]
fn consumer_chats_keep_missing_rows_and_only_offer_known_metrics() {
    // the chats as the test opens them (`chats_balance`): the refund first (it is the one with a
    // balance), the rows without usage after it with their titles hidden (`Chat usage
    // unavailable`), and their details never open; the final
    // snapshot is the same report with all three metrics (`tasks`, 110x30). The test's amount
    // formatting table is a state test (`task_panel::amount`), not a screen's.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(90, 30)
        .entry("usage", "chats_balance")
        .open();
    s.assert_in_order(&[
        "Refunded task",
        "Chat usage unavailable",
        "Chat usage unavailable",
    ]);
    s.assert_text_count("Chat usage unavailable", 2);
    s.assert_absent("Private title");
    s.assert_absent("Unverified backend row");
    s.keys("down enter");
    s.assert_absent("Model / effort / speed");
    s.assert_absent("Private title");
    s.keys("up enter");
    s.assert_contains("Model / effort / speed");
    s.assert_contains("-0.000004");
    let mut s = Screen::build(Owner::RoutesLists)
        .size(110, 30)
        .entry("usage", "tasks")
        .open();
    s.keys("enter");
    s.assert_snap_state_rows("consumer_chat_all_metrics", 0, 30);
    s.keys("s");
    s.assert_contains("s sort: 5-hour %");
}

// ---- oracle: analytics/dashboard_tests.rs (1 screen-level tests here; 2 state tests left to routes-lists)
// ORACLE analytics/dashboard_tests.rs:33 dashboard_cards_align_and_keep_stable_summary_heights kind=snap owner=routes-lists
#[test]
fn dashboard_cards_align_and_keep_stable_summary_heights() {
    // `z` shows the cards: two columns at 144 (the oracle's snapshot holds the wide screen, a
    // blank row, then the narrow one at 72x48 with the day moved and the Plugins report
    // refused); the cards keep their edges whatever their report holds; Enter focuses the
    // report again with its section, day and grouping
    let name = "dashboard_cards_align_and_keep_stable_summary_heights";
    let screen = |entry: &'static str, width, height| {
        Screen::build(Owner::RoutesLists)
            .size(width, height)
            .entry("usage", entry)
            .open()
    };
    let mut wide = screen("dashboard", 144, 64);
    wide.keys("z");
    for (left, right) in [
        ("1 Overview", "2 Usage"),
        ("3 Messages", "4 Plugins"),
        ("5 Skills", "6 Chats"),
    ] {
        wide.assert_row_with_both(left, right);
    }
    wide.assert_absent("Coverage from");
    wide.assert_snap_state_rows(name, 0, 64);
    // the edges of the cards: the same before the Plugins report fails and after
    let edges = |entry: &'static str, keys: &str| {
        let mut s = screen(entry, 140, 56);
        s.keys(keys);
        s.card_edge_rows()
    };
    kit::assert_same(
        edges("dashboard", "z"),
        edges("dashboard_denied", "z left"),
        "the card edges",
    );
    let mut narrow = screen("dashboard_denied", 72, 48);
    narrow.keys("z left");
    narrow.assert_contains("1 Overview");
    narrow.assert_snap_state_rows(name, 65, 48);
    narrow.keys("enter");
    narrow.assert_absent("Dashboard \u{b7}");
    narrow.assert_contains("Sep 1 \u{b7} 100 usage units");
}

// ---- oracle: analytics/navigation_tests.rs (2 screen-level tests here; 7 state tests left to routes-lists)
// ORACLE analytics/navigation_tests.rs:237 configured_question_mark_keeps_its_action_and_hides_help_hint kind=ui owner=routes-lists
#[test]
fn configured_question_mark_keeps_its_action_and_hides_help_hint() {
    // the oracle's `?` bound to the list's cancel (and its Shift spelling): the footer hints no
    // help, the help page has no `toggle this help`, and the key closes the page; the page's
    // own form of it is doors.rs `kit_usage::a_configured_question_mark_cancels_and_hides_its_help_hint`
    for binding in ["?", "shift+?"] {
        let keymap: &'static [(&'static str, &'static str)] = if binding == "?" {
            &[("list.cancel", "?")]
        } else {
            &[("list.cancel", "shift+?")]
        };
        let mut s = Screen::build(Owner::RoutesLists)
            .size(120, 32)
            .entry("usage", "credits_refund")
            .keymap(keymap)
            .open();
        s.assert_contains("Product");
        s.assert_absent("? help");
        s.assert_absent("toggle this help");
        s.keys("?");
        s.assert_absent("Product");
        s.assert_absent("toggle this help");
    }
}

// ORACLE analytics/navigation_tests.rs:301 horizontal_aliases_ignore_modifiers_and_release_but_accept_repeat kind=ui owner=routes-lists
#[test]
fn horizontal_aliases_ignore_modifiers_and_release_but_accept_repeat() {
    // `h` / `l` move the selected day only as a plain press or a repeat: Alt, Shift,
    // Ctrl+Alt and the release of the key leave it; the day is the row `<date> · N usage units`
    let day = |s: &Screen| -> String {
        s.rows()
            .iter()
            .find(|row| row.contains(" \u{b7} ") && row.contains(" usage units"))
            .unwrap_or_else(|| panic!("no selected day row:\n{}", s.text()))
            .trim()
            .to_string()
    };
    let mut s = Screen::build(Owner::RoutesLists)
        .size(100, 40)
        .entry("usage", "dashboard")
        .open();
    s.keys("2 home right right right");
    let third = day(&s);
    for (alias, other) in [("h", "l"), ("l", "h")] {
        for keys in [
            format!("alt+{alias}"),
            format!("shift+{alias}"),
            format!("ctrl+alt+{alias}"),
        ] {
            s.keys(&keys);
            kit::assert_same(day(&s), third.clone(), &keys);
        }
        s.key_release(alias);
        kit::assert_same(day(&s), third.clone(), "the release");
        s.key_repeat(alias);
        assert_ne!(day(&s), third, "the repeat of `{alias}` moves the day");
        s.key_repeat(other);
        kit::assert_same(day(&s), third.clone(), "the repeat back");
    }
}

// ---- oracle: analytics/plan_tests.rs (6 screen-level tests here; 2 state tests left to routes-lists)
// ORACLE analytics/plan_tests.rs:33 resize_keeps_visible_weekly_selection_and_respects_manual_scroll kind=ui owner=routes-lists
#[test]
fn resize_keeps_visible_weekly_selection_and_respects_manual_scroll() {
    kit::pending(
        Owner::RoutesLists,
        "the Plan report does not follow its cursor row (G-new-usage-1: no ensure_visible of a row from a State, tests/pages/usage.rs `the_plan_report_follows_the_cursor_row` is its ignored page test): the test's resize and scroll steps keep the selected weekly row in view, which a 24-row page cannot do; the entry `plan_many` holds the history",
    );
}

// ORACLE analytics/plan_tests.rs:120 period_navigation_preserves_unknowns_and_resets_on_new_snapshot kind=snap owner=routes-lists snap=period_navigation_preserves_unknowns_and_resets_on_new_snapshot
#[test]
fn period_navigation_preserves_unknowns_and_resets_on_new_snapshot() {
    // the Plan report in front, both tables open to their first period (a 5-hour period with an
    // incomplete day and its breakdown, a weekly one without): the oracle's snapshot joins the
    // 120x30 screen and the 58x30 one with a blank row
    let name = "period_navigation_preserves_unknowns_and_resets_on_new_snapshot";
    let mut s = Screen::build(Owner::RoutesLists)
        .size(120, 30)
        .entry("usage", "plan")
        .open();
    s.keys("enter right enter");
    s.assert_snap_state_rows(name, 0, 30);
    s.resize(58, 30);
    s.assert_snap_state_rows(name, 31, 30);
}

// ORACLE analytics/plan_tests.rs:156 unavailable_empty_and_zero_history_are_distinct kind=snap owner=routes-lists snap=unavailable_empty_and_zero_history_are_distinct
#[test]
fn unavailable_empty_and_zero_history_are_distinct() {
    // the history is not loaded, has no periods, used nothing, or the period has not started:
    // four bodies at 100 cells, one under the other in the oracle's snapshot
    let name = "unavailable_empty_and_zero_history_are_distinct";
    let mut states: Vec<Vec<String>> = Vec::new();
    for row in snap_rows(name, "") {
        if row.starts_with("\u{203a} 5-hour limits") {
            states.push(Vec::new());
        }
        if let Some(state) = states.last_mut() {
            state.push(row);
        }
    }
    kit::assert_same(states.len(), 4, "the oracle's four states");
    for (mut want, entry) in
        states
            .into_iter()
            .zip(["plan_unavailable", "plan_empty", "plan_zero", "plan_future"])
    {
        while want.last().is_some_and(|row| row.trim().is_empty()) {
            want.pop();
        }
        let s = Screen::build(Owner::RoutesLists)
            .size(102, 30)
            .entry("usage", entry)
            .open();
        s.assert_body_from("\u{203a} 5-hour limits", &want, entry);
    }
}

// ORACLE analytics/plan_tests.rs:277 plan_escape_collapses_details_before_closing kind=ui owner=routes-lists
#[test]
fn plan_escape_collapses_details_before_closing() {
    // Esc closes the open period's details first (`Tasks` is in them), then the page
    let mut s = Screen::build(Owner::RoutesLists)
        .size(120, 30)
        .entry("usage", "plan")
        .open();
    s.keys("enter");
    s.assert_contains("Tasks");
    s.keys("esc");
    s.assert_absent("Tasks");
    s.assert_contains("5-hour limits");
    s.keys("esc");
    s.assert_absent("5-hour limits");
}

// ORACLE analytics/plan_tests.rs:291 moving_period_selection_collapses_its_previous_details kind=ui owner=routes-lists
#[test]
fn moving_period_selection_collapses_its_previous_details() {
    // moving to another period closes the open one; the other table's key moves to its table and
    // leaves the cursor of this one where it was; Tab leaves the report and Shift+Tab comes back
    let mut s = Screen::build(Owner::RoutesLists)
        .size(120, 30)
        .entry("usage", "plan_many")
        .open();
    s.keys("enter");
    s.assert_contains("Tasks");
    s.keys("down");
    s.assert_absent("Tasks");
    s.assert_contains("\u{203a} Sep 2 05:00");
    s.keys("right");
    s.assert_contains("\u{203a} Weekly limits");
    s.assert_absent("  \u{203a} 5-hour limits");
    s.keys("tab");
    s.assert_absent("5-hour limits");
    s.keys("shift+tab");
    s.assert_contains("\u{203a} Weekly limits");
    s.assert_contains("\u{203a} Sep 2 05:00");
}

// ORACLE analytics/plan_tests.rs:348 plan_clocks_follow_preference_and_keep_utc kind=snap owner=routes-lists snap=plan_clocks_twelve_hour
#[test]
fn plan_clocks_follow_preference_and_keep_utc() {
    // times follow the clock preference (12-hour here) and stay UTC: a body 80 cells wide
    let name = "plan_clocks_twelve_hour";
    let s = Screen::build(Owner::RoutesLists)
        .size(82, 30)
        .entry("usage", "plan_twelve_hour")
        .open();
    s.assert_body_from("\u{203a} 5-hour limits", &snap_rows(name, ""), name);
}

// ---- oracle: analytics/plot_tests.rs (10 screen-level tests here; 4 state tests left to routes-lists)
// ORACLE analytics/plot_tests.rs:178 missing_zero_and_narrow_charts_keep_the_selection_visible kind=snap owner=routes-lists snap=missing_zero_and_narrow_charts_keep_the_selection_visible
#[test]
fn missing_zero_and_narrow_charts_keep_the_selection_visible() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::missing_zero_and_narrow_charts_keep_the_selection_visible runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:243 zero_credit_history_uses_a_readable_scale kind=snap owner=routes-lists snap=zero_credit_history_uses_a_readable_scale
#[test]
fn zero_credit_history_uses_a_readable_scale() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::zero_credit_history_uses_a_readable_scale runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:271 partial_cells_use_the_midpoint_and_descend_from_the_baseline kind=snap owner=routes-lists snap=partial_cells_use_the_midpoint_and_descend_from_the_baseline
#[test]
fn partial_cells_use_the_midpoint_and_descend_from_the_baseline() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::partial_cells_use_the_midpoint_and_descend_from_the_baseline runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:313 oversized_axes_are_hidden_instead_of_truncated kind=snap owner=routes-lists snap=oversized_axes_are_hidden_instead_of_truncated
#[test]
fn oversized_axes_are_hidden_instead_of_truncated() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::oversized_axes_are_hidden_instead_of_truncated runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:348 selected_callout_keeps_the_authoritative_daily_total kind=snap owner=routes-lists snap=selected_callout_keeps_the_authoritative_daily_total
#[test]
fn selected_callout_keeps_the_authoritative_daily_total() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::selected_callout_keeps_the_authoritative_daily_total runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:369 narrow_long_range_charts_never_truncate_dates kind=snap owner=routes-lists snap=narrow_long_range_charts_never_truncate_dates
#[test]
fn narrow_long_range_charts_never_truncate_dates() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::narrow_long_range_charts_never_truncate_dates runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:401 tiny_segments_keep_their_series_color_beside_a_large_day kind=snap owner=routes-lists snap=tiny_segments_keep_their_series_color_beside_a_large_day
#[test]
fn tiny_segments_keep_their_series_color_beside_a_large_day() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::tiny_segments_keep_their_series_color_beside_a_large_day runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:442 duplicate_message_dates_render_the_full_other_remainder kind=snap owner=routes-lists snap=duplicate_message_dates_render_the_full_other_remainder
#[test]
fn duplicate_message_dates_render_the_full_other_remainder() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::duplicate_message_dates_render_the_full_other_remainder runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:472 seven_day_totals_render_without_selecting_each_bar kind=snap owner=routes-lists snap=seven_day_totals
#[test]
fn seven_day_totals_render_without_selecting_each_bar() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::seven_day_totals_render_without_selecting_each_bar runs the same case against the oracle's snapshot",
    );
}

// ORACLE analytics/plot_tests.rs:501 narrow_equal_height_totals_remain_separate_numbers kind=snap owner=routes-lists snap=narrow_equal_height_totals_remain_separate_numbers
#[test]
fn narrow_equal_height_totals_remain_separate_numbers() {
    kit::na(
        Owner::RoutesLists,
        "a state test: the oracle calls `chart(days, categories, cursor, width, height, unit)` at the plot's own size and compares its cells and palette runs with the snapshot; the page lays the plot out in the report's room, so no screen holds that size. core/tests/usage/plot.rs::narrow_equal_height_totals_remain_separate_numbers runs the same case against the oracle's snapshot",
    );
}

// ---- oracle: analytics/recovery_tests.rs (2 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/recovery_tests.rs:19 twelve_hour_metadata_keeps_utc_across_renderers kind=snap owner=routes-lists
#[test]
fn twelve_hour_metadata_keeps_utc_across_renderers() {
    // the 12-hour clock changes the spelling of the time, never its zone: the controls, the
    // report's own line, the help and the Chats list say `4:00 PM UTC`
    let s = Screen::build(Owner::RoutesLists)
        .size(40, 16)
        .entry("usage", "business_updated_12h")
        .open();
    s.assert_contains(" Updated \u{b7} Sep 2 4:00 PM UTC");
    // the card keeps its time row when the screen leaves it room (the oracle draws the card at
    // its own size): a wide, tall screen shows the card and the help whole
    let mut s = Screen::build(Owner::RoutesLists)
        .size(144, 64)
        .entry("usage", "business_updated_12h")
        .open();
    s.keys("z");
    s.assert_contains("Updated Sep 2 4:00 PM UTC");
    s.keys("?");
    s.assert_contains("Report updated Sep 2 4:00 PM UTC");
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 20)
        .entry("usage", "tasks_updated_12h")
        .open();
    s.assert_contains("Updated Sep 2 4:00 PM UTC \u{b7} recent activity may be delayed");
}

// ORACLE analytics/recovery_tests.rs:55 tall_reports_use_available_height_and_wide_reports_put_details_beside_plot kind=snap owner=routes-lists
#[test]
fn tall_reports_use_available_height_and_wide_reports_put_details_beside_plot() {
    // a Team account's Tokens report fills a tall screen (more than 14 plot rows) and shows the
    // day's 731,000 tokens without a scroll hint; at 180 cells the details sit beside the plot
    // (the `4.4M tokens` headline row has a rule) and the screen is the oracle's snapshot
    for (width, height) in [(80, 60), (180, 60)] {
        let s = Screen::build(Owner::RoutesLists)
            .size(width, height)
            .entry("usage", "business_tokens")
            .open();
        let plot_rows = s.rows().iter().filter(|row| row.contains('\u{2588}')).count();
        assert!(plot_rows > 14, "only {plot_rows} plot rows at {width} columns");
        s.assert_contains("731,000 tokens");
        s.assert_absent("scroll");
        if width == 180 {
            assert!(
                s.rows()
                    .iter()
                    .any(|row| row.contains("4.4M tokens") && row.contains('\u{2500}')),
                "no headline row with a rule:\n{}",
                s.text()
            );
            s.assert_snap("tall_reports_use_available_height_and_wide_reports_put_details_beside_plot");
        }
    }
}

// ---- oracle: analytics/redesign_tests.rs (2 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE analytics/redesign_tests.rs:6 reports_keep_tabs_fixed_and_expose_data_on_small_screens kind=snap owner=routes-lists
#[test]
fn reports_keep_tabs_fixed_and_expose_data_on_small_screens() {
    // the tab rows stay where they are when the Tokens report fails (`business_tokens_failed`),
    // whatever the size; the report names its data (`tokens`, `Sep 2`), the failure is the
    // report's own text, and at 40x16 the oracle's snapshot joins the two screens
    let name = "reports_keep_tabs_fixed_and_expose_data_on_small_screens";
    for (width, height) in [(120, 36), (80, 24), (40, 16)] {
        let screen = |entry: &'static str| {
            Screen::build(Owner::RoutesLists)
                .size(width, height)
                .entry("usage", entry)
                .open()
        };
        let tokens = screen("business_tokens");
        tokens.assert_contains("tokens");
        tokens.assert_contains("Sep 2");
        let failed = screen("business_tokens_failed");
        failed.assert_contains("Couldn't load tokens.");
        kit::assert_same(
            failed.rows().into_iter().take(3).collect::<Vec<_>>(),
            tokens.rows().into_iter().take(3).collect::<Vec<_>>(),
            "the tab rows",
        );
        if width == 40 {
            tokens.assert_snap_state_rows(name, 0, 16);
            failed.assert_snap_state_rows(name, 17, 16);
        }
    }
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
    // the profile failed (the upstream error is never shown) and holds zeros: the Overview keeps
    // the zero values and the missing ones apart, 100x40
    let s = Screen::build(Owner::RoutesLists)
        .size(100, 40)
        .entry("usage", "summary_partial")
        .open();
    s.assert_absent("sensitive upstream error");
    s.assert_snap("summary_partial");
}

// ---- oracle: analytics_tests.rs (7 screen-level tests here; 7 state tests left to routes-lists)
// ORACLE analytics_tests.rs:65 analytics_narrow_charts_and_small_terminals_keep_valid_cursors kind=ui owner=routes-lists
#[test]
fn analytics_narrow_charts_and_small_terminals_keep_valid_cursors() {
    // the Usage, Messages and Plugins reports over 30 days at 28x12: the cursor walks from the
    // first day to the last (Sep 2) and stays there; then tiny sizes draw without a panic
    let mut s = Screen::build(Owner::RoutesLists)
        .size(28, 12)
        .entry("usage", "dashboard_month")
        .open();
    for section in ["2", "3", "4"] {
        s.keys(section);
        s.keys("home");
        for _ in 0..30 {
            s.keys("right");
        }
        s.assert_contains("Sep 2");
        s.keys("right");
        s.assert_contains("Sep 2");
    }
    for _ in 0..2 {
        for width in [1, 4, 32, 107, 108] {
            s.resize(width, 1);
        }
    }
    s.resize(0, 0);
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
                s.assert_drawn(now.0.len(), "bars");
                if let Some(previous) = &previous {
                    s.assert_same_frame(
                        previous,
                        &now,
                        &format!("width {width} (expanded {expanded})"),
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
