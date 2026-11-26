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
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/find_live.jsonl"))
        .view_only()
        .open();
    s.live_tail("live needle");
    s.keys("/");
    s.type_text("needle");
    s.assert_text_style("needle", "reversed");
    s.keys("pageup");
    let scrolled = s.shot();
    s.keys("enter");
    s.assert_same_rows(&scrolled);
    s.assert_find_query("needle");
    s.assert_find_footer(80, "Find · ctrl+p older · ctrl+n newer · esc latest");
    // the reader selects a word of the older answer; Find opens again and takes the selection
    // back to the live match it still holds
    s.double_click_text("earlier");
    s.assert_text_style("earlier", "reversed");
    s.keys("/");
    s.assert_find_query("needle");
    s.assert_text_style("earlier", "!reversed");
    // the match it still holds is the live answer's, below the rows the reader is at
    s.keys("pagedown");
    s.assert_text_style("needle", "reversed");
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
    // with no match read the two keys go on from the entry the reader sees: the older and the
    // newer needle
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
        // the reader leaves the match for the entry between the two (the oracle's
        // `jump_to_entry(1)`, which has none), and the query is asked again
        s.keys("pageup");
        s.keys("/");
        s.keys("backspace");
        s.type_text("e");
        s.keys("enter");
        s.keys(key);
        s.assert_contains(expected);
        s.assert_text_style("needle", "reversed");
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 6)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["needle", "latest context"]);
    s.keys("/");
    s.type_text("needle");
    s.keys("pageup");
    // the oracle asks for older history (needs_history); here the app has asked the server
    s.assert_page_requests(2);
    s.keys("enter");
    s.keys("pageup");
    // the request is still the one in flight, and the query stays
    s.assert_page_requests(2);
    s.assert_find_query("needle");
}

// ORACLE transcript_view/search_tests.rs:410 paging_below_a_live_find_result_resumes_current_output kind=snap owner=chat-live
#[test]
fn paging_below_a_live_find_result_resumes_current_output() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/find_live.jsonl"))
        .view_only()
        .open();
    s.live_tail("live needle");
    s.keys("/");
    s.type_text("needle");
    s.keys("pagedown");
    // the answer the model is streaming is another text now, with no match in it: the reader is
    // at the last row and follows it, the query stays
    s.replace_live_tail("updated live output");
    s.assert_contains("Worked for");
    s.assert_row(1, "");
    s.assert_row(2, "• updated live output");
    s.assert_find_query("needle");
}

// ORACLE transcript_view/search_tests.rs:579 empty_find_keeps_following_new_output_after_a_page_join kind=snap owner=chat-live snap=empty_find_follows_output_after_page_join
#[test]
fn empty_find_keeps_following_new_output_after_a_page_join() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 16)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["previous computer action", "latest answer"]);
    s.keys("/");
    s.live_tail("stream before page");
    s.join_older_page(&["older computer action"], false);
    // a new answer commits and the stream goes on, with Find still open and empty
    s.replace_live_tail("new committed answer");
    s.commit_live_tail();
    s.live_tail("stream after page");
    // the oracle's frame ends on the stream, the page joined above out of view: the reader
    // follows the end (its cells have no `Worked for` rows between, nor the older entries it
    // regroups into one)
    s.assert_absent("older computer action");
    s.assert_contains("new committed answer");
    s.assert_contains("stream after page");
    s.assert_row(15, "• stream after page");
    s.assert_find_query("");
}

// ORACLE transcript_view/search_tests.rs:625 empty_find_preserves_reading_inside_a_joined_group kind=ui owner=chat-live
#[test]
fn empty_find_preserves_reading_inside_a_joined_group() {
    // the oracle's two entries are the first page of a partial history; the older page and the
    // group over the displayed entry join while Find is open and empty
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 4)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["displayed computer action", "latest answer"]);
    // the reader is at the first entry: the top, where the app asks for the older page
    s.wheel_up(0, 0, 100);
    s.keys("/");
    let before = s.shot();
    s.assert_contains("displayed computer action");
    s.join_older_page(&["older computer action"], false);
    s.type_text("x");
    s.keys("backspace");
    s.assert_same_rows(&before);
    s.keys("esc");
    s.assert_same_rows(&before);
}

// ORACLE transcript_view/search_tests.rs:663 previous_match_continues_through_regrouped_pages_without_revisiting_newer_hits kind=snap owner=chat-live
#[test]
fn previous_match_continues_through_regrouped_pages_without_revisiting_newer_hits() {
    // a page holds whole turns and a group of tool calls is of one turn, so no entry of the
    // first page regroups with one of the older: the oracle's `replace_group` of `older group`
    // with its neighbour is the page's join here, its hits and its status are the same
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["newer group", "needle middle", "needle newest"]);
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.assert_contains("needle middle");
    s.keys("ctrl+p");
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    // the older page has no hit, the reader stays on the middle one and the search goes on
    s.join_older_page(&["earlier context", "older group"], true);
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    s.assert_contains("needle middle");
    // the oldest page has the oldest hit, and the way back down meets each hit once
    s.join_older_page(&["needle oldest"], false);
    s.assert_contains("needle oldest");
    s.keys("ctrl+n");
    s.assert_contains("needle middle");
    s.keys("ctrl+n");
    s.assert_contains("needle newest");
}

