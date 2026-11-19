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
    kit::pending(
        Owner::RoutesLists,
        "no entry with a repo filter available and a cwd filter (`/tmp/project`) on the picker; the toolbar shows only `Filter: All` and no repo or cwd tab",
    );
    // The oracle's toolbar line at 100 cells, one row; on the screen it is the toolbar row (row 2 at 24 rows).
    let s = Screen::build(Owner::RoutesLists)
        .size(100, 24)
        .entry("resume", "toolbar_repo_cwd")
        .open();
    s.assert_snap_region("resume_picker_search_line_sort_filter_tabs", 2, 0);
}

// ORACLE resume_picker.rs:6393 dense_session_snapshot_omits_cwd_in_cwd_filter kind=snap owner=routes-lists snap=resume_picker_dense_cwd
#[test]
fn dense_session_snapshot_omits_cwd_in_cwd_filter() {
    kit::pending(
        Owner::RoutesLists,
        "no dense density (ctrl+o) on the picker and no entry with one row of this data under a cwd filter; the list has only the comfortable layout",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(100, 10)
        .entry("resume", "dense_cwd")
        .open();
    s.keys("ctrl+o");
    // The oracle's list is three rows at 100 cells; the list starts at row 3 of a 10-row screen.
    s.assert_snap_region("resume_picker_dense_cwd", 3, 2);
}

// ORACLE resume_picker.rs:6407 dense_session_snapshot_drops_metadata_when_narrow kind=snap owner=routes-lists snap=resume_picker_dense_narrow
#[test]
fn dense_session_snapshot_drops_metadata_when_narrow() {
    kit::pending(
        Owner::RoutesLists,
        "no dense density (ctrl+o) on the picker and no entry with one row at 48 cells; the list has only the comfortable layout",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(48, 10)
        .entry("resume", "dense_narrow")
        .open();
    s.keys("ctrl+o");
    s.assert_snap_region("resume_picker_dense_narrow", 3, 2);
}

// ORACLE resume_picker.rs:6417 dense_session_line_prefers_thread_name_over_preview kind=ui owner=routes-lists
#[test]
fn dense_session_line_prefers_thread_name_over_preview() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with a row that has both a thread name (`Named session`) and a preview; no dense density (ctrl+o) on the picker",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no zebra striping on the comfortable list and no entry with a zebra row; the oracle's expected background is `dense_zebra_style().bg`, a theme value the kit's literal style check cannot state",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no dense density (ctrl+o) on the picker and no entry with two rows (`First dense row`, `Second dense row` on branch fcoury/other-branch); the list has only the comfortable layout",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 9)
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
        "no expansion (ctrl+e) with the transcript preview of a row (`Show me the recent transcript`, the assistant's citation line) and no entry for it; the expanded layout is not built",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(90, 21)
        .entry("resume", "expanded_session")
        .open();
    s.keys("ctrl+e");
    // The oracle's 11 rows at 90 cells; the list starts at row 6 of a 21-row screen.
    s.assert_snap_region("resume_picker_expanded_session", 6, 2);
}

// ORACLE resume_picker.rs:6641 narrow_session_snapshot kind=snap owner=routes-lists snap=resume_picker_narrow_session
#[test]
fn narrow_session_snapshot() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with one row (`Investigate picker expansion`, cwd /tmp/codex, branch fcoury/session-picker) at 58 cells; the view's narrow layout of the row is not checked against it yet",
    );
    let s = Screen::build(Owner::RoutesLists)
        .size(58, 16)
        .entry("resume", "narrow_session")
        .open();
    // The oracle's six rows at 58 cells; the list starts at row 6 of a 16-row screen.
    s.assert_snap_region("resume_picker_narrow_session", 6, 2);
}

// ORACLE resume_picker.rs:6694 session_list_more_indicators_snapshot kind=snap owner=routes-lists snap=resume_picker_more_indicators
#[test]
fn session_list_more_indicators_snapshot() {
    kit::pending(
        Owner::RoutesLists,
        "G40 (rtwk-gaps.md): the oracle scrolls by whole rows of the list with the two `more` lines taking window rows; rtwk's SelectionList scrolls by cells and its window shrinks for the `more` lines after it settled, so the third row shows cut (title only) with the second above it; the cursor cannot start on it either (G35)",
    );
    // Five rows (item-0..item-4); the oracle selects the third and the list
    // scrolls to it. The list is six rows at row 6 of a 16-row screen.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "more_indicators")
        .open();
    s.keys("down down");
    s.assert_snap_region("resume_picker_more_indicators", 6, 2);
}

