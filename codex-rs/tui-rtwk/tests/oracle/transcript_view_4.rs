//! Shard `transcript_view_4` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: transcript_view/search_tests.rs (27 screen-level tests here; 9 state tests left to chat-live)
// ORACLE transcript_view/search_tests.rs:34 search_match_separates_user_prompts_only_while_visible kind=snap owner=chat-live
#[test]
fn search_match_separates_user_prompts_only_while_visible() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 12)
        .script(include_str!(
            "fixtures/transcript_view_4/search_prompts.jsonl"
        ))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.keys("ctrl+n");
    // the oracle's rows: `› first`, a blank row, `› needle second` (its inline `#` marks, the prompt
    // background of the rows between, are not asserted); the port closes each turn with its
    // `Worked for` row and draws the transcript at the bottom
    s.assert_row(2, "› first");
    s.assert_row_starts_with(5, "  Worked for");
    s.assert_row(8, "› needle second");
    s.assert_text_style("needle", "reversed");
}

// ORACLE transcript_view/search_tests.rs:115 find_keeps_context_and_allows_reading_without_losing_the_query kind=snap owner=chat-live snap=find_context
#[test]
fn find_keeps_context_and_allows_reading_without_losing_the_query() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/find_context.jsonl"
        ))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("needle");
    s.assert_text_style("needle", "reversed");
    s.keys("pagedown");
    s.assert_contains("latest message");
    s.assert_find_query("needle");
    s.keys("pageup");
    let scrolled = s.shot();
    s.assert_find_query("needle");
    s.keys("enter");
    // the oracle draws its footer apart from the buffer (the bare view has none): at 80 columns
    s.assert_find_footer(80, "Find · ctrl+p older · ctrl+n newer · esc latest");
    s.assert_same_rows(&scrolled);
    s.keys("enter");
    s.assert_same_rows(&scrolled);
    s.keys("/");
    s.assert_same_rows(&scrolled);
    s.assert_find_query("needle");
    s.keys("enter");
    s.keys("ctrl+p");
    s.assert_contains("earlier needle");
    s.keys("esc");
    s.assert_contains("latest message");
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: the oracle keeps `height / 3` display rows above the match (here two: the match is the third row of the window, snapshot `find_context`); the port's list takes no `rows` of a jump in a reverse list and tells no item extent, so the match is shown by the least scroll (the second row, one blank above it)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/find_context.jsonl"
        ))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("needle");
    // the oracle's rows: the context above the match, the match, the entry after it
    s.assert_row_starts_with(2, "• needle in the result");
}

// ORACLE transcript_view/search_tests.rs:187 find_reduces_context_to_keep_a_tall_match_visible kind=snap owner=chat-live snap=find_context_for_tall_matches
#[test]
fn find_reduces_context_to_keep_a_tall_match_visible() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: the oracle keeps the first rows of a match taller than the window (`one` at the top of the window, snapshot `find_context_for_tall_matches`, heights 6, 5 and 3); a reverse list shows an item taller than its window by its end (`ensure_index_visible`) and takes no `rows` of a jump, and no item extent is told to the app, so the first rows of the match are scrolled out",
    );
    // the oracle builds a fresh view for each height
    for height in [6, 5, 3] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(6, height)
            .script(include_str!("fixtures/transcript_view_4/find_tall.jsonl"))
            .view_only()
            .open();
        s.keys("/");
        s.type_text("one two three four five");
        s.assert_row_starts_with(0, "• one");
        s.assert_text_style("one", "reversed");
    }
}

// ORACLE transcript_view/search_tests.rs:211 selection_pauses_find_and_normalized_navigation_resumes_the_same_query kind=snap owner=chat-live snap=selection_owns_find_highlight
#[test]
fn selection_pauses_find_and_normalized_navigation_resumes_the_same_query() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 8)
        .script(include_str!(
            "fixtures/transcript_view_4/find_selection.jsonl"
        ))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    // the match is the older entry's `needle` (row 1); the port scans in the frame the key came
    // in, so no scan is pending while the reader selects (the oracle holds one back)
    s.assert_style(8, 1, "reversed");
    // the reader selects the newer entry's line (the oracle's three clicks; while Find is open the
    // port's rows are one text model that joins the bullet and the text by a line break, so a
    // triple click takes the bullet alone: a drag over the text is the line): the selection owns
    // the highlight
    s.drag(2, 5, 13, 5);
    s.assert_style(2, 5, "reversed");
    s.assert_style(13, 5, "reversed");
    s.assert_style(8, 1, "!reversed");
    s.assert_find_query("needle");
    // Esc lets the selection go and Find goes on: the match is drawn again, the query kept
    s.keys("esc");
    s.assert_find_query("needle");
    s.assert_style(8, 1, "reversed");
    s.assert_style(8, 5, "!reversed");
    s.keys("ctrl+n");
    s.assert_style(8, 5, "reversed");
    s.assert_style(8, 1, "!reversed");
    s.assert_find_query("needle");
    s.keys("ctrl+c");
    s.assert_find_query("");
    s.assert_style(8, 5, "!reversed");
}

// ORACLE transcript_view/search_tests.rs:259 paging_away_from_a_live_match_does_not_restart_find kind=ui owner=chat-live
#[test]
fn paging_away_from_a_live_match_does_not_restart_find() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no find over a streaming answer and no live-match hold; paging away from the live match is not a chat step",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/find_live.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("pageup");
    let scrolled = s.shot();
    s.keys("enter");
    s.assert_same_rows(&scrolled);
    s.double_click(0, 0);
    s.assert_contains("earlier context");
    // begin_search clears the selection and the live match stays the current one: the oracle's keys, the rows are its visible part
    s.keys("/");
    s.assert_contains("live needle");
}

// ORACLE transcript_view/search_tests.rs:303 reading_without_a_match_searches_from_the_visible_entry_in_each_direction kind=ui owner=chat-live
#[test]
fn reading_without_a_match_searches_from_the_visible_entry_in_each_direction() {
    // the find reads the match it found (Enter), and the two keys go on from it
    for (key, expected) in [("ctrl+p", "older needle"), ("ctrl+n", "newer needle")] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(30, 4)
            .script(include_str!(
                "fixtures/transcript_view_4/find_reading.jsonl"
            ))
            .view_only()
            .open();
        s.keys("/");
        s.type_text("needle");
        s.keys("enter");
        s.assert_find_footer(80, "Find · ctrl+p older · ctrl+n newer · esc latest");
        s.keys(key);
        s.assert_contains(expected);
    }
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: with no match (the oracle's `restart_search` after it jumped to entry 1, `context`) Ctrl+P and Ctrl+N go on from the entry the reader sees; the port scans a restarted query from the newest row, and the list tells no item extent, so the pane cannot tell which entry is first in view",
    );
    for (key, expected) in [("ctrl+p", "older needle"), ("ctrl+n", "newer needle")] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(30, 4)
            .script(include_str!(
                "fixtures/transcript_view_4/find_reading.jsonl"
            ))
            .view_only()
            .open();
        s.keys("/");
        s.type_text("needle");
        s.keys("enter");
        // the oracle reads entry 1 (`context`) in the one row it has, with no match
        s.keys("pageup");
        s.keys("/");
        s.keys("backspace");
        s.type_text("e");
        s.keys("enter");
        s.keys(key);
        s.assert_contains(expected);
    }
}