// ORACLE transcript_view/search_tests.rs:734 previous_match_finds_the_prepended_part_of_a_regrouped_entry kind=ui owner=chat-live
#[test]
fn previous_match_finds_the_prepended_part_of_a_regrouped_entry() {
    // the older entry joins above as its own entry (a group of tool calls is of one turn, so
    // nothing prepends to an entry of the first page): the match in it is found and drawn
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 4)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["newer group", "needle newest"]);
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.join_older_page(&["needle older prefix"], false);
    s.assert_contains("needle older prefix");
    s.assert_text_style("needle", "reversed");
    s.keys("ctrl+n");
    s.assert_contains("needle newest");
}

// ORACLE transcript_view/search_tests.rs:787 enter_preserves_the_initial_scan_and_each_older_page_until_the_first_match kind=snap owner=chat-live
#[test]
fn enter_preserves_the_initial_scan_and_each_older_page_until_the_first_match() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&[&"recent text ".repeat(1500)]);
    s.keys("/");
    s.type_text("LONG T0000 USER");
    s.keys("enter");
    // the oracle's first row, `Searching…`, is the scan of the entry over several frames; the
    // port reads the whole transcript in one, so the first row it says is the one of the history
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    // each older page is confirmed by Enter and joins; the first match is in the oldest page
    s.keys("enter");
    s.join_older_page(&["middle page without a hit"], true);
    s.keys("enter");
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    s.join_older_page(&["LONG T0000 USER"], false);
    s.keys("enter");
    s.assert_contains("LONG T0000 USER");
    s.assert_text_style("LONG T0000 USER", "reversed");
}

// ORACLE transcript_view/search_tests.rs:922 search_finds_the_first_older_entry_after_replaying_the_session_header kind=ui owner=chat-live
#[test]
fn search_finds_the_first_older_entry_after_replaying_the_session_header() {
    // the oracle parks the view and replays the thread (the session header, then the recent
    // entry); the port's thread keeps its history across the replay: the header is the first
    // entry the older page joins under
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["recent needle"]);
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    s.keys("ctrl+p");
    s.join_older_page(&["oldest needle", "older messages"], false);
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/transcript_view_4/find_exec.jsonl"))
        .view_compact()
        .open();
    // F4 focuses the last cell with a control, and Enter opens it: a disclosure of the reader's
    s.keys("f4");
    s.keys("enter");
    s.assert_contains("hidden manual");
    s.keys("/");
    s.type_text("needle");
    // only the matching cell expands (the oracle's per-entry expansion): the newest match first,
    // ctrl+p and ctrl+n swap which one
    s.assert_contains("hidden newer needle");
    s.assert_absent("hidden older needle");
    s.assert_absent("hidden unrelated");
    s.keys("ctrl+p");
    s.assert_contains("hidden older needle");
    s.assert_absent("hidden newer needle");
    s.keys("ctrl+n");
    s.assert_contains("hidden newer needle");
    s.assert_absent("hidden older needle");
    s.assert_contains("hidden manual");
    // the oracle's frame (`find_selective_expansion`) is of cells it builds itself: a command that
    // is one quoted argument, `Running` with a finished output, no turn between them. The
    // protocol gives none of that (a command is a script, a finished one `Ran`, each turn a
    // `Worked for` row), so the frame is not compared; what it shows, the match's cell and the
    // reader's open and the others shut, is what the assertions above read.
    s.keys("enter");
    // Reading keeps the result and lets the normal activity controls open another block
    s.keys("f4");
    s.keys("home");
    s.keys("enter");
    s.keys("esc");
    s.assert_contains("hidden unrelated");
    s.keys("f4");
    s.keys("end");
    s.keys("up");
    s.assert_contains("enter collapse");
    for opens in [true, false, true, false] {
        s.keys(if opens { "right" } else { "left" });
        if opens {
            s.assert_contains("hidden newer needle");
        } else {
            s.assert_absent("hidden newer needle");
        }
    }
    s.keys("esc");
    // Find leaves: the match's own expansion goes, the reader's two stay
    s.keys("esc");
    s.assert_contains("hidden unrelated");
    s.assert_contains("hidden manual");
    s.assert_absent("hidden older needle");
    s.assert_absent("hidden newer needle");
}

// ORACLE transcript_view/search_tests.rs:1202 find_searches_hidden_live_content_and_collapses_it_when_leaving_the_match kind=ui owner=chat-live
#[test]
fn find_searches_hidden_live_content_and_collapses_it_when_leaving_the_match() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 14)
        .script(include_str!("fixtures/transcript_view_4/hidden_live.jsonl"))
        .view_compact()
        .open();
    // the compact cell shows the last three lines of the running command's output
    s.assert_contains("compact live");
    s.assert_absent("hidden live needle");
    s.keys("/");
    s.type_text("needle");
    // the match is in the first line, which the compact cell cuts: the cell is open for it
    s.assert_contains("hidden live needle");
    s.keys("enter");
    // at 20 columns the line wraps under its branch
    s.resize(20, 14);
    s.assert_contains("hidden live");
    // the command's output is replaced while the match is held: the match is shown as it was found
    s.replace_exec_output("updated live needle\nl2\nl3\nupdated compact live");
    s.assert_contains("hidden live");
    s.assert_absent("updated");
    s.keys("ctrl+p");
    // the older match is in the answer: the cell that held the match is cut to its tail again
    s.assert_contains("older needle");
    s.assert_absent("hidden live");
    // the tail is cut by the width (`updated c…` at 20 columns)
    s.assert_contains("updated c");
    s.keys("ctrl+n");
    // the match is in the output as it is now
    s.assert_contains("updated live");
}

