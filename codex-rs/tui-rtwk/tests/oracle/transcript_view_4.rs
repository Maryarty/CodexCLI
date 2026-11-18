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
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no find: `/` opens no search, so no match moves between prompts and no prompt row is highlighted",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 12)
        .script(include_str!(
            "fixtures/transcript_view_4/search_prompts.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.keys("ctrl+n");
    // the oracle's rows: the first prompt, a blank row, the matching prompt; its inline `#` marks (the prompt background of rows 0 to 2) are not asserted until the port has the prompt palette
    s.assert_row(0, "› first");
    s.assert_row(1, "");
    s.assert_row(2, "› needle second");
}

// ORACLE transcript_view/search_tests.rs:115 find_keeps_context_and_allows_reading_without_losing_the_query kind=snap owner=chat-live snap=find_context
#[test]
fn find_keeps_context_and_allows_reading_without_losing_the_query() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no find: `/` or F3 opens no search, so no find query, match context or find footer is drawn",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/find_context.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.assert_snap("find_context");
    s.keys("pagedown");
    s.assert_contains("latest message");
    s.keys("pageup");
    let scrolled = s.shot();
    s.keys("enter");
    // the oracle's footer is drawn at 80 columns; this screen is 40 wide, so the full footer string is asserted only once the port draws it
    s.assert_contains("Find · ctrl+p older · ctrl+n newer · esc latest");
    s.assert_same_rows(&scrolled);
    s.keys("/");
    s.assert_same_rows(&scrolled);
    s.keys("enter");
    s.keys("ctrl+p");
    s.assert_contains("earlier needle");
    s.keys("esc");
    s.assert_contains("latest message");
}

// ORACLE transcript_view/search_tests.rs:187 find_reduces_context_to_keep_a_tall_match_visible kind=snap owner=chat-live snap=find_context_for_tall_matches
#[test]
fn find_reduces_context_to_keep_a_tall_match_visible() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no find: no match context is drawn, so a tall match cannot be kept in view",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(6, 6)
        .script(include_str!("fixtures/transcript_view_4/find_tall.jsonl"))
        .open();
    s.keys("/");
    s.type_text("one two three four five");
    s.assert_snap_section("find_context_for_tall_matches", "height 6");
    // the oracle builds a fresh view for each height; the port resizes one screen
    s.resize(6, 5);
    s.assert_snap_section("find_context_for_tall_matches", "height 5");
    s.resize(6, 3);
    s.assert_snap_section("find_context_for_tall_matches", "height 3");
}

// ORACLE transcript_view/search_tests.rs:211 selection_pauses_find_and_normalized_navigation_resumes_the_same_query kind=snap owner=chat-live snap=selection_owns_find_highlight
#[test]
fn selection_pauses_find_and_normalized_navigation_resumes_the_same_query() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no find and no mouse selection of a transcript row; a selection cannot pause a find",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 4)
        .script(include_str!(
            "fixtures/transcript_view_4/find_selection.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    // three clicks on row 1 select its line while the previous-match scan is pending
    s.click(0, 1);
    s.click(0, 1);
    s.click(0, 1);
    s.assert_snap("selection_owns_find_highlight");
    s.keys("esc");
    s.keys("ctrl+n");
    s.assert_contains("newer needle");
    s.keys("ctrl+c");
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
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no find and no reading position to search from: the screen cannot jump to an entry and search from it",
    );
    for (key, expected) in [("ctrl+p", "older needle"), ("ctrl+n", "newer needle")] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(30, 1)
            .script(include_str!(
                "fixtures/transcript_view_4/find_reading.jsonl"
            ))
            .open();
        // the oracle reads entry 1 (`context`) first: the jump is not a kit step yet
        s.keys("/");
        s.type_text("needle");
        s.keys("enter");
        s.keys(key);
        s.assert_contains(expected);
    }
}

