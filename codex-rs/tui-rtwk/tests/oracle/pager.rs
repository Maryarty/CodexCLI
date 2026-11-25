//! Shard `pager` (S16): the static pager (`pager_overlay.rs`, `StaticOverlay`) is a
//! full-screen sheet of the session transcript's directory
//! (`src/pages/session/transcript/pager.rs`, chunk transcript); the transcript
//! overlay's tests are the transcript sheet's (`sheet.rs`), the ones that need
//! selection texts, a live feed or the history states say what they wait for.

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
    // the oracle begins select mode on an overlay of no cells and the next render closes it; here
    // the select chord asks for it on a sheet over a thread with no entries
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 8)
        .entry("transcript", "sheet_empty")
        .open();
    s.assert_contains("T R A N S C R I P T");
    s.keys("leader [");
    s.assert_absent("T R A N S C R I P T");
    s.assert_contains("debug screens");
}

// ORACLE pager_overlay/transcript_tests.rs:45 select_enter_closes_and_remapped_action_marks_in_the_inline_overlay kind=ui owner=chat-live
#[test]
fn select_enter_closes_and_remapped_action_marks_in_the_inline_overlay() {
    fn sheet(keymap: &'static [(&'static str, &'static str)]) -> Screen {
        let s = Screen::build(Owner::ChatLive)
            .size(40, 8)
            .keymap(keymap)
            .entry("transcript", "sheet_copy_me")
            .open();
        s.assert_contains("T R A N S C R I P T");
        s
    }
    // `find_transcript` remapped to `v`: select mode, begun, keeps `v` for its mark (the `c` of `copy me`)
    let mut s = sheet(&[("global.find_transcript", "v")]);
    s.keys("leader [");
    s.keys("v");
    s.assert_style_in("copy me", 0, "reversed");
    s.assert_style_in("copy me", 1, "!reversed");
    // the same key outside select mode begins Find: the next `v` is its query, and no select mode begins
    let mut s = sheet(&[("global.find_transcript", "v")]);
    s.keys("v");
    s.assert_contains("Find:");
    s.keys("v");
    s.assert_contains("Find: v");
    s.assert_absent("v mark");
    // the pager's own find key remapped to Enter: Enter in select mode (nothing marked) closes the sheet
    let mut s = sheet(&[("global.find_transcript", "v"), ("pager.find", "enter")]);
    s.keys("leader [");
    s.assert_contains("v mark");
    s.keys("enter");
    s.assert_absent("T R A N S C R I P T");
    // the oracle's last case begins copy mode on an ordinary answer (`begin_copy_mode`) and copies
    // with Enter, which closes the overlay once the copy is delivered: copy mode is begun by
    // `/copy` over the chat's own transcript and has no key on a sheet yet, so that case is
    // f-copy-mode's (the sheet's `Step::CopyAndFollow` here closes it the same way for select mode)
}

