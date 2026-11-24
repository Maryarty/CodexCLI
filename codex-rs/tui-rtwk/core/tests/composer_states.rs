//! The composer state of a thread (`composer_state`): one read-only fact for
//! the page, strongest cause first.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::composer::{ComposerState, composer_state};

const A: &str = "thread-1";

fn state(shutting_down: bool) -> ComposerState {
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    let app = build(&format!("{script}\n")).expect("plays").app;
    let model = app.thread(&ThreadId::new(A)).expect("the seeded thread");
    composer_state(model, shutting_down)
}

#[test]
fn an_attached_thread_takes_text_and_commands() {
    let open = state(false);
    assert_eq!(open, ComposerState::Open);
    assert!(open.takes_text() && open.takes_commands());
}

#[test]
fn shutdown_beats_every_other_cause_and_takes_nothing() {
    let closing = state(true);
    assert_eq!(closing, ComposerState::ShuttingDown);
    assert!(!closing.takes_text() && !closing.takes_commands());
}

#[test]
fn a_parent_owned_thread_takes_commands_but_no_text() {
    assert!(!ComposerState::ParentOwned.takes_text());
    assert!(ComposerState::ParentOwned.takes_commands());
    assert!(!ComposerState::ReadOnly.takes_commands());
    assert!(!ComposerState::Disabled.takes_commands());
}
