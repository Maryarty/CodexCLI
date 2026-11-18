//! Shard `resume_1` (screens S18, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: resume_picker.rs (32 screen-level tests here; 15 state tests left to routes-lists)
// ORACLE resume_picker.rs:4301 linked_worktree_sessions_follow_directory_scope kind=snap owner=routes-lists snap=resume_picker_linked_worktree
#[test]
fn linked_worktree_sessions_follow_directory_scope() {
    kit::pending(
        Owner::RoutesLists,
        "no entry for the picker on a repo filter of the primary checkout with a linked worktree session (resume registers `linked_worktree`); the repo filter and its toolbar control are not built",
    );
    // The oracle hides the linked worktree's session until the filter moves to
    // all checkouts (`change_filter_mode(Next)`: the filter control, then right).
    let mut s = Screen::build(Owner::RoutesLists)
        .size(102, 12)
        .entry("resume", "linked_worktree")
        .open();
    s.keys("tab right");
    // The list is the screen less its two-cell inset, at row 6 (the toolbar and
    // search line above it); the oracle's snapshot is its two list rows.
    s.assert_snap_region("resume_picker_linked_worktree", 6, 2);
}

// ORACLE resume_picker.rs:4564 local_db_first_falls_back_when_initial_db_page_is_empty kind=snap owner=routes-lists snap=resume_picker_db_fallback_transition
#[test]
fn local_db_first_falls_back_when_initial_db_page_is_empty() {
    kit::pending(
        Owner::RoutesLists,
        "no entry for the picker with the three store rows of a fallback after an empty DB page (resume registers `db_fallback`); the kit's section compare does not match this snapshot's `---- x ----` markers (kit gap, see the report)",
    );
    // The oracle's two states, loading then the repaired rows; the screen shows
    // the second one (the listing of the fallback source is the loader's).
    let s = Screen::build(Owner::RoutesLists)
        .size(82, 12)
        .entry("resume", "db_fallback")
        .open();
    s.assert_snap_section("resume_picker_db_fallback_transition", "repaired results");
}

// ORACLE resume_picker.rs:4658 local_db_first_accepts_db_thread_id_and_latest_history_mode kind=ui owner=routes-lists
#[test]
fn local_db_first_accepts_db_thread_id_and_latest_history_mode() {
    kit::na(
        Owner::RoutesLists,
        "asserts the selection that Enter returns (thread id, cwd, history mode), not rows: a state test",
    );
}