// ORACLE resume_picker.rs:6749 density_toggle_clears_stale_more_indicator kind=ui owner=routes-lists
#[test]
fn density_toggle_clears_stale_more_indicator() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with four rows (item-0..item-3) at 80 cells that overflow a six-row list, and no dense density (ctrl+o) to toggle",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no cwd filter tab on the picker (no entry without a cwd candidate); the status control's key presses are not built, and the requests they send are the backend's",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no entry with twenty rows (item-0..item-19) and the page keys do not move the selection by the list's rows yet; the oracle's five-row view is a 12-row screen",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "the list keymap (page_down, page_up, jump_bottom, jump_top) is not read by the port yet; the kit cannot set the remapped keys",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no entry with ten rows and a next cursor (more pages to load), no footer progress label (`10 / 10… · 100%`) and no load-more on end",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no entry with a row that has no thread id (`missing metadata`, /tmp/missing.jsonl) and no inline error line on the picker; a failed Enter only sets the search line's message",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "missing_metadata")
        .open();
    s.keys("enter");
    s.assert_contains("Failed to read session metadata from /tmp/missing.jsonl");
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
    kit::pending(
        Owner::RoutesLists,
        "no entry with three rows (item-0..item-2) in a five-row viewport; the view's scroll sits in the layout, not the seed, so the oracle's scroll_top state is only visible as the rows shown",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no entry with ten rows (item-0..item-9) and no seed of the selection and scroll state at the bottom; the screen reaches it with the end key",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 12)
        .entry("resume", "ten_rows")
        .open();
    s.keys("end");
    s.assert_contains("item-9");
    s.keys("up");
    // The oracle's viewport moved up one card; the last card stays on screen.
    s.assert_contains("item-8");
    s.assert_contains("item-9");
}

// ORACLE resume_picker.rs:7918 up_scrolls_only_after_crossing_top_edge kind=ui owner=routes-lists
#[test]
fn up_scrolls_only_after_crossing_top_edge() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with ten rows (item-0..item-9); the oracle seeds selected 8 and scroll 8, the screen reaches it with eight down presses",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no search query on the picker: the search line shows only `Type to search` and takes no typed or pasted text",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no search query on the picker: the search line takes no typed or pasted text, so a whitespace-only paste cannot be told apart from no paste",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no search query and no search listing on the picker: a paste into the search line does not start a search",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no search query on the picker and no esc that clears it: the search line shows `Type to search` only",
    );
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
    kit::pending(
        Owner::RoutesLists,
        "no archive shortcut (ctrl+a) and no archive request on the picker; the row `Selected session` is listed by no entry",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "archive_selected")
        .open();
    s.assert_contains("Selected session");
    s.keys("ctrl+a ctrl+a");
    // The oracle's archived row leaves the list once the archive result lands.
    s.assert_absent("Selected session");
}

// ORACLE resume_picker/archive_tests.rs:183 archive_footer_shows_shortcut_for_resume_sessions kind=snap owner=routes-lists
#[test]
fn archive_footer_shows_shortcut_for_resume_sessions() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with a selected row at 220 cells (`footer_wide` has no rows) and no archive hint (`⌃a archive`) in the footer's key hints",
    );
    let s = Screen::build(Owner::RoutesLists)
        .size(220, 24)
        .entry("resume", "archive_footer")
        .open();
    s.assert_has_row(
        "enter resume   ⌃a archive   esc start new   ⌃c quit   tab focus sort/filter   ←/→ change option",
    );
    s.assert_has_row("⌃o dense view   ⌃t transcript   ⌃e expand   ↑/↓ browse");
}

// ORACLE resume_picker/archive_tests.rs:199 archived_status_preserves_directory_filter_and_hides_archive_shortcut kind=snap owner=routes-lists
#[test]
fn archived_status_preserves_directory_filter_and_hides_archive_shortcut() {
    kit::pending(
        Owner::RoutesLists,
        "no archived status control with a cwd filter (no entry with it), and no footer that swaps `archive` for `restore`; the requests the status change sends are the backend's",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(220, 24)
        .entry("resume", "archived_cwd")
        .open();
    // The oracle's status control is moved with the tab focus then right (the view's toolbar key).
    s.keys("tab right");
    // The oracle's footer at 220 cells in the archived status: restore, not archive.
    s.assert_has_row(
        "enter restore   esc start new   ⌃c quit   tab focus sort/filter   ←/→ change option",
    );
    s.assert_has_row("⌃o dense view   ⌃t transcript   ⌃e expand   ↑/↓ browse");
    s.assert_absent("⌃a archive");
    // The oracle's compact toolbar in the archived status with the cwd filter.
    s.assert_contains("Filter: Cwd   Archived  Sort: Updated ");
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
    kit::pending(
        Owner::RoutesLists,
        "no entry with a local and a cloud copy of one thread (same id), no expansion with a transcript preview, and no archive of the cloud copy on the picker",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
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
    kit::pending(
        Owner::RoutesLists,
        "no entry with a healthy row kept after one source fails (the message `Could not load one backend. Reopen the picker to retry.` on the search line); `search_error` is a whole listing failure",
    );
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "partial_failure")
        .open();
    // The oracle's search line at 80 cells; the search line is row 4 of a 16-row screen.
    s.assert_snap_region("partial_backend_failure", 4, 0);
}

// ---- oracle: resume_picker_color_tests.rs (4 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE resume_picker_color_tests.rs:10 thread_colors_and_selection_contrast_in_both_picker_layouts kind=ui owner=routes-lists
#[test]
fn thread_colors_and_selection_contrast_in_both_picker_layouts() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with an unread row named `Named task` at 40 cells, no thread colour on the title and no dense/comfortable selection colour check; the title colour is `readable_color_on(thread_color(id))`, a computed value the kit's literal style check cannot state",
    );
    let s = Screen::build(Owner::RoutesLists)
        .size(40, 16)
        .entry("resume", "colors_unread")
        .open();
    // The oracle's unread dot at column 2 of the first row, blue (59,130,246).
    s.assert_style(2, 0, "fg=Rgb(59,130,246)");
}

