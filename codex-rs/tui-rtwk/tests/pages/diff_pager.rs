// test root: pages
//! `/diff` over the chat (oracle `event_dispatch.rs` `DiffResult`, `Overlay::new_static_with_lines`
//! titled `D I F F`): the slice's presenter shows the pager over the document the host answered
//! with; the chat below keeps its place and `q` returns to it.

use codex_tui_core::app::Command;
use codex_tui_core::app::effects::{Ask as Effect, Done};
use codex_tui_core::screens::transcript::diff::{Failure, Report};

use crate::kit::{Owner, Screen};

fn chat() -> Screen {
    Screen::build(Owner::ChatLive).size(60, 14).live().open()
}

/// `/diff` typed and entered; the host's answer is `result`.
fn diff(s: &mut Screen, result: Result<Report, Failure>) {
    s.type_text("/diff");
    s.keys("enter");
    let ask = s
        .commands()
        .into_iter()
        .find(|c| {
            matches!(
                c,
                Command::Effect {
                    ask: Effect::GitDiff { .. },
                    ..
                }
            )
        })
        .expect("the diff was asked of the host");
    s.answer_effect(&ask, Done::GitDiff(result));
    s.wait(300);
}

#[test]
fn the_diff_shows_in_the_pager_under_its_title() {
    let mut s = chat();
    diff(
        &mut s,
        Ok(Report {
            in_repo: true,
            text: "diff --git a/x b/x\n\u{1b}[32m+added\u{1b}[0m\n".to_owned(),
        }),
    );
    s.assert_row_starts_with(0, "/ D I F F");
    s.assert_contains("diff --git a/x b/x");
    s.assert_contains("+added");
    s.assert_contains("close");
}

#[test]
fn q_returns_to_the_chat_as_it_was() {
    let mut s = chat();
    let before = s.rows();
    diff(
        &mut s,
        Ok(Report {
            in_repo: true,
            text: "diff --git a/x b/x\n".to_owned(),
        }),
    );
    assert_ne!(s.rows(), before, "the pager covers the chat");
    s.keys("q");
    assert_eq!(s.rows(), before, "the chat below keeps its place");
}

#[test]
fn escape_is_no_key_of_the_pager() {
    let mut s = chat();
    diff(
        &mut s,
        Ok(Report {
            in_repo: true,
            text: "diff --git a/x b/x\n".to_owned(),
        }),
    );
    s.keys("escape");
    s.assert_contains("D I F F");
}

#[test]
fn an_empty_diff_says_no_changes_in_italics() {
    let mut s = chat();
    diff(
        &mut s,
        Ok(Report {
            in_repo: true,
            text: "\n".to_owned(),
        }),
    );
    s.assert_row_starts_with(1, "No changes detected.");
}

#[test]
fn a_directory_outside_a_repository_says_so() {
    let mut s = chat();
    diff(
        &mut s,
        Ok(Report {
            in_repo: false,
            text: String::new(),
        }),
    );
    s.assert_contains("`/diff` — _not inside a git repository_");
}

#[test]
fn a_failure_says_why() {
    let mut s = chat();
    diff(&mut s, Err(Failure::Shell("boom".to_owned())));
    s.assert_contains("Failed to compute diff: boom");
}