// ORACLE transcript_view/search_tests.rs:1268 selected_hidden_live_match_keeps_its_revision_without_persisting_expansion kind=ui owner=chat-live
#[test]
fn selected_hidden_live_match_keeps_its_revision_without_persisting_expansion() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 20)
        .script(include_str!("fixtures/transcript_view_4/hidden_live.jsonl"))
        .view_compact()
        .open();
    s.keys("/");
    s.type_text("needle");
    // the oracle selects by three clicks at column 2 of the hidden match row; the kit's click_text clicks the text's first cell
    s.click_text("hidden live needle");
    s.click_text("hidden live needle");
    s.click_text("hidden live needle");
    // the live output is replaced twice and the room narrows under the selection: the match is
    // still the revision that was found
    s.replace_exec_output("updated live needle\nl2\nl3\nupdated compact live");
    s.replace_exec_output("updated live needle\nl2\nl3\nupdated compact live");
    s.resize(14, 20);
    s.assert_contains("hidden");
    s.assert_absent("updated");
    s.keys("ctrl+p");
    s.assert_contains("updated c");
    s.assert_absent("hidden");
    s.keys("ctrl+n");
    // the match is in the output as it is now: its cell is open again (one word a row at 14 columns)
    s.assert_contains("Hide");
    s.assert_absent("hidden");
}

// ORACLE transcript_view/search_tests.rs:1343 find_reveals_hidden_command_output_and_restores_compact_presentation kind=ui owner=chat-live
#[test]
fn find_reveals_hidden_command_output_and_restores_compact_presentation() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: Find reads the command's whole output and opens its cell for the match (the same test at 80x50, and `find_tool_text::a_matched_command_shows_its_hidden_output_and_the_compact_rows_come_back`, pass), but the match is the 22nd of 41 lines of the open cell and the window is 4 rows: a reverse list shows an item taller than its window by its end (`ensure_index_visible`), so the match line is scrolled out; the oracle scrolls to the line",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!(
            "fixtures/transcript_view_4/find_command.jsonl"
        ))
        .view_compact()
        .open();
    // the compact cell cuts the output to its last three lines
    s.assert_absent("hidden needle");
    s.keys("/");
    s.type_text("hidden needle");
    let found = s.shot();
    s.assert_contains("hidden needle");
    s.keys("ctrl+t");
    s.assert_same_rows(&found);
    s.keys("enter");
    s.assert_same_rows(&found);
    s.keys("esc");
    s.assert_absent("hidden needle");
    let compact = s.shot();
    s.keys("/");
    s.keys("esc");
    // the oracle also replaces the group under the match before it cancels; the port's rows are
    // items by key, so the compact rows come back by themselves
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/live_commit.jsonl"))
        .view_only()
        .open();
    s.keys("/");
    s.live_tail("live needle");
    s.type_text("needle");
    s.assert_row(2, "• live needle");
    // the answer grows under the match: the match is shown as it was found
    s.replace_live_tail("live needle again needle");
    s.assert_row(2, "• live needle");
    // the next match is in the answer as it is now
    s.keys("ctrl+n");
    s.assert_row(2, "• live needle again needle");
    // the answer commits with more text and a new answer streams: the match is still shown as it
    // was found
    s.replace_live_tail("live needle again needle committed needle");
    s.assert_row(2, "• live needle again needle");
    s.commit_live_tail();
    s.live_tail("current needle");
    s.assert_contains("• live needle again needle");
    s.assert_absent("committed needle");
    s.keys("ctrl+n");
    s.assert_contains("committed needle");
    s.keys("ctrl+n");
    s.assert_contains("current needle");
    s.keys("ctrl+p");
    s.assert_contains("committed needle");
    // a fresh query reads the current content
    s.keys("ctrl+u");
    s.type_text("current");
    s.assert_text_style("current", "reversed");
    s.keys("ctrl+n");
    s.assert_text_style("current", "reversed");
    // no match is newer: the one found stays (the oracle's exhausted search retains its match)
    s.assert_find_footer(
        80,
        "No more matches · enter accept · ⌃p older · ⌃n newer · esc cancel",
    );
}

