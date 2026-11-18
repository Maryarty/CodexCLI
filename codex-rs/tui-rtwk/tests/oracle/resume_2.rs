//! Shard `resume_2` (screens S18, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner};

// ---- oracle: resume_picker.rs (31 screen-level tests here; 15 state tests left to routes-lists)
// ORACLE resume_picker.rs:6277 toolbar_renders_sort_and_filter_tabs kind=snap owner=routes-lists snap=resume_picker_search_line_sort_filter_tabs
#[test]
fn toolbar_renders_sort_and_filter_tabs() {
    kit::todo();
}

// ORACLE resume_picker.rs:6393 dense_session_snapshot_omits_cwd_in_cwd_filter kind=snap owner=routes-lists snap=resume_picker_dense_cwd
#[test]
fn dense_session_snapshot_omits_cwd_in_cwd_filter() {
    kit::todo();
}

// ORACLE resume_picker.rs:6407 dense_session_snapshot_drops_metadata_when_narrow kind=snap owner=routes-lists snap=resume_picker_dense_narrow
#[test]
fn dense_session_snapshot_drops_metadata_when_narrow() {
    kit::todo();
}

// ORACLE resume_picker.rs:6417 dense_session_line_prefers_thread_name_over_preview kind=ui owner=routes-lists
#[test]
fn dense_session_line_prefers_thread_name_over_preview() {
    kit::todo();
}

// ORACLE resume_picker.rs:6486 comfortable_zebra_lines_use_full_width_background kind=ui owner=routes-lists
#[test]
fn comfortable_zebra_lines_use_full_width_background() {
    kit::todo();
}

// ORACLE resume_picker.rs:6520 dense_session_snapshot_uses_no_blank_lines_between_rows kind=snap owner=routes-lists snap=resume_picker_dense_no_blank_lines
#[test]
fn dense_session_snapshot_uses_no_blank_lines_between_rows() {
    kit::todo();
}

// ORACLE resume_picker.rs:6566 expanded_session_snapshot kind=snap owner=routes-lists snap=resume_picker_expanded_session
#[test]
fn expanded_session_snapshot() {
    kit::todo();
}

// ORACLE resume_picker.rs:6641 narrow_session_snapshot kind=snap owner=routes-lists snap=resume_picker_narrow_session
#[test]
fn narrow_session_snapshot() {
    kit::todo();
}

// ORACLE resume_picker.rs:6694 session_list_more_indicators_snapshot kind=snap owner=routes-lists snap=resume_picker_more_indicators
#[test]
fn session_list_more_indicators_snapshot() {
    kit::todo();
}

// ORACLE resume_picker.rs:6749 density_toggle_clears_stale_more_indicator kind=ui owner=routes-lists
#[test]
fn density_toggle_clears_stale_more_indicator() {
    kit::todo();
}

// ORACLE resume_picker.rs:7047 list_viewport_width_matches_rendered_list_inset kind=ui owner=routes-lists
#[test]
fn list_viewport_width_matches_rendered_list_inset() {
    kit::todo();
}

// ORACLE resume_picker.rs:7053 toggle_sort_key_reloads_with_new_sort kind=ui owner=routes-lists
#[test]
fn toggle_sort_key_reloads_with_new_sort() {
    kit::todo();
}

// ORACLE resume_picker.rs:7096 default_filter_focus_arrows_reload_with_new_filter kind=ui owner=routes-lists
#[test]
fn default_filter_focus_arrows_reload_with_new_filter() {
    kit::todo();
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
    kit::todo();
}

// ORACLE resume_picker.rs:7232 page_navigation_uses_view_rows kind=ui owner=routes-lists
#[test]
fn page_navigation_uses_view_rows() {
    kit::todo();
}

// ORACLE resume_picker.rs:7313 page_and_jump_navigation_use_list_keymap kind=ui owner=routes-lists
#[test]
fn page_and_jump_navigation_use_list_keymap() {
    kit::todo();
}

// ORACLE resume_picker.rs:7376 ctrl_c_exits_even_when_cancel_is_remapped_to_ctrl_c kind=ui owner=routes-lists
#[test]
fn ctrl_c_exits_even_when_cancel_is_remapped_to_ctrl_c() {
    kit::todo();
}

// ORACLE resume_picker.rs:7398 end_jumps_to_last_known_row_and_starts_loading_more kind=ui owner=routes-lists
#[test]
fn end_jumps_to_last_known_row_and_starts_loading_more() {
    kit::todo();
}

// ORACLE resume_picker.rs:7447 enter_on_row_without_resolvable_thread_id_shows_inline_error kind=ui owner=routes-lists
#[test]
fn enter_on_row_without_resolvable_thread_id_shows_inline_error() {
    kit::todo();
}

// ORACLE resume_picker.rs:7489 enter_on_pathless_thread_uses_thread_id kind=ui owner=routes-lists
#[test]
fn enter_on_pathless_thread_uses_thread_id() {
    kit::todo();
}

// ORACLE resume_picker.rs:7580 thread_to_transcript_cells_renders_core_message_types kind=snap owner=routes-lists
#[test]
fn thread_to_transcript_cells_renders_core_message_types() {
    kit::todo();
}

// ORACLE resume_picker.rs:7763 thread_to_transcript_cells_retains_summary_and_raw_reasoning_when_enabled kind=ui owner=routes-lists
#[test]
fn thread_to_transcript_cells_retains_summary_and_raw_reasoning_when_enabled() {
    kit::todo();
}