// ORACLE transcript_view/search_tests.rs:337 resizing_a_read_find_result_preserves_relative_navigation kind=snap owner=chat-live snap=find_context_after_resize
#[test]
fn resizing_a_read_find_result_preserves_relative_navigation() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 6)
        .script(include_str!("fixtures/transcript_view_4/find_resize.jsonl"))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    // the oracle re-wraps at width 20 with its area kept at 30 by prepare_width; the kit resize changes the terminal width too
    s.resize(20, 6);
    // the match read stays the newer entry's, and the keys go on from it
    s.assert_text_style("needle", "reversed");
    s.assert_contains("newer explanation");
    s.keys("ctrl+p");
    s.assert_contains("older explanation");
    s.keys("ctrl+n");
    s.assert_contains("newer explanation");
    s.keys("ctrl+n");
    s.resize(25, 6);
    s.keys("ctrl+p");
    s.assert_contains("older explanation");
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: the oracle keeps `height / 3` display rows above the read match after the resize (snapshot `find_context_after_resize`, three rows at width 20); the port's list takes no `rows` of a jump in a reverse list and tells no item extent, so the match is shown by the least scroll",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/find_resize.jsonl"))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    s.resize(20, 3);
    s.assert_snap("find_context_after_resize");
}

// ORACLE transcript_view/search_tests.rs:389 reading_a_find_result_loads_older_context kind=ui owner=chat-live
#[test]
fn reading_a_find_result_loads_older_context() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find and no older-history load: the kit has no step that marks the history partial and answers a page request",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!(
            "fixtures/transcript_view_4/find_history.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("pageup");
    // the oracle asks for older history twice (needs_history); the request itself is not drawn, the query stays
    s.keys("enter");
    s.keys("pageup");
    s.assert_contains("needle");
}

// ORACLE transcript_view/search_tests.rs:410 paging_below_a_live_find_result_resumes_current_output kind=snap owner=chat-live
#[test]
fn paging_below_a_live_find_result_resumes_current_output() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find over a live answer; the oracle replaces the live tail, and the chat appends deltas, which no kit step does yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/find_live.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("pagedown");
    s.assert_row(0, "earlier context");
    s.assert_row(1, "");
    s.assert_row(2, "updated live output");
    s.assert_contains("needle");
}

// ORACLE transcript_view/search_tests.rs:579 empty_find_keeps_following_new_output_after_a_page_join kind=snap owner=chat-live snap=empty_find_follows_output_after_page_join
#[test]
fn empty_find_keeps_following_new_output_after_a_page_join() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find with an empty query, and the kit has no step that joins an older page of history under a live answer",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 8)
        .script(include_str!(
            "fixtures/transcript_view_4/page_join_follow.jsonl"
        ))
        .open();
    s.keys("/");
    // the oracle joins an older page (`older computer action`, then `joined computer actions`) and a new answer before this frame: those are feed and history steps the kit does not have yet
    s.assert_snap("empty_find_follows_output_after_page_join");
}

// ORACLE transcript_view/search_tests.rs:625 empty_find_preserves_reading_inside_a_joined_group kind=ui owner=chat-live
#[test]
fn empty_find_preserves_reading_inside_a_joined_group() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find with an empty query and no reading position that a page join keeps; the kit has no step that joins an older page",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 4)
        .script(include_str!(
            "fixtures/transcript_view_4/joined_reading.jsonl"
        ))
        .open();
    s.keys("pageup");
    s.keys("/");
    let before = s.shot();
    // the oracle joins an older page and a group over the displayed entry here: the join is not a kit step yet
    s.type_text("x");
    s.keys("backspace");
    s.assert_same_rows(&before);
    s.keys("esc");
    s.assert_same_rows(&before);
}

// ORACLE transcript_view/search_tests.rs:663 previous_match_continues_through_regrouped_pages_without_revisiting_newer_hits kind=snap owner=chat-live
#[test]
fn previous_match_continues_through_regrouped_pages_without_revisiting_newer_hits() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find over a partial history: no `Searching earlier history` status and no older page can join while the find runs",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/transcript_view_4/regroup_hits.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.assert_contains("needle middle");
    s.keys("ctrl+p");
    s.assert_contains("Searching earlier history… · full transcript · esc cancel");
    // the oracle then joins `earlier context`, `older group` and `needle oldest`, a regrouped page: those are history steps the kit does not have yet
    s.keys("ctrl+n");
    s.assert_contains("needle middle");
    s.keys("ctrl+n");
    s.assert_contains("needle newest");
}

// ORACLE transcript_view/search_tests.rs:734 previous_match_finds_the_prepended_part_of_a_regrouped_entry kind=ui owner=chat-live
#[test]
fn previous_match_finds_the_prepended_part_of_a_regrouped_entry() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find and no regrouped entry: a page join that prepends to a group is not a kit step, and no match is drawn reversed",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!(
            "fixtures/transcript_view_4/regroup_prefix.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    // the older prefix joins the group in the oracle: the join is not a kit step yet
    s.assert_contains("needle older prefix and newer group");
    s.assert_style(0, 0, "reversed");
    s.keys("ctrl+n");
    s.assert_contains("needle newest");
}

// ORACLE transcript_view/search_tests.rs:787 enter_preserves_the_initial_scan_and_each_older_page_until_the_first_match kind=snap owner=chat-live
#[test]
fn enter_preserves_the_initial_scan_and_each_older_page_until_the_first_match() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find, no scanning status line and no older-page loads: an 18 KiB entry scans in the oracle over several frames, and the kit has no older page to join",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!("fixtures/transcript_view_4/scan_long.jsonl"))
        .open();
    s.keys("/");
    s.type_text("LONG T0000 USER");
    s.keys("enter");
    s.assert_contains("Searching… · full transcript · esc cancel");
    s.wait_for("Searching earlier history… · full transcript · esc cancel");
    s.assert_contains("Searching earlier history… · full transcript · esc cancel");
    // the oracle then confirms each older page (`middle page without a hit`, `LONG T0000 USER`), which join by history steps the kit does not have yet
    s.keys("enter");
    s.assert_contains("LONG T0000 USER");
}

// ORACLE transcript_view/search_tests.rs:922 search_finds_the_first_older_entry_after_replaying_the_session_header kind=ui owner=chat-live
#[test]
fn search_finds_the_first_older_entry_after_replaying_the_session_header() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find over a partial history and no session replay: the replayed header and the older page are kit steps that do not exist yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!("fixtures/transcript_view_4/find_replay.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    s.keys("ctrl+p");
    // the oracle replays the header, then an older page (`oldest needle`, `older messages`) joins above it: a history step the kit does not have yet
    s.assert_contains("oldest needle");
}

