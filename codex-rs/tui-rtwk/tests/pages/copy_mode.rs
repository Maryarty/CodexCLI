// test root: pages
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Copy mode of a chat that owns its transcript (oracle `transcript_view/copy_mode.rs`): `/copy`
//! begins it over the transcript instead of opening the picker, the target is filled whole
//! (every cell of its rows, blank and padding ones included, reversed once: the selection is
//! not drawn on top, which would toggle it back), a prompt is a target when it is all there is,
//! and `esc` takes the fill away.

use crate::kit::{Owner, Screen};

const HINTS: &str = "↑/↓/j/k select · g/G ends · enter copy · esc close";

fn owned(fixture: &'static str, width: u16, height: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(width, height)
        .owned_transcript()
        .script(fixture)
        .open()
}

fn begin(s: &mut Screen) {
    s.type_text("/copy").keys("enter");
}

#[test]
fn an_owned_transcript_begins_copy_mode_where_a_plain_chat_opens_the_picker() {
    let fixture = include_str!("../oracle/fixtures/transcript_view_2/highlight_blank.jsonl");
    let mut s = owned(fixture, 52, 19);
    begin(&mut s);
    s.assert_contains(HINTS);
    let mut plain = Screen::build(Owner::ChatLive)
        .size(52, 19)
        .script(fixture)
        .open();
    begin(&mut plain);
    plain.assert_absent(HINTS);
}

#[test]
fn a_prompt_with_no_response_is_the_target_and_the_walk_stays_on_prompts() {
    let fixture = include_str!("../oracle/fixtures/transcript_view_2/user_vim.jsonl");
    let mut s = owned(fixture, 52, 21);
    begin(&mut s);
    s.assert_contains("Copy user message");
    s.assert_text_style("Plain prompt with wrapping", "reversed");
    s.keys("g");
    s.assert_text_style("First user message", "reversed");
    s.assert_text_style("Plain prompt with wrapping", "!reversed");
    s.keys("esc");
    s.assert_text_style("First user message", "!reversed");
    s.assert_absent(HINTS);
}

#[test]
fn the_filled_block_is_reversed_once_across_blank_rows_and_padding() {
    let mut s = owned(
        include_str!("../oracle/fixtures/transcript_view_2/highlight_blank.jsonl"),
        16,
        19,
    );
    begin(&mut s);
    s.keys("up");
    let top = s.row_of("Before");
    // the block is `longer line`, a blank row and `x`: every cell of the three rows is reversed
    // (a glyph too: not toggled back by a selection over the same cells), the rows around it
    // are not
    for row in 0..7 {
        let spec = if (2..5).contains(&row) {
            "reversed"
        } else {
            "!reversed"
        };
        for x in 0..16 {
            s.assert_style(x, top + row, spec);
        }
    }
    s.keys("esc");
    for row in 0..7 {
        for x in 0..16 {
            s.assert_style(x, top + row, "!reversed");
        }
    }
}

#[test]
fn a_whole_prompt_is_filled_with_its_padding_rows() {
    let mut s = owned(
        include_str!("../oracle/fixtures/transcript_view_2/user_vim.jsonl"),
        52,
        21,
    );
    begin(&mut s);
    let row = s.row_of("Plain prompt with wrapping");
    // the cell above and the one below the text are the prompt's padding rows
    for y in [row - 1, row, row + 1] {
        s.assert_style(0, y, "reversed");
        s.assert_style(40, y, "reversed");
    }
}
