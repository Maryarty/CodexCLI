//! Shard `resume_2` (screens S18, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: resume_picker.rs (31 screen-level tests here; 15 state tests left to routes-lists)
// ORACLE resume_picker.rs:6277 toolbar_renders_sort_and_filter_tabs kind=snap owner=routes-lists snap=resume_picker_search_line_sort_filter_tabs
#[test]
fn toolbar_renders_sort_and_filter_tabs() {
    // The oracle's toolbar line at 100 cells, one row; the screen is two cells wider (the chrome is
    // inset by one on each side) and the toolbar is its row 2 at 24 rows.
    let s = Screen::build(Owner::RoutesLists)
        .size(102, 24)
        .entry("resume", "toolbar_repo_cwd")
        .open();
    s.assert_snap_region("resume_picker_search_line_sort_filter_tabs", 2, 1);
}

// ORACLE resume_picker.rs:6393 dense_session_snapshot_omits_cwd_in_cwd_filter kind=snap owner=routes-lists snap=resume_picker_dense_cwd
#[test]
fn dense_session_snapshot_omits_cwd_in_cwd_filter() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(104, 10)
        .entry("resume", "dense_cwd")
        .open();
    s.keys("ctrl+o");
    // The oracle's list is three rows at 100 cells (the screen is four cells wider: the list is
    // inset by two on each side); the list starts at row 3 of a 10-row screen.
    s.assert_snap_region("resume_picker_dense_cwd", 3, 2);
}

// ORACLE resume_picker.rs:6407 dense_session_snapshot_drops_metadata_when_narrow kind=snap owner=routes-lists snap=resume_picker_dense_narrow
#[test]
fn dense_session_snapshot_drops_metadata_when_narrow() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(52, 10)
        .entry("resume", "dense_narrow")
        .open();
    s.keys("ctrl+o");
    s.assert_snap_region("resume_picker_dense_narrow", 3, 2);
}

// ORACLE resume_picker.rs:6417 dense_session_line_prefers_thread_name_over_preview kind=ui owner=routes-lists
#[test]
fn dense_session_line_prefers_thread_name_over_preview() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(100, 16)
        .entry("resume", "dense_named")
        .open();
    s.keys("ctrl+o");
    s.assert_contains("Named session");
    s.assert_absent("Raw conversation preview");
}

// ORACLE resume_picker.rs:6486 comfortable_zebra_lines_use_full_width_background kind=ui owner=routes-lists
#[test]
fn comfortable_zebra_lines_use_full_width_background() {
    let s = Screen::build(Owner::RoutesLists)
        .size(100, 16)
        .entry("resume", "zebra_rows")
        .open();
    // The oracle's two lines of the zebra row, each 100 cells wide with the zebra background.
    s.assert_contains("Zebra comfortable row");
}

// ORACLE resume_picker.rs:6520 dense_session_snapshot_uses_no_blank_lines_between_rows kind=snap owner=routes-lists snap=resume_picker_dense_no_blank_lines
#[test]
fn dense_session_snapshot_uses_no_blank_lines_between_rows() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 9)
        .entry("resume", "dense_two_rows")
        .open();
    s.keys("ctrl+o down");
    // The oracle's two rows at 80 cells, no blank line between them; the list starts at row 3.
    s.assert_snap_region("resume_picker_dense_no_blank_lines", 3, 2);
}

