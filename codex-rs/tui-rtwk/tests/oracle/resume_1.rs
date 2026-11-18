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
    kit::todo();
}

// ORACLE resume_picker.rs:4564 local_db_first_falls_back_when_initial_db_page_is_empty kind=snap owner=routes-lists snap=resume_picker_db_fallback_transition
#[test]
fn local_db_first_falls_back_when_initial_db_page_is_empty() {
    kit::todo();
}

// ORACLE resume_picker.rs:4658 local_db_first_accepts_db_thread_id_and_latest_history_mode kind=ui owner=routes-lists
#[test]
fn local_db_first_accepts_db_thread_id_and_latest_history_mode() {
    kit::todo();
}

// ORACLE resume_picker.rs:4759 expanded_session_details_include_metadata kind=ui owner=routes-lists
#[test]
fn expanded_session_details_include_metadata() {
    kit::todo();
}

// ORACLE resume_picker.rs:4806 footer_prioritizes_active_sort_timestamp kind=ui owner=routes-lists
#[test]
fn footer_prioritizes_active_sort_timestamp() {
    kit::todo();
}

// ORACLE resume_picker.rs:4837 footer_marks_missing_branch kind=ui owner=routes-lists
#[test]
fn footer_marks_missing_branch() {
    kit::todo();
}

// ORACLE resume_picker.rs:4856 footer_branch_expands_when_line_has_room kind=ui owner=routes-lists
#[test]
fn footer_branch_expands_when_line_has_room() {
    kit::todo();
}

// ORACLE resume_picker.rs:4873 footer_cwd_truncates_to_responsive_column kind=ui owner=routes-lists
#[test]
fn footer_cwd_truncates_to_responsive_column() {
    kit::todo();
}

// ORACLE resume_picker.rs:4895 footer_omits_cwd_when_hidden kind=ui owner=routes-lists
#[test]
fn footer_omits_cwd_when_hidden() {
    kit::todo();
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
    kit::todo();
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
    kit::todo();
}

// ORACLE resume_picker.rs:5320 hint_line_preserves_primary_action_labels_when_very_narrow kind=ui owner=routes-lists
#[test]
fn hint_line_preserves_primary_action_labels_when_very_narrow() {
    kit::todo();
}

// ORACLE resume_picker.rs:5349 hint_line_shows_loading_transcript_mode kind=ui owner=routes-lists
#[test]
fn hint_line_shows_loading_transcript_mode() {
    kit::todo();
}

// ORACLE resume_picker.rs:5516 ctrl_o_toggles_density_without_typing_into_search kind=ui owner=routes-lists
#[test]
fn ctrl_o_toggles_density_without_typing_into_search() {
    kit::todo();
}

// ORACLE resume_picker.rs:5539 ctrl_t_requests_selected_session_transcript kind=ui owner=routes-lists
#[test]
fn ctrl_t_requests_selected_session_transcript() {
    kit::todo();
}

// ORACLE resume_picker.rs:5586 transcript_loading_consumes_picker_input kind=ui owner=routes-lists
#[test]
fn transcript_loading_consumes_picker_input() {
    kit::todo();
}

// ORACLE resume_picker.rs:5645 escape_cancels_transcript_loading_and_restores_picker_navigation kind=ui owner=routes-lists
#[test]
fn escape_cancels_transcript_loading_and_restores_picker_navigation() {
    kit::todo();
}

// ORACLE resume_picker.rs:5702 transcript_loading_still_allows_ctrl_c_exit kind=ui owner=routes-lists
#[test]
fn transcript_loading_still_allows_ctrl_c_exit() {
    kit::todo();
}

// ORACLE resume_picker.rs:5724 transcript_loading_overlay_snapshot kind=snap owner=routes-lists snap=resume_picker_transcript_loading_overlay
#[test]
fn transcript_loading_overlay_snapshot() {
    kit::todo();
}

// ORACLE resume_picker.rs:5793 raw_ctrl_t_requests_selected_session_transcript kind=ui owner=routes-lists
#[test]
fn raw_ctrl_t_requests_selected_session_transcript() {
    kit::todo();
}

// ORACLE resume_picker.rs:5833 ctrl_t_on_row_without_thread_id_shows_inline_error kind=ui owner=routes-lists
#[test]
fn ctrl_t_on_row_without_thread_id_shows_inline_error() {
    kit::todo();
}

// ORACLE resume_picker.rs:5869 loaded_transcript_waits_for_loading_frame_before_opening_overlay kind=ui owner=routes-lists
#[test]
fn loaded_transcript_waits_for_loading_frame_before_opening_overlay() {
    kit::todo();
}

// ORACLE resume_picker.rs:6020 cached_transcript_still_shows_loading_frame_before_opening_overlay kind=ui owner=routes-lists
#[test]
fn cached_transcript_still_shows_loading_frame_before_opening_overlay() {
    kit::todo();
}

// ORACLE resume_picker.rs:6071 ctrl_o_persists_density_preference kind=ui owner=routes-lists
#[test]
fn ctrl_o_persists_density_preference() {
    kit::todo();
}

// ORACLE resume_picker.rs:6104 ctrl_o_keeps_toggled_density_when_persistence_fails kind=ui owner=routes-lists
#[test]
fn ctrl_o_keeps_toggled_density_when_persistence_fails() {
    kit::todo();
}

// ORACLE resume_picker.rs:6139 raw_ctrl_o_toggles_density_without_typing_into_search kind=ui owner=routes-lists
#[test]
fn raw_ctrl_o_toggles_density_without_typing_into_search() {
    kit::todo();
}

// ORACLE resume_picker.rs:6162 space_appends_to_search_query kind=ui owner=routes-lists
#[test]
fn space_appends_to_search_query() {
    kit::todo();
}

// ORACLE resume_picker.rs:6189 ctrl_e_toggles_selected_session_expansion kind=ui owner=routes-lists
#[test]
fn ctrl_e_toggles_selected_session_expansion() {
    kit::todo();
}

// ORACLE resume_picker.rs:6240 raw_ctrl_e_toggles_selected_session_expansion kind=ui owner=routes-lists
#[test]
fn raw_ctrl_e_toggles_selected_session_expansion() {
    kit::todo();
}