// ORACLE transcript_view/search_tests.rs:968 next_previous_highlight_and_cancel_share_the_transcript_position kind=snap owner=chat-live
#[test]
fn next_previous_highlight_and_cancel_share_the_transcript_position() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 6)
        .script(include_str!("fixtures/transcript_view_4/find_jump.jsonl"))
        .view_only()
        .open();
    s.keys("pageup");
    let original = s.shot();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.assert_text_style("Needle", "reversed");
    s.keys("ctrl+n");
    // the current match `Needle two` is reversed for its first six cells, the rest of the row is plain
    s.assert_contains("Needle two");
    s.assert_text_style("Needle", "reversed");
    s.assert_text_style("two", "!reversed");
    // the oracle draws its status row apart from the buffer, 80 columns wide
    s.assert_find_footer(
        80,
        "enter accept · ⌃p older · ⌃n newer · full transcript · esc cancel",
    );
    s.keys("left");
    s.keys("esc");
    s.assert_same_rows(&original);
}

// ORACLE transcript_view/search_tests.rs:1026 legacy_search_shortcuts_navigate_in_both_directions_without_wrapping kind=snap owner=chat-live
#[test]
fn legacy_search_shortcuts_navigate_in_both_directions_without_wrapping() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/transcript_view_4/legacy_short.jsonl"
        ))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("USER]");
    s.keys("ctrl+p");
    s.assert_find_footer(
        80,
        "enter accept · ⌃p older · ⌃n newer · full transcript · esc cancel",
    );
    s.keys("ctrl+p");
    s.keys("ctrl+p");
    // the third older press has no older match left: `No more matches`, the match stays on the first entry
    s.assert_find_footer(
        80,
        "No more matches · enter accept · ⌃p older · ⌃n newer · esc cancel",
    );
    s.keys("ctrl+n");
    s.keys("ctrl+n");
    s.keys("ctrl+n");
    s.assert_find_footer(
        80,
        "No more matches · enter accept · ⌃p older · ⌃n newer · esc cancel",
    );
}

// ORACLE transcript_view/search_tests.rs:1066 find_expands_only_the_match_and_preserves_manual_disclosures kind=snap owner=chat-live snap=find_selective_expansion
#[test]
fn find_expands_only_the_match_and_preserves_manual_disclosures() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find over command cells, no hidden exec output and no per-match expansion; the chat draws commands as their default compact cells only",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_4/find_exec.jsonl"))
        .open();
    s.keys("f4");
    s.keys("enter");
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    // only the matching cells expand: `hidden` shows for the needle entries (the oracle's per-entry expansion)
    s.assert_contains("hidden newer needle");
    s.assert_snap("find_selective_expansion");
    s.keys("enter");
    s.keys("f4");
    s.keys("home");
    s.keys("enter");
    s.keys("esc");
    s.assert_contains("hidden unrelated");
    s.keys("f4");
    s.keys("end");
    s.keys("up");
    s.keys("right");
    s.assert_contains("hidden newer needle");
    s.keys("left");
    s.keys("esc");
}

// ORACLE transcript_view/search_tests.rs:1202 find_searches_hidden_live_content_and_collapses_it_when_leaving_the_match kind=ui owner=chat-live
#[test]
fn find_searches_hidden_live_content_and_collapses_it_when_leaving_the_match() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find over a live answer: a search of the hidden live text and its collapse to the compact live cell when the match is left are not built",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 4)
        .script(include_str!("fixtures/transcript_view_4/hidden_live.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    // the oracle's first run has no compact live text; this is its second run (`compact live`), the first run is a fixture of its own
    s.assert_contains("hidden live needle");
    s.keys("enter");
    s.resize(20, 4);
    s.assert_contains("hidden live needle");
    s.keys("ctrl+p");
    s.assert_contains("compact live");
    s.keys("ctrl+n");
    s.assert_contains("updated live needle");
}

// ORACLE transcript_view/search_tests.rs:1268 selected_hidden_live_match_keeps_its_revision_without_persisting_expansion kind=ui owner=chat-live
#[test]
fn selected_hidden_live_match_keeps_its_revision_without_persisting_expansion() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript selection over a hidden live match and no revision kept across live updates; the kit has no step that updates a live answer under a selection",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/hidden_live.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    // the oracle selects by three clicks at column 2 of the hidden match row; the kit's click_text clicks the text's first cell
    s.click_text("hidden live needle");
    s.click_text("hidden live needle");
    s.click_text("hidden live needle");
    // the live answer updates (`updated compact live`, `updated live needle`) under the selection: those are kit steps not written yet
    s.assert_contains("hidden live needle");
    s.keys("ctrl+p");
    s.assert_contains("updated compact live");
    s.keys("ctrl+n");
    s.assert_contains("updated live needle");
}

// ORACLE transcript_view/search_tests.rs:1343 find_reveals_hidden_command_output_and_restores_compact_presentation kind=ui owner=chat-live
#[test]
fn find_reveals_hidden_command_output_and_restores_compact_presentation() {
    kit::pending(
        Owner::ChatLive,
        "the chat draws command output compact with no reveal of hidden output by a transcript find, and no find state to restore the compact view from",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!(
            "fixtures/transcript_view_4/find_command.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("hidden needle");
    let found = s.shot();
    s.assert_contains("hidden needle");
    s.keys("ctrl+t");
    s.assert_same_rows(&found);
    s.keys("enter");
    s.assert_same_rows(&found);
    s.keys("esc");
    let compact = s.shot();
    s.keys("/");
    s.keys("esc");
    // the oracle replaces the group under the match and cancels: the compact rows come back unchanged
    s.assert_same_rows(&compact);
}

// ORACLE transcript_view/search_tests.rs:1415 search_preserves_both_presentation_positions kind=ui owner=chat-live
#[test]
fn search_preserves_both_presentation_positions() {
    kit::na(
        Owner::ChatLive,
        "asserts the saved reading positions of the detailed and compact presentations and the search's current anchor index (state), not drawn; the frames it renders are not compared",
    );
}

// ORACLE transcript_view/search_tests.rs:1537 live_match_stays_displayed_after_commit_and_a_fresh_query_searches_current_content kind=ui owner=chat-live
#[test]
fn live_match_stays_displayed_after_commit_and_a_fresh_query_searches_current_content() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find over a live answer: the live match stays on screen while the answer streams and commits, and no fresh query searches the current content",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/live_commit.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.assert_row(0, "live needle");
    // the oracle's later steps: the live text grows to `live needle again needle`, commits, and a fresh query `current` searches the new text; those are kit steps not written yet
    s.keys("ctrl+n");
    s.assert_contains("live needle again needle");
    s.keys("ctrl+n");
    s.keys("ctrl+n");
    s.keys("ctrl+p");
    s.keys("ctrl+u");
    s.type_text("current");
    s.assert_contains("current needle");
}

// ORACLE transcript_view/search_tests.rs:1664 leaving_a_regrouped_search_match_restarts_from_current_history kind=ui owner=chat-live
#[test]
fn leaving_a_regrouped_search_match_restarts_from_current_history() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find over a regrouped history: a held match does not survive a join, and the chat draws no reversed match in a merged group",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!(
            "fixtures/transcript_view_4/regroup_leave.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("needle");
    // the oracle joins `surviving before` and `older group` and merges the group into `merged needle`: those are history steps the kit does not have yet
    s.keys("ctrl+n");
    s.assert_contains("merged needle");
    s.assert_style(7, 1, "reversed");
}