// ORACLE transcript_view/search_tests.rs:1664 leaving_a_regrouped_search_match_restarts_from_current_history kind=ui owner=chat-live
#[test]
fn leaving_a_regrouped_search_match_restarts_from_current_history() {
    // no entry regroups when a page joins (see the two tests before), so the match is not
    // retired: it stays the one the join found it in, and a step from it searches the history
    // as it is now
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 4)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["retired needle", &"trailing words ".repeat(20)]);
    s.keys("/");
    s.type_text("needle");
    s.join_older_page(&["surviving before", "older group"], false);
    s.keys("ctrl+n");
    s.keys("ctrl+n");
    s.assert_contains("retired needle");
    s.assert_text_style("needle", "reversed");
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
    // oracle case 1 (`top`, `bottom` at 32x6): `v` marks `b`, the first cell of the last line
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_1.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("b", "reversed");
    // oracle case 2 (`text`, empty line): `t`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_2.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("t", "reversed");
    // oracle case 5 (`a` then nine blank rows): select mode starts (its footer is drawn)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_5.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.assert_contains("v mark · enter/q close");
    // oracle case 6 (one empty line): begin_select_mode is refused, no select footer is drawn
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_6.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.assert_absent("v mark · enter/q close");
    // oracle case 3 (`abc` then six blank rows, all six rows of the view blank): the caret falls
    // back to the last grapheme of the text, `c`; and case 4 (`a`..`j`, scrolled two rows up): `h`,
    // the last selectable line in view. The mode places its caret on the last line of the whole
    // transcript, wherever the reader is (the list does not tell which rows are in view)
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal (visible start): select mode starts on the last selectable line of the transcript and scrolls to it, not on the last selectable line in view (`h` with the reader two rows up) or the last grapheme of the text where no line in view has text (`c`): rtwk's list does not say which items are in view",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 2)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_3.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("c", "reversed");
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_entry_4.jsonl"))
        .view_only()
        .open();
    s.wheel_up(0, 0, 2);
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("h", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:72 character_motion_crosses_line_and_entry_boundaries_without_invisible_stops kind=ui owner=chat-live
#[test]
fn character_motion_crosses_line_and_entry_boundaries_without_invisible_stops() {
    // oracle case 1: `a` / blank / blank / `b`; left, v marks `a`; esc, right, v marks `b`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_1.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("left v");
    s.assert_text_style("a", "reversed");
    s.keys("esc right v");
    s.assert_text_style("b", "reversed");
    // oracle case 2: `ab` with the caret forced onto its separator row; right, v marks `a`. The
    // port has no caret stop on a separator (the blank rows between entries are no line of the
    // text), so the caret that `right` leaves the oracle's on is the first grapheme, where select
    // mode begins on a one-line transcript starting at its first column: v marks `a`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_2.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("v");
    s.assert_text_style("a", "reversed");
    s.assert_text_style("b", "!reversed");
    // oracle case 4: `ab` / `cd`, a range from the start of `ab` to the start of `cd` that the
    // oracle sets through its API (adopting a selection into the mode is f-selection-keys'); the
    // keys that make the same marks: the range `ab`, o swaps its ends, l shrinks it to `b`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_4.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("g v l");
    s.assert_text_style("ab", "reversed");
    s.keys("o l");
    s.assert_text_style("b", "reversed");
    s.assert_text_style("a", "!reversed");
    // oracle case 5: `abc`, the range `bc` (set through the oracle's API); h moves the caret to `b`
    // (the range shrinks to it), a second h to `a` (the range grows to `ab`)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_5.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("g l v l");
    s.assert_text_style("bc", "reversed");
    s.keys("h");
    s.assert_text_style("b", "reversed");
    s.assert_text_style("c", "!reversed");
    s.keys("h");
    s.assert_text_style("ab", "reversed");
    // oracle case 3: a wrapped row of `1234567890` x3 then ` one`; left crosses the invisible space
    // onto the `0` at column 29 of row 0
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-display-rows: the caret moves over the lines of the text, not the display rows of a wrapped line: left from `one` stops on the space the wrap hides instead of crossing it onto the `0` of the first row",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_motion_3.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("left v");
    // the thirty digits fill the first row of the port's width after its gutter
    s.assert_style_in("123456789012345678901234567890", 29, "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:166 vertical_motion_keeps_the_preferred_display_column kind=ui owner=chat-live
#[test]
fn vertical_motion_keeps_the_preferred_display_column() {
    // oracle: `abcdef` / `x` / `abcdef`; right x3, up x2, v marks the `d` of the first line (column
    // 3 of the oracle's row 0): the port's answer starts after its two-column gutter, so the cell is
    // read in the row of the text
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_vertical.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("right right right up up v");
    s.assert_style_in("abcdef", 3, "reversed");
    s.assert_text_style("x", "!reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:181 enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows kind=ui owner=chat-live
#[test]
fn enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_enter.jsonl"))
        .view_only()
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_word.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    // the oracle's `j` from `alpha` stops on the blank row that separates the two entries, and `w`
    // goes from there to `beta`; the port's blank rows between entries are no line of the text a
    // caret stops on, so `w` crosses the entries from `alpha` itself
    s.keys("g w v e");
    s.assert_text_style("beta", "reversed");
    s.assert_text_style("gamma", "!reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:223 reverse_line_selection_copies_complete_lines_in_document_order kind=ui owner=chat-live
#[test]
fn reverse_line_selection_copies_complete_lines_in_document_order() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_reverse.jsonl"))
        .view_only()
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
    // oracle case 1: `abc` / blank / `z`; g then $ then v marks `c`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_logical_1.jsonl"
        ))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("g $ v");
    s.assert_text_style("c", "reversed");
    // then esc, j, End, v marks nothing: no cell of the `z` row is reversed (the oracle reads row 2,
    // the port's `z` row is found by its text). The empty line between is a line of a code block
    // here: the blank row between two paragraphs is no line of the text a caret stops on
    s.keys("esc j end v");
    s.assert_text_style("z", "!reversed");
    // oracle case 2: `abc` / blank; g j End
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_logical_2.jsonl"
        ))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("g j end");
    s.assert_contains("abc");
}

// ORACLE transcript_view/select_mode_tests.rs:270 configured_pager_page_motion_moves_the_select_cursor kind=ui owner=chat-live
#[test]
fn configured_pager_page_motion_moves_the_select_cursor() {
    // the oracle's view is 6 rows with no footer; the port's select hint row takes one, so the
    // screen has 7 (a page is the list's six rows less one)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 7)
        .keymap(&[("pager.page_down", "x")])
        .script(include_str!("fixtures/transcript_view_4/sm_pager.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("g v x");
    // the cursor lands on `line 5`, a page (the list's six rows, less one) of lines below `line 0`;
    // the twelve lines are twelve answers here: the oracle's one cell of twelve lines is an item
    // taller than the list, which the list cannot scroll to a row of (G-new-sel-reveal)
    // (the mark made at `line 0` runs to it: its first cell is the last one reversed)
    s.assert_style_in("line 5", 0, "reversed");
    s.assert_style_in("line 5", 1, "!reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:303 transcript_end_character_selection_includes_the_last_grapheme kind=ui owner=chat-live
#[test]
fn transcript_end_character_selection_includes_the_last_grapheme() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_transcript_end.jsonl"
        ))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("G v");
    s.assert_text_style("🙂", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:315 swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint kind=ui owner=chat-live
#[test]
fn swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_swap.jsonl"))
        .view_only()
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_confirmed.jsonl"
        ))
        .view_only()
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_find_accept.jsonl"
        ))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("/");
    s.type_text("gamma");
    s.keys("enter");
    // the accepted match `gamma` is on the first entry; the caret stays there when a newer query is cancelled
    s.keys("/");
    s.type_text("distant");
    s.keys("esc");
    // the view stays where the accepted match put it, and the caret on `g`: v, right marks `ga`
    s.assert_contains("alpha beta gamma");
    s.keys("v right");
    s.assert_text_style("ga", "reversed");
    // a narrow render (8 columns), then esc, then ctrl+c cancels a find and keeps `ga` marked
    s.resize(8, 6);
    s.keys("/");
    s.resize(32, 6);
    s.keys("esc");
    s.keys("/ ctrl+c");
    s.assert_text_style("ga", "reversed");
    // a fresh query `alpha` accepted at its match: the marked range runs `alpha beta g` (the oracle then sets the range `beta` through its API and begins select mode again: adopting a selection is f-selection-keys')
    s.keys("/");
    s.type_text("alpha");
    s.keys("enter");
    s.assert_text_style("alpha beta g", "reversed");
    // the oracle's last cases: a query of the combining mark alone is accepted at the grapheme it
    // belongs to (`á`), one that starts at the line break at the `b` after it
    for (query, marked) in [("\u{301}", "a\u{301}"), ("\nbeta", "b")] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 6)
            .script(include_str!(
                "fixtures/transcript_view_4/sm_find_combining.jsonl"
            ))
            .view_only()
            .open();
        s.keys("leader [");
        s.keys("/");
        s.type_text(query);
        s.keys("enter");
        s.keys("v");
        s.assert_text_style(marked, "reversed");
    }
}

