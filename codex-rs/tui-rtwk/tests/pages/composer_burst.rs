//! The composer's paste detector on the live chat screen: a terminal without
//! bracketed paste sends a paste as a stream of keys, and the keys that come
//! within 8 ms of each other are one paste, not typing (an Enter must not
//! submit, a `?` must not open the shortcuts). The oracle holds this in
//! state tests of `paste_burst.rs` and `chat_composer.rs`; the core unit
//! tests of `paste_burst.rs` hold the machine, these the page's wiring
//! (the keys' door, the app timer, the page's `?` and `Left`).

#[path = "../common/mod.rs"]
mod common;
#[path = "../kit/mod.rs"]
mod kit;

use kit::{Owner, Screen};

/// A live chat screen, the composer empty.
fn chat() -> Screen {
    let mut s = Screen::build(Owner::ChatLive).size(100, 24).live().open();
    s.wait_for("› Ask Codex to do anything");
    s
}

/// The composer's prompt row (`›`), the last one.
fn prompt(s: &Screen) -> usize {
    s.rows()
        .iter()
        .rposition(|r| r.starts_with('›'))
        .unwrap_or(usize::MAX)
}

#[test]
fn an_enter_inside_a_burst_is_a_newline_of_the_paste() {
    let mut s = chat();
    s.type_burst("one\ntwo");
    s.wait(50);
    let p = prompt(&s);
    // one paste of two lines: the Enter did not submit what came before it
    s.assert_row(p, "› one");
    s.assert_row(p + 1, "  two");
}

#[test]
fn a_question_mark_inside_a_burst_is_text_not_the_shortcuts() {
    let mut s = chat();
    s.type_burst("hi?there");
    s.wait(50);
    s.assert_row(prompt(&s), "› hi?there");
    // no overlay: the footer under the prompt is what it is with any draft
    let mut typed = chat();
    typed.type_text("hi?there");
    typed.assert_same_rows(&s.shot());
}

#[test]
fn a_question_mark_typed_alone_still_opens_the_shortcuts() {
    let mut s = chat();
    let before = s.shot();
    s.keys("?");
    s.assert_rows_differ(&before);
}

#[test]
fn left_behind_a_held_char_moves_the_caret_not_the_page() {
    let mut s = chat();
    // `h` is held for 9 ms in case a burst follows; Left right behind it
    // flushes it and moves in the text (`is_in_paste_burst` waits the page)
    s.type_burst("h");
    s.keys("left");
    s.type_text("X");
    s.assert_row(prompt(&s), "› Xh");
}

#[test]
fn a_char_nothing_follows_goes_in_as_typing_after_nine_milliseconds() {
    let mut s = chat();
    s.type_burst("x");
    s.wait(50);
    s.assert_row(prompt(&s), "› x");
}

#[test]
fn a_space_inside_a_burst_is_kept() {
    let mut s = chat();
    s.type_burst("a b");
    s.wait(50);
    s.assert_row(prompt(&s), "› a b");
}

#[test]
fn typed_at_a_persons_pace_a_space_is_a_space() {
    let mut s = chat();
    s.type_text("a b");
    s.assert_row(prompt(&s), "› a b");
}

#[test]
fn keys_of_one_call_are_typing_and_the_enter_submits() {
    let mut s = chat();
    s.keys("h i enter");
    // the message went to the thread: the composer is empty again
    s.assert_row(prompt(&s), "› Ask Codex to do anything");
}

#[test]
fn a_large_burst_goes_in_as_one_placeholder_once_it_stops() {
    let mut s = chat();
    s.type_burst(&"z".repeat(1005));
    s.wait(50);
    s.assert_row(prompt(&s), "› [Pasted Content 1005 chars]");
}