// ORACLE resume_picker_color_tests.rs:117 loading_overlay_clears_inherited_selection_colors_and_reversal kind=ui owner=routes-lists
#[test]
fn loading_overlay_clears_inherited_selection_colors_and_reversal() {
    kit::pending(
        Owner::RoutesLists,
        "no transcript loading overlay on the picker (no entry with a transcript being loaded); the oracle's overlay style is `transcript_loading_overlay_style()`, a theme value the kit's literal style check cannot state",
    );
    let s = Screen::build(Owner::RoutesLists)
        .size(19, 16)
        .entry("resume", "transcript_loading")
        .open();
    // The oracle's overlay text, one row, the text's own width.
    s.assert_contains("Loading transcript…");
}

// ORACLE resume_picker_color_tests.rs:154 compact_picker_keeps_metadata_and_labeled_primary_actions kind=snap owner=routes-lists snap=codex_tui__resume_picker__color_tests__compact_picker_keeps_metadata_and_labeled_primary_actions
#[test]
fn compact_picker_keeps_metadata_and_labeled_primary_actions() {
    kit::pending(
        Owner::RoutesLists,
        "no fork action on the picker and no entry with six rows at /tmp/codex (branch fcoury/contrast) at 118, 70 and 38 cells; the compact footer's labelled actions are not checked at these widths",
    );
    let snap = "codex_tui__resume_picker__color_tests__compact_picker_keeps_metadata_and_labeled_primary_actions";
    let s = Screen::build(Owner::RoutesLists)
        .size(118, 30)
        .entry("resume", "compact_six_rows")
        .open();
    // The oracle's joined snapshot: one frame per action and size.
    s.assert_snap_section(snap, "Resume 118x30");
    s.assert_snap_section(snap, "Resume 70x12");
    s.assert_snap_section(snap, "Resume 38x12");
    s.assert_snap_section(snap, "Fork 38x12");
}

// ORACLE resume_picker_color_tests.rs:263 narrow_toolbar_keeps_keyboard_focused_control_visible kind=snap owner=routes-lists snap=codex_tui__resume_picker__color_tests__narrow_toolbar_keeps_keyboard_focused_control_visible
#[test]
fn narrow_toolbar_keeps_keyboard_focused_control_visible() {
    kit::pending(
        Owner::RoutesLists,
        "no keyboard focus on the toolbar controls (tab focus) at 38 and 24 cells and no archived status with a cwd filter; the view's toolbar only compacts",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(38, 12)
        .entry("resume", "toolbar_focus_narrow")
        .open();
    s.keys("tab");
    // The oracle's joined snapshot: the focused control stays visible at each width and action.
    s.assert_snap_section(
        "codex_tui__resume_picker__color_tests__narrow_toolbar_keeps_keyboard_focused_control_visible",
        "Resume 38x12",
    );
}

// ---- oracle: resume_picker_pagination_error_tests.rs (1 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE resume_picker_pagination_error_tests.rs:9 later_page_error_preserves_loaded_sessions kind=snap owner=routes-lists snap=codex_tui__resume_picker__pagination_error_tests__resume_picker_pagination_error
#[test]
fn later_page_error_preserves_loaded_sessions() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with one loaded row (`Loaded session`) whose next page fails (`Could not load more sessions` on the search line); a failed page is a whole-listing `Failed` state in the view",
    );
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 16)
        .entry("resume", "later_page_error")
        .open();
    // The oracle keeps the loaded row listed after the later page fails.
    s.assert_contains("Loaded session");
    // The oracle's search line at 80 cells: `Could not load more sessions`.
    s.assert_snap_region(
        "codex_tui__resume_picker__pagination_error_tests__resume_picker_pagination_error",
        4,
        0,
    );
}