// ORACLE transcript_view/search_tests.rs:337 resizing_a_read_find_result_preserves_relative_navigation kind=snap owner=chat-live snap=find_context_after_resize
#[test]
fn resizing_a_read_find_result_preserves_relative_navigation() {
    kit::pending(
        Owner::ChatLive,
        "the chat's transcript has no find and no read result to re-wrap: the screen has no find state to keep across a resize",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 3)
        .script(include_str!("fixtures/transcript_view_4/find_resize.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    s.keys("enter");
    // the oracle re-wraps at width 20 with its area kept at 30 by prepare_width; the kit resize changes the terminal width too
    s.resize(20, 3);
    s.assert_snap("find_context_after_resize");
    s.keys("ctrl+p");
    s.assert_contains("older explanation");
    s.keys("ctrl+n");
    s.assert_contains("newer explanation");
    s.keys("ctrl+n");
    s.resize(25, 3);
    s.keys("ctrl+p");
    s.assert_contains("older explanation");
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
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find: no match is highlighted, no find status line is drawn and esc does not return to a saved position",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 3)
        .script(include_str!("fixtures/transcript_view_4/find_jump.jsonl"))
        .open();
    s.keys("pageup");
    let original = s.shot();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.keys("ctrl+n");
    // the current match `Needle two` is reversed for its first six cells, the rest of the row is plain
    for x in 0..6 {
        s.assert_style(x, 1, "reversed");
    }
    // the status line is 80 columns wide in the oracle; this screen is 20 wide, so the full line is asserted once the port draws it
    s.assert_contains("enter accept · ⌃p older · ⌃n newer · full transcript · esc cancel");
    s.keys("left");
    s.keys("esc");
    s.assert_same_rows(&original);
}

// ORACLE transcript_view/search_tests.rs:1026 legacy_search_shortcuts_navigate_in_both_directions_without_wrapping kind=snap owner=chat-live
#[test]
fn legacy_search_shortcuts_navigate_in_both_directions_without_wrapping() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find and no find status line: `No more matches` and the ⌃p/⌃n shortcuts are not drawn",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/transcript_view_4/legacy_short.jsonl"
        ))
        .open();
    s.keys("/");
    s.type_text("USER]");
    s.keys("ctrl+p");
    s.keys("ctrl+p");
    s.keys("ctrl+p");
    // the third older press has no older match left: the oracle draws `No more matches` and stays on the first entry
    s.assert_contains("No more matches · enter accept · ⌃p older · ⌃n newer · esc cancel");
    s.keys("ctrl+n");
    s.keys("ctrl+n");
    s.keys("ctrl+n");
    s.assert_contains("No more matches · enter accept · ⌃p older · ⌃n newer · esc cancel");
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
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript find and no history load under a query: the screen cannot show a loading search with the reading position held",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 6)
        .script(include_str!(
            "fixtures/transcript_view_4/refine_history.jsonl"
        ))
        .open();
    s.keys("/");
    let before = s.shot();
    s.type_text("F");
    s.assert_contains("Fixture link");
    s.type_text("IRST ANSWER");
    // the query needs older history to find `First answer`: the loading search draws the same rows as before the query was typed
    s.assert_same_rows(&before);
    s.keys("ctrl+c");
}

// ORACLE transcript_view/search_tests.rs:1772 bounded_query_paste_keeps_the_suffix_and_whole_graphemes kind=snap owner=chat-live
#[test]
fn bounded_query_paste_keeps_the_suffix_and_whole_graphemes() {
    kit::pending(
        Owner::ChatLive,
        "the chat's find has no query editor that bounds a paste to 4 KiB and keeps its suffix, and no `query limited to 4 KiB` status line",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!("fixtures/transcript_view_4/query_limit.jsonl"))
        .open();
    s.keys("/");
    // a query longer than 4 KiB: the paste keeps its last `Z` (the oracle's query is `a` x 4094 then `Z`)
    let original = format!("{}Z", "a".repeat(4096 - 2));
    s.paste(&original);
    // the oracle then pastes e-acute past the limit (dropped) and types b and c; the delete keeps the suffix: those steps are not written yet
    s.assert_contains("Searching… · full transcript · esc cancel · query limited to 4 KiB");
}

