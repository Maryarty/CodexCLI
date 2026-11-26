// test root: pages
//! The answer field of the async-question editor is the plain-text composer: the drafts it keeps
//! while the editor is closed, vim and its label, the countdown that opening the editor stops,
//! and what a message of the user's own does to the questions. The oracle's tests of the
//! editor are `tests/oracle/{approvals_1,approvals_2,chat_approvals_2}.rs`.

use crate::kit::{Owner, Screen};

const ONE: &str = include_str!("../oracle/fixtures/chat_approvals_2/q_plain.jsonl");
const NEXT: &str = include_str!("../oracle/fixtures/chat_approvals_2/q_next.jsonl");

fn open(script: &'static str) -> Screen {
    Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(script)
        .open()
}

fn clock(millis: i64) -> String {
    format!("{{\"step\":\"clock\",\"millis\":{millis}}}\n")
}

#[test]
fn a_draft_is_kept_while_the_editor_is_closed() {
    let mut s = open(ONE);
    s.keys("alt+up");
    s.paste("half an answer");
    s.keys("alt+down");
    s.assert_absent("1 of 1");
    s.assert_contains("? 1 question");
    s.keys("alt+up");
    s.assert_contains("half an answer");
}

#[test]
fn vim_in_the_answer_shows_its_mode_in_the_footer_row() {
    let mut s = open(ONE);
    s.type_text("/vim");
    s.keys("enter");
    s.keys("alt+up");
    s.keys("i");
    s.assert_contains("Vim: Insert");
    s.paste("word");
    s.keys("esc");
    s.assert_contains("Vim: Normal");
    s.assert_absent("Vim: Insert");
    // vim keys edit the answer: `x` deletes the character under the caret
    s.keys("x");
    s.assert_contains("wor");
    s.assert_absent("word");
}

#[test]
fn the_editor_open_stops_the_countdown_of_the_summary() {
    let mut s = open(ONE);
    let base = 1_000_000_000_000_i64;
    s.feed(&clock(base));
    s.feed(NEXT);
    s.feed(&clock(base + 15_000));
    s.wait(100);
    s.assert_contains("2 questions · 15s");
    s.keys("alt+up");
    s.keys("alt+down");
    s.feed(&clock(base + 16_000));
    s.wait(100);
    s.assert_contains("2 questions");
    s.assert_absent("questions ·");
}

#[test]
fn a_message_of_the_users_own_drops_the_questions() {
    let mut s = open(ONE);
    s.assert_contains("? 1 question");
    s.type_text("hello");
    s.keys("enter");
    s.wait(200);
    s.assert_absent("? 1 question");
    s.assert_absent("to answer");
}

#[test]
fn the_inline_history_field_returns_focus_to_the_open_question() {
    let mut s = open(ONE);
    s.keys("alt+up");
    s.type_text("answer draft");
    for suffix in ["!", "?"] {
        s.keys("ctrl+r");
        s.type_text("ne");
        s.paste("edle");
        s.assert_contains("reverse-i-search: needle");
        s.assert_cursor_on_row_of("reverse-i-search: needle");
        s.keys("esc");
        s.assert_absent("reverse-i-search");
        s.assert_cursor_on_row_of("answer draft");
        s.type_text(suffix);
        s.keys("alt+down");
        s.assert_absent("answer draft");
        s.keys("alt+up");
    }
    s.assert_contains("answer draft!?");
}