// ORACLE transcript_view/search_tests.rs:1721 refining_a_query_restores_the_reading_position_while_history_loads kind=ui owner=chat-live
#[test]
fn refining_a_query_restores_the_reading_position_while_history_loads() {
    // the oracle's two positions: the latest row, and a reading position at the first entry
    for top in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(30, 6)
            .replay(Vec::new())
            .view_only()
            .open();
        s.partial_history_of(&[
            "Earlier context",
            "Middle context",
            "Last answer",
            "Fixture link",
        ]);
        if top {
            s.wheel_up(0, 0, 100);
        }
        s.keys("/");
        let before = s.shot();
        s.type_text("F");
        s.assert_contains("Fixture link");
        s.type_text("IRST ANSWER");
        // the query needs older history to find `First answer`: the loading search draws the same rows as before the query was typed
        s.assert_find_footer(
            80,
            "Searching earlier history… · full transcript · esc cancel",
        );
        s.assert_same_rows(&before);
        s.keys("ctrl+c");
    }
}

// ORACLE transcript_view/search_tests.rs:1772 bounded_query_paste_keeps_the_suffix_and_whole_graphemes kind=snap owner=chat-live
#[test]
fn bounded_query_paste_keeps_the_suffix_and_whole_graphemes() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/transcript_view_4/query_limit.jsonl"))
        .view_only()
        .open();
    s.keys("/");
    s.paste("X\r\nY\rZ");
    s.assert_find_query("X\nY\nZ");
    s.keys("esc");
    s.keys("/");
    // a query of 4 KiB less one byte: `a` x 4094 then `Z`
    let original = format!("{}Z", "a".repeat(4096 - 2));
    s.paste(&original);
    s.keys("left");
    // e and a combining accent do not fit: the whole grapheme is left out, the limit is said
    s.paste("e\u{301}");
    s.assert_find_query(&original);
    s.paste("b");
    let full = format!("{}bZ", "a".repeat(4096 - 2));
    s.type_text("c");
    s.assert_find_query(&full);
    // the oracle reads the status before its scan runs (`Searching…`); the port scans in the
    // frame the query changed, and nothing holds this query
    s.assert_find_footer(
        100,
        "No matches · full transcript · esc cancel · query limited to 4 KiB",
    );
    s.keys("delete");
    s.assert_find_query(&full[..full.len() - 1]);
}

// ORACLE transcript_view/search_tests.rs:1819 selection_keys_precede_find_and_typing_returns_to_query kind=ui owner=chat-live
#[test]
fn selection_keys_precede_find_and_typing_returns_to_query() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 6)
        .script(include_str!("fixtures/transcript_view_4/needle_only.jsonl"))
        .view_only()
        .open();
    s.keys("/");
    s.type_text("needle");
    // the oracle selects with Ctrl+Space and Right: the port has no keyboard selection, the reader
    // selects the entry's text with the mouse (a drag over it)
    s.drag(2, 2, 7, 2);
    // the selection keys go to the selection (ctrl+c copies it), Find stays open
    s.keys("ctrl+c");
    s.assert_copy_asked("needle");
    s.assert_find_query("needle");
    // typing returns to the query and ends the selection
    s.type_text("x");
    s.assert_find_query("needlex");
    s.assert_style(2, 2, "!reversed");
    s.assert_style(7, 2, "!reversed");
}

// ---- oracle: transcript_view/select_mode_tests.rs (19 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/select_mode_tests.rs:37 entry_starts_on_the_last_selectable_visible_line kind=ui owner=chat-live
#[test]
fn entry_starts_on_the_last_selectable_visible_line() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: the select_transcript chord (leader, then `[`) starts none, so no cursor lands on the last selectable visible line",
    );
    // oracle case 1 (`top`, `bottom` at 32x6): `v` marks `b`, the first cell of the last line
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_1.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("b", "reversed");
    // oracle case 2 (`text`, empty line): `t`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_2.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("t", "reversed");
    // oracle case 3 (`abc` then six blank rows): `c`, the last text cell
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_3.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("c", "reversed");
    // oracle case 4 (`a`..`j`, scrolled two rows up): `h` is the last selectable visible line
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_4.jsonl"))
        .open();
    s.wheel_up(0, 0, 2);
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("h", "reversed");
    // oracle case 5 (`a` then nine blank rows): select mode starts (its footer is drawn)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_5.jsonl"))
        .open();
    s.keys("leader [");
    s.assert_contains("v mark · enter/q close");
    // oracle case 6 (one empty line): begin_select_mode is refused, no select footer is drawn
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_6.jsonl"))
        .open();
    s.keys("leader [");
    s.assert_absent("v mark · enter/q close");
}

// ORACLE transcript_view/select_mode_tests.rs:72 character_motion_crosses_line_and_entry_boundaries_without_invisible_stops kind=ui owner=chat-live
#[test]
fn character_motion_crosses_line_and_entry_boundaries_without_invisible_stops() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode with a character cursor: left, right and v do not move or mark a grapheme, and no reversed cell is drawn",
    );
    // oracle case 1: `a` / blank / blank / `b`; left, v marks `a`; esc, right, v marks `b`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_1.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("left v");
    s.assert_text_style("a", "reversed");
    s.keys("esc right v");
    s.assert_text_style("b", "reversed");
    // oracle case 2: `ab` with the caret on its separator; right, v marks `a`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_2.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("right v");
    s.assert_text_style("a", "reversed");
    // oracle case 3: a wrapped row of `1234567890` x3 then ` one`; left crosses the invisible space onto the `0` at column 29 of row 0
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_3.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("left v");
    s.assert_style(29, 0, "reversed");
    // oracle case 4: `ab` / `cd`, a keyboard range from the start of `ab` to the start of `cd` (set through the oracle's API: no key reaches it yet)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_4.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("o l");
    s.assert_text_style("b", "reversed");
    // oracle case 5: `abc`, the range `bc`; h moves the caret to `b`, a second h to `a`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_5.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("h");
    s.assert_text_style("b", "reversed");
    s.keys("h");
    s.assert_text_style("ab", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:166 vertical_motion_keeps_the_preferred_display_column kind=ui owner=chat-live
#[test]
fn vertical_motion_keeps_the_preferred_display_column() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: up does not move a caret, and v marks no grapheme",
    );
    // oracle: `abcdef` / `x` / `abcdef`; right x3, up x2, v marks the `d` of the first line (column 3)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_vertical.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("right right right up up v");
    s.assert_style(3, 0, "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:181 enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows kind=ui owner=chat-live
#[test]
fn enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: enter neither exits it nor copies a marked grapheme and follows the latest output",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_enter.jsonl"))
        .open();
    // enter without a mark exits: the select footer is gone
    s.keys("leader [");
    s.assert_contains("v mark · enter/q close");
    s.keys("enter");
    s.assert_absent("v mark · enter/q close");
    // with a mark (v, then a repeated v), enter copies `c` and follows: the select footer is gone and the copied `c` is no longer reversed
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("c", "reversed");
    s.keys("enter");
    s.assert_absent("v mark · enter/q close");
}

