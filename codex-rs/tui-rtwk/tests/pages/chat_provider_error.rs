// test root: pages
//! The provider-error notice's keys and taps (oracle `chatwidget/provider_error.rs`, the notice
//! keys of `app/input.rs`): what the oracle's snapshot shard does not say. The notice's rows
//! are the oracle's (`tests/oracle/chat_live_1.rs`); the hints also being tap targets goes
//! beyond the oracle (ADR-011).

use crate::kit::{Owner, Screen};

/// A saved chat the server refused for its provider (the rollout path names the file `c` points the
/// new chat at).
const SAVED_CHAT: &str = include_str!("../oracle/fixtures/chat_commands_3/rollout.jsonl");
const ERROR: &str = r#"{"step":"screen","dir":"chat","ev":{"provider_error":{"thread":"thread-1","error":{"type":"modelProviderNotConfigured","modelProvider":"old-provider"}}}}"#;

fn refused() -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(SAVED_CHAT)
        .open();
    s.type_text("unsent draft");
    s.feed(ERROR);
    s
}

#[test]
fn the_continue_key_starts_a_new_chat_that_names_the_saved_one() {
    let mut s = refused();
    s.assert_contains("continue in new chat");
    s.keys("c");
    assert_eq!(s.commands_named("StartThread").len(), 2, "{}", s.text());
    s.assert_absent("start fresh");
    s.assert_no_turn();
    // the new chat's composer holds the words that point at the saved one: the thread is started
    // by the backend's answer, the words come once the page has it
    s.wait_for("let’s continue where we left off");
    s.assert_contains("let’s continue where we left off");
}

#[test]
fn the_new_key_starts_a_fresh_chat() {
    let mut s = refused();
    s.keys("n");
    assert_eq!(s.commands_named("StartThread").len(), 2, "{}", s.text());
    s.assert_absent("start fresh");
}

#[test]
fn the_hints_are_tap_targets() {
    let mut s = refused();
    s.click_text("start fresh");
    assert_eq!(s.commands_named("StartThread").len(), 2, "{}", s.text());
    let mut s = refused();
    s.click_text("continue in new chat");
    assert_eq!(s.commands_named("StartThread").len(), 2, "{}", s.text());
    let mut s = refused();
    s.click_text("other chats");
    s.assert_absent("start fresh");
    s.assert_no_turn();
}
