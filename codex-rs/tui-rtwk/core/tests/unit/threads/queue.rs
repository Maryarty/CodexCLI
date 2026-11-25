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
        Queued::Literal(text("!echo hi"), None)
    );
    // a line led by a space is not a command
    assert_eq!(
        Queued::of(text(" /model"), false),
        Queued::Message(text("/model"))
    );
}

#[test]
fn a_hidden_paste_keeps_the_label_the_composer_showed() {
    let payload = format!("!echo {}", "x".repeat(40));
    let literal = Queued::of(text(&payload), false);
    // the row reads as the composer did, the message is the whole text
    let label = "[Pasted Content 46 chars]";
    let shown = literal.clone().shown_as(&format!(" {label} "));
    assert_eq!(shown.shown(), label);
    assert_eq!(shown.message().text, payload);
    // a line the composer showed as it is has no label of its own
    assert_eq!(literal.clone().shown_as(&payload).shown(), payload);
    assert_eq!(
        literal.shown_as(&payload),
        Queued::Literal(text(&payload), None)
    );
    // the rest are shown as their text
    let message = Queued::of(text("hello"), false).shown_as("[Pasted Content 5 chars]");
    assert_eq!(message.shown(), "hello");
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
