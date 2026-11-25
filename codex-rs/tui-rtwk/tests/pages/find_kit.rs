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
