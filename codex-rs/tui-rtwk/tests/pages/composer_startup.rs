// test root: pages
//! The composer before the first thread exists (the session is `Starting`):
//! the draft can be typed, a message is kept until the thread starts and goes
//! out then; a `!cmd` is kept with it, in order, and runs then (oracle:
//! input queued until the session is configured, `queue_user_message`,
//! `submit_queued_shell_prompt`).

// a test of the page: the harness has the connection by now
#![allow(clippy::expect_used)]

use crate::common::{self, Harness, start_driven};
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, chat};

/// The app up and connected, its first thread not yet answered.
fn starting() -> Harness {
    let mut h = start_driven(false, 100, 24);
    h.fake.hold_thread_start();
    for _ in 0..200 {
        if h.fake.epoch().is_some() {
            break;
        }
        h.t.pump_for(std::time::Duration::from_millis(20));
        h.pause(std::time::Duration::from_millis(5));
    }
    h.quiesce();
    h
}

/// The thread starts: the backend answers the one `StartThread`.
fn start_thread(h: &mut Harness) {
    let ticket = h
        .fake
        .commands()
        .into_iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(ticket),
            _ => None,
        })
        .expect("the app asked for a thread");
    let epoch = h.fake.epoch().expect("connected");
    h.fake.inject(
        epoch,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
    );
    h.advance(100);
}

/// What went out after the start: a turn's text, or `!cmd` for a shell command.
fn sent(h: &Harness) -> Vec<String> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text),
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Shell(command)),
                ..
            } => Some(format!("!{command}")),
            _ => None,
        })
        .collect()
}

#[test]
fn a_draft_typed_while_the_thread_starts_survives_the_start() {
    let mut h = starting();
    h.t.type_text("draft in progress");
    h.advance(100);
    assert!(
        h.t.screen().contains("› draft in progress"),
        "{}",
        h.t.screen()
    );
    assert!(sent(&h).is_empty());
    start_thread(&mut h);
    assert!(
        h.t.screen().contains("› draft in progress"),
        "{}",
        h.t.screen()
    );
    assert!(
        sent(&h).is_empty(),
        "the draft is not submitted by the start"
    );
}

#[test]
fn a_message_sent_before_the_thread_starts_goes_out_when_it_starts() {
    let mut h = starting();
    h.t.type_text("early words");
    h.press("enter");
    // the draft went into the queue: the composer is empty again
    assert!(!h.t.screen().contains("› early words"), "{}", h.t.screen());
    assert!(sent(&h).is_empty());
    start_thread(&mut h);
    common::wait::wait_until(&mut h, "the turn", |h| !sent(h).is_empty());
    assert_eq!(sent(&h), ["early words"]);
}

#[test]
fn a_shell_command_sent_before_the_thread_starts_runs_when_it_starts() {
    let mut h = starting();
    h.t.type_text("!ls -la");
    h.press("enter");
    // it waits on the queue, drawn as a queued row (`queued_user_message_texts`)
    assert!(h.t.screen().contains("↳ !ls -la"), "{}", h.t.screen());
    assert!(sent(&h).is_empty());
    start_thread(&mut h);
    common::wait::wait_until(&mut h, "the shell command", |h| !sent(h).is_empty());
    assert_eq!(sent(&h), ["!ls -la"]);
}

#[test]
fn messages_and_shell_commands_before_the_start_keep_their_order() {
    let mut h = starting();
    for line in ["!pwd", "then this", "!date"] {
        h.t.type_text(line);
        h.press("enter");
    }
    start_thread(&mut h);
    common::wait::wait_until(&mut h, "all three", |h| sent(h).len() == 3);
    assert_eq!(sent(&h), ["!pwd", "then this", "!date"]);
}