// ORACLE transcript_view/select_mode_tests.rs:469 accepted_find_repeats_into_the_select_cursor kind=ui owner=chat-live
#[test]
fn accepted_find_repeats_into_the_select_cursor() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_accepted_repeat.jsonl"
        ))
        .view_only()
        .open();
    // the oracle's columns are read in the row of the text (the port's answer starts at column 2):
    // the first `needle` is offset 0 of it, the second offset 13
    let line = "needle first needle second";
    s.keys("leader [");
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    // the cursor is on the second `needle`
    s.assert_style_in(line, 13, "reversed");
    // / then enter repeats the accepted needle: the cursor stays there
    s.keys("/ enter");
    s.assert_style_in(line, 13, "reversed");
    // the oracle's `n right` ends one cell on, because the key after `n` cancels the scan `n` began
    // (the oracle scans on later); the port's `n` goes to its match at once, so only the step on
    // is kept: right, then n wraps back to the match before it, the first needle
    s.keys("right");
    s.assert_style_in(line, 14, "reversed");
    s.keys("n");
    s.assert_style_in(line, 0, "reversed");
    s.assert_style_in(line, 13, "!reversed");
    // v then shift+n extends the mark forward to the second needle
    s.keys("v shift+n");
    s.assert_style_in(line, 0, "reversed");
    s.assert_style_in(line, 13, "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:536 accepted_find_pins_an_expanded_match_before_select_mode_exits kind=ui owner=chat-live
#[test]
fn accepted_find_pins_an_expanded_match_before_select_mode_exits() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: the match is inside a 41-line command output in the six-row reverse list; Find expands and pins the cell, but ListController::ensure_index_visible only reveals the bottom of a tall reverse item and jump_to_index_rows ignores its intra-item row offset",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_pin.jsonl"))
        .view_compact()
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
    // the oracle starts with `newer` only (partial history); g asks for the older pages, and once
    // the last is in the caret is on the first grapheme of the first row
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["newer"]);
    s.keys("leader [");
    s.keys("g");
    s.assert_page_requests(2);
    s.history_page(kit::turns_of("older", 0..1), None);
    s.assert_contains("older 0");
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
    // oracle case 1: a double click on `alpha` (column 1), select mode, h: the word unit keeps `alph` marked
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_mouse_word.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.double_click_text("alpha");
    s.assert_text_style("alpha", "reversed");
    s.keys("leader [");
    s.keys("h");
    s.assert_text_style("alph", "reversed");
    s.assert_style(6, s.row_of("alpha"), "!reversed");
    // oracle case 2: a triple click on `second` (row 1), select mode, k: the line unit keeps `first\nsecond`
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sm_mouse_line.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    let row = u16::try_from(s.row_of("second")).unwrap();
    s.triple_click(3, row);
    s.assert_text_style("second", "reversed");
    s.keys("leader [");
    s.keys("k");
    s.assert_text_style("first", "reversed");
    s.assert_text_style("second", "reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:699 passive_mouse_events_do_not_reanchor_a_reverse_line_mark kind=ui owner=chat-live
#[test]
fn passive_mouse_events_do_not_reanchor_a_reverse_line_mark() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 6)
        .script(include_str!("fixtures/transcript_view_4/sm_passive.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.keys("g j V k");
    // the passive pointer move and wheel over the transcript and a press outside it do not move the
    // mark; j then leaves `second` alone. The oracle's press outside the view is at (80, 80); the
    // port's screen is the view itself, so the press is on the footer's hint row (not the list)
    let footer = s.row_of("enter copy");
    s.pointer_move(0, 0);
    s.wheel_down(0, 0, 1);
    s.click(0, u16::try_from(footer).unwrap());
    s.keys("j");
    s.assert_text_style("second", "reversed");
    s.assert_text_style("first", "!reversed");
}

// ORACLE transcript_view/select_mode_tests.rs:735 select_mode_renders_cursor_and_scannable_footer_hints kind=snap owner=chat-live snap=select_mode_renders_cursor_and_scannable_footer_hints
#[test]
fn select_mode_renders_cursor_and_scannable_footer_hints() {
    // the oracle's footer is its own one-row buffer under a 32x6 view; here the hint row is a row
    // of the screen (7 high), the answer's cells are read where its text is (a two-column gutter,
    // the transcript stands at the bottom), and the wide footer is the 64-column one of the snapshot
    // the narrow footer (32 columns)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 7)
        .script(include_str!("fixtures/transcript_view_4/sm_footer.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    s.assert_contains("v mark · enter/q close");
    let mut s = Screen::build(Owner::ChatLive)
        .size(64, 7)
        .script(include_str!("fixtures/transcript_view_4/sm_footer.jsonl"))
        .view_only()
        .open();
    s.keys("leader [");
    // unmarked: the cursor is on `g` of `gamma` (bold, underlined, reversed), the rest is plain
    s.assert_style_in("gamma", 0, "bold underlined reversed");
    s.assert_style_in("gamma", 1, "!reversed");
    s.assert_style_in("alpha beta", 0, "!reversed");
    s.assert_contains("arrows/hjkl move · v/space/V mark · / find · enter/q/esc close");
    // marked (g, v, right x5): the range `alpha ` is reversed, up to the cursor's cell
    s.keys("g v right right right right right");
    s.assert_text_style("alpha", "reversed");
    s.assert_style_in("alpha beta", 6, "!reversed");
    s.assert_contains("y/⌃c copy · enter copy & latest · esc clear · q close");
    kit::pending(
        Owner::ChatLive,
        "needs f-sel-caret-span (port feature: the cell draws the caret grapheme as its own bold-underlined span under the selection area's reversed style): today the cursor's cell inside a marked range (bold underlined reversed on the active end, the rest reversed) and the cursor shown beside the match Find reads (the snapshot's `find` frame keeps it bold underlined at column 5 while `beta` is reversed) are not drawn as the oracle draws them",
    );
    s.assert_style_in("alpha beta", 5, "bold underlined reversed");
    // find: esc, / then `beta`, advance: the match `beta` is reversed from column 6, the cursor
    // stays where it was, and the find footer shows the query
    s.keys("esc /");
    s.type_text("beta");
    s.assert_contains("Find: beta");
    s.assert_contains("enter accept · ⌃p older · ⌃n newer · esc cancel");
    s.assert_style_in("alpha beta", 5, "bold underlined reversed");
    s.assert_style_in("alpha beta", 6, "reversed");
    // accepted and marked again (enter, v, right): the mark starts at column 6 and the cursor is on column 7
    s.keys("enter v right");
    s.assert_style_in("alpha beta", 6, "reversed");
    s.assert_style_in("alpha beta", 7, "bold underlined reversed");
}

// ---- oracle: transcript_view/selection_tests.rs (14 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/selection_tests.rs:22 selection_highlights_hard_breaks_within_and_between_entries kind=snap owner=chat-live snap=selection_highlights_hard_breaks_within_and_between_entries
#[test]
fn selection_highlights_hard_breaks_within_and_between_entries() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-lines: a hard break inside one rendered entry is selected. At the boundary between entries, the mounted selection drops the break or includes the next entry's first cells.",
    );
    for (script, different_entry) in [
        (
            include_str!("fixtures/transcript_view_4/sl_hard_1.jsonl"),
            false,
        ),
        (
            r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"agent","thread":"thread-1","turn":"turn-1","item":"first","text":"code","start":true,"end":false}
{"step":"entry","kind":"agent","thread":"thread-1","turn":"turn-1","item":"second","text":"next","start":false,"end":true}"#,
            true,
        ),
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 12)
            .script(script)
            .view_only()
            .open();
        s.selection_copy_policy(false, false);
        let from = s.row_of("code");
        let to = s.row_of("next");
        s.mouse_down(0, from as u16);
        s.mouse_drag(9, from as u16);
        s.assert_text_style("code", "reversed");
        s.assert_style_in("code", 4, "!reversed");
        let next_text_start = if different_entry { 2 } else { 0 };
        s.mouse_drag(next_text_start, to as u16);
        s.mouse_up(next_text_start, to as u16);
        if different_entry {
            s.assert_row(from + 1, "");
        }
        s.assert_text_style("next", "!reversed");
        s.keys("ctrl+c");
        s.assert_copy_asked("code\n");
    }
}

