// test root: pages
#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The focus on the folds of the turn details (oracle `transcript_view/disclosure.rs`
//! `handle_disclosure_key`, F4): the keys of a list walk the disclosure rows, Enter opens or closes
//! the one in focus, any other key lets go and goes on to the composer. The chat is two resumed
//! turns, each folded behind its prompt (`2 previous messages`, `3 previous messages`).

use crate::kit::{Owner, Screen, replay_turn};

fn user(id: &str, text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "userMessage", "id": id, "clientId": null,
        "content": [{"type": "text", "text": text, "text_elements": []}],
        "startedAtMs": null, "completedAtMs": null
    })
}

fn message(id: &str, text: &str, phase: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "agentMessage", "moderation": null, "id": id, "text": text, "phase": phase,
        "memoryCitation": null, "delivery": null, "questions": null,
        "startedAtMs": null, "completedAtMs": null
    })
}

fn turn(id: &str, items: serde_json::Value) -> serde_json::Value {
    let mut turn = replay_turn(id, items);
    turn["startedAt"] = serde_json::Value::Null;
    turn["completedAt"] = serde_json::Value::Null;
    turn["durationMs"] = serde_json::Value::Null;
    turn
}

const FIRST: &str = "▸ 2 previous messages";
const SECOND: &str = "▸ 3 previous messages";
const HINTS: &str = "↑ previous · ↓ next · enter expand · esc back";

fn chat(width: u16) -> Screen {
    // oldest first here; the server lists the turns of a thread newest first
    let turns = vec![
        turn(
            "turn-1",
            serde_json::json!([
                user("user-1", "First question"),
                message("msg-1", "Thinking one", "commentary"),
                message("msg-2", "Still one", "commentary"),
                message("msg-3", "Answer one", "final_answer"),
            ]),
        ),
        turn(
            "turn-2",
            serde_json::json!([
                user("user-2", "Second question"),
                message("msg-4", "Thinking two", "commentary"),
                message("msg-5", "Reading two", "commentary"),
                message("msg-6", "Writing two", "commentary"),
                message("msg-7", "Answer two", "final_answer"),
            ]),
        ),
    ];
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, 30)
        .replay(turns.into_iter().rev().collect())
        .live()
        .open();
    s.collapse_turn_details();
    s.wait(50);
    s
}

#[test]
fn f4_focuses_the_last_fold_and_the_footer_says_its_keys() {
    let mut s = chat(80);
    s.assert_contains(FIRST);
    s.assert_contains(SECOND);
    s.assert_absent(HINTS);
    s.keys("f4");
    s.wait(50);
    s.assert_contains(HINTS);
    s.assert_text_style(SECOND, "bold");
    s.assert_text_style(FIRST, "!bold");
}

#[test]
fn the_footer_drops_the_walking_keys_where_the_width_asks() {
    let mut s = chat(34);
    s.keys("f4");
    s.wait(50);
    s.assert_contains("enter expand · esc back");
    s.assert_absent("↑ previous");
}

#[test]
fn up_and_down_walk_the_folds_and_the_ends_stop() {
    let mut s = chat(80);
    s.keys("f4");
    s.keys("up");
    s.assert_text_style(FIRST, "bold");
    s.assert_text_style(SECOND, "!bold");
    s.keys("up");
    s.assert_text_style(FIRST, "bold");
    s.keys("down");
    s.assert_text_style(SECOND, "bold");
    s.keys("down");
    s.assert_text_style(SECOND, "bold");
    s.keys("home");
    s.assert_text_style(FIRST, "bold");
    s.keys("end");
    s.assert_text_style(SECOND, "bold");
    s.keys("pageup");
    s.assert_text_style(FIRST, "bold");
    s.keys("pagedown");
    s.assert_text_style(SECOND, "bold");
}

#[test]
fn enter_opens_the_fold_in_focus_and_the_footer_offers_to_close_it() {
    let mut s = chat(80);
    s.keys("f4");
    s.keys("up");
    s.assert_absent("Thinking one");
    s.keys("enter");
    s.assert_contains("▾ 2 previous messages");
    s.assert_contains("Thinking one");
    s.assert_contains("Still one");
    s.assert_absent("Thinking two");
    s.assert_contains("enter collapse");
    s.assert_absent("enter expand");
    s.keys("enter");
    s.assert_absent("Thinking one");
    s.assert_contains("enter expand");
}

#[test]
fn right_opens_and_left_closes_and_neither_toggles_twice() {
    let mut s = chat(80);
    s.keys("f4");
    s.keys("left");
    s.assert_absent("Thinking two");
    s.keys("right");
    s.assert_contains("Thinking two");
    s.keys("right");
    s.assert_contains("Thinking two");
    s.keys("left");
    s.assert_absent("Thinking two");
}

#[test]
fn esc_and_the_key_again_give_the_keys_back() {
    let mut s = chat(80);
    s.keys("f4");
    s.assert_contains(HINTS);
    s.keys("esc");
    s.assert_absent(HINTS);
    s.assert_text_style(SECOND, "!bold");
    s.keys("f4");
    s.assert_contains(HINTS);
    s.keys("f4");
    s.assert_absent(HINTS);
}

#[test]
fn any_other_key_lets_go_and_goes_on_to_the_composer() {
    let mut s = chat(80);
    s.keys("f4");
    s.type_text("hello");
    s.wait(50);
    s.assert_absent(HINTS);
    s.assert_contains("hello");
}

#[test]
fn a_tap_on_a_fold_opens_it_and_focuses_it() {
    let mut s = chat(80);
    s.click_text(FIRST);
    s.wait(50);
    s.assert_contains("▾ 2 previous messages");
    s.assert_contains("Thinking one");
    s.assert_contains("enter collapse");
    s.assert_text_style("▾ 2 previous messages", "bold");
    s.keys("esc");
    s.assert_absent("enter collapse");
}

#[test]
fn without_folds_the_key_does_nothing() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 30).live().open();
    s.keys("f4");
    s.wait(50);
    s.assert_absent("esc back");
}