// ORACLE transcript_view/search_tests.rs:1819 selection_keys_precede_find_and_typing_returns_to_query kind=ui owner=chat-live
#[test]
fn selection_keys_precede_find_and_typing_returns_to_query() {
    kit::pending(
        Owner::ChatLive,
        "the chat has no transcript selection that a find can precede: ctrl+space starts no selection and ctrl+c copies nothing from the transcript",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 3)
        .script(include_str!("fixtures/transcript_view_4/needle_only.jsonl"))
        .open();
    s.keys("/");
    s.type_text("needle");
    // the oracle's selection keys go to the selection (ctrl+c copies it, the copy is not drawn); then typing returns to the query
    s.keys("ctrl+space");
    s.keys("right");
    s.keys("ctrl+c");
    s.type_text("x");
    s.assert_contains("needlex");
}

// ---- oracle: transcript_view/select_mode_tests.rs (19 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/select_mode_tests.rs:37 entry_starts_on_the_last_selectable_visible_line kind=ui owner=chat-live
#[test]
fn entry_starts_on_the_last_selectable_visible_line() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:72 character_motion_crosses_line_and_entry_boundaries_without_invisible_stops kind=ui owner=chat-live
#[test]
fn character_motion_crosses_line_and_entry_boundaries_without_invisible_stops() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:166 vertical_motion_keeps_the_preferred_display_column kind=ui owner=chat-live
#[test]
fn vertical_motion_keeps_the_preferred_display_column() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:181 enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows kind=ui owner=chat-live
#[test]
fn enter_without_a_mark_exits_but_enter_with_a_mark_copies_and_follows() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:207 word_motion_crosses_entries_and_word_end_selects_the_destination_word kind=ui owner=chat-live
#[test]
fn word_motion_crosses_entries_and_word_end_selects_the_destination_word() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:223 reverse_line_selection_copies_complete_lines_in_document_order kind=ui owner=chat-live
#[test]
fn reverse_line_selection_copies_complete_lines_in_document_order() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:238 logical_line_end_handles_empty_lines_and_shifted_dollar kind=ui owner=chat-live
#[test]
fn logical_line_end_handles_empty_lines_and_shifted_dollar() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:270 configured_pager_page_motion_moves_the_select_cursor kind=ui owner=chat-live
#[test]
fn configured_pager_page_motion_moves_the_select_cursor() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:303 transcript_end_character_selection_includes_the_last_grapheme kind=ui owner=chat-live
#[test]
fn transcript_end_character_selection_includes_the_last_grapheme() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:315 swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint kind=ui owner=chat-live
#[test]
fn swapping_the_active_end_preserves_text_and_moves_the_shrinking_endpoint() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:334 confirmed_copy_keeps_the_select_mode_caret_on_the_active_grapheme kind=ui owner=chat-live
#[test]
fn confirmed_copy_keeps_the_select_mode_caret_on_the_active_grapheme() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:360 find_accepts_at_the_match_and_cancel_restores_the_saved_selection kind=ui owner=chat-live
#[test]
fn find_accepts_at_the_match_and_cancel_restores_the_saved_selection() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:469 accepted_find_repeats_into_the_select_cursor kind=ui owner=chat-live
#[test]
fn accepted_find_repeats_into_the_select_cursor() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:536 accepted_find_pins_an_expanded_match_before_select_mode_exits kind=ui owner=chat-live
#[test]
fn accepted_find_pins_an_expanded_match_before_select_mode_exits() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:617 beginning_motion_finishes_at_the_first_grapheme_after_paging kind=ui owner=chat-live
#[test]
fn beginning_motion_finishes_at_the_first_grapheme_after_paging() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:637 shift_click_extends_from_the_keyboard_marks_fixed_anchor kind=ui owner=chat-live
#[test]
fn shift_click_extends_from_the_keyboard_marks_fixed_anchor() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:664 existing_mouse_selection_keeps_its_word_or_line_unit kind=ui owner=chat-live
#[test]
fn existing_mouse_selection_keeps_its_word_or_line_unit() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:699 passive_mouse_events_do_not_reanchor_a_reverse_line_mark kind=ui owner=chat-live
#[test]
fn passive_mouse_events_do_not_reanchor_a_reverse_line_mark() {
    kit::todo();
}