// ORACLE transcript_view/select_mode_tests.rs:207 word_motion_crosses_entries_and_word_end_selects_the_destination_word kind=ui owner=chat-live
#[test]
fn word_motion_crosses_entries_and_word_end_selects_the_destination_word() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: g, j, w, v and e move no cursor across entries and mark no word",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_word.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("g j w v e");
    s.assert_text_style("beta", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:223 reverse_line_selection_copies_complete_lines_in_document_order kind=ui owner=chat-live
#[test]
fn reverse_line_selection_copies_complete_lines_in_document_order() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: V marks no line, so the second and third lines are not drawn reversed",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_reverse.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("g j j V k");
    // the marked lines `second` and `third` are drawn reversed in document order (the copy payload is not drawn)
    s.assert_text_style("second", "reversed");
    s.assert_text_style("third", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:238 logical_line_end_handles_empty_lines_and_shifted_dollar kind=ui owner=chat-live
#[test]
fn logical_line_end_handles_empty_lines_and_shifted_dollar() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: g, $, End and v do not move a caret to a logical line end, so no cell is drawn reversed",
    );
    // oracle case 1: `abc` / blank / `z`; g then $ then v marks `c`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_logical_1.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("g $ v");
    s.assert_text_style("c", "reversed");
    // then esc, j, End, v marks nothing: no cell of the `z` row is reversed
    s.keys("esc j End v");
    s.assert_style(0, 2, "!reversed");
    // oracle case 2: `abc` / blank; g j End
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_logical_2.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("g j End");
    s.assert_contains("abc");
}

