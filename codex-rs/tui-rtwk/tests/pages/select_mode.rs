// test root: pages
//! Select mode on the chat's transcript (`ctrl+x [`, the default leader; oracle `transcript_view/select_mode.rs`):
//! a caret on the last line that the keys move, a mark that starts a range at it (`v`, `V`), the
//! footer that says the keys, Esc and `q` and Enter that end it. The transcript is the oracle's
//! `sm_footer` (one answer, `alpha beta` over `gamma`) and `sm_word` (`alpha`, then `beta gamma`).

use crate::kit::{Owner, Screen};

const FOOTER: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_footer.jsonl");
const WORDS: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_word.jsonl");
const EMPTY: &str = include_str!("../oracle/fixtures/transcript_view_4/sm_entry_6.jsonl");

fn chat(script: &'static str, width: u16) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(width, 14)
        .script(script)
        .open()
}

#[test]
fn ctrl_x_bracket_begins_select_mode_and_the_footer_says_its_keys() {
    let mut s = chat(FOOTER, 32);
    s.assert_absent("v mark · enter/q close");
    s.keys("ctrl+x [");
    s.assert_contains("v mark · enter/q close");
    let mut s = chat(FOOTER, 80);
    s.keys("ctrl+x [");
    s.assert_contains("arrows/hjkl move · v/space/V mark · / find · enter/q/esc close");
}

#[test]
fn the_caret_starts_on_the_last_line_and_a_line_mark_draws_it_reversed() {
    let mut s = chat(FOOTER, 32);
    s.keys("ctrl+x [");
    s.keys("V");
    s.assert_text_style("gamma", "reversed");
    s.assert_text_style("alpha beta", "!reversed");
    s.assert_contains("enter copy · q close");
}

#[test]
fn k_and_j_move_the_caret_by_lines() {
    let mut s = chat(FOOTER, 32);
    s.keys("ctrl+x [");
    s.keys("k V");
    s.assert_text_style("alpha beta", "reversed");
    s.assert_text_style("gamma", "!reversed");
    s.keys("esc j V");
    s.assert_text_style("gamma", "reversed");
}

#[test]
fn the_footer_says_the_keys_of_a_mark_by_the_width() {
    let mut s = chat(FOOTER, 60);
    s.keys("ctrl+x [");
    s.keys("V");
    s.assert_contains("y/⌃c copy · enter copy & latest · esc clear · q close");
    let mut s = chat(FOOTER, 100);
    s.keys("ctrl+x [");
    s.keys("V");
    s.assert_contains(
        "arrows/hjkl move · o swap · y/⌃c copy · enter copy & latest · esc clear · q close",
    );
}

#[test]
fn a_character_mark_runs_from_the_mark_through_the_caret() {
    let mut s = chat(FOOTER, 32);
    s.keys("ctrl+x [");
    s.keys("g v l l");
    s.assert_text_style("alp", "reversed");
    s.assert_text_style("ha beta", "!reversed");
    s.keys("o");
    s.keys("h");
    s.assert_text_style("al", "reversed");
}

#[test]
fn a_range_across_two_answers_is_drawn_reversed() {
    let mut s = chat(WORDS, 32);
    s.keys("ctrl+x [");
    s.keys("g V j");
    s.assert_text_style("alpha", "reversed");
    s.assert_text_style("beta gamma", "reversed");
}

#[test]
fn esc_clears_the_mark_and_then_ends_the_mode() {
    let mut s = chat(FOOTER, 32);
    s.keys("ctrl+x [");
    s.keys("V");
    s.assert_contains("enter copy · q close");
    s.keys("esc");
    s.assert_contains("v mark · enter/q close");
    s.assert_text_style("alpha beta", "!reversed");
    s.keys("esc");
    s.assert_absent("v mark · enter/q close");
}

#[test]
fn q_and_enter_without_a_mark_end_the_mode() {
    let mut s = chat(FOOTER, 32);
    s.keys("ctrl+x [");
    s.keys("q");
    s.assert_absent("v mark · enter/q close");
    s.keys("ctrl+x [");
    s.keys("enter");
    s.assert_absent("v mark · enter/q close");
}

#[test]
fn nothing_selectable_begins_no_mode() {
    let mut s = chat(EMPTY, 32);
    s.keys("ctrl+x [");
    s.assert_absent("v mark · enter/q close");
}

#[test]
fn the_keys_of_the_mode_do_not_edit_the_draft_and_the_others_do() {
    let mut s = chat(FOOTER, 80);
    s.type_text("draft");
    s.keys("ctrl+x [");
    s.keys("k j");
    s.assert_contains("draft");
    s.keys("esc");
    s.assert_contains("draft");
}

#[test]
fn find_inside_select_mode_lands_the_caret_on_the_match() {
    let mut s = chat(FOOTER, 80);
    s.keys("ctrl+x [");
    s.keys("/");
    s.type_text("beta");
    s.assert_contains("Find: beta");
    s.assert_text_style("beta", "reversed");
    s.keys("enter");
    // Find is done, the caret is at the match: a mark runs from `b`
    s.assert_absent("Find:");
    s.keys("v l l l");
    s.assert_text_style("beta", "reversed");
}

/// The transcript view alone (no composer): the keys reach the sheet, not a draft.
fn view() -> Screen {
    Screen::build(Owner::ChatLive)
        .size(64, 7)
        .script(FOOTER)
        .view_only()
        .open()
}

#[test]
fn the_caret_cell_is_bold_underlined_and_reversed_over_a_mark_and_alone() {
    let mut s = view();
    s.keys("leader [");
    s.assert_style_in("gamma", 0, "bold underlined reversed");
    s.assert_style_in("gamma", 1, "!reversed");
    s.keys("g v right right");
    s.assert_style_in("alpha beta", 0, "reversed");
    s.assert_style_in("alpha beta", 1, "reversed");
    s.assert_style_in("alpha beta", 2, "bold underlined reversed");
    s.assert_style_in("alpha beta", 3, "!reversed");
    s.keys("left");
    s.assert_style_in("alpha beta", 1, "bold underlined reversed");
    s.assert_style_in("alpha beta", 2, "!reversed");
}

#[test]
fn esc_clears_the_mark_and_the_footer_follows_in_the_view_alone() {
    let mut s = view();
    s.keys("leader [");
    s.keys("g v right");
    s.assert_contains("y/⌃c copy");
    s.keys("esc");
    s.assert_contains("arrows/hjkl move");
    s.assert_absent("y/⌃c copy");
    s.assert_style_in("alpha beta", 0, "!reversed");
    s.assert_style_in("alpha beta", 1, "bold underlined reversed");
    s.keys("esc");
    s.assert_absent("arrows/hjkl move");
}
