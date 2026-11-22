//! Shard `pager` (S16): the static pager (`pager_overlay.rs`, `StaticOverlay`) is a
//! full-screen sheet of the session transcript's directory
//! (`src/pages/session/transcript/pager.rs`, chunk transcript); the transcript
//! overlay's tests wait for the transcript sheet.

use crate::kit::{self, Owner, Screen};

// ---- oracle: pager_overlay.rs (3 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE pager_overlay.rs:568 static_overlay_snapshot_basic kind=snap owner=chat-live snap=static_overlay_snapshot_basic
#[test]
fn static_overlay_snapshot_basic() {
    let s = Screen::build(Owner::ChatLive)
        .size(40, 10)
        .entry("transcript", "pager_basic")
        .open();
    s.assert_snap("static_overlay_snapshot_basic");
}

// ORACLE pager_overlay.rs:581 static_overlay_wraps_long_lines kind=snap owner=chat-live snap=static_overlay_wraps_long_lines
#[test]
fn static_overlay_wraps_long_lines() {
    let s = Screen::build(Owner::ChatLive)
        .size(24, 8)
        .entry("transcript", "pager_wraps_long_lines")
        .open();
    s.assert_snap("static_overlay_wraps_long_lines");
}

// ORACLE pager_overlay.rs:593 pager_view_content_height_counts_renderables kind=ui owner=chat-live
#[test]
fn pager_view_content_height_counts_renderables() {
    kit::na(
        Owner::ChatLive,
        "the sum of the desired heights of ratatui renderables; the pager's text is one rtwk Column, whose height rtwk measures (the 100% and the ~ rows of the snapshots show it)",
    );
}

// ---- oracle: pager_overlay/scrolling_tests.rs (2 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE pager_overlay/scrolling_tests.rs:18 scrolled_static_renderables_match_clipped_buffer_slices kind=ui owner=chat-live
#[test]
fn scrolled_static_renderables_match_clipped_buffer_slices() {
    kit::na(
        Owner::ChatLive,
        "ratatui buffer slicing of scrolled renderables (render_offset_content); rtwk's scroll view clips its child, renderer replaced (ADR-004)",
    );
}

// ORACLE pager_overlay/scrolling_tests.rs:72 fallback_handles_offsets_near_maximum_height kind=ui owner=chat-live
#[test]
fn fallback_handles_offsets_near_maximum_height() {
    kit::na(
        Owner::ChatLive,
        "a ratatui renderable of u16::MAX rows drawn from an offset near it (render_offset_content); rtwk's offsets are i64 and its scroll view lays out what is visible",
    );
}

// ---- oracle: pager_overlay/transcript_tests.rs (18 screen-level tests here; 4 state tests left to routes-lists)
// ORACLE pager_overlay/transcript_tests.rs:32 empty_pending_select_mode_closes_instead_of_retaining_overlay_focus kind=ui owner=chat-live
#[test]
fn empty_pending_select_mode_closes_instead_of_retaining_overlay_focus() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:45 select_enter_closes_and_remapped_action_marks_in_the_inline_overlay kind=ui owner=chat-live
#[test]
fn select_enter_closes_and_remapped_action_marks_in_the_inline_overlay() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:113 scrolling_prepend_and_resize_preserve_reading_content kind=ui owner=chat-live
#[test]
fn scrolling_prepend_and_resize_preserve_reading_content() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:168 activity_hint_uses_the_configured_pager_jump_binding kind=snap owner=chat-live
#[test]
fn activity_hint_uses_the_configured_pager_jump_binding() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:215 pager_bindings_use_rendered_page_height_and_keep_control_f_navigation kind=ui owner=chat-live
#[test]
fn pager_bindings_use_rendered_page_height_and_keep_control_f_navigation() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:248 focus_loss_stops_edge_drag_and_preserves_selection_after_focus_returns kind=snap owner=chat-live
#[test]
fn focus_loss_stops_edge_drag_and_preserves_selection_after_focus_returns() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:354 opening_find_retains_the_selected_mutable_revision kind=ui owner=chat-live
#[test]
fn opening_find_retains_the_selected_mutable_revision() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:438 jump_top_holds_the_visible_page_until_all_history_arrives kind=snap owner=chat-live
#[test]
fn jump_top_holds_the_visible_page_until_all_history_arrives() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:505 highlighting_reveals_prompt_body_when_only_padding_is_visible kind=ui owner=chat-live
#[test]
fn highlighting_reveals_prompt_body_when_only_padding_is_visible() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:540 transcript_overlay_snapshots_paginated_history_states kind=snap owner=chat-live snap=codex_tui__pager_overlay__transcript_tests__transcript_overlay_paginated_history_states
#[test]
fn transcript_overlay_snapshots_paginated_history_states() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:565 transcript_overlay_preserves_semantic_web_links kind=ui owner=chat-live
#[test]
fn transcript_overlay_preserves_semantic_web_links() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:586 transcript_overlay_preserves_live_tail_when_prepending_history kind=ui owner=chat-live
#[test]
fn transcript_overlay_preserves_live_tail_when_prepending_history() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:616 transcript_overlay_absorbs_older_tail_into_live_without_duplicate_cells kind=ui owner=chat-live
#[test]
fn transcript_overlay_absorbs_older_tail_into_live_without_duplicate_cells() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:659 transcript_overlay_live_tail_preserves_semantic_web_links kind=ui owner=chat-live
#[test]
fn transcript_overlay_live_tail_preserves_semantic_web_links() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:715 transcript_overlay_history_rebuild_preserves_only_the_live_tail kind=snap owner=chat-live snap=codex_tui__pager_overlay__transcript_tests__transcript_overlay_completed_stream
#[test]
fn transcript_overlay_history_rebuild_preserves_only_the_live_tail() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:796 transcript_overlay_consolidation_remaps_highlight_after_range kind=ui owner=chat-live
#[test]
fn transcript_overlay_consolidation_remaps_highlight_after_range() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:838 completing_agent_and_plan_streams_preserves_reading_in_later_fragments kind=ui owner=chat-live
#[test]
fn completing_agent_and_plan_streams_preserves_reading_in_later_fragments() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:928 navigation_supersedes_home_before_and_after_the_first_draw kind=ui owner=chat-live
#[test]
fn navigation_supersedes_home_before_and_after_the_first_draw() {
    kit::missing(
        Owner::ChatLive,
        "the transcript overlay: the transcript sheet (step 9 of the transcript chunk)",
    );
}
