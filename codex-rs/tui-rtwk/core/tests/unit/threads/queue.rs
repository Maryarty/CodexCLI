use codex_tui_core::commands::QueueDrain;
use codex_tui_core::commands::SlashCommand;
use codex_tui_core::threads::UserMessage;
use codex_tui_core::threads::queue::*;
use std::ops::Deref;

fn text(text: &str) -> UserMessage {
    UserMessage::text(text)
}

#[test]
fn the_queue_key_sorts_a_line_by_its_first_character() {
    assert_eq!(
        Queued::of(text("hello"), false),
        Queued::Message(text("hello"))
    );
    assert_eq!(
        Queued::of(text("/review x"), false),
        Queued::Command(text("/review x"))
    );
    assert_eq!(
        Queued::of(text("!echo hi"), true),
        Queued::Shell(text("!echo hi"))
    );
    assert_eq!(Queued::of(text("!"), true), Queued::Shell(text("!")));
    // a `!` the user did not type in shell mode is a message as it stands
    assert_eq!(
        Queued::of(text(" !echo hi"), false),
        Queued::Literal(text("!echo hi"))
    );
    // a line led by a space is not a command
    assert_eq!(
        Queued::of(text(" /model"), false),
        Queued::Message(text("/model"))
    );
}

#[test]
fn a_popup_or_a_running_turn_stops_the_queue_before_the_rule() {
    let ran = Dispatched {
        command: Some(SlashCommand::Status),
        modal: false,
        can_change_directory: true,
        managed_worktree: true,
    };
    assert_eq!(ran.drain(false), QueueDrain::Continue);
    assert_eq!(ran.drain(true), QueueDrain::Stop);
    assert_eq!(
        Dispatched { modal: true, ..ran }.drain(false),
        QueueDrain::Stop
    );
    assert_eq!(
        Dispatched {
            command: None,
            ..ran
        }
        .drain(false),
        QueueDrain::Continue
    );
    assert_eq!(
        Dispatched {
            command: Some(SlashCommand::Model),
            ..ran
        }
        .drain(false),
        QueueDrain::Stop
    );
}