// ORACLE resume_picker.rs:6566 expanded_session_snapshot kind=snap owner=routes-lists snap=resume_picker_expanded_session
#[test]
fn expanded_session_snapshot() {
    kit::pending(
        Owner::RoutesLists,
        "the last row differs only by the link: the oracle writes `docs (https://example.com)` (its display for a terminal not known to support OSC 8, tui web_links.rs WebLinkDisplay::for_terminal); the port draws the web label alone with its hyperlink (`LinkDestinations::Auto`, as the oracle does on a supporting terminal), the same as markdown_streaming_3 url_link_shows_destination; the other ten rows equal the snapshot",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(94, 21)
        .entry("resume", "expanded_session")
        .open();
    s.keys("ctrl+e");
    // The oracle's 11 rows at 90 cells; the list starts at row 6 of a 21-row screen.
    s.assert_snap_region("resume_picker_expanded_session", 6, 2);
}

// ORACLE resume_picker.rs:6641 narrow_session_snapshot kind=snap owner=routes-lists snap=resume_picker_narrow_session
#[test]
fn narrow_session_snapshot() {
    let s = Screen::build(Owner::RoutesLists)
        .size(62, 16)
        .entry("resume", "narrow_session")
        .open();
    // The oracle's six rows at 58 cells; the list starts at row 6 of a 16-row screen.
    s.assert_snap_region("resume_picker_narrow_session", 6, 2);
}

// ORACLE resume_picker.rs:6694 session_list_more_indicators_snapshot kind=snap owner=routes-lists snap=resume_picker_more_indicators
#[test]
fn session_list_more_indicators_snapshot() {
    // 82 cells: the oracle's list is 80 wide, ours is inset by 2.
    // Five rows (item-0..item-4); the oracle selects the third and the list
    // scrolls to it. The list is six rows at row 6 of a 16-row screen.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 16)
        .entry("resume", "more_indicators")
        .open();
    s.keys("down down");
    s.assert_snap_region("resume_picker_more_indicators", 6, 2);
}

// ORACLE resume_picker.rs:6749 density_toggle_clears_stale_more_indicator kind=ui owner=routes-lists
#[test]
fn density_toggle_clears_stale_more_indicator() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "four_rows")
        .open();
    s.assert_contains("↓ more");
    s.keys("ctrl+o");
    s.assert_absent("↓ more");
}

// ORACLE resume_picker.rs:7047 list_viewport_width_matches_rendered_list_inset kind=ui owner=routes-lists
#[test]
fn list_viewport_width_matches_rendered_list_inset() {
    kit::na(
        Owner::RoutesLists,
        "asserts the return values of `list_viewport_width` (76 at 80 cells, 0 at 3 cells), a helper's numbers that are never drawn",
    );
}

// ORACLE resume_picker.rs:7053 toggle_sort_key_reloads_with_new_sort kind=ui owner=routes-lists
#[test]
fn toggle_sort_key_reloads_with_new_sort() {
    kit::na(
        Owner::RoutesLists,
        "asserts the page-load requests the picker sends to the backend (the sort key of each), not rows: a state test",
    );
}

// ORACLE resume_picker.rs:7096 default_filter_focus_arrows_reload_with_new_filter kind=ui owner=routes-lists
#[test]
fn default_filter_focus_arrows_reload_with_new_filter() {
    kit::na(
        Owner::RoutesLists,
        "asserts the page-load requests and the `filter_mode` state field, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:7149 all_filter_can_switch_back_to_cwd_when_cwd_candidate_exists kind=ui owner=routes-lists
#[test]
fn all_filter_can_switch_back_to_cwd_when_cwd_candidate_exists() {
    kit::na(
        Owner::RoutesLists,
        "asserts the page-load requests the picker sends to the backend, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:7184 status_changes_when_directory_filter_is_unavailable kind=ui owner=routes-lists
#[test]
fn status_changes_when_directory_filter_is_unavailable() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "no_cwd_filter")
        .open();
    // The oracle's toolbar at 80 cells with no directory filter has no `Cwd` tab.
    s.assert_absent("Cwd");
    s.keys("right tab right");
    // The oracle then asserts the Active then Archived requests (not drawn; not asserted here).
}

// ORACLE resume_picker.rs:7232 page_navigation_uses_view_rows kind=ui owner=routes-lists
#[test]
fn page_navigation_uses_view_rows() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("resume", "twenty_rows")
        .open();
    s.keys("pagedown");
    s.assert_contains("› item-5");
    s.keys("pagedown");
    s.assert_contains("› item-10");
    s.keys("pageup");
    s.assert_contains("› item-5");
    s.keys("end");
    s.assert_contains("› item-19");
    s.keys("home");
    s.assert_contains("› item-0");
}

// ORACLE resume_picker.rs:7313 page_and_jump_navigation_use_list_keymap kind=ui owner=routes-lists
#[test]
fn page_and_jump_navigation_use_list_keymap() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .keymap(&[
            ("list.page_down", "ctrl+d"),
            ("list.page_up", "ctrl+u"),
            ("list.jump_bottom", "ctrl+y"),
            ("list.jump_top", "ctrl+a"),
        ])
        .entry("resume", "twenty_rows")
        .open();
    s.keys("pagedown");
    s.assert_contains("› item-0");
    s.keys("ctrl+d");
    s.assert_contains("› item-5");
    s.keys("ctrl+u");
    s.assert_contains("› item-0");
    s.keys("ctrl+y");
    s.assert_contains("› item-19");
    s.keys("ctrl+a");
    s.assert_contains("› item-0");
}

