// test root: pages
//! What Find over a live and a joining transcript needs of the kit and of the pane: the live tail
//! that goes on after a commit (a turn of its own), an answer replaced as a whole, the compact
//! view, and an older page joining under an open Find.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use crate::kit::{Owner, Screen};

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"agent","text":"earlier context"}
"#;

const THOUGHT: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"reasoning","text":"quiet thought"}
{"step":"entry","kind":"agent","text":"the answer"}
"#;

fn bare(script: &'static str) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(30, 8)
        .script(script)
        .view_only()
        .open()
}

#[test]
fn a_live_tail_after_a_commit_is_an_answer_of_a_new_turn() {
    let mut s = bare(SESSION);
    s.live_tail("first live");
    s.commit_live_tail();
    s.live_tail("second live");
    s.assert_contains("• first live");
    s.assert_contains("• second live");
    // the answer is the whole text again, and the older one stays as it was committed
    s.replace_live_tail("replaced live");
    s.assert_contains("• replaced live");
    s.assert_absent("second live");
    s.assert_contains("• first live");
}

#[test]
fn the_compact_view_leaves_out_what_only_the_detailed_transcript_draws() {
    let detailed = bare(THOUGHT);
    detailed.assert_contains("quiet thought");
    let compact = Screen::build(Owner::ChatLive)
        .size(30, 8)
        .script(THOUGHT)
        .view_compact()
        .open();
    compact.assert_absent("quiet thought");
    compact.assert_contains("the answer");
}

#[test]
fn a_match_in_the_live_answer_is_shown_as_found_until_the_reader_pages() {
    let mut s = bare(SESSION);
    s.live_tail("live needle");
    s.keys("/");
    s.type_text("needle");
    s.assert_text_style("needle", "reversed");
    // the answer is another text, and the match stays as it was found
    s.replace_live_tail("live other words");
    s.assert_contains("• live needle");
    // a page key releases it: the answer is read as it is now, the query stays
    s.keys("pagedown");
    s.assert_contains("• live other words");
    s.assert_absent("live needle");
    s.assert_find_query("needle");
}

#[test]
fn an_older_page_joining_under_find_leaves_the_match_and_the_rows_where_they_are() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 6)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["needle first", "second", "last"]);
    // the reader is at the first row: the top, where the app asks for the older page
    s.wheel_up(0, 0, 100);
    s.keys("/");
    s.type_text("needle");
    s.assert_text_style("needle", "reversed");
    let found = s.shot();
    s.join_older_page(&["older text"], false);
    s.assert_same_rows(&found);
    s.assert_text_style("needle", "reversed");
    // the match is the newest and only one: a step finds no other
    s.keys("ctrl+p");
    s.assert_same_rows(&found);
    s.keys("esc");
    s.assert_same_rows(&found);
}
