//! Shard `renderer_internals` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit;

// ---- oracle: markdown_stream.rs (7 screen-level tests here; 17 state tests left to chat-live)
// ORACLE markdown_stream.rs:370 heading_starts_on_new_line_when_following_paragraph kind=ui owner=chat-live
#[test]
fn heading_starts_on_new_line_when_following_paragraph() {
    kit::todo();
}

// ORACLE markdown_stream.rs:425 heading_not_inlined_when_split_across_chunks kind=ui owner=chat-live
#[test]
fn heading_not_inlined_when_split_across_chunks() {
    kit::todo();
}

// ORACLE markdown_stream.rs:557 utf8_boundary_safety_and_wide_chars kind=ui owner=chat-live
#[test]
fn utf8_boundary_safety_and_wide_chars() {
    kit::todo();
}

// ORACLE markdown_stream.rs:641 empty_fenced_block_is_dropped_and_separator_preserved_before_heading kind=ui owner=chat-live
#[test]
fn empty_fenced_block_is_dropped_and_separator_preserved_before_heading() {
    kit::todo();
}

// ORACLE markdown_stream.rs:678 loose_list_with_split_dashes_matches_full_render kind=ui owner=chat-live
#[test]
fn loose_list_with_split_dashes_matches_full_render() {
    kit::todo();
}

// ORACLE markdown_stream.rs:704 loose_vs_tight_list_items_streaming_matches_full kind=ui owner=chat-live
#[test]
fn loose_vs_tight_list_items_streaming_matches_full() {
    kit::todo();
}

// ORACLE markdown_stream.rs:861 collector_source_chunks_round_trip_into_agent_fence_unwrapping kind=ui owner=chat-live
#[test]
fn collector_source_chunks_round_trip_into_agent_fence_unwrapping() {
    kit::todo();
}

// ---- oracle: render/highlight.rs (2 screen-level tests here; 41 state tests left to chat-live)
// ORACLE render/highlight.rs:986 highlight_crlf_strips_carriage_return kind=ui owner=chat-live
#[test]
fn highlight_crlf_strips_carriage_return() {
    kit::todo();
}

// ORACLE render/highlight.rs:1188 ansi_family_foreground_palette_snapshot kind=snap owner=chat-live snap=ansi_family_foreground_palette
#[test]
fn ansi_family_foreground_palette_snapshot() {
    kit::todo();
}

// ---- oracle: render/highlight_streaming_tests.rs (1 screen-level tests here; 3 state tests left to chat-live)
// ORACLE render/highlight_streaming_tests.rs:50 crossing_each_highlighting_limit_requires_a_canonical_render kind=ui owner=chat-live
#[test]
fn crossing_each_highlighting_limit_requires_a_canonical_render() {
    kit::todo();
}

// ---- oracle: render/renderable_tests.rs (1 screen-level tests here; 2 state tests left to chat-live)
// ORACLE render/renderable_tests.rs:76 flex_caches_child_height_across_frame_passes kind=ui owner=chat-live
#[test]
fn flex_caches_child_height_across_frame_passes() {
    kit::todo();
}

// ---- oracle: wrapping.rs (8 screen-level tests here; 39 state tests left to chat-live)
// ORACLE wrapping.rs:1578 wrap_lines_applies_initial_indent_only_once kind=ui owner=chat-live
#[test]
fn wrap_lines_applies_initial_indent_only_once() {
    kit::todo();
}

// ORACLE wrapping.rs:1596 wrap_lines_without_indents_is_concat_of_single_wraps kind=ui owner=chat-live
#[test]
fn wrap_lines_without_indents_is_concat_of_single_wraps() {
    kit::todo();
}

// ORACLE wrapping.rs:1604 wrap_lines_accepts_borrowed_iterators kind=ui owner=chat-live
#[test]
fn wrap_lines_accepts_borrowed_iterators() {
    kit::todo();
}

// ORACLE wrapping.rs:1612 wrap_lines_accepts_str_slices kind=ui owner=chat-live
#[test]
fn wrap_lines_accepts_str_slices() {
    kit::todo();
}

// ORACLE wrapping.rs:1817 adaptive_wrap_line_mixed_line_counts_leading_spaces_before_first_word kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_line_counts_leading_spaces_before_first_word() {
    kit::todo();
}

// ORACLE wrapping.rs:1832 adaptive_wrap_line_mixed_line_resplits_long_token_for_continuation_width kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_line_resplits_long_token_for_continuation_width() {
    kit::todo();
}

// ORACLE wrapping.rs:1851 adaptive_wrap_line_mixed_url_counts_halfwidth_sound_marks kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_url_counts_halfwidth_sound_marks() {
    kit::todo();
}

// ORACLE wrapping.rs:1860 adaptive_wrap_line_mixed_url_makes_progress_for_an_indivisible_grapheme kind=ui owner=chat-live
#[test]
fn adaptive_wrap_line_mixed_url_makes_progress_for_an_indivisible_grapheme() {
    kit::todo();
}