// ORACLE resume_picker.rs:7376 ctrl_c_exits_even_when_cancel_is_remapped_to_ctrl_c kind=ui owner=routes-lists
#[test]
fn ctrl_c_exits_even_when_cancel_is_remapped_to_ctrl_c() {
    kit::na(
        Owner::RoutesLists,
        "asserts the `Exit` selection that ctrl+c returns when cancel is remapped to ctrl+c; the exit is the app's and nothing is drawn",
    );
}

// ORACLE resume_picker.rs:7398 end_jumps_to_last_known_row_and_starts_loading_more kind=ui owner=routes-lists
#[test]
fn end_jumps_to_last_known_row_and_starts_loading_more() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("resume", "end_ten_rows")
        .open();
    s.keys("end");
    // The oracle's footer progress label at 80 cells, the last known row selected.
    s.assert_contains(" 10 / 10… · 100% ");
}

// ORACLE resume_picker.rs:7447 enter_on_row_without_resolvable_thread_id_shows_inline_error kind=ui owner=routes-lists
#[test]
fn enter_on_row_without_resolvable_thread_id_shows_inline_error() {
    kit::na(
        Owner::RoutesLists,
        "a listed row always carries its thread id (core `Row.id` is not optional, the protocol's thread has one), so a row without one cannot exist in the port; the nearest state is core's `a_row_that_is_not_listed_opens_nothing_and_says_so`",
    );
}

// ORACLE resume_picker.rs:7489 enter_on_pathless_thread_uses_thread_id kind=ui owner=routes-lists
#[test]
fn enter_on_pathless_thread_uses_thread_id() {
    kit::na(
        Owner::RoutesLists,
        "asserts the `Resume` selection Enter returns (thread id, no path, no cwd, no history mode), not drawn: a state test",
    );
}

// ORACLE resume_picker.rs:7580 thread_to_transcript_cells_renders_core_message_types kind=snap owner=routes-lists
#[test]
fn thread_to_transcript_cells_renders_core_message_types() {
    kit::pending(
        Owner::RoutesLists,
        "no thread with turns (a user message, an agent message, a plan, an error turn) to resume into a transcript; `thread_to_transcript_cells` is not built and the picker has no history seed",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 24)
        .entry("resume", "thread_turns")
        .open();
    s.keys("enter");
    s.assert_contains("hello from user");
    s.assert_contains("hello from assistant");
    s.assert_contains("Proposed Plan");
    s.assert_contains("Do the thing");
    // The oracle's last transcript line at 80 cells.
    s.assert_contains("■ Connection lost");
}

// ORACLE resume_picker.rs:7763 thread_to_transcript_cells_retains_summary_and_raw_reasoning_when_enabled kind=ui owner=routes-lists
#[test]
fn thread_to_transcript_cells_retains_summary_and_raw_reasoning_when_enabled() {
    kit::pending(
        Owner::RoutesLists,
        "no thread with a reasoning turn (summary `public summary`, raw content `raw reasoning content`) to resume into a transcript with raw reasoning visible",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 24)
        .entry("resume", "reasoning_turn")
        .open();
    s.keys("enter");
    // The oracle's transcript lines at 80 cells: the summary, a blank line, the raw content.
    s.assert_in_order(&["• public summary", "raw reasoning content"]);
}

