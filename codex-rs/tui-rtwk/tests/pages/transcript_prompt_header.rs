// test root: pages
//! The normal owned transcript keeps the question of the visible answer in reach.
use serde_json::{Value, json};

use crate::kit::{self, Owner, Screen};

const QUESTION: &str = "Question whose answer fills the reader";
const EARLIER: &str = "Earlier question in this conversation";

fn turn(id: &str, question: &str, answer: &str, paragraphs: usize) -> Value {
    let text = (0..paragraphs)
        .map(|number| format!("{answer} paragraph {number:02}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    kit::replay_turn(
        id,
        json!([
            {"type": "userMessage", "id": format!("{id}-prompt"), "content": [
                {"type": "text", "text": question, "text_elements": []}
            ]},
            {"type": "agentMessage", "id": format!("{id}-answer"), "text": text, "phase": "final_answer"}
        ]),
    )
}

fn reader(turns: Vec<Value>) -> Screen {
    Screen::build(Owner::ChatLive)
        .size(68, 16)
        .owned_transcript()
        .replay(turns)
        .open()
}

#[test]
fn a_prompt_above_the_owned_tail_stays_in_reach_and_clicks_to_its_turn() {
    let mut s = reader(vec![turn("recent", QUESTION, "response", 35)]);
    s.wait_for("response paragraph 34");
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    let (frames, _, _) = s.ambient_frame_counts(240);
    assert!(frames <= 2, "{frames} idle frames");
    s.type_text("leave this draft alone");
    s.wheel_up(3, 3, 1);
    s.assert_contains("Back to bottom");
    s.assert_text_at(2, 0, QUESTION);
    s.click_text(QUESTION);
    s.wait(300);
    s.assert_contains("response paragraph 00");
    s.assert_contains(QUESTION);
    s.assert_text_count(QUESTION, 1);
    s.assert_composer("leave this draft alone");
    s.assert_no_turn();
}

#[test]
fn only_a_plain_primary_press_jumps_to_the_prompt() {
    let mut s = reader(vec![turn("recent", QUESTION, "response", 35)]);
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    s.shift_click(4, 0);
    s.assert_text_at(2, 0, QUESTION);
    s.assert_contains("response paragraph 34");
    s.ctrl_click(4, 0);
    s.assert_text_at(2, 0, QUESTION);
    s.assert_contains("response paragraph 34");
    s.click(4, 0);
    s.wait(300);
    s.assert_contains("response paragraph 00");
    s.assert_no_turn();
}

#[test]
fn a_held_range_and_copy_preserve_the_pinned_question() {
    let mut s = reader(vec![turn("recent", QUESTION, "response", 35)]);
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    s.selection_copy_policy(false, false);
    s.double_click_text("34");
    s.assert_text_style("34", "reversed");
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    s.keys("ctrl+c");
    s.assert_copies_asked(&["34"]);
    s.assert_text_at(2, 0, QUESTION);
}

/// The oracle takes a plain press of the header whatever the reader holds: it ends the range,
/// cancels Find and clears the fold's focus, then jumps (`handle_mouse_with_mode`).
#[test]
fn a_plain_press_closes_find_and_jumps_to_the_prompt() {
    let mut s = reader(vec![turn("recent", QUESTION, "response", 35)]);
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    s.keys("f3");
    s.assert_contains("Find:");
    s.assert_contains("response paragraph 34");
    s.click(4, 0);
    s.wait(300);
    s.assert_absent("Find:");
    s.assert_contains("response paragraph 00");
    s.assert_text_count(QUESTION, 1);
    s.assert_no_turn();
}

#[test]
fn a_plain_press_ends_a_held_range_and_jumps_to_the_prompt() {
    let mut s = reader(vec![turn("recent", QUESTION, "response", 35)]);
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    s.selection_copy_policy(false, false);
    s.double_click_text("34");
    s.assert_text_style("34", "reversed");
    s.wait(300);
    s.click(4, 0);
    s.wait(300);
    s.assert_contains("response paragraph 00");
    s.assert_absent("response paragraph 34");
    s.assert_text_count(QUESTION, 1);
    s.assert_no_turn();
    // nothing of the range is left to copy
    s.keys("ctrl+c");
    s.assert_copies_asked(&[]);
}

#[test]
fn a_plain_press_leaves_copy_mode_and_jumps_to_the_prompt() {
    let mut s = reader(vec![turn("recent", QUESTION, "response", 35)]);
    s.wait(300);
    s.type_text("/copy").keys("enter");
    s.assert_contains("enter copy");
    s.assert_text_at(2, 0, QUESTION);
    s.click(4, 0);
    s.wait(300);
    s.assert_absent("enter copy");
    s.assert_contains("response paragraph 00");
    s.assert_no_turn();
    // the transcript takes the keys of the chat again: /copy begins the mode once more
    s.type_text("/copy").keys("enter");
    s.assert_contains("enter copy");
}

#[test]
fn the_header_follows_the_visible_turn_across_scroll_and_resize() {
    let mut s = reader(vec![
        turn("recent", QUESTION, "response", 35),
        turn("earlier", EARLIER, "earlier answer", 35),
    ]);
    s.wait(300);
    s.assert_contains(QUESTION);
    s.assert_absent(EARLIER);
    for _ in 0..12 {
        if s.rows().first().is_some_and(|row| row.contains(EARLIER)) {
            break;
        }
        s.keys("pageup");
    }
    s.assert_text_at(2, 0, EARLIER);
    s.assert_text_count(EARLIER, 1);
    s.resize(51, 17);
    s.wait(300);
    s.assert_contains(EARLIER);
    s.click_text(EARLIER);
    s.wait(300);
    s.assert_contains("earlier answer paragraph 00");
    s.assert_text_count(EARLIER, 1);
    s.assert_no_turn();
}

#[test]
fn the_owned_reader_does_not_pin_a_visible_prompt_or_take_the_short_viewport() {
    let s = reader(vec![turn("short", QUESTION, "short answer", 1)]);
    s.assert_contains("short answer paragraph 00");
    s.assert_text_count(QUESTION, 1);

    let mut s = reader(vec![turn("recent", QUESTION, "response", 35)]);
    s.wait(300);
    s.assert_contains(QUESTION);
    s.resize(68, 6);
    s.assert_absent(QUESTION);
    s.resize(68, 16);
    s.wait_for("response paragraph 34");
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
}

#[test]
fn a_page_inserted_before_the_header_does_not_change_the_clicked_prompt() {
    let mut s = reader(Vec::new());
    s.history_page(
        vec![turn("recent", QUESTION, "response", 35)],
        Some("older"),
    );
    s.wait(300);
    s.assert_contains(QUESTION);
    s.keys("ctrl+home");
    s.assert_page_requests(2);
    s.keys("esc");
    s.wait_for("response paragraph 34");
    s.history_page(vec![turn("earlier", EARLIER, "earlier answer", 14)], None);
    s.wait(300);
    s.assert_contains(EARLIER);
    s.click_text("↓ Back to bottom");
    s.wait_for("response paragraph 34");
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    s.click_text(QUESTION);
    s.wait(300);
    s.assert_contains("response paragraph 00");
    s.assert_contains(QUESTION);
    s.assert_absent("earlier answer paragraph 00");
    s.assert_no_turn();
}

#[test]
fn a_pinned_prompt_does_not_keep_a_selection_after_switching_presentations() {
    let mut turn = turn("recent", QUESTION, "response", 35);
    let Some(items) = turn["items"].as_array_mut() else {
        panic!("expected replay items");
    };
    items.push(
        json!({"type":"reasoning","id":"reasoning","summary":["quiet thought"],"content":[]}),
    );
    let mut s = reader(vec![turn]);
    s.wait(300);
    s.assert_text_at(2, 0, QUESTION);
    s.assert_absent("quiet thought");
    s.selection_copy_policy(false, false);
    s.double_click_text("34");
    s.assert_text_style("34", "reversed");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["34"]);
    s.answer_copy_pending(1);
    s.keys("ctrl+t");
    s.assert_contains("quiet thought");
    s.keys("ctrl+t");
    s.assert_absent("quiet thought");
    s.assert_text_style("34", "!reversed");

    s.wait(350);
    s.double_click_text("34");
    s.answer_copy(Ok(()));
    s.assert_text_style("34", "reversed");
}
