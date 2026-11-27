// test root: pages
//! Input boundaries for chat transcript reading and browsing.

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::{
    agent_completed, turn_completed, turn_started, user_item,
};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use crate::common::{Harness, start_driven};

fn paused_reader() -> Harness {
    let mut h = start_driven(false, 80, 16);
    h.wait_idle();
    for (turn, prompt) in [("first", "first prompt"), ("second", "second prompt")] {
        let answer = (0..30)
            .map(|line| format!("{turn} reply line {line:02}"))
            .collect::<Vec<_>>()
            .join("\n");
        h.inject([
            AppEv::Backend(turn_started(FAKE_THREAD, turn)),
            AppEv::Backend(user_item(FAKE_THREAD, turn, "prompt", prompt)),
            AppEv::Backend(agent_completed(FAKE_THREAD, turn, "answer", &answer)),
            AppEv::Backend(turn_completed(FAKE_THREAD, turn)),
        ]);
    }
    h.press("pageup");
    let reading = h.t.screen();
    assert!(reading.contains("Back to bottom"), "{reading}");
    assert!(!reading.contains("second reply line 29"), "{reading}");
    h
}

#[test]
fn two_escapes_in_one_input_preserve_the_paused_browsing_origin() {
    let mut h = paused_reader();
    let reading = h.t.screen();

    // Two kitty Escape press reports arrive in one read of the terminal.
    h.t.input(b"\x1b[27u\x1b[27u");
    h.advance(150);
    h.press("left");
    assert!(h.t.screen().contains("first prompt"));
    h.t.resize(40, 12);
    h.advance(150);
    h.press("esc");
    h.t.resize(80, 16);
    h.advance(150);
    let after = h.t.screen();
    assert_eq!(
        after.lines().take(8).collect::<Vec<_>>(),
        reading.lines().take(8).collect::<Vec<_>>(),
        "{after}",
    );
}

#[test]
fn pasting_a_draft_restores_the_paused_browsing_origin() {
    let mut h = paused_reader();
    let reading = h.t.screen();
    h.press("esc esc left");
    assert!(h.t.screen().contains("first prompt"));

    h.t.input(b"\x1b[200~pasted draft\x1b[201~");
    h.advance(150);

    let after = h.t.screen();
    assert!(after.contains("› pasted draft"), "{after}");
    assert_eq!(
        after.lines().take(8).collect::<Vec<_>>(),
        reading.lines().take(8).collect::<Vec<_>>(),
        "{after}",
    );
}