// ORACLE resume_picker.rs:7832 moving_to_last_card_scrolls_when_cards_exceed_viewport kind=ui owner=routes-lists
#[test]
fn moving_to_last_card_scrolls_when_cards_exceed_viewport() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("resume", "three_rows")
        .open();
    s.keys("down down");
    // The oracle's second move scrolls the viewport so the last card is on screen and the first two are not.
    s.assert_contains("item-2");
    s.assert_absent("item-0");
    s.assert_absent("item-1");
}

// ORACLE resume_picker.rs:7875 up_from_bottom_keeps_viewport_stable_when_card_remains_visible kind=ui owner=routes-lists
#[test]
fn up_from_bottom_keeps_viewport_stable_when_card_remains_visible() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("resume", "ten_rows")
        .open();
    s.keys("end");
    s.assert_contains("item-9");
    s.keys("up");
    // The oracle's viewport moved up one card (scroll_top 9 to 8): `item-8` is the first card
    // under `↑ more`, and `item-9` lies below the window (`↓ more` takes its row, the oracle's
    // `available_content_rows`).
    s.assert_contains("↑ more");
    s.assert_contains("› item-8");
    s.assert_absent("item-9");
    s.assert_contains("↓ more");
}

// ORACLE resume_picker.rs:7918 up_scrolls_only_after_crossing_top_edge kind=ui owner=routes-lists
#[test]
fn up_scrolls_only_after_crossing_top_edge() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("resume", "ten_rows")
        .open();
    s.keys("down down down down down down down down");
    s.keys("up");
    // The oracle's selection moves to 7 and the viewport top to 7: item-7 is the first row shown.
    s.assert_contains("item-7");
    s.assert_absent("item-8");
}

// ORACLE resume_picker.rs:8145 paste_appends_to_existing_query kind=ui owner=routes-lists
#[test]
fn paste_appends_to_existing_query() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "search_query")
        .open();
    s.type_text("resize");
    s.paste("results");
    s.assert_contains("resize results");
}

// ORACLE resume_picker.rs:8164 whitespace_only_paste_is_ignored kind=ui owner=routes-lists
#[test]
fn whitespace_only_paste_is_ignored() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "search_query")
        .open();
    s.type_text("resize");
    s.paste("  \n\t  ");
    s.assert_contains("resize");
}

// ORACLE resume_picker.rs:8183 paste_uses_existing_search_loading_path kind=ui owner=routes-lists
#[test]
fn paste_uses_existing_search_loading_path() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "one_row_next_page")
        .open();
    s.paste("target");
    // The oracle's search line shows the pasted query; the search request it sends is the backend's.
    s.assert_contains("target");
}

// ORACLE resume_picker.rs:8221 esc_with_empty_query_starts_fresh kind=ui owner=routes-lists
#[test]
fn esc_with_empty_query_starts_fresh() {
    kit::na(
        Owner::RoutesLists,
        "asserts the `StartFresh` selection Esc returns with an empty query, not drawn: a state test",
    );
}

// ORACLE resume_picker.rs:8242 esc_with_query_clears_search_and_preserves_selected_result kind=ui owner=routes-lists
#[test]
fn esc_with_query_clears_search_and_preserves_selected_result() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "two_rows_alpha_beta")
        .open();
    s.type_text("beta");
    s.keys("esc");
    // The oracle's query is empty again, both rows are listed and `beta` stays selected.
    s.assert_contains("Type to search");
    s.assert_contains("alpha");
    s.assert_contains("› beta");
}

// ---- oracle: resume_picker/archive_tests.rs (5 screen-level tests here; 4 state tests left to routes-lists)
// ORACLE resume_picker/archive_tests.rs:68 archive_shortcut_archives_selected_session_once kind=ui owner=routes-lists
#[test]
fn archive_shortcut_archives_selected_session_once() {
    kit::na(
        Owner::RoutesLists,
        "asserts the one archive request the loader receives and the row leaving after its result, not rows an entry can show (an entry has no server to answer): core state tests `one_archive_runs_at_a_time` and `archiving_a_row_removes_it_and_a_page_in_flight_cannot_bring_it_back`; the key is the page's ctrl+a",
    );
}

