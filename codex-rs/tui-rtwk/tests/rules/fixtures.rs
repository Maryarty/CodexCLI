//! The fixture scripts through the debug menu: an entry plays its JSONL
//! script over a fresh core model through `ingest`, and the screen shows that
//! model. One worked example per kind: a chat transcript with a streamed
//! answer, an exec approval request, a resume list from fake rollouts.

#[path = "../common/mod.rs"]
mod common;

use codex_tui_core::app::Command;
use codex_tui_core::fixtures::script::build;
use codex_tui_rtwk::app::debug;
use common::{slot, start_debug};

#[test]
fn every_debug_entry_script_parses_and_applies() {
    let mut count = 0;
    for dir in debug::dirs() {
        for entry in (dir.debug_entries)() {
            build(entry.script).unwrap_or_else(|err| {
                panic!(
                    "{}/{}: line {}: {}",
                    dir.name, entry.name, err.line, err.message
                )
            });
            count += 1;
        }
    }
    assert!(count >= 3, "the three worked examples are registered");
}

#[test]
fn the_chat_entry_shows_the_streamed_answer() {
    let mut h = start_debug();
    h.open_debug_row("chat", Some("streamed_answer"));
    let screen = h.t.screen();
    assert!(screen.contains("hello"), "{screen}");
    assert!(screen.contains("The transport kept this."), "{screen}");
    assert!(screen.contains("And dropped this."), "{screen}");
}

#[test]
fn the_approvals_entry_shows_the_request() {
    let mut h = start_debug();
    h.open_debug_row("approvals", Some("exec_approval"));
    // The dialog fades in.
    h.t.pump_for(common::ms(800));
    let screen = h.t.screen();
    // The skeleton's dialog says which request waits (`needs_answer`); it is
    // the seeded queue's, so the dialog stays open instead of closing itself
    // on the real, empty queue.
    assert!(screen.contains("1 needs an answer."), "{screen}");
}

#[test]
fn the_resume_entry_lists_the_fake_rollouts() {
    let mut h = start_debug();
    h.open_debug_row("resume", Some("fake_rollouts"));
    let screen = h.t.screen();
    assert_eq!(screen.matches("mock_provider").count(), 3, "{screen}");
    assert!(screen.contains("2025-01-02"), "{screen}");
    assert!(screen.contains("Hello"), "{screen}");
}

#[test]
fn key_and_resize_steps_drive_the_tester() {
    let script = build(
        r#"{"step":"key","keys":"down"}
{"step":"key","keys":"enter"}
{"step":"resize","width":60,"height":20}"#,
    )
    .expect("plays");
    let mut h = start_debug();
    h.play_inputs(&script.inputs);
    // `down`, `enter` opened the second row of the menu: the chat's entry.
    assert!(
        h.t.screen().contains("The transport kept this."),
        "{}",
        h.t.screen()
    );
    // The screen is the resized terminal's: 20 rows of at most 60 columns.
    let screen = h.t.screen();
    assert_eq!(screen.lines().count(), 20, "{screen}");
    assert!(screen.lines().all(|l| l.chars().count() <= 60), "{screen}");
}

#[test]
fn a_seeded_entry_leaves_the_real_model_alone() {
    // The entry's model is its own: the app behind the menu is not seeded.
    let mut h = start_debug();
    h.open_debug_row("chat", Some("streamed_answer"));
    h.press("esc");
    slot("chat");
    assert!(
        !h.t.screen().contains("The transport kept this."),
        "{}",
        h.t.screen()
    );
}

/// A seeded screen runs over its own model: what the user does there reaches
/// the seeded copy, never the live app's backend.
#[test]
fn a_seeded_submit_sends_no_command_to_the_backend() {
    let mut h = start_debug();
    h.settle();
    let before = h.fake.commands().len();
    h.open_debug_row("chat", Some("streamed_answer"));
    h.t.type_text("a seeded question");
    h.press("enter");
    h.settle();
    let sent = h.fake.commands();
    assert!(
        !sent.iter().any(|c| matches!(c, Command::StartTurn { .. })),
        "a turn was started on the live backend: {sent:?}"
    );
    assert_eq!(sent.len(), before, "{sent:?}");
    // The seeded model took the submit: its composer cleared.
    assert!(
        !h.t.screen().contains("a seeded question"),
        "{}",
        h.t.screen()
    );
}

#[test]
fn a_seeded_dismiss_rejects_nothing_on_the_backend() {
    let mut h = start_debug();
    h.settle();
    h.open_debug_row("approvals", Some("exec_approval"));
    h.t.pump_for(common::ms(800));
    assert!(
        h.t.screen().contains("1 needs an answer."),
        "{}",
        h.t.screen()
    );
    h.press("enter");
    h.t.pump_for(common::ms(800));
    h.settle();
    let sent = h.fake.commands();
    assert!(
        !sent
            .iter()
            .any(|c| matches!(c, Command::RejectRequest { .. })),
        "a request was rejected on the live backend: {sent:?}"
    );
    assert!(
        !h.t.screen().contains("needs an answer"),
        "the dialog closed: {}",
        h.t.screen()
    );
}
