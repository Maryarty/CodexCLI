#![allow(clippy::expect_used)]

//! State tests of the submission (core-pass for chunk-commands-submission): a turn or steer whose
//! images could not be prepared never left, so its whole message goes back to the composer, the
//! queue waits, and the next explicit submit goes out first (oracle `on_images_prepared`,
//! `recovered_queue`).

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::fixtures::builders::{turn_completed, turn_started};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, ScreenThreadIntent, chat};
use codex_tui_core::threads::{LocalImage, UserMessage};

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn with_image(text: &str) -> UserMessage {
    UserMessage {
        images: vec![LocalImage {
            path: "/tmp/removed.png".into(),
            range: 0..text.len().min(3),
        }],
        ..UserMessage::text(text)
    }
}

fn submit(app: &mut CodexApp, message: UserMessage) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            message,
        }),
    )
}

fn start_turn(fx: &AppFx) -> Option<(Ticket, String)> {
    fx.commands().iter().find_map(|c| match c {
        Command::StartTurn {
            ticket, message, ..
        } => Some((*ticket, message.text.clone())),
        _ => None,
    })
}

fn steer(fx: &AppFx) -> Option<Ticket> {
    fx.commands().iter().find_map(|c| match c {
        Command::Screen {
            ticket,
            command: ScreenCommand::Chat(chat::Command::Steer { .. }),
            ..
        } => Some(*ticket),
        _ => None,
    })
}

#[test]
fn a_turn_whose_images_failed_comes_back_whole_and_sends_nothing() {
    let mut app = ready();
    let message = with_image("[Image #1] describe");
    let (ticket, _) = start_turn(&submit(&mut app, message.clone())).expect("a turn");
    let fx = run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::prepare_failed("Failed to prepare image: gone"),
        ),
    );
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    let thread = app.active_thread();
    let restore = thread.restore().expect("the message is given back");
    assert_eq!(restore.messages(), [message]);
    assert!(!restore.replaces());
    assert!(thread.last_error().is_some());
    assert!(thread.queue().is_empty());
    assert!(!thread.is_running());
}

#[test]
fn a_follow_up_waits_after_a_failed_preparation_and_the_next_submit_goes_first() {
    let mut app = ready();
    run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
    // a follow-up waits behind the running turn (Tab)
    run(
        &mut app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(
                UserMessage::text("follow-up").into(),
            )),
        }),
    );
    // an image steer fails to prepare
    let fx = submit(&mut app, with_image("[Image #1] steer"));
    let ticket = steer(&fx).expect("a steer");
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::prepare_failed("Failed to prepare image: gone"),
        ),
    );
    let thread = app.active_thread();
    assert!(thread.pending_steers().is_empty());
    assert_eq!(
        thread.restore().expect("given back").merged().text,
        "[Image #1] steer"
    );
    // the turn ends: the queue stays, nothing is sent on its own
    let fx = run(&mut app, AppEv::Backend(turn_completed(T, "turn-1")));
    assert!(start_turn(&fx).is_none(), "{:?}", fx.commands());
    let queued: Vec<_> = app
        .active_thread()
        .queue()
        .iter()
        .map(|m| m.text.clone())
        .collect();
    assert_eq!(queued, ["follow-up"]);
    // what the user sends now goes out, not behind the follow-up
    let fx = submit(&mut app, UserMessage::text("now"));
    assert_eq!(start_turn(&fx).map(|(_, text)| text), Some("now".into()));
}
