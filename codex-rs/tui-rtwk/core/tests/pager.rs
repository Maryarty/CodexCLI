//! State tests of S16 (the pager): the seed forms of a script's `screen`
//! step go through `ingest` into the model. No oracle test checks these (the
//! oracle keeps them in the overlay's own fields); the oracle's own
//! `TranscriptHistoryState::has_unloaded_history` is ported.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_rtwk_core::fixtures::sandbox::Sandbox;
use codex_tui_rtwk_core::screens::pager::{HistoryState, StaticPage};
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn play(step: &str) -> Sandbox {
    Sandbox::play(&format!("{SESSION}\n{step}\n")).expect("plays")
}

#[test]
fn a_page_seed_opens_the_static_page() {
    let sandbox = play(
        r#"{"step":"screen","dir":"pager","ev":{"page":{"title":"S T A T I C","lines":["one","two"]}}}"#,
    );
    assert_eq!(
        sandbox.app().screens.pager.page,
        Some(StaticPage {
            title: "S T A T I C".into(),
            lines: vec!["one".into(), "two".into()],
        })
    );
}

#[test]
fn a_history_seed_moves_the_pagination() {
    for (name, state) in [
        ("loading_older", HistoryState::LoadingOlder),
        ("loading_beginning", HistoryState::LoadingBeginning),
        ("partial", HistoryState::Partial),
        ("failed", HistoryState::Failed),
        ("complete", HistoryState::Complete),
    ] {
        let sandbox = play(&format!(
            r#"{{"step":"screen","dir":"pager","ev":{{"history":"{name}"}}}}"#
        ));
        assert_eq!(sandbox.app().screens.pager.history, state, "{name}");
    }
}

/// Oracle: `TranscriptHistoryState::has_unloaded_history`
/// (`pager_overlay.rs:408`).
#[test]
fn only_a_complete_history_has_nothing_unloaded() {
    assert!(!HistoryState::Complete.has_unloaded_history());
    for state in [
        HistoryState::LoadingOlder,
        HistoryState::LoadingBeginning,
        HistoryState::Partial,
        HistoryState::Failed,
    ] {
        assert!(state.has_unloaded_history(), "{state:?}");
    }
}

#[test]
fn a_bad_seed_names_its_line() {
    let err = Sandbox::play(&format!(
        "{SESSION}\n{}\n",
        r#"{"step":"screen","dir":"pager","ev":{"history":"nonsense"}}"#
    ))
    .err()
    .expect("fails");
    assert_eq!(err.line, 2);
}
