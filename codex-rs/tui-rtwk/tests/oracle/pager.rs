//! Shard `pager` (screens S16, owner routes-lists): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: pager_overlay.rs (3 screen-level tests here; 1 state tests left to routes-lists)
// ORACLE pager_overlay.rs:568 static_overlay_snapshot_basic kind=snap owner=routes-lists snap=static_overlay_snapshot_basic
#[test]
fn static_overlay_snapshot_basic() {
    Screen::build(Owner::RoutesLists)
        .size(40, 10)
        .entry("pager", "static_basic")
        .open()
        .assert_snap("static_overlay_snapshot_basic");
}

// ORACLE pager_overlay.rs:581 static_overlay_wraps_long_lines kind=snap owner=routes-lists snap=static_overlay_wraps_long_lines
#[test]
fn static_overlay_wraps_long_lines() {
    Screen::build(Owner::RoutesLists)
        .size(24, 8)
        .entry("pager", "static_wraps_long_lines")
        .open()
        .assert_snap("static_overlay_wraps_long_lines");
}

// ORACLE pager_overlay.rs:593 pager_view_content_height_counts_renderables kind=ui owner=routes-lists
#[test]
fn pager_view_content_height_counts_renderables() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with the pager over two blocks, `a0 a1` and `b0 b1 b2`; the content height shows as the extent of the page",
    );
    let s = Screen::build(Owner::RoutesLists)
        .size(80, 24)
        .entry("pager", "two_blocks")
        .open();
    // The oracle's content height is 5 at width 80: the five lines follow one another, nothing else.
    s.assert_in_order(&["a0", "a1", "b0", "b1", "b2"]);
}

// ---- oracle: pager_overlay/scrolling_tests.rs (2 screen-level tests here; 0 state tests left to routes-lists)
// ORACLE pager_overlay/scrolling_tests.rs:18 scrolled_static_renderables_match_clipped_buffer_slices kind=ui owner=routes-lists
#[test]
fn scrolled_static_renderables_match_clipped_buffer_slices() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with the pager over the styled, wrapped, wide-glyph lines of the oracle",
    );
    // The oracle clips a scrolled renderable to the slice of the full render, at four widths.
    // A scrolled page keeps every style: the first line green, bold wide glyphs, blue words, the last line red.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(28, 8)
        .entry("pager", "styled_wrapped")
        .open();
    for width in [5u16, 7, 13, 28] {
        s.resize(width, 8);
        s.keys("home");
        s.assert_text_style("prefix", "fg=Green");
        s.keys("end");
        s.assert_text_style("last", "fg=Red");
    }
}

// ORACLE pager_overlay/scrolling_tests.rs:72 fallback_handles_offsets_near_maximum_height kind=ui owner=routes-lists
#[test]
fn fallback_handles_offsets_near_maximum_height() {
    kit::pending(
        Owner::RoutesLists,
        "no entry with the pager over one renderable of 65535 rows that draws `x` on its last row",
    );
    // At the maximum offset only the last row is left: the `x`, at the top of the page.
    let mut s = Screen::build(Owner::RoutesLists)
        .size(1, 2)
        .entry("pager", "maximum_height")
        .open();
    s.keys("end");
    s.assert_text_at(0, 0, "x");
}

// ---- oracle: pager_overlay/transcript_tests.rs (18 screen-level tests here; 4 state tests left to routes-lists)
// ORACLE pager_overlay/transcript_tests.rs:32 empty_pending_select_mode_closes_instead_of_retaining_overlay_focus kind=ui owner=routes-lists
#[test]
fn empty_pending_select_mode_closes_instead_of_retaining_overlay_focus() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no select mode (ctrl+space): it closes on q only",
    );
    let mut s = Screen::build(Owner::RoutesLists).size(40, 8).live().open();
    s.keys("ctrl+t");
    s.keys("ctrl+space");
    // The oracle's overlay is done after the render: the pager is closed.
    s.assert_absent("T R A N S C R I P T");
}

