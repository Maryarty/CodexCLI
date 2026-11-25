// test root: pages
//! Find over the text of the shell calls the model ran: the command as written, its whole output
//! (what the compact cell cuts to a tail), reached through the data (`cells::exec::texts`), and
//! the reveal of the matched cell only (`Disclosed::set_found`): it opens for the match and shuts
//! again when Find leaves it, a cell the reader opened apart.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use crate::kit::{Owner, Screen};

/// Four finished commands: `echo visible`, and a hidden second line that names the entry.
const FOUR: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"exec","turn":"t1","item":"c1","command":"echo visible\necho hidden unrelated","exit":0}
{"step":"entry","kind":"exec","turn":"t2","item":"c2","command":"echo visible\necho hidden older needle","exit":0}
{"step":"entry","kind":"exec","turn":"t3","item":"c3","command":"echo visible\necho hidden newer needle","exit":0}
{"step":"entry","kind":"exec","turn":"t4","item":"c4","command":"echo visible\necho hidden manual","exit":0}
"#;

/// One command with eight lines of output, `needle` the third: the compact cell shows the last
/// three.
const OUTPUT: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"exec","command":"echo","output":"l1\nl2\nhidden needle\nl4\nl5\nl6\nl7\ntail","exit":0}
"#;

fn compact(script: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(width, height)
        .script(script)
        .view_compact()
        .open()
}

#[test]
fn only_the_matching_cell_expands_and_leaving_it_collapses() {
    let mut s = compact(FOUR, 40, 30);
    s.assert_absent("hidden");
    s.keys("/");
    s.type_text("needle");
    // the newest match: its cell shows the line the compact cell cuts, the others stay as they are
    s.assert_contains("hidden newer needle");
    s.assert_absent("hidden older needle");
    s.assert_absent("hidden unrelated");
    s.assert_absent("hidden manual");
    s.assert_text_style("needle", "reversed");
    s.keys("ctrl+p");
    s.assert_contains("hidden older needle");
    s.assert_absent("hidden newer needle");
    s.keys("ctrl+n");
    s.assert_contains("hidden newer needle");
    s.assert_absent("hidden older needle");
    s.keys("esc");
    s.assert_absent("hidden");
}

#[test]
fn a_cell_the_reader_opened_stays_open_when_find_leaves() {
    let mut s = compact(FOUR, 40, 30);
    // the first control on the screen is the oldest cell's
    s.click_text("+ Show details");
    s.assert_contains("hidden unrelated");
    s.keys("/");
    s.type_text("needle");
    s.assert_contains("hidden newer needle");
    s.assert_contains("hidden unrelated");
    s.keys("esc");
    s.assert_absent("hidden newer needle");
    s.assert_contains("hidden unrelated");
}

#[test]
fn a_matched_command_shows_its_hidden_output_and_the_compact_rows_come_back() {
    let mut s = compact(OUTPUT, 40, 16);
    s.assert_absent("hidden needle");
    s.assert_contains("tail");
    let before = s.shot();
    s.keys("/");
    s.type_text("hidden needle");
    s.assert_contains("hidden needle");
    s.assert_text_style("hidden needle", "reversed");
    // Enter accepts the match, and Esc leaves Find: the cell is cut to its tail again
    s.keys("enter");
    s.assert_contains("hidden needle");
    s.keys("esc");
    s.assert_absent("hidden needle");
    s.assert_same_rows(&before);
}

#[test]
fn a_manual_close_ends_what_find_opened_for_the_match() {
    let mut s = compact(OUTPUT, 40, 16);
    s.keys("/");
    s.type_text("hidden needle");
    s.assert_contains("hidden needle");
    s.keys("enter");
    // the control of the open cell closes it: the match's cell is the reader's to shut
    s.click_text("− Hide details");
    s.assert_absent("hidden needle");
}

#[test]
fn the_command_as_written_is_found_in_its_hidden_second_line() {
    let mut s = compact(FOUR, 40, 30);
    s.keys("/");
    s.type_text("hidden manual");
    s.assert_contains("echo hidden manual");
    s.assert_text_style("hidden manual", "reversed");
}

/// A command whose first line is longer than the room: its header is cut with a `…` of its own
/// (`dots_spans`), a node the text of the cell must account for, or every match after it lands a
/// character off.
const LONG: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"exec","command":"echo a-rather-long-first-line-of-the-script\necho second","output":"one\ntwo\nhere is the needle\nfour\nfive\nsix\nseven\neight","exit":0}
"#;

#[test]
fn a_header_cut_by_a_narrow_room_does_not_shift_the_match() {
    let mut s = compact(LONG, 24, 30);
    s.keys("/");
    s.type_text("needle");
    s.assert_contains("here is the needle");
    s.assert_text_style("needle", "reversed");
    s.keys("esc");
    s.keys("/");
    s.type_text("second");
    s.assert_text_style("second", "reversed");
}