// ORACLE transcript_view/selection_tests.rs:60 copy_shortcuts_clear_selection_only_after_confirmed_delivery kind=ui owner=chat-live
#[test]
fn copy_shortcuts_clear_selection_only_after_confirmed_delivery() {
    kit::pending(
        Owner::ChatLive,
        "f-entries-tabs-styles: a tab in a rich transcript row splits its selection into text spans and is unavailable as a tab in the painted clipboard text",
    );
    for key in ["enter", "ctrl+c", "super+c", "ctrl+shift+c", "ctrl+C"] {
        for searching in [false, true] {
            let mut s = Screen::build(Owner::ChatLive)
                .size(32, 8)
                .script(include_str!("fixtures/transcript_view_4/sl_copy.jsonl"))
                .view_only()
                .open();
            s.selection_copy_policy(false, true);
            if searching {
                s.keys("/");
            }
            let row = u16::try_from(s.row_of("selected")).unwrap();
            s.triple_click(3, row);
            s.assert_text_style("selected", "reversed");
            s.keys(key);
            s.assert_copies_asked(&["selected\tcafé"]);
            s.answer_copy(Err("clipboard unavailable"));
            s.assert_text_style("selected", "reversed");
            s.keys(key);
            s.assert_copies_asked(&["selected\tcafé", "selected\tcafé"]);
            s.answer_copy_unconfirmed();
            s.assert_text_style("selected", "reversed");
            s.keys(key);
            s.assert_copies_asked(&["selected\tcafé", "selected\tcafé", "selected\tcafé"]);
            s.answer_copy(Ok(()));
            s.assert_text_style("selected", "!reversed");
        }
    }
}

