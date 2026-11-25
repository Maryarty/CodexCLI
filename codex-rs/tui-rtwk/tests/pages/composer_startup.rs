// test root: pages
//! The composer before the first thread exists (the session is `Starting`):
//! the draft can be typed and Enter CONFIRMS it (the text stays, the footer reads `Waiting for
//! startup  esc cancel`); the confirmed draft is sent once when the session is ready, and Esc,
//! an edit or a failed start cancel it, the draft staying. A `!cmd` is confirmed too but never
//! run by the start (oracle: `startup_draft_input.rs`, `startup_submission.rs`).

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

/// The backend refuses the one `StartThread`.
fn fail_start(h: &mut Harness) {
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
        AppEv::Reply(ticket, Reply::failed("the server refused the start")),
    );
    h.advance(100);
}

const WAITING: &str = "Waiting for startup";

#[test]
fn enter_confirms_the_startup_draft_and_it_goes_out_once_when_the_thread_starts() {
    let mut h = starting();
    h.t.type_text("early words");
    h.press("enter");
    // confirmed, not queued: the text stays and the footer says what it waits for
    let screen = h.t.screen();
    assert!(screen.contains("› early words"), "{screen}");
    assert!(screen.contains(WAITING), "{screen}");
    assert!(screen.contains("esc cancel"), "{screen}");
    assert!(!screen.contains("↳"), "no queued row: {screen}");
    assert!(sent(&h).is_empty());
    start_thread(&mut h);
    common::wait::wait_until(&mut h, "the turn", |h| !sent(h).is_empty());
    assert_eq!(sent(&h), ["early words"]);
    // it went out once: the composer is empty again and the footer is the ordinary one
    h.advance(300);
    let screen = h.t.screen();
    assert!(screen.contains("Ask Codex to do anything"), "{screen}");
    assert!(!screen.contains(WAITING), "{screen}");
    assert_eq!(sent(&h), ["early words"]);
}

#[test]
fn escape_cancels_the_confirmation_and_keeps_the_draft() {
    let mut h = starting();
    h.t.type_text("early words");
    h.press("enter");
    h.press("esc");
    let screen = h.t.screen();
    assert!(screen.contains("› early words"), "{screen}");
    assert!(!screen.contains(WAITING), "{screen}");
    start_thread(&mut h);
    h.advance(300);
    assert!(sent(&h).is_empty(), "nothing was confirmed");
    assert!(h.t.screen().contains("› early words"), "{}", h.t.screen());
    // Enter after the start sends it as any message
    h.press("enter");
    common::wait::wait_until(&mut h, "the turn", |h| !sent(h).is_empty());
    assert_eq!(sent(&h), ["early words"]);
}

#[test]
fn an_edit_cancels_the_confirmation() {
    let mut h = starting();
    h.t.type_text("early words");
    h.press("enter");
    h.t.type_text("!");
    h.advance(100);
    let screen = h.t.screen();
    assert!(screen.contains("› early words!"), "{screen}");
    assert!(!screen.contains(WAITING), "{screen}");
    start_thread(&mut h);
    h.advance(300);
    assert!(sent(&h).is_empty(), "an edited draft is no confirmed one");
    assert!(h.t.screen().contains("› early words!"), "{}", h.t.screen());
}

#[test]
fn a_shell_draft_is_confirmed_but_not_run_by_the_start() {
    let mut h = starting();
    h.t.type_text("!ls -la");
    h.press("enter");
    let screen = h.t.screen();
    assert!(screen.contains("ls -la"), "{screen}");
    assert!(screen.contains(WAITING), "{screen}");
    start_thread(&mut h);
    h.advance(300);
    // the draft shows a command, so it is the user's to run: it stays for Enter
    assert!(sent(&h).is_empty(), "the start does not run a shell draft");
    assert!(h.t.screen().contains("ls -la"), "{}", h.t.screen());
    h.press("enter");
    common::wait::wait_until(&mut h, "the shell command", |h| !sent(h).is_empty());
    assert_eq!(sent(&h), ["!ls -la"]);
}

#[test]
fn a_slash_command_is_not_held_for_the_start() {
    let mut h = starting();
    h.t.type_text("/status");
    h.press("enter");
    assert!(!h.t.screen().contains(WAITING), "{}", h.t.screen());
}

#[test]
fn a_failed_start_drops_the_confirmation_and_keeps_the_draft() {
    let mut h = starting();
    h.t.type_text("early words");
    h.press("enter");
    assert!(h.t.screen().contains(WAITING), "{}", h.t.screen());
    fail_start(&mut h);
    h.advance(300);
    let screen = h.t.screen();
    assert!(!screen.contains(WAITING), "{screen}");
    assert!(screen.contains("› early words"), "{screen}");
    assert!(sent(&h).is_empty());
}

#[test]
fn a_view_over_the_chat_holds_the_confirmed_draft_until_it_closes() {
    let mut h = starting();
    h.t.type_text("early words");
    h.press("enter");
    // the debug menu (F12) is a route over the chat: the start happens behind it
    h.press("f12");
    start_thread(&mut h);
    h.advance(300);
    assert!(sent(&h).is_empty(), "the draft waits behind the view");
    h.press("esc");
    common::wait::wait_until(&mut h, "the turn", |h| !sent(h).is_empty());
    assert_eq!(sent(&h), ["early words"]);
}

#[test]
fn only_the_last_confirmed_draft_goes_out_after_the_start() {
    // a cancelled confirmation leaves nothing behind: only the draft confirmed last goes out
    let mut h = starting();
    h.t.type_text("first");
    h.press("enter");
    h.press("esc");
    h.press("ctrl+u");
    h.t.type_text("second");
    h.press("enter");
    start_thread(&mut h);
    common::wait::wait_until(&mut h, "the turn", |h| !sent(h).is_empty());
    assert_eq!(sent(&h), ["second"]);
}