// ORACLE transcript_view/select_mode_tests.rs:270 configured_pager_page_motion_moves_the_select_cursor kind=ui owner=chat-live
#[test]
fn configured_pager_page_motion_moves_the_select_cursor() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: the configured pager page-down key (x) moves no select cursor",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .keymap(&[("pager.page_down", "x")])
        .script(include_str!("fixtures/transcript_view_4/sm_pager.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("g v x");
    // the cursor lands on row 5 (`line 5`) of the visible rows
    s.assert_row(5, "line 5");
    s.assert_style(0, 5, "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:303 transcript_end_character_selection_includes_the_last_grapheme kind=ui owner=chat-live
#[test]
fn transcript_end_character_selection_includes_the_last_grapheme() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: G moves no caret to the transcript end, so the last grapheme is not marked",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_transcript_end.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("G v");
    s.assert_text_style("🙂", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:315 swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint kind=ui owner=chat-live
#[test]
fn swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: v, o and l do not mark or swap a range, so no highlight is drawn",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_swap.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("v o");
    s.assert_text_style("a", "reversed");
    s.keys("o l l l");
    s.assert_text_style("abcd", "reversed");
    s.keys("o l");
    s.assert_text_style("bcd", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:334 confirmed_copy_keeps_the_select_mode_caret_on_the_active_grapheme kind=ui owner=chat-live
#[test]
fn confirmed_copy_keeps_the_select_mode_caret_on_the_active_grapheme() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: y copies nothing, so the marked range is not cleared and the caret does not stay on `b`",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_confirmed.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("v l");
    // the copy of `ab` is confirmed (not drawn); the select mode stays and its footer is still drawn
    s.keys("y");
    s.assert_contains("v mark · enter/q close");
    s.keys("v");
    s.assert_text_style("b", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:360 find_accepts_at_the_match_and_cancel_restores_the_saved_selection kind=ui owner=chat-live
#[test]
fn find_accepts_at_the_match_and_cancel_restores_the_saved_selection() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no find inside select mode: / opens no search, so enter accepts no match and esc restores no saved selection",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_find_accept.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("/");
    s.type_text("gamma");
    s.keys("enter");
    // the accepted match `gamma` is on the first entry; the caret stays there when a newer query is cancelled
    s.keys("/");
    s.type_text("distant");
    s.keys("esc");
    s.keys("v right");
    s.assert_text_style("ga", "reversed");
    // a narrow render (8 columns), then esc, then ctrl+c cancels a find and keeps `ga` marked
    s.resize(8, 6);
    s.keys("/");
    s.resize(32, 6);
    s.keys("esc");
    s.keys("/ ctrl+c");
    s.assert_text_style("ga", "reversed");
    // a fresh query `alpha` accepted at its match: the marked range runs `alpha beta g` (the range setup through the oracle's API is not reachable by keys yet)
    s.keys("/");
    s.type_text("alpha");
    s.keys("enter");
    s.assert_text_style("alpha beta g", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:469 accepted_find_repeats_into_the_select_cursor kind=ui owner=chat-live
#[test]
fn accepted_find_repeats_into_the_select_cursor() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no find inside select mode: an accepted needle does not place the select cursor on the second match",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_accepted_repeat.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    // the cursor is on the second `needle` (column 13)
    s.assert_style(13, 0, "reversed");
    // / then enter repeats the accepted needle: the cursor stays at column 13
    s.keys("/ enter");
    s.assert_style(13, 0, "reversed");
    // n then right, then the next match wraps to column 14 after the search runs on
    s.keys("n right");
    s.assert_style(14, 0, "reversed");
    // n wraps back to the first match at column 0
    s.keys("n");
    s.assert_style(0, 0, "reversed");
    // v then shift+n extends the mark back to column 13
    s.keys("v shift+n");
    s.assert_style(13, 0, "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:536 accepted_find_pins_an_expanded_match_before_select_mode_exits kind=ui owner=chat-live
#[test]
fn accepted_find_pins_an_expanded_match_before_select_mode_exits() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no find inside select mode and no expandable command output: `hidden needle` is not in the transcript view",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_pin.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    // the accepted hidden match is brought into view with its cursor
    s.assert_contains("hidden needle");
    // / then esc, then q exits: the expanded match stays on screen
    s.keys("/ esc q");
    s.assert_contains("hidden needle");
}

// ORACLE transcript_view/select_mode_tests.rs:617 beginning_motion_finishes_at_the_first_grapheme_after_paging kind=ui owner=chat-live
#[test]
fn beginning_motion_finishes_at_the_first_grapheme_after_paging() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no partial history load: g starts no beginning load, and the older entry is not paged in under the select cursor",
    );
    // the oracle starts with `newer` only (partial history); g loads `older` and the caret finishes on its first grapheme
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_beginning.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("g");
    s.wait_for("older");
    s.keys("v");
    s.assert_text_style("o", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:637 shift_click_extends_from_the_keyboard_marks_fixed_anchor kind=ui owner=chat-live
#[test]
fn shift_click_extends_from_the_keyboard_marks_fixed_anchor() {
    kit::pending(
        Owner::ChatLive,
        "screen feature: the chat has no transcript select mode (the keyboard mark `cde` does not extend to the clicked column); the kit sends the shift+click (`shift_click`)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_shift_click.jsonl"
        ))
        .open();
    s.keys("leader [");
    s.keys("g right right v right right");
    s.assert_text_style("cde", "reversed");
    // a shift+click at column 0, row 0 extends from the fixed anchor
    s.shift_click(0, 0);
    s.assert_text_style("abc", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:664 existing_mouse_selection_keeps_its_word_or_line_unit kind=ui owner=chat-live
#[test]
fn existing_mouse_selection_keeps_its_word_or_line_unit() {
    kit::pending(
        Owner::ChatLive,
        "screen feature: the chat has no transcript select mode taking over a mouse selection (word unit from a double click, line unit from a triple click)",
    );
    // oracle case 1: a double click on `alpha` (column 1), select mode, h: the word unit keeps `alph` marked
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_mouse_word.jsonl"
        ))
        .open();
    s.double_click(1, 0);
    s.keys("leader [");
    s.keys("h");
    s.assert_text_style("alph", "reversed");
    // oracle case 2: a triple click on `second` (row 1), select mode, k: the line unit keeps `first\nsecond`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_mouse_line.jsonl"
        ))
        .open();
    s.triple_click(1, 1);
    s.keys("leader [");
    s.keys("k");
    s.assert_text_style("first", "reversed");
    s.assert_text_style("second", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:699 passive_mouse_events_do_not_reanchor_a_reverse_line_mark kind=ui owner=chat-live
#[test]
fn passive_mouse_events_do_not_reanchor_a_reverse_line_mark() {
    kit::pending(
        Owner::ChatLive,
        "screen feature: the chat has no transcript select mode with a reverse line mark (V marks no line); the kit sends the pointer move (`pointer_move`), the wheel and the press outside the transcript",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_passive.jsonl"))
        .open();
    s.keys("leader [");
    s.keys("g j V k");
    // the passive pointer move and wheel at (0,0) and a press outside the transcript at (80,80) do not move the mark; j then leaves `second` alone
    s.pointer_move(0, 0);
    s.wheel_down(0, 0, 1);
    s.click(80, 80);
    s.keys("j");
    s.assert_text_style("second", "reversed");
    s.assert_style(0, 0, "!reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:735 select_mode_renders_cursor_and_scannable_footer_hints kind=snap owner=chat-live snap=select_mode_renders_cursor_and_scannable_footer_hints
#[test]
fn select_mode_renders_cursor_and_scannable_footer_hints() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript select mode: no cursor is drawn on `alpha beta` and no select footer hint row is drawn",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_footer.jsonl"))
        .open();
    s.keys("leader [");
    // unmarked: the rows are the oracle's; the cursor is on `g` of `gamma` (bold, underlined, reversed); the wide footer is the one-line hint row
    s.assert_row(0, "alpha beta");
    s.assert_row(1, "gamma");
    s.assert_style(0, 1, "bold underlined reversed");
    s.assert_contains("arrows/hjkl move · v/space/V mark · / find · enter/q/esc close");
    // the narrow footer (32 columns)
    s.assert_contains("v mark · enter/q close");
    // marked (g, v, right x5): `alpha` is reversed and the cursor is on the sixth column of row 0
    s.keys("g v right right right right right");
    s.assert_style(0, 0, "reversed");
    s.assert_style(5, 0, "bold underlined reversed");
    s.assert_contains("y/⌃c copy · enter copy & latest · esc clear · q close");
    // find: esc, / then `beta`, advance: the match `beta` is reversed from column 6 and the find footer shows the query
    s.keys("esc /");
    s.type_text("beta");
    s.assert_contains("Find: beta");
    s.assert_contains("enter accept · ⌃p older · ⌃n newer · esc cancel");
    s.assert_style(6, 0, "reversed");
    // accepted and marked again (enter, v, right): the mark starts at column 6 and the cursor is on column 7
    s.keys("enter v right");
    s.assert_style(6, 0, "reversed");
    s.assert_style(7, 0, "bold underlined reversed");
}

// ---- oracle: transcript_view/selection_tests.rs (14 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/selection_tests.rs:22 selection_highlights_hard_breaks_within_and_between_entries kind=snap owner=chat-live snap=selection_highlights_hard_breaks_within_and_between_entries
#[test]
fn selection_highlights_hard_breaks_within_and_between_entries() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript mouse selection: a drag from column 0 to column 7 on the first row reverses no cells, and the hard break is not highlighted",
    );
    // oracle frame 1 (one entry `code` / `next`, 8x3): the selection `code` reverses columns 0 to 4, the rest is plain
    let mut s = Screen::build(Owner::ChatLive)
        .size(8, 3)
        .script(include_str!("fixtures/transcript_view_4/sl_hard_1.jsonl"))
        .open();
    s.mouse_down(0, 0);
    s.mouse_drag(7, 0);
    s.assert_row(0, "code");
    s.assert_row(1, "next");
    s.assert_style(0, 0, "reversed");
    s.assert_style(4, 0, "reversed");
    s.assert_style(5, 0, "!reversed");
    s.mouse_drag(0, 1);
    s.mouse_up(0, 1);
    s.assert_style(0, 1, "!reversed");
    // oracle frame 2 (two entries `code`, `next` with a separator row, 8x3): the same `code` reverse, the separator row blank, `next` plain
    let mut s = Screen::build(Owner::ChatLive)
        .size(8, 3)
        .script(include_str!("fixtures/transcript_view_4/sl_hard_2.jsonl"))
        .open();
    s.mouse_down(0, 0);
    s.mouse_drag(7, 0);
    s.assert_style(0, 0, "reversed");
    s.assert_row(1, "");
    s.assert_row(2, "next");
    s.mouse_drag(0, 2);
    s.mouse_up(0, 2);
    s.assert_style(0, 2, "!reversed");
}

// ORACLE transcript_view/selection_tests.rs:60 copy_shortcuts_clear_selection_only_after_confirmed_delivery kind=ui owner=chat-live
#[test]
fn copy_shortcuts_clear_selection_only_after_confirmed_delivery() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript selection with a triple click and no copy shortcut that clears it: the selection is not drawn reversed and no copy is delivered",
    );
    // the oracle loops over the copy keys (enter, ctrl+c, cmd+c, ctrl+shift+c, ctrl+C) x delivery (confirmed, unconfirmed) x a find open or not
    // KIT GAP: the clipboard delivery status (confirmed / unconfirmed / failed) cannot be driven by keys; the first case is written with the confirmed status
    for key in ["enter", "ctrl+c", "super+c", "ctrl+shift+c", "ctrl+C"] {
        for searching in [false, true] {
            let mut s = Screen::build(Owner::ChatLive)
                .size(32, 3)
                .script(include_str!("fixtures/transcript_view_4/sl_copy.jsonl"))
                .open();
            if searching {
                s.keys("/");
            }
            // a triple click selects the whole entry (KIT GAP: no triple click; a double click is written as its nearest step)
            s.double_click(0, 0);
            s.assert_text_style("selected", "reversed");
            s.keys(key);
            // a confirmed copy clears the selection (the copy text itself is not drawn)
            s.assert_style(0, 0, "!reversed");
        }
    }
}

// ORACLE transcript_view/selection_tests.rs:147 command_c_ignores_release_and_copies_only_an_active_selection kind=ui owner=chat-live
#[test]
fn command_c_ignores_release_and_copies_only_an_active_selection() {
    kit::pending(
        Owner::ChatLive,
        "screen feature: the chat has no transcript selection to copy (cmd+c, a triple click selecting `selected café`, the confirmed copy clearing it); the kit sends cmd+c, its release (`key_release`) and the triple click",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 3)
        .script(include_str!(
            "fixtures/transcript_view_4/sl_command_c.jsonl"
        ))
        .open();
    // no selection yet: cmd+c copies nothing and draws nothing reversed
    s.keys("super+c");
    s.assert_style(0, 0, "!reversed");
    // a triple click selects the line; the release of cmd+c leaves it
    s.triple_click(0, 0);
    s.assert_text_style("selected café", "reversed");
    s.key_release("super+c");
    s.assert_text_style("selected café", "reversed");
    // cmd+c copies the selection; the confirmed copy clears it
    s.keys("super+c");
    s.assert_style(0, 0, "!reversed");
    // a later cmd+c and ctrl+c with no selection draw nothing
    s.keys("super+c ctrl+c");
    s.assert_style(0, 0, "!reversed");
}

// ORACLE transcript_view/selection_tests.rs:187 enter_on_an_empty_selection_does_not_copy_or_reach_the_composer kind=ui owner=chat-live
#[test]
fn enter_on_an_empty_selection_does_not_copy_or_reach_the_composer() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no selection that a single click starts, and enter on an empty selection is not a copy action in the chat yet: nothing is drawn reversed and the composer is not checked",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 3)
        .script(include_str!(
            "fixtures/transcript_view_4/sl_enter_empty.jsonl"
        ))
        .open();
    // a single click starts an empty selection; enter on it neither copies nor leaves a reversed cell
    s.click(0, 0);
    s.keys("enter");
    s.assert_style(0, 0, "!reversed");
    s.assert_contains("select this");
}

// ORACLE transcript_view/selection_tests.rs:258 clicking_and_clearing_selection_preserves_the_separator_row kind=snap owner=chat-live
#[test]
fn clicking_and_clearing_selection_preserves_the_separator_row() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript reading position that a click on the separator keeps, and no double click selects a word in an agent entry",
    );
    // prepend=false: `first`, `alpha beta gamma` / `second line` / `third line`, `tail` (32x3), scrolled one row up
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 3)
        .script(include_str!(
            "fixtures/transcript_view_4/sl_clicking_1.jsonl"
        ))
        .open();
    s.wheel_up(0, 0, 1);
    let before = s.shot();
    s.assert_row(0, "• alpha beta gamma");
    s.assert_row(1, "  second line");
    s.assert_row(2, "");
    // a single click at (9,1) and its release leave the frame and the position as they were
    s.click(9, 1);
    s.assert_same_rows(&before);
    // a double click on `beta` selects it; esc clears it, and the frame is the one before
    s.double_click(9, 1);
    s.assert_text_style("beta", "reversed");
    s.keys("esc");
    s.assert_same_rows(&before);
    // repeated clicks on padding (31,1) and on the separator (9,0) select nothing
    s.double_click(31, 1);
    s.double_click(9, 0);
    s.assert_same_rows(&before);
    // prepend=true: `older` is loaded above; the frame is unchanged
    s.feed(include_str!(
        "fixtures/transcript_view_4/sl_clicking_older.jsonl"
    ));
    s.assert_same_rows(&before);
}