// ORACLE transcript_view/selection_tests.rs:147 command_c_ignores_release_and_copies_only_an_active_selection kind=ui owner=chat-live
#[test]
fn command_c_ignores_release_and_copies_only_an_active_selection() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/sl_command_c.jsonl"
        ))
        .view_only()
        .open();
    s.selection_copy_policy(false, true);
    s.keys("super+c");
    s.assert_copies_asked(&[]);
    let row = u16::try_from(s.row_of("selected café")).unwrap();
    s.triple_click(3, row);
    s.assert_text_style("selected café", "reversed");
    s.key_release("super+c");
    s.assert_copies_asked(&[]);
    s.assert_text_style("selected café", "reversed");
    s.keys("super+c");
    s.assert_copies_asked(&["selected café"]);
    s.answer_copy(Ok(()));
    s.assert_text_style("selected café", "!reversed");
    s.keys("super+c ctrl+c");
    s.assert_copies_asked(&["selected café"]);
}

// ORACLE transcript_view/selection_tests.rs:187 enter_on_an_empty_selection_does_not_copy_or_reach_the_composer kind=ui owner=chat-live
#[test]
fn enter_on_an_empty_selection_does_not_copy_or_reach_the_composer() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-point: a stationary one-click anchor is not exposed by SelectionAreaController, so it cannot own Enter without a selected range",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 12)
        .script(include_str!(
            "fixtures/transcript_view_4/sl_enter_empty.jsonl"
        ))
        .draft("draft stays here")
        .open();
    let row = u16::try_from(s.row_of("select this")).unwrap();
    s.click(3, row);
    s.keys("enter");
    s.assert_text_style("select this", "!reversed");
    s.assert_no_turn();
    s.assert_copies_asked(&[]);
    s.assert_contains("draft stays here");
}

// ORACLE transcript_view/selection_tests.rs:258 clicking_and_clearing_selection_preserves_the_separator_row kind=snap owner=chat-live
#[test]
fn clicking_and_clearing_selection_preserves_the_separator_row() {
    for prepend in [false, true] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 6)
            .replay(Vec::new())
            .view_only()
            .open();
        s.partial_history_of(&["first", "alpha beta gamma\nsecond line\nthird line", "tail"]);
        s.wheel_up(0, 0, 2);
        if prepend {
            s.join_older_page(&["older"], false);
        }
        let before = s.shot();
        let row = u16::try_from(s.row_of("alpha beta gamma")).unwrap();
        s.click(9, row);
        s.assert_same_rows(&before);
        s.wait(350);
        s.double_click_text("beta");
        s.assert_text_style("beta", "reversed");
        s.keys("esc");
        s.assert_same_rows(&before);
        s.assert_text_style("beta", "!reversed");
        s.wait(350);
        s.double_click(31, row);
        s.assert_same_rows(&before);
        s.assert_text_style("beta", "!reversed");
    }
}

// ORACLE transcript_view/selection_tests.rs:350 live_selection_survives_commits_finalization_new_live_content_and_resize kind=snap owner=chat-live
#[test]
fn live_selection_survives_commits_finalization_new_live_content_and_resize() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 9)
        .replay(Vec::new())
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.partial_history_of(&["first"]);
    s.live_tail("tail selected");
    let row = u16::try_from(s.row_of("tail selected")).unwrap();
    s.triple_click(4, row);
    s.assert_text_style("tail selected", "reversed");
    s.commit_live_tail();
    s.live_tail("unrelated next tail");
    s.resize(17, 8);
    s.assert_text_style("tail selected", "reversed");
    s.join_older_page(&["older page"], false);
    s.resize(32, 8);
    s.assert_text_style("tail selected", "reversed");
    s.keys("esc");
    s.assert_text_style("tail selected", "!reversed");
    s.keys("ctrl+end");
    s.assert_contains("unrelated next tail");
}

// ORACLE transcript_view/selection_tests.rs:405 tool_selection_copies_its_displayed_revision_after_the_tool_commits kind=ui owner=chat-live
#[test]
fn tool_selection_copies_its_displayed_revision_after_the_tool_commits() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 12)
        .live()
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.feed(r#"{"step":"entry","kind":"exec","thread":"thread-1","turn":"turn-1","item":"tool","command":"printf output","output":"selected tool output","running":true,"start":true,"end":false}"#);
    let row = u16::try_from(s.row_of("selected tool output")).unwrap();
    s.triple_click(5, row);
    s.assert_text_style("selected tool output", "reversed");
    s.commit_live_tail();
    s.live_tail("another answer");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["selected tool output"]);
}