// ORACLE resume_picker/archive_tests.rs:183 archive_footer_shows_shortcut_for_resume_sessions kind=snap owner=routes-lists
#[test]
fn archive_footer_shows_shortcut_for_resume_sessions() {
    let s = Screen::build(Owner::RoutesLists)
        .size(220, 24)
        .entry("resume", "archive_footer")
        .open();
    s.assert_has_row(
        " enter resume   ⌃a archive   esc start new   ⌃c quit   tab focus sort/filter   ←/→ change option",
    );
    s.assert_has_row(" ⌃o dense view   ⌃t transcript   ⌃e expand   ↑/↓ browse");
}

// ORACLE resume_picker/archive_tests.rs:199 archived_status_preserves_directory_filter_and_hides_archive_shortcut kind=snap owner=routes-lists
#[test]
fn archived_status_preserves_directory_filter_and_hides_archive_shortcut() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(220, 24)
        .entry("resume", "archived_cwd")
        .open();
    // The oracle's status control is moved with the tab focus then right (the view's toolbar key).
    s.keys("tab right");
    // The oracle's footer at 220 cells in the archived status: restore, not archive.
    s.assert_has_row(
        " enter restore   esc start new   ⌃c quit   tab focus sort/filter   ←/→ change option",
    );
    s.assert_has_row(" ⌃o dense view   ⌃t transcript   ⌃e expand   ↑/↓ browse");
    s.assert_absent("⌃a archive");
    // The oracle's compact toolbar line in the archived status with the cwd filter (the line ends
    // with a blank the screen does not keep); the screen is narrow enough to compact it.
    s.resize(60, 24);
    s.assert_contains("Filter: Cwd   Archived  Sort: Updated");
}

// ORACLE resume_picker/archive_tests.rs:264 archived_session_restore_resumes_after_completion kind=ui owner=routes-lists
#[test]
fn archived_session_restore_resumes_after_completion() {
    kit::na(
        Owner::RoutesLists,
        "asserts the `Restoring` archive state, the restore requests and the `Resume` selection returned after completion, not drawn: a state test",
    );
}

// ORACLE resume_picker/archive_tests.rs:338 archive_cloud_preserves_same_id_current_local_row_and_transcript kind=ui owner=routes-lists
#[test]
fn archive_cloud_preserves_same_id_current_local_row_and_transcript() {
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 24)
        .entry("resume", "archive_cloud")
        .open();
    s.keys("ctrl+e");
    // The oracle's expanded row keeps the local copy's transcript preview, and not the cloud copy's.
    s.assert_contains("Local preview");
    s.assert_absent("Cloud preview");
}

// ---- oracle: resume_picker/catalog_tests.rs (1 screen-level tests here; 2 state tests left to routes-lists)
// ORACLE resume_picker/catalog_tests.rs:101 picker_selections_retain_each_rows_source kind=ui owner=routes-lists
#[test]
fn picker_selections_retain_each_rows_source() {
    kit::na(
        Owner::RoutesLists,
        "asserts the `Resume` selections Enter returns (thread id and backend id of each row), not drawn: a state test",
    );
}

// ---- oracle: resume_picker/sources_tests.rs (1 screen-level tests here; 8 state tests left to routes-lists)
// ORACLE resume_picker/sources_tests.rs:520 partial_failure_keeps_healthy_rows_visible kind=snap owner=routes-lists snap=partial_backend_failure
#[test]
fn partial_failure_keeps_healthy_rows_visible() {
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "partial_failure")
        .open();
    // The oracle's search line at 80 cells; the search line is row 4 of a 16-row screen, one cell in.
    s.assert_snap_region("partial_backend_failure", 4, 1);
}

// ---- oracle: resume_picker_color_tests.rs (4 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE resume_picker_color_tests.rs:10 thread_colors_and_selection_contrast_in_both_picker_layouts kind=ui owner=routes-lists
#[test]
fn thread_colors_and_selection_contrast_in_both_picker_layouts() {
    // The oracle checks the unread dot, the title and the selection in both layouts; the list is
    // row 6 of the screen, two cells in. Comfortable: dot at list column 2, title at 4; dense:
    // dot at 14, title at 16.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 16)
        .entry("resume", "colors_unread")
        .open();
    s.assert_style(4, 6, "fg=Rgb(59,130,246)");
    s.assert_style(6, 6, "bold");
    s.keys("ctrl+o");
    s.assert_style(16, 6, "fg=Rgb(59,130,246)");
    s.assert_style(18, 6, "bold");
    kit::pending(
        Owner::RoutesLists,
        "the title's colour is `readable_color_on(thread_color(id), bg)` (the accent made readable on the row's background, and `selected_session_style()` when theme colours are off); the kit cannot state the oracle's default colours (OSC 10/11) and the page draws the palette accent as is",
    );
}