// ORACLE transcript_view/selection_tests.rs:350 live_selection_survives_commits_finalization_new_live_content_and_resize kind=snap owner=chat-live
#[test]
fn live_selection_survives_commits_finalization_new_live_content_and_resize() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no live tail that a mouse selection survives: a streaming tail cannot be selected, committed, finalized or resized under a selection",
    );
    // the streaming tail `tail selected` is selected by a triple click (KIT GAP: no triple click; a double click is written as its nearest step)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 4)
        .script(include_str!("fixtures/transcript_view_4/sl_live.jsonl"))
        .open();
    s.double_click(4, 1);
    s.assert_row(0, "• first");
    s.assert_row(1, "  tail selected");
    s.assert_text_style("tail selected", "reversed");
    // the tail is committed and finalized as `first` / `tail selected`; the selection still covers `tail selected` after the reflow to 10x6
    s.resize(10, 6);
    s.assert_text_style("tail selected", "reversed");
    // an older page is loaded above: the selection stays on `tail selected`
    s.feed(include_str!(
        "fixtures/transcript_view_4/sl_clicking_older.jsonl"
    ));
    s.resize(32, 5);
    s.assert_text_style("tail selected", "reversed");
    // esc clears the selection and leaves the following mode; a second esc follows the output again: the unrelated next tail shows
    s.keys("esc");
    s.assert_style(0, 1, "!reversed");
    s.keys("esc");
    s.assert_contains("unrelated next tail");
}

// ORACLE transcript_view/selection_tests.rs:405 tool_selection_copies_its_displayed_revision_after_the_tool_commits kind=ui owner=chat-live
#[test]
fn tool_selection_copies_its_displayed_revision_after_the_tool_commits() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no tool output row that a mouse selection can hold across a commit: the tool call is not a selectable row of the chat",
    );
    // the tool prints `selected tool output` (32x8, live); a triple click on its row (KIT GAP: a double click is written as its nearest step) selects it
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 8)
        .script(include_str!("fixtures/transcript_view_4/sl_tool.jsonl"))
        .open();
    s.double_click(4, 1);
    s.assert_text_style("selected tool output", "reversed");
    // another answer starts; the tool commits: the selection still shows the revision it was made on
    s.feed(include_str!(
        "fixtures/transcript_view_4/sl_clicking_older.jsonl"
    ));
    s.assert_text_style("selected", "reversed");
    // ctrl+c copies the displayed revision (the copy is not drawn)
    s.keys("ctrl+c");
}

// ORACLE transcript_view/selection_tests.rs:452 word_and_line_drags_keep_the_initial_unit_when_direction_changes kind=ui owner=chat-live
#[test]
fn word_and_line_drags_keep_the_initial_unit_when_direction_changes() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no word or line unit for a drag: a double-click drag keeps no word unit and a triple-click drag keeps no line unit (no triple click in the kit)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 5)
        .script(include_str!("fixtures/transcript_view_4/sl_drags.jsonl"))
        .open();
    // a word drag from column 9 (double click) to 16 selects `beta gamma`; back to 3 it selects `alpha beta`; back to 16 `beta gamma` again
    s.mouse_down(9, 0);
    s.mouse_drag(16, 0);
    s.assert_text_style("beta gamma", "reversed");
    s.mouse_drag(3, 0);
    s.assert_text_style("alpha beta", "reversed");
    s.mouse_drag(16, 0);
    s.mouse_up(16, 0);
    s.assert_text_style("beta gamma", "reversed");
    // a line drag from row 1 (KIT GAP: a triple click starts it) to row 0 selects the first two lines
    s.mouse_down(4, 1);
    s.mouse_drag(3, 0);
    s.assert_text_style("alpha beta gamma", "reversed");
    s.assert_text_style("second line", "reversed");
    s.mouse_drag(4, 2);
    s.mouse_up(4, 2);
    s.assert_text_style("second line", "reversed");
    s.assert_text_style("third", "reversed");
}

