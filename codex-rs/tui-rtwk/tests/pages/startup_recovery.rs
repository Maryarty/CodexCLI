// test root: pages
//! What a launch that failed leaves for the user (oracle `startup_recovery.rs`, its tests in
//! `startup_recovery_tests.rs`): the app ends on `StartFailed` and the text the user wrote and
//! the session never took goes into the slot the host prints from, once the terminal is
//! restored. The page side of it: the composer's draft with every paste placeholder expanded,
//! the keys the paste detector still holds, a queued message in full. The core's rules (the
//! queue, the escaping, nothing after the first thread) are in `core/tests/startup_recovery.rs`;
//! the oracle's stage machine is not ported (the queue's drain is the acknowledgement).

// a test of the page: the harness has the connection by now
#![allow(clippy::expect_used)]

use crate::common::{Harness, ms, start_driven_as};
use codex_tui_core::app::AppEv;
use codex_tui_rtwk::app::UnsentSlot;
use rtwk_test::Options;

/// The app up and connected, its first thread not yet answered, with the slot the host gives it.
fn starting() -> (Harness, UnsentSlot) {
    let slot = UnsentSlot::default();
    let mut h = start_driven_as(false, Options::new().size(100, 24), {
        let slot = slot.clone();
        move |app| app.with_unsent_slot(slot)
    });
    h.fake.hold_thread_start();
    for _ in 0..200 {
        if h.fake.epoch().is_some() {
            break;
        }
        h.t.pump_for(ms(20));
        h.pause(ms(5));
    }
    h.quiesce();
    (h, slot)
}

/// The server does not start: the app ends (the host reports why).
fn fail_start(h: &mut Harness) {
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(epoch, AppEv::StartFailed("no server".into()));
    h.advance(100);
}

/// What the slot holds for the host to print.
fn unsent(slot: &UnsentSlot) -> Option<String> {
    slot.lock().expect("the slot").clone()
}

/// Keys as a terminal without bracketed paste delivers a paste: a millisecond apart, nothing
/// waits after the last.
fn burst(h: &mut Harness, text: &str) {
    let mut utf8 = [0u8; 4];
    for ch in text.chars() {
        h.t.type_text(ch.encode_utf8(&mut utf8));
        h.t.pump_for(ms(1));
    }
}

// oracle `recovery_keeps_edits_and_pastes_after_chatwidget_handoff`: what was typed and what was
// pasted (a large paste is a placeholder in the composer) is printed in full.
#[test]
fn startup_recovery_keeps_edits_and_pastes() {
    let (mut h, slot) = starting();
    h.t.type_text("startup draf");
    h.advance(100);
    let pasted = format!(" edited {}", "x".repeat(1100));
    h.t.paste(&pasted);
    h.advance(100);
    assert!(
        h.t.screen().contains("[Pasted"),
        "the paste is a placeholder in the composer:\n{}",
        h.t.screen()
    );
    fail_start(&mut h);
    assert_eq!(unsent(&slot), Some(format!("startup draf{pasted}")));
}

// oracle `recovery_keeps_buffered_input_on_exit_or_queued_submission` (queue = false): keys the
// paste detector still holds are printed with the draft.
#[test]
fn startup_recovery_keeps_buffered_input_on_exit() {
    let (mut h, slot) = starting();
    h.t.type_text("startup draft");
    h.advance(100);
    burst(&mut h, "xy");
    assert!(
        !h.t.screen().contains("startup draftxy"),
        "the keys are held, not in the composer yet:\n{}",
        h.t.screen()
    );
    fail_start(&mut h);
    assert_eq!(unsent(&slot), Some("startup draftxy".to_string()));
}

// oracle `recovery_keeps_buffered_input_on_exit_or_queued_submission` (queue = true): Enter
// queues the message with the held keys in it.
#[test]
fn startup_recovery_keeps_buffered_input_in_a_queued_submission() {
    let (mut h, slot) = starting();
    h.t.type_text("startup draft");
    h.advance(100);
    burst(&mut h, "xy");
    h.press("enter");
    h.advance(100);
    fail_start(&mut h);
    assert_eq!(unsent(&slot), Some("startup draftxy".to_string()));
}

// oracle `recovery_uses_pending_pastes_from_the_actual_queued_result`: a paste sent before the
// session is ready is printed whole, tabs and newlines as they are (the composer drops the
// control bytes of a paste before the draft holds them: the escaping of the rest is the core's
// test).
#[test]
fn startup_recovery_uses_pastes_from_the_queued_message() {
    let (mut h, slot) = starting();
    let pasted = format!("{}\n\twith a tab", "y".repeat(1100));
    h.t.paste(&pasted);
    h.advance(100);
    h.press("enter");
    h.advance(100);
    fail_start(&mut h);
    assert_eq!(
        unsent(&slot),
        Some(pasted),
        "the queued message, not the placeholder"
    );
}

// oracle `recovery_after_handoff_preserves_pending_vim_replace_key`: a key typed in Replace mode
// and still held by the detector is printed as applied, over the character it covers.
#[test]
fn startup_recovery_applies_the_pending_vim_replace_key() {
    let (mut h, slot) = starting();
    h.set_keymap(&[("global.toggle_vim_mode", "f9")]);
    h.t.type_text("abcd");
    h.advance(100);
    h.press("f9");
    h.press("0");
    h.press("R");
    h.t.type_text("X");
    h.t.pump_for(ms(1));
    fail_start(&mut h);
    assert_eq!(unsent(&slot), Some("Xbcd".to_string()));
}

// an app that ends otherwise, and a draft nobody typed, leave nothing to print
#[test]
fn startup_recovery_prints_nothing_for_an_empty_draft() {
    let (mut h, slot) = starting();
    fail_start(&mut h);
    assert_eq!(unsent(&slot), None);
}

// the first thread started: what is typed after it is the thread's, not a startup draft
#[test]
fn startup_recovery_forgets_the_draft_once_the_thread_started() {
    let (mut h, slot) = starting();
    h.t.type_text("typed in the chat");
    h.advance(100);
    let ticket = h
        .fake
        .commands()
        .into_iter()
        .find_map(|c| match c {
            codex_tui_core::app::Command::StartThread { ticket, .. } => Some(ticket),
            _ => None,
        })
        .expect("the app asked for a thread");
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(
        epoch,
        AppEv::Reply(
            ticket,
            codex_tui_core::rpc::Reply::thread_started(codex_tui_core::ids::ThreadId::new(
                "thread-1",
            )),
        ),
    );
    h.advance(100);
    fail_start(&mut h);
    assert_eq!(unsent(&slot), None);
}