// ORACLE pager_overlay/transcript_tests.rs:45 select_enter_closes_and_remapped_action_marks_in_the_inline_overlay kind=ui owner=routes-lists
#[test]
fn select_enter_closes_and_remapped_action_marks_in_the_inline_overlay() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no select mode (ctrl+space) and no copy action; the remapped key needs `.keymap(..)`, which is pending in the kit until the port reads a keymap",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 8)
        .script(include_str!("fixtures/pager/copy_me.jsonl"))
        .open();
    s.keys("ctrl+t");
    s.keys("ctrl+space");
    s.keys("enter");
    // The oracle: Enter closes the select mode and the pager is done.
    s.assert_absent("T R A N S C R I P T");
    // Remapped find (`global.find_transcript` = v): in the find prompt `v` does not select.
    let mut r = Screen::build(Owner::RoutesLists)
        .size(40, 8)
        .script(include_str!("fixtures/pager/copy_me.jsonl"))
        .keymap(&[("global.find_transcript", "v")])
        .open();
    r.keys("ctrl+t");
    r.keys("v");
    r.assert_contains("copy me");
    // (the selected text `c`, the copy mode and its `browsing prompts` footer are state, not asserted)
}

// ORACLE pager_overlay/transcript_tests.rs:113 scrolling_prepend_and_resize_preserve_reading_content kind=ui owner=routes-lists
#[test]
fn scrolling_prepend_and_resize_preserve_reading_content() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no jump to an entry (the reading position) and takes no older history while open: no key and no seed for either",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 12)
        .script(include_str!("fixtures/pager/lines30.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle jumps to entry 12 (no key does it yet) and prepends `older`: the rows do not move.
    let before = s.shot();
    // here the older-history step: `s.feed(..)` once the port has one
    s.assert_same_rows(&before);
    s.resize(24, 9);
    s.assert_contains("line 12");
}

// ORACLE pager_overlay/transcript_tests.rs:168 activity_hint_uses_the_configured_pager_jump_binding kind=snap owner=routes-lists
#[test]
fn activity_hint_uses_the_configured_pager_jump_binding() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager shows no `New activity` hint: no hint state and no arrival of a cell while scrolled up; the remapped jump key needs `.keymap(..)`, pending in the kit",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(64, 12)
        .script(include_str!("fixtures/pager/rows20.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle jumps to the top and a cell `new output` arrives: the status row says so.
    s.assert_contains("New activity · end latest");
    // remapped: pager.jump_bottom = f8
    let mut r = Screen::build(Owner::RoutesLists)
        .size(64, 12)
        .script(include_str!("fixtures/pager/rows20.jsonl"))
        .keymap(&[("pager.jump_bottom", "f8")])
        .open();
    r.keys("ctrl+t");
    r.assert_contains("New activity · f8 latest");
    // unbound: no key; the hint carries no key.
    let mut u = Screen::build(Owner::RoutesLists)
        .size(64, 12)
        .script(include_str!("fixtures/pager/rows20.jsonl"))
        .keymap(&[("pager.jump_bottom", "")])
        .open();
    u.keys("ctrl+t");
    u.assert_row_ends_with(8, "New activity");
}

// ORACLE pager_overlay/transcript_tests.rs:215 pager_bindings_use_rendered_page_height_and_keep_control_f_navigation kind=ui owner=routes-lists
#[test]
fn pager_bindings_use_rendered_page_height_and_keep_control_f_navigation() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no jump to an entry (the oracle's reading position): no key does it",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 11)
        .script(include_str!("fixtures/pager/lines40.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle jumps to entry 8, then presses ctrl+f and pageup: the page is as before.
    let before = s.shot();
    s.keys("ctrl+f");
    s.keys("pageup");
    s.assert_same_rows(&before);
}

// ORACLE pager_overlay/transcript_tests.rs:248 focus_loss_stops_edge_drag_and_preserves_selection_after_focus_returns kind=snap owner=routes-lists
#[test]
fn focus_loss_stops_edge_drag_and_preserves_selection_after_focus_returns() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no mouse selection, no copy and follow hint and no edge drag",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 12)
        .script(include_str!("fixtures/pager/rows30.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle presses at column 0 row 2 and drags to column 6 row 7.
    s.drag(0, 2, 6, 7);
    s.assert_row(1, "row 01");
    s.assert_row(2, "row 02");
    s.assert_row(3, "row 03");
    s.assert_row(4, "row 04");
    s.assert_row(5, "row 05");
    s.assert_row(6, "row 06");
    s.assert_row(7, "row 07");
    s.assert_row(8, "enter copy & follow · esc clear");
    // (focus loss and gain, and the repaint, leave these rows as they are: the screen has no focus event)
}

// ORACLE pager_overlay/transcript_tests.rs:354 opening_find_retains_the_selected_mutable_revision kind=ui owner=routes-lists
#[test]
fn opening_find_retains_the_selected_mutable_revision() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no select mode (ctrl+space) and no find (f3); a tool call that changes while the pager is open has no seed",
    );
    let mut s = Screen::build(Owner::RoutesLists).size(60, 12).live().open();
    s.keys("ctrl+t");
    let before = s.shot();
    s.keys("ctrl+space");
    // here the dynamic tool call is marked interrupted (no seed does it)
    s.keys("f3");
    s.keys("esc");
    // The content rows 1 to 7 equal the rows before.
    s.assert_same_rows_in(&before, 1..8);
}

// ORACLE pager_overlay/transcript_tests.rs:438 jump_top_holds_the_visible_page_until_all_history_arrives kind=snap owner=routes-lists
#[test]
fn jump_top_holds_the_visible_page_until_all_history_arrives() {
    kit::pending(
        Owner::RoutesLists,
        "home does not request older history while the history is partial, and no seed prepends older messages while the pager is open",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(27, 8)
        .script(include_str!("fixtures/pager/lines8.jsonl"))
        .open();
    s.keys("ctrl+t");
    s.keys("home");
    s.assert_row(1, "line 5");
    s.assert_row(2, "line 6");
    s.assert_row(3, "line 7");
    s.assert_row(4, "↑ Loading… · end latest");
    // The oracle prepends three messages per label (`earlier`, `oldest`)
    // while the rows 1 to 3 hold, and completes the history: no seed does this
    // while the pager is open. The last rows are then `oldest zero`, `oldest one`,
    // `oldest two`, `end latest`.
}

// ORACLE pager_overlay/transcript_tests.rs:505 highlighting_reveals_prompt_body_when_only_padding_is_visible kind=ui owner=routes-lists
#[test]
fn highlighting_reveals_prompt_body_when_only_padding_is_visible() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no highlighted cell and no scroll to a padding-only view",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(120, 10)
        .script(include_str!("fixtures/pager/user_prompt.jsonl"))
        .open();
    s.keys("ctrl+t");
    s.assert_absent("first prompt");
    // The oracle highlights cell 0 (state, no key): the prompt scrolls into view in reverse video.
    s.assert_contains("first prompt");
    s.assert_text_style("first prompt", "reversed");
}

// ORACLE pager_overlay/transcript_tests.rs:540 transcript_overlay_snapshots_paginated_history_states kind=snap owner=routes-lists snap=codex_tui__pager_overlay__transcript_tests__transcript_overlay_paginated_history_states
#[test]
fn transcript_overlay_snapshots_paginated_history_states() {
    for (entry, section) in [
        ("transcript_loading", "loading"),
        ("transcript_partial", "partial"),
        ("transcript_failed", "failed"),
        ("transcript_complete", "complete"),
    ] {
        Screen::build(Owner::RoutesLists)
            .size(72, 10)
            .entry("pager", entry)
            .open()
            .assert_snap_section(
                "codex_tui__pager_overlay__transcript_tests__transcript_overlay_paginated_history_states",
                section,
            );
    }
}

// ORACLE pager_overlay/transcript_tests.rs:565 transcript_overlay_preserves_semantic_web_links kind=ui owner=routes-lists
#[test]
fn transcript_overlay_preserves_semantic_web_links() {
    kit::pending(
        Owner::RoutesLists,
        "no seed turns the streamed text into a markdown cell that carries a web link; the pager draws no OSC 8 link yet",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(24, 10)
        .script(include_str!("fixtures/pager/web_link.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle finds the OSC 8 of the destination on any cell; the link text wraps over rows at 24 columns.
    s.assert_link(
        "https://example.com",
        "https://example.com/a/very/long/path",
    );
}

// ORACLE pager_overlay/transcript_tests.rs:586 transcript_overlay_preserves_live_tail_when_prepending_history kind=ui owner=routes-lists
#[test]
fn transcript_overlay_preserves_live_tail_when_prepending_history() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager takes no older history while open (no prepend seed); the live tail is seeded at start only",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 10)
        .script(include_str!("fixtures/pager/live_tail.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle prepends `older` here (no seed does it while open).
    s.assert_contains("older");
    s.assert_contains("recent");
    s.assert_contains("live tail");
}

// ORACLE pager_overlay/transcript_tests.rs:616 transcript_overlay_absorbs_older_tail_into_live_without_duplicate_cells kind=ui owner=routes-lists
#[test]
fn transcript_overlay_absorbs_older_tail_into_live_without_duplicate_cells() {
    kit::pending(
        Owner::RoutesLists,
        "the live tail is not absorbed into the history cells while the pager is open: no seed for the absorption of a streamed tail",
    );
    let mut s = Screen::build(Owner::RoutesLists).size(40, 10).live().open();
    s.keys("ctrl+t");
    // The oracle prepends `older group`, syncs the live tail `current group`,
    // then absorbs it as `hydrated group` (state, no key); the rows then hold the
    // hydrated group and not the older one.
    s.assert_contains("hydrated group");
    s.assert_absent("older group");
    s.assert_absent("New activity");
}

// ORACLE pager_overlay/transcript_tests.rs:659 transcript_overlay_live_tail_preserves_semantic_web_links kind=ui owner=routes-lists
#[test]
fn transcript_overlay_live_tail_preserves_semantic_web_links() {
    kit::pending(
        Owner::RoutesLists,
        "no seed turns the streamed tail into a markdown cell that carries a web link; the pager draws no OSC 8 link yet",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(24, 10)
        .script(include_str!("fixtures/pager/web_link_tail.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle syncs the link cell as the live tail and finds its OSC 8 on any cell.
    s.assert_link("https://example.com", "https://example.com/a/streamed/path");
}

// ORACLE pager_overlay/transcript_tests.rs:715 transcript_overlay_history_rebuild_preserves_only_the_live_tail kind=snap owner=routes-lists snap=codex_tui__pager_overlay__transcript_tests__transcript_overlay_completed_stream
#[test]
fn transcript_overlay_history_rebuild_preserves_only_the_live_tail() {
    // The oracle's loop consolidates the two completed cells (state, core's
    // business) and ends on this snapshot for the state without a live tail:
    // the view of the two completed cells, as the pager shows them.
    Screen::build(Owner::RoutesLists)
        .size(40, 10)
        .entry("pager", "transcript_completed_stream")
        .open()
        .assert_snap(
            "codex_tui__pager_overlay__transcript_tests__transcript_overlay_completed_stream",
        );
}

// ORACLE pager_overlay/transcript_tests.rs:796 transcript_overlay_consolidation_remaps_highlight_after_range kind=ui owner=routes-lists
#[test]
fn transcript_overlay_consolidation_remaps_highlight_after_range() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no highlighted cell and no consolidation of cells while open",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 7)
        .script(include_str!("fixtures/pager/lines7.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle prepends `older`, highlights cell 6, consolidates cells 3 to 6 (state, no key):
    // the highlighted cell stays `line6`, in reverse video.
    s.assert_contains("line6");
    s.assert_text_style("line6", "reversed");
}

// ORACLE pager_overlay/transcript_tests.rs:838 completing_agent_and_plan_streams_preserves_reading_in_later_fragments kind=ui owner=routes-lists
#[test]
fn completing_agent_and_plan_streams_preserves_reading_in_later_fragments() {
    kit::pending(
        Owner::RoutesLists,
        "the transcript pager has no reading position to set (jump to entry 2, one row down) and no consolidation of a stream while open",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 9)
        .script(include_str!("fixtures/pager/streamed_passages.jsonl"))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("Passage");
    // The oracle jumps to entry 2, scrolls one row, consolidates the stream (state, no key):
    // the content rows are as before.
    let before = s.shot();
    // here the stream completes (no seed does it while the pager is open)
    s.assert_same_rows_in(&before, 1..8);
}

// ORACLE pager_overlay/transcript_tests.rs:928 navigation_supersedes_home_before_and_after_the_first_draw kind=ui owner=routes-lists
#[test]
fn navigation_supersedes_home_before_and_after_the_first_draw() {
    kit::pending(
        Owner::RoutesLists,
        "home does not request older history while the history is partial, and no seed prepends older messages while the pager is open",
    );
    let mut s = Screen::build(Owner::RoutesLists)
        .size(40, 10)
        .script(include_str!("fixtures/pager/lines30_partial.jsonl"))
        .open();
    s.keys("ctrl+t");
    // The oracle: the same keys without home give the screen the home press must also give.
    let without_home = s.shot();
    s.keys("home");
    // here `received older history` arrives and the history completes (no seed does it while open)
    s.assert_same_rows_in(&without_home, 1..6);
}
