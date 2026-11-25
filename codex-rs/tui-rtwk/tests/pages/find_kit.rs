// test root: pages
//! The find doors of the kit (`find_footer`, `find_query`, `assert_find_query`): the finder's status
//! row and query as the handle reads them, in the bare transcript view.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use crate::kit::{Owner, Screen};

fn bare(script: &'static str) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(40, 6)
        .script(script)
        .view_only()
        .open()
}

#[test]
fn find_kit_reads_the_query_typed_in_the_bare_view() {
    let mut s = bare(include_str!(
        "../oracle/fixtures/transcript_view_4/find_context.jsonl"
    ));
    s.keys("/");
    s.type_text("needle");
    s.assert_find_query("needle");
    s.assert_text_style("needle", "reversed");
    s.keys("backspace");
    s.assert_find_query("needl");
}

#[test]
fn find_kit_sets_the_query_and_reads_the_status_row() {
    let mut s = bare(include_str!(
        "../oracle/fixtures/transcript_view_4/find_context.jsonl"
    ));
    s.keys("/");
    s.find_query("needle");
    s.assert_find_query("needle");
    s.assert_text_style("needle", "reversed");
    s.find_query("nothing like it");
    s.assert_find_footer(80, "No matches · full transcript · esc cancel");
}

#[test]
fn closing_find_puts_the_reader_back_where_it_began() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 6)
        .script(include_str!(
            "../oracle/fixtures/transcript_view_4/find_jump.jsonl"
        ))
        .view_only()
        .open();
    s.keys("pageup");
    let original = s.shot();
    s.keys("/");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.keys("ctrl+n");
    s.keys("esc");
    s.assert_same_rows(&original);
}

fn partial() -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 6)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["First answer", "Second answer", "Last answer"]);
    s
}

#[test]
fn a_query_not_in_the_loaded_rows_waits_for_the_older_page_and_finds_it_there() {
    let mut s = partial();
    s.keys("/");
    s.type_text("zeta");
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    assert_eq!(s.page_requests(), [None, Some("cursor-1".to_owned())]);
    s.join_older_page(&["zeta report"], false);
    s.assert_contains("zeta report");
    s.assert_text_style("zeta", "reversed");
    s.assert_find_footer(
        80,
        "enter accept · ⌃p older · ⌃n newer · full transcript · esc cancel",
    );
    assert_eq!(s.page_requests().len(), 2);
}

#[test]
fn a_query_the_older_page_lacks_goes_on_to_the_next_page_and_ends_with_no_matches() {
    let mut s = partial();
    s.keys("/");
    s.type_text("zeta");
    s.join_older_page(&["Older answer"], true);
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    assert_eq!(s.page_requests().len(), 3);
    s.join_older_page(&["Oldest answer"], false);
    s.assert_find_footer(80, "No matches · full transcript · esc cancel");
}

#[test]
fn a_failed_older_page_says_so_and_ctrl_p_asks_for_it_again() {
    let mut s = partial();
    s.keys("/");
    s.type_text("zeta");
    s.history_failed("history unavailable");
    s.assert_find_footer(
        80,
        "History unavailable · ⌃p retry · full transcript · esc cancel",
    );
    s.keys("ctrl+p");
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    assert_eq!(s.page_requests().len(), 3);
    s.join_older_page(&["zeta report"], false);
    s.assert_contains("zeta report");
}

#[test]
fn closing_find_while_it_waits_for_history_puts_the_reader_back() {
    let mut s = partial();
    s.keys("pageup");
    let original = s.shot();
    s.keys("/");
    s.type_text("zeta");
    s.assert_find_footer(
        80,
        "Searching earlier history… · full transcript · esc cancel",
    );
    s.keys("esc");
    s.assert_same_rows(&original);
}
