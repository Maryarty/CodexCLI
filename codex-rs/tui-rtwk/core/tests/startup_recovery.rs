#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The text a launch that failed leaves for the user (oracle `startup_recovery.rs`, its tests in
//! `startup_recovery_tests.rs`): what waits for the first thread, else the draft, with control
//! characters escaped, and nothing once the first thread took the queue. The stage machine of the
//! oracle (`HandedOff`, `Submitted`, `Dispatched`) is not ported: the queue's drain is the
//! acknowledgement.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::UserMessage;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn submit(app: &mut CodexApp, text: &str) {
    run(
        app,
        AppEv::Intent(Intent::Submit {
            thread: None,
            message: UserMessage::text(text),
        }),
    );
}

fn shell(app: &mut CodexApp, command: &str) {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Shell {
            thread: None,
            command: command.to_string(),
        }))),
    );
}

fn start_ticket(fx: &AppFx) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("the app asked for a thread")
}

// oracle `recovery_keeps_edits_and_pastes_after_chatwidget_handoff`: the draft is what a failed
// launch prints, whole, whether the composer held text before or not.
#[test]
fn unsent_text_is_the_draft_when_nothing_was_asked() {
    let app = CodexApp::new();
    assert_eq!(
        app.unsent_startup_text("existing draft\nstartup draf edited"),
        Some("existing draft\nstartup draf edited".to_string())
    );
    assert_eq!(app.unsent_startup_text(""), None);
}

// oracle `recovery_keeps_buffered_input_on_exit_or_queued_submission` (queue = true): a message
// queued before the session is configured is what is printed; the asks come in the order they
// were made, a shell line and a `/plan` prompt with them, the command line's prompt first.
#[test]
fn unsent_text_is_the_queued_asks_in_order() {
    let mut app = CodexApp::new().with_initial_prompt(UserMessage::text("from the command line"));
    submit(&mut app, "startup draftxy");
    shell(&mut app, "ls -la");
    run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: None,
            command: SlashCommand::Plan,
            intent: SessionIntent::Plan {
                prompt: Some("make a plan".to_string()),
            },
        }))),
    );
    assert_eq!(
        app.unsent_startup_text("typed after"),
        Some("from the command line\nstartup draftxy\n!ls -la\n/plan make a plan".to_string()),
        "the queue is what the user asked for; the composer text typed after it is not added"
    );
}

// oracle `recovery_uses_pending_pastes_from_the_actual_queued_result`: a queued message holds the
// paste in full (the composer expands its placeholders when it sends), tabs and newlines stay,
// other control bytes are printed escaped, never raw.
#[test]
fn unsent_text_expands_pastes_and_escapes_control_characters() {
    let mut app = CodexApp::new();
    submit(&mut app, "complete queued paste\n\twith control\u{1b}[31m");
    assert_eq!(
        app.unsent_startup_text(""),
        Some("complete queued paste\n\twith control\\u{1b}[31m".to_string())
    );
    // the draft is escaped the same way
    assert_eq!(
        CodexApp::new().unsent_startup_text("a\u{7}b\r\nc"),
        Some("a\\u{7}b\\r\nc".to_string())
    );
}

// oracle `recovery_refresh_stops_after_submission_or_startup_completion`: the stage machine is
// not ported; the queue drains when the first thread starts, and nothing is unsent after it.
#[test]
fn unsent_text_is_nothing_after_the_first_thread_started() {
    let mut app = CodexApp::new();
    submit(&mut app, "early");
    assert_eq!(app.unsent_startup_text(""), Some("early".to_string()));
    let fx = run(&mut app, AppEv::Connected);
    let ticket = start_ticket(&fx);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
    );
    assert_eq!(app.unsent_startup_text(""), None, "the queue went out");
    assert_eq!(
        app.unsent_startup_text("typed in the thread"),
        None,
        "the composer is the thread's now"
    );
}