// ORACLE transcript_view/select_mode_tests.rs:735 select_mode_renders_cursor_and_scannable_footer_hints kind=snap owner=chat-live snap=select_mode_renders_cursor_and_scannable_footer_hints
#[test]
fn select_mode_renders_cursor_and_scannable_footer_hints() {
    kit::todo();
}

// ---- oracle: transcript_view/selection_tests.rs (14 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/selection_tests.rs:22 selection_highlights_hard_breaks_within_and_between_entries kind=snap owner=chat-live snap=selection_highlights_hard_breaks_within_and_between_entries
#[test]
fn selection_highlights_hard_breaks_within_and_between_entries() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:60 copy_shortcuts_clear_selection_only_after_confirmed_delivery kind=ui owner=chat-live
#[test]
fn copy_shortcuts_clear_selection_only_after_confirmed_delivery() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:147 command_c_ignores_release_and_copies_only_an_active_selection kind=ui owner=chat-live
#[test]
fn command_c_ignores_release_and_copies_only_an_active_selection() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:187 enter_on_an_empty_selection_does_not_copy_or_reach_the_composer kind=ui owner=chat-live
#[test]
fn enter_on_an_empty_selection_does_not_copy_or_reach_the_composer() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:258 clicking_and_clearing_selection_preserves_the_separator_row kind=snap owner=chat-live
#[test]
fn clicking_and_clearing_selection_preserves_the_separator_row() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:350 live_selection_survives_commits_finalization_new_live_content_and_resize kind=snap owner=chat-live
#[test]
fn live_selection_survives_commits_finalization_new_live_content_and_resize() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:405 tool_selection_copies_its_displayed_revision_after_the_tool_commits kind=ui owner=chat-live
#[test]
fn tool_selection_copies_its_displayed_revision_after_the_tool_commits() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:452 word_and_line_drags_keep_the_initial_unit_when_direction_changes kind=ui owner=chat-live
#[test]
fn word_and_line_drags_keep_the_initial_unit_when_direction_changes() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:478 keyboard_selection_moves_by_grapheme_and_keeps_its_column_across_short_lines kind=ui owner=chat-live
#[test]
fn keyboard_selection_moves_by_grapheme_and_keeps_its_column_across_short_lines() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:511 plain_and_shift_arrows_move_selection_on_press_and_repeat kind=ui owner=chat-live
#[test]
fn plain_and_shift_arrows_move_selection_on_press_and_repeat() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:535 modified_arrows_end_selection_and_return_input_to_the_owner kind=ui owner=chat-live
#[test]
fn modified_arrows_end_selection_and_return_input_to_the_owner() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:569 released_arrows_preserve_selection_and_reading_position kind=ui owner=chat-live
#[test]
fn released_arrows_preserve_selection_and_reading_position() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:600 loading_older_history_extends_the_snapshot_without_adopting_new_commits kind=ui owner=chat-live
#[test]
fn loading_older_history_extends_the_snapshot_without_adopting_new_commits() {
    kit::todo();
}

// ORACLE transcript_view/selection_tests.rs:630 pending_copy_only_finishes_the_original_selection kind=ui owner=chat-live
#[test]
fn pending_copy_only_finishes_the_original_selection() {
    kit::todo();
}