// ORACLE pager_overlay/transcript_tests.rs:113 scrolling_prepend_and_resize_preserve_reading_content kind=ui owner=chat-live
#[test]
fn scrolling_prepend_and_resize_preserve_reading_content() {
    kit::missing(
        Owner::ChatLive,
        "a history page prepended to the open sheet: a seeded sheet entry plays its notifications before the first draw and no route feeds it a `thread/turns/list` page (the thread's history paging is chunk-live's, `ThreadAsk::History`)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:168 activity_hint_uses_the_configured_pager_jump_binding kind=snap owner=chat-live
#[test]
fn activity_hint_uses_the_configured_pager_jump_binding() {
    kit::missing(
        Owner::ChatLive,
        "the news reaches an open sheet only while the reader is away and the thread changes: a seeded sheet entry plays before its first draw and the chat does not open the sheet yet (Ctrl+T in session_page is chunk-live's); the remapped and unbound jump keys wait for the keymap to reach the pager. The row itself is built and tested away from the tail: tests/pages/transcript_sheet.rs",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:215 pager_bindings_use_rendered_page_height_and_keep_control_f_navigation kind=ui owner=chat-live
#[test]
fn pager_bindings_use_rendered_page_height_and_keep_control_f_navigation() {
    // 40x11 over forty rows: Ctrl+F is a page forward, PageUp a page back by
    // the same rendered page height, so the pair restores the screen
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 11)
        .entry("transcript", "sheet_basic")
        .open();
    s.keys("home pagedown");
    let initial = s.shot();
    s.keys("ctrl+f");
    s.assert_rows_differ(&initial);
    s.keys("pageup");
    s.assert_same_rows(&initial);
}

// ORACLE pager_overlay/transcript_tests.rs:248 focus_loss_stops_edge_drag_and_preserves_selection_after_focus_returns kind=snap owner=chat-live
#[test]
fn focus_loss_stops_edge_drag_and_preserves_selection_after_focus_returns() {
    kit::missing(
        Owner::ChatLive,
        "the selection, find and copy of a sheet are made over the texts the rows show (rtwk `ListView::item_text`, one line per transcript row equal to what it renders, the items drawing their part through `ListView::selection_of`); the texts are chunk-cells' and are not wired on the list yet (w2-transcript wires find, select and copy on it; transcript chunk steps 4-7)",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:354 opening_find_retains_the_selected_mutable_revision kind=ui owner=chat-live
#[test]
fn opening_find_retains_the_selected_mutable_revision() {
    kit::missing(
        Owner::ChatLive,
        "G-new-find-room-text: the selected cell is an in-progress dynamic tool call whose revision the sheet keeps while the call is interrupted; a tool row has no text (`Gap::Rows`), so select mode cannot take it and Find has no revision of it to retain",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:438 jump_top_holds_the_visible_page_until_all_history_arrives kind=snap owner=chat-live
#[test]
fn jump_top_holds_the_visible_page_until_all_history_arrives() {
    kit::missing(
        Owner::ChatLive,
        "Home loads the history to its beginning while the sheet holds the page and says Loading: the thread model has no Loading or Failed history state yet (chunk-live) and no route feeds the sheet a history page",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:505 highlighting_reveals_prompt_body_when_only_padding_is_visible kind=ui owner=chat-live
#[test]
fn highlighting_reveals_prompt_body_when_only_padding_is_visible() {
    // 120x10: the sheet keeps four rows for the transcript, which at its end
    // shows the answer and only the prompt's bottom padding
    let mut s = Screen::build(Owner::ChatLive)
        .size(120, 10)
        .entry("transcript", "sheet_reveal")
        .open();
    s.assert_absent("first prompt");
    // Esc marks a prompt (the oracle's `set_highlight_cell(Some(0))`)
    s.keys("esc");
    s.assert_contains("first prompt");
    // exactly the body is reversed: not the `›` before it, no other cell
    let rows = s.rows();
    let (row, column) = rows
        .iter()
        .enumerate()
        .find_map(|(row, text)| text.find("first prompt").map(|at| (row, at)))
        .expect("the prompt is on the screen");
    let column = rows[row][..column].chars().count();
    for (y, text) in rows.iter().enumerate() {
        for x in 0..text.chars().count() {
            let body = y == row && (column..column + "first prompt".len()).contains(&x);
            s.assert_style(x, y, if body { "reversed" } else { "!reversed" });
        }
    }
}

// ORACLE pager_overlay/transcript_tests.rs:540 transcript_overlay_snapshots_paginated_history_states kind=snap owner=chat-live snap=codex_tui__pager_overlay__transcript_tests__transcript_overlay_paginated_history_states
#[test]
fn transcript_overlay_snapshots_paginated_history_states() {
    kit::missing(
        Owner::ChatLive,
        "the four history states (partial, loading, failed, complete) of the status row: the thread model keeps only whether older history can be loaded (`can_load_history`); Loading and Failed are chunk-live's, and the sheet's rows for Partial and Complete are built (status row in sheet.rs) without a route to feed a page",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:565 transcript_overlay_preserves_semantic_web_links kind=ui owner=chat-live
#[test]
fn transcript_overlay_preserves_semantic_web_links() {
    let s = Screen::build(Owner::ChatLive)
        .size(24, 10)
        .entry("transcript", "sheet_link")
        .open();
    // the link wraps over rows: every cell of the first piece links to the whole target
    s.assert_link(
        "https://example.com/",
        "https://example.com/a/very/long/path",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:586 transcript_overlay_preserves_live_tail_when_prepending_history kind=ui owner=chat-live
#[test]
fn transcript_overlay_preserves_live_tail_when_prepending_history() {
    kit::na(
        Owner::ChatLive,
        "the overlay keeps its own copy of the cells and syncs the active cell into it (`sync_live_tail`, `prepend`); the sheet reads the thread's one transcript (ADR-003), so there is no copy and no tail to keep in step",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:616 transcript_overlay_absorbs_older_tail_into_live_without_duplicate_cells kind=ui owner=chat-live
#[test]
fn transcript_overlay_absorbs_older_tail_into_live_without_duplicate_cells() {
    kit::na(
        Owner::ChatLive,
        "`absorb_tail_into_live` repairs the overlay's own cell copy after a hydrated revision; the sheet reads the thread's one transcript (ADR-003), so no cell can be there twice",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:659 transcript_overlay_live_tail_preserves_semantic_web_links kind=ui owner=chat-live
#[test]
fn transcript_overlay_live_tail_preserves_semantic_web_links() {
    kit::na(
        Owner::ChatLive,
        "the live tail is a synced copy of the active cell (`sync_live_tail`); the sheet renders the same item in place, and the link of a streamed or finished answer is the one test transcript_overlay_preserves_semantic_web_links",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:715 transcript_overlay_history_rebuild_preserves_only_the_live_tail kind=snap owner=chat-live snap=codex_tui__pager_overlay__transcript_tests__transcript_overlay_completed_stream
#[test]
fn transcript_overlay_history_rebuild_preserves_only_the_live_tail() {
    kit::na(
        Owner::ChatLive,
        "`replace_cells`/`consolidate_cells` rebuild the overlay's own cell copy and must keep only the live tail; the sheet reads the thread's one transcript (ADR-003), a rebuild is the model's `apply`",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:796 transcript_overlay_consolidation_remaps_highlight_after_range kind=ui owner=chat-live
#[test]
fn transcript_overlay_consolidation_remaps_highlight_after_range() {
    kit::na(
        Owner::ChatLive,
        "the oracle's highlight is a cell index that regrouping shifts; the sheet marks the nth prompt of the thread's transcript (`nth_user`), which consolidation of cells cannot move",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:838 completing_agent_and_plan_streams_preserves_reading_in_later_fragments kind=ui owner=chat-live
#[test]
fn completing_agent_and_plan_streams_preserves_reading_in_later_fragments() {
    kit::missing(
        Owner::ChatLive,
        "a stream that completes under a reading sheet: a seeded sheet entry plays before its first draw and the chat does not open the sheet yet (Ctrl+T in session_page is chunk-live's), so no route changes the thread under an open sheet",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:928 navigation_supersedes_home_before_and_after_the_first_draw kind=ui owner=chat-live
#[test]
fn navigation_supersedes_home_before_and_after_the_first_draw() {
    kit::missing(
        Owner::ChatLive,
        "Home asks for the older history and a later key supersedes it: the thread model has no LoadingOlder state yet (chunk-live) and no route feeds the sheet a history page",
    );
}
