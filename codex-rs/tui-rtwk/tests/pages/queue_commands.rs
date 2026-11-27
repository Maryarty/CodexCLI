// test root: pages
//! The queue behind a running turn, as the person drives it (oracle `slash_input.rs`,
//! `chat_composer.rs` `Queued`, `slash_dispatch.rs` `is_task_running`): Enter on a `!` paste
//! hidden behind its placeholder queues it, an idle Tab on a command typed in full changes
//! nothing, and a `/compact` holds the thread busy from the dispatch. The rows are the oracle's:
//! `tests/oracle/chat_commands_*.rs`.

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::fixtures::builders::turn_completed;
use codex_tui_core::screens::{ScreenCommand, chat};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;

use crate::common::{Harness, ms, start};

/// A chat whose first turn runs and does not end.
fn running() -> Harness {
    let mut h = start();
    h.wait_idle();
    h.fake.script_turn(vec![]);
    h.t.type_text("work");
    h.press("enter");
    h.wait_for("the turn to run", |s| s.contains("esc to interrupt"));
    h
}

fn started(h: &Harness) -> Vec<String> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text),
            _ => None,
        })
        .collect()
}

fn compactions(h: &Harness) -> usize {
    h.fake
        .commands()
        .iter()
        .filter(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Chat(chat::Command::Compact),
                    ..
                }
            )
        })
        .count()
}

/// Enter on a paste that starts with `!` (the composer shows `[Pasted Content N chars]`, no shell
/// mode) queues it behind the turn as Tab does, and does not steer: the queue row keeps the
/// placeholder, the next turn is the whole text, once.
#[test]
fn enter_on_a_hidden_shell_paste_while_a_turn_runs_queues_it() {
    let mut h = running();
    let payload = format!("!echo {}", "x".repeat(1000));
    h.t.paste(&payload);
    h.wait_for("the placeholder", |s| {
        s.contains("[Pasted Content 1006 chars]")
    });
    h.press("enter");
    h.wait_for("the queue row", |s| {
        s.contains("↳ [Pasted Content 1006 chars]")
    });
    assert_eq!(started(&h), ["work"]);
    let first_turn_completed = || AppEv::Backend(turn_completed(FAKE_THREAD, "turn-1"));
    h.inject([first_turn_completed()]);
    h.wait_until("the queued paste to start", |h| started(h).len() >= 2);

    // Check for another submission after the repeated completion has been applied.
    h.inject([first_turn_completed()]);
    let mut drained = h.fake.barrier();
    h.wait_until("the fake to take the commands", |_| drained.passed());
    assert_eq!(started(&h), ["work".to_owned(), payload]);
    assert!(
        h.fake.commands().iter().all(|command| !matches!(
            command,
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Steer { .. }),
                ..
            }
        )),
        "the paste steered the turn"
    );
}

/// Tab on a command typed in full completes nothing and runs nothing while the thread is idle
/// (`handle_key_event_with_slash_popup`: Tab queues only a running task's line).
#[test]
fn tab_on_a_complete_command_while_idle_changes_nothing() {
    let mut h = start();
    h.wait_idle();
    h.t.type_text("/compact");
    h.wait_for("the popup", |s| {
        s.contains("/compact") && s.contains("summarize")
    });
    h.press("tab");
    h.t.pump_for(ms(300));
    let screen = h.t.screen();
    assert!(screen.contains("› /compact"), "{screen}");
    assert_eq!(compactions(&h), 0, "Tab ran the command");
    assert!(started(&h).is_empty(), "Tab sent a turn");
}

/// The oracle's `is_task_running` holds from the dispatch of `/compact`: a command that needs an
/// idle thread is disabled before the compaction's turn has begun.
#[test]
fn a_slash_command_typed_after_compact_is_disabled_as_during_a_turn() {
    let mut h = start();
    h.wait_idle();
    h.t.type_text("/compact");
    h.press("enter");
    h.t.pump_for(ms(400));
    assert_eq!(compactions(&h), 1);
    h.t.type_text("/clear");
    h.press("enter");
    let message = "'/clear' is disabled while a task is in progress.";
    h.wait_for(message, |s| s.contains(message));
}