// ORACLE resume_picker_color_tests.rs:117 loading_overlay_clears_inherited_selection_colors_and_reversal kind=ui owner=routes-lists
#[test]
fn loading_overlay_clears_inherited_selection_colors_and_reversal() {
    // The oracle draws the overlay over a 19x1 area (the message's own width); on the screen it
    // is the 29x3 box centred in the 80-cell, seven-row list (row 6, two cells in), whose message
    // is row 9 from column 32. The oracle's check: every cell of the message is bold, not dim,
    // not reversed.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(84, 17)
        .entry("resume", "transcript_loading")
        .open();
    s.keys("ctrl+t");
    s.assert_contains("Loading transcript…");
    for x in 32..32 + 19 {
        s.assert_style(x, 9, "bold !dim !reversed");
    }
}

// ORACLE resume_picker_color_tests.rs:154 compact_picker_keeps_metadata_and_labeled_primary_actions kind=snap owner=routes-lists snap=codex_tui__resume_picker__color_tests__compact_picker_keeps_metadata_and_labeled_primary_actions
#[test]
fn compact_picker_keeps_metadata_and_labeled_primary_actions() {
    // The oracle joined the frames of four sizes under its own `{action} {W}x{H}` lines.
    let labels = [
        "Resume 118x30",
        "Resume 70x12",
        "Resume 38x12",
        "Fork 38x12",
    ];
    for (label, width, height, entry) in [
        ("Resume 118x30", 118, 30, "compact_six_rows"),
        ("Resume 70x12", 70, 12, "compact_six_rows"),
        ("Resume 38x12", 38, 12, "compact_six_rows"),
        ("Fork 38x12", 38, 12, "compact_six_rows_fork"),
    ] {
        let s = Screen::build(Owner::RoutesLists)
            .size(width, height)
            .entry("resume", entry)
            .open();
        // The oracle's own assertions at each size (all four are below its footer's wide breakpoint).
        s.assert_contains(" fcoury/contrast");
        s.assert_contains("now");
        s.assert_contains(if label.starts_with("Fork") {
            "enter fork"
        } else {
            "enter resume"
        });
        s.assert_contains("esc new");
        s.snap("compact_picker_keeps_metadata_and_labeled_primary_actions")
            .labelled(&labels)
            .frame(label)
            .assert();
    }
}

// ORACLE resume_picker_color_tests.rs:263 narrow_toolbar_keeps_keyboard_focused_control_visible kind=snap owner=routes-lists snap=codex_tui__resume_picker__color_tests__narrow_toolbar_keeps_keyboard_focused_control_visible
#[test]
fn narrow_toolbar_keeps_keyboard_focused_control_visible() {
    // The oracle's own assertion at the first size: the focused control is on the screen.
    let s = Screen::build(Owner::RoutesLists)
        .size(38, 12)
        .entry("resume", "toolbar_focus_narrow")
        .open();
    s.assert_contains("Filter: Cwd");
}

// ORACLE resume_picker_pagination_error_tests.rs:9 later_page_error_preserves_loaded_sessions kind=snap owner=routes-lists snap=codex_tui__resume_picker__pagination_error_tests__resume_picker_pagination_error
#[test]
fn later_page_error_preserves_loaded_sessions() {
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "later_page_error")
        .open();
    // The oracle keeps the loaded row listed after the later page fails.
    s.assert_contains("Loaded session");
    // The oracle's search line at 80 cells, one cell in from the screen's edge.
    s.assert_snap_region(
        "codex_tui__resume_picker__pagination_error_tests__resume_picker_pagination_error",
        4,
        1,
    );
}
