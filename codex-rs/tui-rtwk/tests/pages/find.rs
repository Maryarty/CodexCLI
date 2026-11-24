// test root: pages
//! Find on the chat's transcript (F3; oracle `transcript_view/search.rs`, `search_footer.rs`,
//! `app/owned_transcript.rs`): the query line under the composer, the newest match first, the
//! match drawn reversed, `Ctrl+P`/`Ctrl+N` stepping without wrapping, Enter reading a match,
//! Esc and `Ctrl+C` closing it. The transcript is two agent answers, `Needle one` and `Needle two`.
//!
//! The footer's place is the page's footer slot, which calls `find_footer::slot` once.

use crate::kit::{Owner, Screen};

const NEEDLES: &str = include_str!("../oracle/fixtures/transcript_view_4/find_jump.jsonl");

fn chat() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(NEEDLES)
        .open()
}

#[test]
fn f3_opens_the_query_line_and_the_newest_match_is_drawn_reversed() {
    let mut s = chat();
    s.keys("f3");
    s.assert_contains("Find: ");
    s.assert_contains("Type to find");
    s.type_text("needle two");
    s.assert_contains("Find: needle two");
    s.assert_contains("enter accept");
    s.assert_text_style("Needle two", "reversed");
    s.assert_text_style("Needle one", "!reversed");
}

#[test]
fn the_query_is_found_whatever_its_case() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("NEEDLE TWO");
    s.assert_text_style("Needle two", "reversed");
}

#[test]
fn ctrl_p_and_ctrl_n_step_older_and_newer_and_stop_at_the_ends() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("needle");
    s.keys("ctrl+p");
    s.assert_text_style("Needle one", "reversed");
    s.keys("ctrl+p");
    s.assert_contains("No more matches");
    s.assert_text_style("Needle one", "reversed");
    s.keys("ctrl+n");
    s.assert_text_style("Needle two", "reversed");
    s.keys("ctrl+n");
    s.assert_contains("No more matches");
}

#[test]
fn a_query_nothing_matches_says_so() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("absent");
    s.assert_contains("No matches");
    s.keys("enter");
    // an empty result is not accepted: the query stays open
    s.assert_contains("Find: absent");
}

#[test]
fn esc_closes_find_and_the_composer_has_its_draft_back() {
    let mut s = chat();
    s.type_text("my draft");
    s.keys("f3");
    s.type_text("needle");
    s.assert_contains("Find: needle");
    s.keys("esc");
    s.assert_absent("Find: ");
    s.assert_composer("my draft");
    s.type_text("!");
    s.assert_composer("my draft!");
}

#[test]
fn enter_reads_the_match_and_esc_then_goes_back_to_the_latest() {
    let mut s = chat();
    s.type_text("my draft");
    s.keys("f3");
    s.type_text("needle");
    s.keys("enter");
    s.assert_contains("Find · ctrl+p older · ctrl+n newer · esc latest");
    s.assert_absent("Find: ");
    s.assert_composer("my draft");
    // read: the keys of the match are Find's, the draft is not touched
    s.keys("ctrl+p");
    s.assert_text_style("Needle one", "reversed");
    s.keys("enter");
    s.assert_composer("my draft");
    s.keys("esc");
    s.assert_absent("Find · ");
    s.assert_composer("my draft");
}

#[test]
fn ctrl_c_cancels_find_instead_of_quitting() {
    let mut s = chat();
    s.keys("f3");
    s.type_text("needle");
    s.keys("ctrl+c");
    s.assert_absent("Find: ");
    s.assert_contains("Needle two");
}

#[test]
fn a_paste_past_the_limit_is_cut_whole_and_says_so() {
    let mut s = chat();
    s.keys("f3");
    s.paste(&"é".repeat(3000));
    s.assert_contains("query limited to 4 KiB");
}
