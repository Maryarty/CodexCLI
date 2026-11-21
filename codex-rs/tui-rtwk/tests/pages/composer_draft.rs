//! The composer's draft against the thread: shell mode and the history walk
//! end where the composer clears (the thread took the submit, a command took
//! the line, Ctrl+C), not where Enter was pressed, and a paste at the start
//! that opens with `!` is shell mode (`insert_str_at_target`). The oracle's
//! composer clears on its own submit; here the thread decides, so the draft
//! of a submit it did not take has to stay as it was.

use crate::common;

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_completed};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use common::{Harness, start};

/// The composer's prompt row (`›` or the shell's `!`), the last one.
fn prompt(h: &Harness) -> String {
    h.t.screen()
        .lines()
        .rfind(|row| row.starts_with('›') || row.starts_with('!'))
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

/// A chat whose session is up and idle, the composer empty.
fn chat() -> Harness {
    let mut h = start();
    h.wait_idle();
    h
}

#[test]
fn a_shell_draft_the_thread_did_not_take_stays_a_shell_draft() {
    let mut h = chat();
    // the connection is lost: the thread has no writer, a submit is no-op
    h.inject([AppEv::Disconnected]);
    h.t.type_text("!ls");
    h.advance(100);
    assert_eq!(prompt(&h), "! ls");
    h.press("enter");
    assert_eq!(prompt(&h), "! ls", "the draft is still the shell line");
    // and it stays one while it is edited on
    h.t.type_text("x");
    h.advance(100);
    assert_eq!(prompt(&h), "! lsx");
}

#[test]
fn a_shell_draft_the_thread_took_clears_and_leaves_shell_mode() {
    let mut h = chat();
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, "turn-1", "msg-1", "Done"),
        agent_completed(FAKE_THREAD, "turn-1", "msg-1", "Done"),
        turn_completed(FAKE_THREAD, "turn-1"),
    ]);
    h.t.type_text("!ls");
    h.press("enter");
    h.wait_for("the answer", |s| s.contains("• Done"));
    h.wait_idle();
    // the prompt is the plain one again: the next text is not a shell line
    h.t.type_text("x");
    h.advance(100);
    assert_eq!(prompt(&h), "› x");
}

/// Sends `text` and waits for the scripted answer `turn`.
fn ask(h: &mut Harness, text: &str, turn: &str) {
    let (msg, answer) = (format!("msg-{turn}"), format!("answer {turn}"));
    h.fake.script_turn(vec![
        agent_delta(FAKE_THREAD, turn, &msg, &answer),
        agent_completed(FAKE_THREAD, turn, &msg, &answer),
        turn_completed(FAKE_THREAD, turn),
    ]);
    h.t.type_text(text);
    h.press("enter");
    h.wait_for("the answer", |s| s.contains(&format!("• {answer}")));
    h.wait_idle();
}

#[test]
fn the_history_walk_goes_on_over_a_submit_the_thread_did_not_take() {
    let mut h = chat();
    ask(&mut h, "one", "turn-1");
    ask(&mut h, "two", "turn-2");
    h.press("up");
    assert_eq!(prompt(&h), "› two");
    h.inject([AppEv::Disconnected]);
    h.press("enter");
    assert_eq!(prompt(&h), "› two", "nothing was taken");
    // still walking: Up goes on to the entry before the one recalled
    h.press("up");
    assert_eq!(prompt(&h), "› one");
}

#[test]
fn a_pasted_bang_at_the_start_is_shell_mode() {
    let mut h = chat();
    h.t.paste("!ls -la");
    h.advance(100);
    assert_eq!(prompt(&h), "! ls -la");
}

#[test]
fn a_pasted_bang_after_text_stays_text() {
    let mut h = chat();
    h.t.type_text("echo ");
    h.advance(100);
    h.t.paste("!ls");
    h.advance(100);
    assert_eq!(prompt(&h), "› echo !ls");
}