// ORACLE transcript_view/selection_tests.rs:452 word_and_line_drags_keep_the_initial_unit_when_direction_changes kind=ui owner=chat-live
#[test]
fn word_and_line_drags_keep_the_initial_unit_when_direction_changes() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 5)
        .script(include_str!("fixtures/transcript_view_4/sl_drags.jsonl"))
        .view_only()
        .open();
    // a word drag from column 9 (double click) to 16 selects `beta gamma`; back to 3 it selects `alpha beta`; back to 16 `beta gamma` again
    s.click(9, 0);
    s.mouse_down(9, 0);
    s.mouse_drag(16, 0);
    s.assert_text_style("beta gamma", "reversed");
    s.mouse_drag(3, 0);
    s.assert_text_style("alpha beta", "reversed");
    s.mouse_drag(16, 0);
    s.mouse_up(16, 0);
    s.assert_text_style("beta gamma", "reversed");
    // The third press starts the line drag.
    s.wait(350);
    s.double_click(4, 1);
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
        "G-new-sel-point: the pinned controller does not expose the initial byte after a stationary single click. Extending a nonempty mouse range is exercised separately.",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 5)
        .script(include_str!("fixtures/transcript_view_4/sl_keyboard.jsonl"))
        .view_only()
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
        "G-new-sel-point: the click-first test needs the byte of the stationary click. A selected range already exposes endpoints, including for press and repeat.",
    );
    // the oracle: `abc` / `def` / `ghi` (32x5) with a click at (3,1); left marks `d`, right `e`, up `bc` and `d`, down `ef` and `g`
    for (key, expected) in [("left", "d"), ("right", "e"), ("up", "bc"), ("down", "ef")] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 5)
            .script(include_str!("fixtures/transcript_view_4/sl_arrows.jsonl"))
            .view_only()
            .open();
        s.click(3, 1);
        s.keys(key);
        s.assert_text_style(expected, "reversed");
    }
    // the shift variant and the repeat variant are the same moves (KIT GAP: no key-repeat kind for an arrow with a modifier)
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 5)
        .script(include_str!("fixtures/transcript_view_4/sl_arrows.jsonl"))
        .view_only()
        .open();
    s.click(3, 1);
    s.key_repeat("shift+left");
    s.assert_text_style("d", "reversed");
}

// ORACLE transcript_view/selection_tests.rs:535 modified_arrows_end_selection_and_return_input_to_the_owner kind=ui owner=chat-live
#[test]
fn modified_arrows_end_selection_and_return_input_to_the_owner() {
    for modifier in ["ctrl", "alt", "super"] {
        for arrow in ["left", "right", "up", "down"] {
            let mut s = Screen::build(Owner::ChatLive)
                .size(32, 7)
                .script(include_str!("fixtures/transcript_view_4/sl_modified.jsonl"))
                .view_only()
                .open();
            s.double_click_text("beta");
            s.assert_text_style("beta", "reversed");
            s.keys(&format!("{modifier}+{arrow}"));
            s.assert_text_style("beta", "!reversed");
        }
    }
}

// ORACLE transcript_view/selection_tests.rs:569 released_arrows_preserve_selection_and_reading_position kind=ui owner=chat-live
#[test]
fn released_arrows_preserve_selection_and_reading_position() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 7)
        .script(include_str!("fixtures/transcript_view_4/sl_modified.jsonl"))
        .view_only()
        .open();
    s.double_click_text("beta");
    let before = s.shot();
    s.assert_text_style("beta", "reversed");
    for mods in ["", "shift+", "ctrl+", "alt+shift+"] {
        for key in ["left", "right", "up", "down"] {
            s.key_release(&format!("{mods}{key}"));
            s.assert_same_rows(&before);
            s.assert_text_style("beta", "reversed");
        }
    }
}

// ORACLE transcript_view/selection_tests.rs:600 loading_older_history_extends_the_snapshot_without_adopting_new_commits kind=ui owner=chat-live
#[test]
fn loading_older_history_extends_the_snapshot_without_adopting_new_commits() {
    // the oracle's snapshot is the cells the selection was made in; the port selects in the live
    // list, so the selection stays on its text through a commit below and a page above
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 12)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["header", "selected"]);
    s.double_click_text("selected");
    s.assert_text_style("selected", "reversed");
    // a new commit arrives and an older entry is prepended: `selected` stays the one selected
    s.live_tail("new commit");
    s.commit_live_tail();
    s.join_older_page(&["older"], false);
    s.resize(32, 14);
    s.assert_text_style("selected", "reversed");
}

// ORACLE transcript_view/selection_tests.rs:630 pending_copy_only_finishes_the_original_selection kind=ui owner=chat-live
#[test]
fn pending_copy_only_finishes_the_original_selection() {
    for change in [
        "none",
        "replace",
        "move",
        "navigate",
        "hide",
        "presentation",
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(32, 8)
            .script(include_str!("fixtures/transcript_view_4/sl_pending.jsonl"))
            .view_only()
            .open();
        s.selection_copy_policy(false, true);
        let row = s.row_of("selected text") as u16;
        s.mouse_down(0, row).mouse_drag(24, row).mouse_up(24, row);
        s.assert_text_style("selected text", "reversed");
        s.keys("enter");
        s.assert_copies_asked(&["selected text"]);
        s.answer_copy_pending(1);
        match change {
            "replace" => {
                s.wait(600)
                    .mouse_down(0, row)
                    .mouse_drag(24, row)
                    .mouse_up(24, row);
                s.assert_text_style("selected text", "reversed");
            }
            "move" => {
                s.keys("left");
            }
            "navigate" => {
                s.keys("ctrl+end");
            }
            "hide" => {
                s.keys("esc");
            }
            "presentation" => {
                s.keys("ctrl+t");
            }
            _ => {}
        }
        let before = s.shot();
        s.answer_copy(Ok(()));
        match change {
            "none" => s.assert_text_style("selected text", "!reversed"),
            "replace" => s.assert_text_style("selected text", "reversed"),
            "move" => s.assert_text_style("selected tex", "reversed"),
            _ => s.assert_same_rows(&before),
        }
    }
}