// ORACLE resume_picker.rs:4759 expanded_session_details_include_metadata kind=ui owner=routes-lists
#[test]
fn expanded_session_details_include_metadata() {
    kit::na(
        Owner::RoutesLists,
        "asserts the detail lines returned by render_expanded_session_details, a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:4806 footer_prioritizes_active_sort_timestamp kind=ui owner=routes-lists
#[test]
fn footer_prioritizes_active_sort_timestamp() {
    kit::na(
        Owner::RoutesLists,
        "asserts footer lines returned by render_footer_lines, a helper (with prefix and order checks the kit has none of), not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:4837 footer_marks_missing_branch kind=ui owner=routes-lists
#[test]
fn footer_marks_missing_branch() {
    kit::na(
        Owner::RoutesLists,
        "asserts a footer line returned by render_footer_lines, a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:4856 footer_branch_expands_when_line_has_room kind=ui owner=routes-lists
#[test]
fn footer_branch_expands_when_line_has_room() {
    kit::na(
        Owner::RoutesLists,
        "asserts a footer line returned by render_footer_lines at 140 cells, a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:4873 footer_cwd_truncates_to_responsive_column kind=ui owner=routes-lists
#[test]
fn footer_cwd_truncates_to_responsive_column() {
    kit::na(
        Owner::RoutesLists,
        "asserts a footer line returned by render_footer_lines (cwd truncation), a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:4895 footer_omits_cwd_when_hidden kind=ui owner=routes-lists
#[test]
fn footer_omits_cwd_when_hidden() {
    kit::na(
        Owner::RoutesLists,
        "asserts a footer line returned by render_footer_lines with show_cwd off, a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:5037 mixed_backend_resume_table_snapshot kind=snap owner=routes-lists snap=resume_picker_table
#[test]
fn mixed_backend_resume_table_snapshot() {
    // Our screen is 82 wide (the list is the screen less its two-cell inset); the
    // oracle's test selects the second row.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(82, 22)
        .entry("resume", "table")
        .open();
    s.keys("down");
    s.assert_snap_region("resume_picker_table", 6, 2);
}

// ORACLE resume_picker.rs:5139 resume_search_error_snapshot kind=snap owner=routes-lists snap=resume_picker_search_error
#[test]
fn resume_search_error_snapshot() {
    Screen::build(Owner::RoutesLists)
        .size(80, 13)
        .entry("resume", "search_error")
        .open()
        .assert_snap_region("resume_picker_search_error", 2, 1);
}

// ORACLE resume_picker.rs:5256 hint_line_compacts_on_narrow_width kind=ui owner=routes-lists
#[test]
fn hint_line_compacts_on_narrow_width() {
    kit::na(
        Owner::RoutesLists,
        "asserts the footer text returned by footer_lines_text at 119 cells, a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:5280 hint_line_snapshot_uses_distributed_wide_footer kind=snap owner=routes-lists snap=resume_picker_footer_wide
#[test]
fn hint_line_snapshot_uses_distributed_wide_footer() {
    Screen::build(Owner::RoutesLists)
        .size(220, 22)
        .entry("resume", "footer_wide")
        .open()
        .assert_snap_region("resume_picker_footer_wide", 18, 0);
}

// ORACLE resume_picker.rs:5299 hint_line_snapshot_uses_compact_footer kind=snap owner=routes-lists snap=resume_picker_footer_compact
#[test]
fn hint_line_snapshot_uses_compact_footer() {
    kit::pending(
        Owner::RoutesLists,
        "the search line takes typed text and ctrl+o toggles the dense density; the resume view binds only up, down, enter and esc",
    );
    // The oracle's query `picker` and dense density, at 96 cells; the footer is the
    // screen's bottom four rows (row 18 of 22), its three snapshot rows.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(96, 22)
        .entry("resume", "footer_wide")
        .open();
    s.type_text("picker");
    s.keys("ctrl+o");
    s.assert_snap_region("resume_picker_footer_compact", 18, 0);
}

// ORACLE resume_picker.rs:5320 hint_line_preserves_primary_action_labels_when_very_narrow kind=ui owner=routes-lists
#[test]
fn hint_line_preserves_primary_action_labels_when_very_narrow() {
    kit::na(
        Owner::RoutesLists,
        "asserts the footer lines returned by footer_hint_lines at 38 cells, a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:5349 hint_line_shows_loading_transcript_mode kind=ui owner=routes-lists
#[test]
fn hint_line_shows_loading_transcript_mode() {
    kit::na(
        Owner::RoutesLists,
        "asserts the footer text returned by footer_lines_text while a transcript loads, a helper, not the picker's rows: a helper test",
    );
}

// ORACLE resume_picker.rs:5516 ctrl_o_toggles_density_without_typing_into_search kind=ui owner=routes-lists
#[test]
fn ctrl_o_toggles_density_without_typing_into_search() {
    kit::na(
        Owner::RoutesLists,
        "asserts the density and query fields after ctrl+o, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:5539 ctrl_t_requests_selected_session_transcript kind=ui owner=routes-lists
#[test]
fn ctrl_t_requests_selected_session_transcript() {
    kit::na(
        Owner::RoutesLists,
        "asserts the transcript request sent to the loader and the loading fields, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:5586 transcript_loading_consumes_picker_input kind=ui owner=routes-lists
#[test]
fn transcript_loading_consumes_picker_input() {
    kit::na(
        Owner::RoutesLists,
        "asserts the selection and query fields while a transcript loads, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:5645 escape_cancels_transcript_loading_and_restores_picker_navigation kind=ui owner=routes-lists
#[test]
fn escape_cancels_transcript_loading_and_restores_picker_navigation() {
    kit::na(
        Owner::RoutesLists,
        "asserts the pending-transcript and cell fields after esc, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:5702 transcript_loading_still_allows_ctrl_c_exit kind=ui owner=routes-lists
#[test]
fn transcript_loading_still_allows_ctrl_c_exit() {
    kit::na(
        Owner::RoutesLists,
        "asserts the Exit selection that ctrl+c returns, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:5724 transcript_loading_overlay_snapshot kind=snap owner=routes-lists snap=resume_picker_transcript_loading_overlay
#[test]
fn transcript_loading_overlay_snapshot() {
    kit::pending(
        Owner::RoutesLists,
        "no entry for the picker with two rows without timestamps (resume registers `transcript_loading`); the transcript loading overlay (ctrl+t on a row) is not built in the resume view",
    );
    // The oracle's 80-cell list with the loading overlay on the second row; the
    // list is at row 6 of the screen, its five snapshot rows.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(82, 15)
        .entry("resume", "transcript_loading")
        .open();
    s.keys("ctrl+t");
    s.assert_snap_region("resume_picker_transcript_loading_overlay", 6, 2);
}

// ORACLE resume_picker.rs:5793 raw_ctrl_t_requests_selected_session_transcript kind=ui owner=routes-lists
#[test]
fn raw_ctrl_t_requests_selected_session_transcript() {
    kit::na(
        Owner::RoutesLists,
        "asserts the transcript request sent for the raw control byte, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:5833 ctrl_t_on_row_without_thread_id_shows_inline_error kind=ui owner=routes-lists
#[test]
fn ctrl_t_on_row_without_thread_id_shows_inline_error() {
    kit::na(
        Owner::RoutesLists,
        "asserts the inline_error field; a listed row always carries its thread id (core Row.id), so the state cannot be seeded: a state test",
    );
}

// ORACLE resume_picker.rs:5869 loaded_transcript_waits_for_loading_frame_before_opening_overlay kind=ui owner=routes-lists
#[test]
fn loaded_transcript_waits_for_loading_frame_before_opening_overlay() {
    kit::na(
        Owner::RoutesLists,
        "asserts the overlay and pending fields and the chord routing, not rows: a state test (the transcript overlay is the pager's)",
    );
}

// ORACLE resume_picker.rs:6020 cached_transcript_still_shows_loading_frame_before_opening_overlay kind=ui owner=routes-lists
#[test]
fn cached_transcript_still_shows_loading_frame_before_opening_overlay() {
    kit::na(
        Owner::RoutesLists,
        "asserts the overlay and pending fields around a cached transcript, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:6071 ctrl_o_persists_density_preference kind=ui owner=routes-lists
#[test]
fn ctrl_o_persists_density_preference() {
    kit::na(
        Owner::RoutesLists,
        "asserts the density field and the saved view-mode preference, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:6104 ctrl_o_keeps_toggled_density_when_persistence_fails kind=ui owner=routes-lists
#[test]
fn ctrl_o_keeps_toggled_density_when_persistence_fails() {
    kit::na(
        Owner::RoutesLists,
        "asserts the density field and the inline_error field after a failed save, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:6139 raw_ctrl_o_toggles_density_without_typing_into_search kind=ui owner=routes-lists
#[test]
fn raw_ctrl_o_toggles_density_without_typing_into_search() {
    kit::na(
        Owner::RoutesLists,
        "asserts the density and query fields after the raw control byte, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:6162 space_appends_to_search_query kind=ui owner=routes-lists
#[test]
fn space_appends_to_search_query() {
    kit::na(
        Owner::RoutesLists,
        "asserts the query and expansion fields, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:6189 ctrl_e_toggles_selected_session_expansion kind=ui owner=routes-lists
#[test]
fn ctrl_e_toggles_selected_session_expansion() {
    kit::na(
        Owner::RoutesLists,
        "asserts the expanded_thread_id field and the transcript request, not rows: a state test",
    );
}

// ORACLE resume_picker.rs:6240 raw_ctrl_e_toggles_selected_session_expansion kind=ui owner=routes-lists
#[test]
fn raw_ctrl_e_toggles_selected_session_expansion() {
    kit::na(
        Owner::RoutesLists,
        "asserts the expanded_thread_id field after the raw control byte, not rows: a state test",
    );
}
