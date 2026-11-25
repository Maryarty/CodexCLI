//! The welcome blossom over a fresh conversation (oracle `app/empty_state_policy.rs`,
//! `empty_state_animation/policy.rs`, `app/empty_state_animation_tests.rs`): it stays while the
//! thread holds startup metadata only, and the activity, the detailed transcript and the raw
//! rendering take it away.

#![allow(clippy::expect_used)]

use codex_tui_core::app::{AppEv, BackendEv};
use codex_tui_core::fixtures::builders::user_item;
use codex_tui_core::screens::{ScreenEv, chat};
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use serde_json::json;

use crate::common::{Harness, start_sized};

fn has_blossom(screen: &str) -> bool {
    screen
        .chars()
        .any(|ch| ('\u{2801}'..='\u{28ff}').contains(&ch))
}

/// The page with the welcome on (the config's `tui.effects.welcome`).
fn fresh() -> Harness {
    let mut h = start_sized(false, 80, 30);
    h.wait_idle();
    h.inject([AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
        chat::StatusFacts {
            empty_state: true,
            ..chat::StatusFacts::default()
        },
    ))))]);
    h.wait_for("the blossom", has_blossom);
    h
}

fn notification(value: serde_json::Value) -> AppEv {
    let notification = serde_json::from_value(value).expect("notification");
    AppEv::Backend(BackendEv::Notification(Box::new(notification)))
}

#[test]
fn a_startup_warning_leaves_the_blossom_where_it_is() {
    let mut h = fresh();
    h.inject([notification(json!({
        "method": "mcpServer/startupStatus/updated",
        "params": {"threadId": FAKE_THREAD, "name": "alpha", "status": "failed",
            "error": "spawn failed", "failureReason": null}
    }))]);
    h.wait_for("the warning", |s| s.contains("alpha"));
    h.settle();
    let screen = h.t.screen();
    assert!(has_blossom(&screen), "{screen}");
}

#[test]
fn another_entry_ends_the_welcome_for_the_thread() {
    let mut h = fresh();
    h.inject([AppEv::Backend(user_item(
        FAKE_THREAD,
        "turn-1",
        "user-1",
        "hello there",
    ))]);
    h.wait_for("the prompt", |s| s.contains("hello there"));
    h.settle();
    let screen = h.t.screen();
    assert!(!has_blossom(&screen), "{screen}");
}

#[test]
fn the_detailed_transcript_hides_the_blossom_and_gives_it_back() {
    let mut h = fresh();
    h.press("ctrl+t");
    h.wait_for("the blossom gone", |s| !has_blossom(s));
    h.press("ctrl+t");
    h.wait_for("the blossom back", has_blossom);
}

#[test]
fn raw_rendering_hides_the_blossom() {
    let mut h = fresh();
    h.press("alt+r");
    h.wait_for("the blossom gone", |s| !has_blossom(s));
    h.press("alt+r");
    h.wait_for("the blossom back", has_blossom);
}