// ORACLE resume_picker.rs:7832 moving_to_last_card_scrolls_when_cards_exceed_viewport kind=ui owner=routes-lists
#[test]
fn moving_to_last_card_scrolls_when_cards_exceed_viewport() {
    kit::todo();
}

// ORACLE resume_picker.rs:7875 up_from_bottom_keeps_viewport_stable_when_card_remains_visible kind=ui owner=routes-lists
#[test]
fn up_from_bottom_keeps_viewport_stable_when_card_remains_visible() {
    kit::todo();
}

// ORACLE resume_picker.rs:7918 up_scrolls_only_after_crossing_top_edge kind=ui owner=routes-lists
#[test]
fn up_scrolls_only_after_crossing_top_edge() {
    kit::todo();
}

// ORACLE resume_picker.rs:8145 paste_appends_to_existing_query kind=ui owner=routes-lists
#[test]
fn paste_appends_to_existing_query() {
    kit::todo();
}

// ORACLE resume_picker.rs:8164 whitespace_only_paste_is_ignored kind=ui owner=routes-lists
#[test]
fn whitespace_only_paste_is_ignored() {
    kit::todo();
}

// ORACLE resume_picker.rs:8183 paste_uses_existing_search_loading_path kind=ui owner=routes-lists
#[test]
fn paste_uses_existing_search_loading_path() {
    kit::todo();
}

// ORACLE resume_picker.rs:8221 esc_with_empty_query_starts_fresh kind=ui owner=routes-lists
#[test]
fn esc_with_empty_query_starts_fresh() {
    kit::todo();
}

// ORACLE resume_picker.rs:8242 esc_with_query_clears_search_and_preserves_selected_result kind=ui owner=routes-lists
#[test]
fn esc_with_query_clears_search_and_preserves_selected_result() {
    kit::todo();
}

// ---- oracle: resume_picker/archive_tests.rs (5 screen-level tests here; 4 state tests left to routes-lists)
// ORACLE resume_picker/archive_tests.rs:68 archive_shortcut_archives_selected_session_once kind=ui owner=routes-lists
#[test]
fn archive_shortcut_archives_selected_session_once() {
    kit::todo();
}

// ORACLE resume_picker/archive_tests.rs:183 archive_footer_shows_shortcut_for_resume_sessions kind=snap owner=routes-lists
#[test]
fn archive_footer_shows_shortcut_for_resume_sessions() {
    kit::todo();
}

// ORACLE resume_picker/archive_tests.rs:199 archived_status_preserves_directory_filter_and_hides_archive_shortcut kind=snap owner=routes-lists
#[test]
fn archived_status_preserves_directory_filter_and_hides_archive_shortcut() {
    kit::todo();
}

// ORACLE resume_picker/archive_tests.rs:264 archived_session_restore_resumes_after_completion kind=ui owner=routes-lists
#[test]
fn archived_session_restore_resumes_after_completion() {
    kit::todo();
}

// ORACLE resume_picker/archive_tests.rs:338 archive_cloud_preserves_same_id_current_local_row_and_transcript kind=ui owner=routes-lists
#[test]
fn archive_cloud_preserves_same_id_current_local_row_and_transcript() {
    kit::todo();
}

// ---- oracle: resume_picker/catalog_tests.rs (1 screen-level tests here; 2 state tests left to routes-lists)
// ORACLE resume_picker/catalog_tests.rs:101 picker_selections_retain_each_rows_source kind=ui owner=routes-lists
#[test]
fn picker_selections_retain_each_rows_source() {
    kit::todo();
}

// ---- oracle: resume_picker/sources_tests.rs (1 screen-level tests here; 8 state tests left to routes-lists)
// ORACLE resume_picker/sources_tests.rs:520 partial_failure_keeps_healthy_rows_visible kind=snap owner=routes-lists snap=partial_backend_failure
#[test]
fn partial_failure_keeps_healthy_rows_visible() {
    kit::todo();
}

// ---- oracle: resume_picker_color_tests.rs (4 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE resume_picker_color_tests.rs:10 thread_colors_and_selection_contrast_in_both_picker_layouts kind=ui owner=routes-lists
#[test]
fn thread_colors_and_selection_contrast_in_both_picker_layouts() {
    kit::todo();
}

// ORACLE resume_picker_color_tests.rs:117 loading_overlay_clears_inherited_selection_colors_and_reversal kind=ui owner=routes-lists
#[test]
fn loading_overlay_clears_inherited_selection_colors_and_reversal() {
    kit::todo();
}

// ORACLE resume_picker_color_tests.rs:154 compact_picker_keeps_metadata_and_labeled_primary_actions kind=snap owner=routes-lists snap=codex_tui__resume_picker__color_tests__compact_picker_keeps_metadata_and_labeled_primary_actions
#[test]
fn compact_picker_keeps_metadata_and_labeled_primary_actions() {
    kit::todo();
}

// ORACLE resume_picker_color_tests.rs:263 narrow_toolbar_keeps_keyboard_focused_control_visible kind=snap owner=routes-lists snap=codex_tui__resume_picker__color_tests__narrow_toolbar_keeps_keyboard_focused_control_visible
#[test]
fn narrow_toolbar_keeps_keyboard_focused_control_visible() {
    kit::todo();
}

// ---- oracle: resume_picker_pagination_error_tests.rs (1 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE resume_picker_pagination_error_tests.rs:9 later_page_error_preserves_loaded_sessions kind=snap owner=routes-lists snap=codex_tui__resume_picker__pagination_error_tests__resume_picker_pagination_error
#[test]
fn later_page_error_preserves_loaded_sessions() {
    kit::todo();
}
