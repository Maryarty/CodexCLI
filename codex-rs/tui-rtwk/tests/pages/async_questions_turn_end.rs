// test root: pages
//! Questions belong to the turn that asked them, even when several events reach one frame.

use crate::kit::{self, Owner, Screen};

const QUESTION: &str = include_str!("../oracle/fixtures/chat_approvals_2/q_turn_end_one.jsonl");
const PARTIAL: &str =
    include_str!("../oracle/fixtures/chat_approvals_2/q_turn_end_in_progress.jsonl");
const COALESCED: &str =
    include_str!("../oracle/fixtures/chat_approvals_2/q_turn_end_coalesced.jsonl");
const SECOND_DONE: &str =
    include_str!("../oracle/fixtures/chat_approvals_2/q_turn_end_second.jsonl");
const DONE: &str = include_str!("../oracle/fixtures/chat_approvals_2/q_two_done.jsonl");
const INTERRUPTED: &str =
    include_str!("../oracle/fixtures/chat_approvals_2/q_turn_end_interrupted.jsonl");

#[test]
fn escape_from_an_open_question_keeps_the_running_turn_and_the_draft() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(QUESTION)
        .open();
    s.keys("alt+up").paste("unsent answer").keys("esc");
    s.assert_contains("? 1 question");
    s.assert_contains("Working");
    s.assert_absent("unsent answer");
    s.assert_interrupts(0);
    s.keys("alt+up");
    s.assert_contains("unsent answer");
}

#[test]
fn a_remapped_interrupt_is_still_available_while_answering() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(QUESTION)
        .keymap(&[("chat.interrupt_turn", "f12"), ("list.cancel", "ctrl+g")])
        .open();
    s.keys("alt+up f12");
    s.assert_interrupts(1);
    s.assert_contains("Type your answer");
    s.keys("esc");
    s.assert_contains("? 1 question");
    s.assert_interrupts(1);
}

#[test]
fn a_live_end_recovers_only_its_question_when_next_turn_arrives_before_the_frame() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(QUESTION)
        .open();
    s.keys("alt+up").paste("first answer").keys("alt+down");
    s.feed(PARTIAL);
    s.assert_contains("? 1 question");
    s.assert_absent("first answer");

    // The end, next turn's question, and two late old items all arrive before rendering.
    s.feed(COALESCED);
    s.assert_composer("> Which way?\n\nfirst answer");
    s.assert_contains("? 1 question");
    s.keys("alt+up").paste("second answer");
    s.assert_contains("A new question?");
    s.feed(SECOND_DONE);
    s.assert_saved_draft("> Which way?\n\nfirst answer\n\n> A new question?\n\nsecond answer");
    s.assert_absent("to answer");
}

#[test]
fn both_live_ends_survive_one_frame() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(QUESTION)
        .open();
    s.keys("alt+up").paste("first answer").keys("alt+down");
    s.feed(include_str!(
        "../oracle/fixtures/chat_approvals_2/q_turn_end_two.jsonl"
    ));
    s.assert_composer("> Which way?\n\nfirst answer");
    s.assert_absent("to answer");
}

#[test]
fn an_actual_live_end_recovers_the_draft_after_history_reconciled_the_turn() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .replay(Vec::new())
        .open();
    let setup = QUESTION.replace("thread-1", kit::REPLAYED);
    s.feed(&setup);
    s.keys("alt+up").paste("still waiting").keys("alt+down");
    s.history_page(
        vec![kit::replay_turn("turn-1", serde_json::json!([]))],
        None,
    );
    s.assert_contains("? 1 question");
    s.assert_absent("still waiting");
    s.keys("alt+up");
    s.assert_contains("still waiting");
    s.feed(&DONE.replace("thread-1", kit::REPLAYED));
    s.assert_composer("> Which way?\n\nstill waiting");
}

#[test]
fn background_interrupt_restores_its_composer_once_on_return() {
    for desktop_answer in [false, true] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .script(QUESTION)
            .draft("first session")
            .open();
        s.restore_writer();
        s.keys("alt+up").paste("saved answer");
        s.visit_thread("thread-other");
        s.draft("second session");
        s.feed(INTERRUPTED);
        if desktop_answer {
            s.feed(include_str!(
                "../oracle/fixtures/chat_approvals_2/q_turn_end_desktop_reply.jsonl"
            ));
        }
        s.assert_composer("second session");
        let expected = if desktop_answer {
            "pending steer\nqueued prompt\nfirst session"
        } else {
            "pending steer\nqueued prompt\nfirst session\n\n> Which way?\n\nsaved answer"
        };
        s.visit_thread("thread-1");
        s.assert_saved_draft(expected);
        s.visit_thread("thread-other");
        s.assert_composer("second session");
        s.visit_thread("thread-1");
        s.assert_saved_draft(expected);
        s.assert_no_turn();
    }
}