// ORACLE transcript_view/selection_tests.rs:478 keyboard_selection_moves_by_grapheme_and_keeps_its_column_across_short_lines kind=ui owner=chat-live
#[test]
fn keyboard_selection_moves_by_grapheme_and_keeps_its_column_across_short_lines() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no keyboard selection extension: right does not move the selection end by grapheme, and down/up keep no column",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 5)
        .script(include_str!("fixtures/transcript_view_4/sl_keyboard.jsonl"))
        .open();
    // a click at (2,0) starts the selection; four right presses extend it over the family emoji, the wide 界 and `e` with its combining mark
    s.click(2, 0);
    s.keys("right right right right");
    s.assert_text_style("A👨‍👩‍👧‍👦界e\u{301}", "reversed");
    // down, down, up, up leave the selection end where it was
    let original = s.shot();
    s.keys("down down up up");
    s.assert_same_rows(&original);
    // ten downs, then ten ups, end back on the original column
    s.keys("down down down down down down down down down down");
    s.keys("up up up up up up up up up up");
    s.assert_same_rows(&original);
}

// ORACLE transcript_view/selection_tests.rs:511 plain_and_shift_arrows_move_selection_on_press_and_repeat kind=ui owner=chat-live
#[test]
fn plain_and_shift_arrows_move_selection_on_press_and_repeat() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no keyboard movement of a mouse selection: left, right, up and down do not move the selection end on press or repeat, plain or shifted",
    );
    // the oracle: `abc` / `def` / `ghi` (32x5) with a click at (3,1); left marks `d`, right `e`, up `bc` and `d`, down `ef` and `g`
    for (key, expected) in [("left", "d"), ("right", "e"), ("up", "bc"), ("down", "ef")] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 5)
            .script(include_str!("fixtures/transcript_view_4/sl_arrows.jsonl"))
            .open();
        s.click(3, 1);
        s.keys(key);
        s.assert_text_style(expected, "reversed");
    }
    // the shift variant and the repeat variant are the same moves (KIT GAP: no key-repeat kind for an arrow with a modifier)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 5)
        .script(include_str!("fixtures/transcript_view_4/sl_arrows.jsonl"))
        .open();
    s.click(3, 1);
    s.keys("shift+left");
    s.assert_text_style("d", "reversed");
}

// ORACLE transcript_view/selection_tests.rs:535 modified_arrows_end_selection_and_return_input_to_the_owner kind=ui owner=chat-live
#[test]
fn modified_arrows_end_selection_and_return_input_to_the_owner() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no word selection that a modified arrow ends: ctrl, alt, super, hyper and meta arrows do not clear a double-click selection of `beta`",
    );
    // the oracle: `alpha beta gamma` (32x5) with `beta` selected by a double click at (9,0); each modified arrow (press and repeat, with and without shift) ends the selection and goes to the owner
    for key in [
        "ctrl+left",
        "ctrl+right",
        "ctrl+up",
        "ctrl+down",
        "alt+left",
        "alt+right",
        "alt+up",
        "alt+down",
        "cmd+left",
        "cmd+right",
        "cmd+up",
        "cmd+down",
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 5)
            .script(include_str!("fixtures/transcript_view_4/sl_modified.jsonl"))
            .open();
        s.double_click(9, 0);
        s.assert_text_style("beta", "reversed");
        s.keys(key);
        s.assert_text_style("beta", "!reversed");
    }
}

// ORACLE transcript_view/selection_tests.rs:569 released_arrows_preserve_selection_and_reading_position kind=ui owner=chat-live
#[test]
fn released_arrows_preserve_selection_and_reading_position() {
    kit::pending(
        Owner::ChatLive,
        "screen feature: the chat has no transcript selection that a released arrow preserves (a double click selecting `beta`); the kit sends the arrow release (`key_release`)",
    );
    for mods in ["", "shift+", "ctrl+", "alt+shift+"] {
        for key in ["left", "right", "up", "down"] {
            let mut s = Screen::build(Owner::ChatLive)
                .size(32, 5)
                .script(include_str!("fixtures/transcript_view_4/sl_modified.jsonl"))
                .open();
            s.double_click(9, 0);
            let before = s.shot();
            s.assert_text_style("beta", "reversed");
            s.key_release(&format!("{mods}{key}"));
            s.assert_same_rows(&before);
            s.assert_text_style("beta", "reversed");
        }
    }
}

// ORACLE transcript_view/selection_tests.rs:600 loading_older_history_extends_the_snapshot_without_adopting_new_commits kind=ui owner=chat-live
#[test]
fn loading_older_history_extends_the_snapshot_without_adopting_new_commits() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no history load under a selection: an older page is not prepended with the selection held, and the frame keeps `selected` reversed",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 5)
        .script(include_str!("fixtures/transcript_view_4/sl_loading.jsonl"))
        .open();
    s.double_click(4, 2);
    s.assert_text_style("selected", "reversed");
    // a new commit arrives and an older entry is prepended: `selected` stays the one selected, the new commit is not adopted into the snapshot
    s.feed(include_str!(
        "fixtures/transcript_view_4/sl_loading_older.jsonl"
    ));
    s.resize(32, 6);
    s.assert_text_style("selected", "reversed");
}

// ORACLE transcript_view/selection_tests.rs:630 pending_copy_only_finishes_the_original_selection kind=ui owner=chat-live
#[test]
fn pending_copy_only_finishes_the_original_selection() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no pending copy that a selection finishes: the copy's delivery status (pending, then confirmed) cannot be driven by keys, and there is no triple click, hide or detail-toggle key in the kit",
    );
    // the oracle: `selected text` (32x3), a triple click selects it, a pending copy is followed, then one change happens and the copy finishes confirmed
    for change in [
        "none",
        "replace",
        "move",
        "navigate",
        "hide",
        "presentation",
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 3)
            .script(include_str!("fixtures/transcript_view_4/sl_pending.jsonl"))
            .open();
        // KIT GAP: a triple click selects the whole entry; a double click is written as its nearest step
        s.double_click(0, 0);
        s.assert_text_style("selected text", "reversed");
        match change {
            "replace" => s.double_click(0, 0),
            "move" => s.keys("left"),
            "navigate" => s.keys("ctrl+end"),
            "presentation" => s.keys("ctrl+t"),
            // KIT GAP: `hide` (the pane hidden under the copy) has no key or step in the kit
            _ => &mut s,
        };
        if change == "none" {
            // the copy finished on the original selection: it is cleared
            s.assert_style(0, 0, "!reversed");
        } else {
            // the selection is kept on the changed view
            s.assert_contains("selected text");
        }
    }
}
