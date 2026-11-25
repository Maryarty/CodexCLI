// test root: pages
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Older pages of a thread's history (S10; oracle `transcript_view/footer.rs`
//! `status_line_with_navigation`, `app_backtrack.rs`): the app asks the server for the page
//! before the first one when the reader reaches the top, a loading row says so while it is on its
//! way, the page joins above without moving what the reader looks at, and a refused page says how
//! to try again.
//!
//! The thread is a replayed one (`Open::replay`); the server answers each page request when the
//! test says so (`history_page`, `history_failed`).

use serde_json::{Value, json};

use crate::kit::{Owner, Screen, replay_turn};

/// One finished turn: a prompt and its one-line answer.
fn turn(id: usize) -> Value {
    replay_turn(
        &format!("turn-{id}"),
        json!([
            {"type": "userMessage", "id": format!("user-{id}"), "clientId": null,
             "content": [{"type": "text", "text": format!("prompt {id:02}"), "text_elements": []}]},
            {"type": "agentMessage", "id": format!("agent-{id}"), "text": format!("answer {id:02}"),
             "phase": "final_answer", "memoryCitation": null, "moderation": null,
             "delivery": null, "questions": null}
        ]),
    )
}

/// A chat of the newest `n` turns (the server lists them newest first) whose server has older
/// ones behind a cursor, `width` columns wide, a `draft` typed so that the footer is the
/// transcript's alone. The opened thread asks for its first page; the server answers it.
fn partial(width: u16, n: usize) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, 30)
        .replay(Vec::new())
        .open();
    s.wait_for("Ask Codex to do anything");
    assert_eq!(s.pages_waiting(), 1, "{}", s.text());
    s.history_page((1..=n).rev().map(turn).collect(), Some("cursor-1"));
    s.type_text("draft");
    s
}

/// The reader goes up to the top row: the wheel up as far as the rows go.
fn to_the_top(s: &mut Screen) {
    for _ in 0..14 {
        s.wheel_up(10, 10, 5);
    }
}

/// The row of the screen that holds `text`.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains(text))
        .unwrap_or_else(|| panic!("no row with {text:?}:\n{}", s.text()))
}

#[test]
fn a_thread_opens_with_one_page_request() {
    let s = partial(80, 3);
    assert_eq!(s.page_requests(), vec![None]);
    assert_eq!(s.pages_waiting(), 0);
}

#[test]
fn older_messages_are_named_once_the_reader_is_away() {
    let mut s = partial(80, 12);
    // at the last row there is nothing to say of the history
    assert!(
        !s.text().contains("Earlier messages available"),
        "{}",
        s.text()
    );
    s.wheel_up(10, 10, 5);
    s.assert_contains("Earlier messages available.  esc latest");
    assert_eq!(s.page_requests(), vec![None]);
}

#[test]
fn reaching_the_top_asks_for_the_older_page_and_says_so() {
    let mut s = partial(80, 12);
    to_the_top(&mut s);
    assert_eq!(s.page_requests(), vec![None, Some("cursor-1".to_owned())]);
    s.assert_contains("↑ Loading earlier messages… · esc latest");
    // the wheel at the top asks once, not once per tick
    s.wheel_up(10, 10, 5);
    assert_eq!(s.page_requests().len(), 2);
}

#[test]
fn a_narrow_window_shortens_the_loading_row_before_it_clips_a_key() {
    let mut s = partial(40, 12);
    to_the_top(&mut s);
    s.assert_contains("↑ Loading… · esc latest");
    assert!(!s.text().contains("Loading earlier"), "{}", s.text());
}

#[test]
fn a_refused_page_says_how_to_ask_again() {
    let mut s = partial(80, 12);
    to_the_top(&mut s);
    s.history_failed("the server is gone");
    s.assert_contains("Retry history: ⌥</⌃home.  esc latest");
    assert!(!s.text().contains("Loading earlier"), "{}", s.text());
}

#[test]
fn a_narrow_window_keeps_the_retry_of_a_refused_page() {
    let mut s = partial(28, 12);
    to_the_top(&mut s);
    s.history_failed("the server is gone");
    s.assert_contains("Retry: ⌥</⌃home");
}

#[test]
fn a_refused_page_is_asked_again_at_the_top() {
    let mut s = partial(80, 12);
    to_the_top(&mut s);
    s.history_failed("the server is gone");
    s.wheel_up(10, 10, 5);
    assert_eq!(
        s.page_requests(),
        vec![
            None,
            Some("cursor-1".to_owned()),
            Some("cursor-1".to_owned())
        ]
    );
    s.assert_contains("↑ Loading earlier messages… · esc latest");
}

#[test]
fn the_page_joins_above_without_moving_what_the_reader_sees() {
    let mut s = partial(80, 12);
    to_the_top(&mut s);
    let before = s.shot();
    s.history_page(vec![turn(0)], None);
    // the rows are where they were, and the older ones are above them
    assert!(!s.text().contains("prompt 00"), "{}", s.text());
    assert_eq!(row_of(&s, "prompt 01"), 1, "{}", s.text());
    s.assert_same_rows_in(&before, 0..24);
    s.wheel_up(10, 10, 10);
    s.assert_contains("prompt 00");
    // nothing is left to load: no more asks, and the status row has nothing of it to say
    assert_eq!(s.page_requests().len(), 2);
    assert!(!s.text().contains("Loading earlier"), "{}", s.text());
    assert!(
        !s.text().contains("Earlier messages available"),
        "{}",
        s.text()
    );
}

#[test]
fn the_page_is_not_news_to_a_reader_who_is_away() {
    let mut s = partial(80, 12);
    to_the_top(&mut s);
    s.history_page(vec![turn(0)], None);
    assert!(!s.text().contains("New activity"), "{}", s.text());
    s.assert_contains("esc latest");
}

#[test]
fn a_page_joining_above_keeps_a_selection_held_elsewhere() {
    // the transcript alone, as the oracle's page-join tests read it: a row selected by three
    // clicks, the reader at the top of the loaded rows, a page joining above
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 8)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["one", "two", "three", "four"]);
    s.wheel_up(0, 0, 100);
    s.click_text("two");
    s.click_text("two");
    s.click_text("two");
    s.assert_text_style("two", "reversed");
    let before = s.shot();
    s.join_older_page(&["older group", "older output"], false);
    s.assert_same_rows(&before);
    s.assert_text_style("two", "reversed");
    // the page is above: the reader goes up to it and the selection stays where it was
    s.wheel_up(0, 0, 100);
    s.assert_contains("older group");
    s.wheel_down(0, 0, 100);
    s.assert_contains("four");
}
