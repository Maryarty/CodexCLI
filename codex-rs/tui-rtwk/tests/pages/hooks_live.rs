// test root: pages
//! The hook runs under way in the chat (S10; oracle `history_cell/hook_cell.rs`,
//! `chatwidget/hook_lifecycle.rs`): the status row names a run only after the oracle's 300 ms
//! delay (the page's timer, on the app clock), a hook that is over sooner never shows, a restart of
//! the same id waits its delay again, and the edges of a turn clear the runs whose end never came,
//! the timer with them. What the oracle's rows look like is the shard's
//! (`oracle::chat_status_2`); these are the states and the timing.

use crate::kit::{Owner, Screen};

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn turn_started(id: &str) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"turn/started","params":{{"threadId":"thread-1","turn":{{"id":"{id}","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}}}}}"#
    )
}

fn turn_ended(id: &str, status: &str) -> String {
    let error = if status == "failed" {
        r#"{"message":"the model failed"}"#
    } else {
        "null"
    };
    format!(
        r#"{{"step":"notify","message":{{"method":"turn/completed","params":{{"threadId":"thread-1","turn":{{"id":"{id}","rootTurnId":null,"items":[],"itemsView":"full","status":"{status}","error":{error},"startedAt":null,"completedAt":null,"durationMs":null}}}}}}}}"#
    )
}

fn run(id: &str, status: &str, message: Option<&str>, entries: &str) -> String {
    let message = message.map_or("null".to_string(), |text| format!("{text:?}"));
    let (completed_at, duration) = if status == "running" {
        ("null", "null")
    } else {
        ("11", "10")
    };
    format!(
        r#""run":{{"id":"{id}","eventName":"preToolUse","handlerType":"command","executionMode":"sync","scope":"turn","sourcePath":"/tmp/hooks.json","source":"user","displayOrder":0,"status":"{status}","statusMessage":{message},"startedAt":1,"completedAt":{completed_at},"durationMs":{duration},"entries":{entries}}}"#
    )
}

fn hook_started(id: &str, message: Option<&str>) -> String {
    let run = run(id, "running", message, "[]");
    format!(
        r#"{{"step":"notify","message":{{"method":"hook/started","params":{{"threadId":"thread-1","turnId":null,{run}}}}}}}"#
    )
}

/// A hook that ends with nothing to say.
fn hook_quiet(id: &str) -> String {
    let run = run(id, "completed", None, "[]");
    format!(
        r#"{{"step":"notify","message":{{"method":"hook/completed","params":{{"threadId":"thread-1","turnId":null,{run}}}}}}}"#
    )
}

fn chat() -> Screen {
    let mut s = Screen::build(Owner::UiPane).size(80, 24).live().open();
    s.feed(SESSION);
    s
}

/// The turn `turn-1` under way with a hook of `message` just started.
fn hooked(id: &str, message: Option<&str>) -> Screen {
    let mut s = chat();
    s.feed(&format!(
        "{}\n{}",
        turn_started("turn-1"),
        hook_started(id, message)
    ));
    s
}

/// Whether the status row names the hook text.
fn named(s: &Screen, text: &str) -> bool {
    s.rows()
        .iter()
        .any(|row| row.contains("Working") && row.contains(text))
}

#[test]
fn a_running_hook_is_named_only_after_its_delay() {
    let mut s = hooked("pre-tool-use:0", Some("checking command policy"));
    // the feed let 200 ms pass: the run is held back
    s.assert_contains("Working");
    s.assert_absent("checking command policy");
    s.wait(150);
    assert!(named(&s, "checking command policy"), "{}", s.text());
    // and stays while it runs
    s.wait(5000);
    assert!(named(&s, "checking command policy"), "{}", s.text());
}

#[test]
fn a_hook_that_ends_before_its_delay_never_shows() {
    let mut s = hooked("pre-tool-use:0", Some("checking command policy"));
    s.assert_absent("checking command policy");
    s.feed(&hook_quiet("pre-tool-use:0"));
    s.wait(1000);
    s.assert_absent("checking command policy");
    s.assert_absent("Hook");
    // with no turn: no activity row either
    let mut idle = chat();
    idle.feed(&hook_started("session-end", Some("saving session summary")));
    idle.feed(&hook_quiet("session-end"));
    idle.wait(1000);
    idle.assert_absent("Saving session summary");
}

#[test]
fn a_restart_of_the_same_id_waits_its_delay_again() {
    let mut s = hooked("stop:0", Some("checking stop condition"));
    s.wait(150);
    assert!(named(&s, "checking stop condition"), "{}", s.text());
    // started again with no end between: a new run, held back again
    s.feed(&hook_started("stop:0", Some("checking stop condition")));
    s.assert_absent("checking stop condition");
    s.wait(150);
    assert!(named(&s, "checking stop condition"), "{}", s.text());
    // ended, then started again
    s.feed(&hook_quiet("stop:0"));
    s.assert_absent("checking stop condition");
    s.feed(&hook_started("stop:0", Some("checking stop condition")));
    s.assert_absent("checking stop condition");
    s.wait(150);
    assert!(named(&s, "checking stop condition"), "{}", s.text());
}

#[test]
fn the_end_of_a_turn_clears_a_hook_that_never_ended_and_its_timer() {
    for status in ["interrupted", "completed", "failed"] {
        // revealed before the turn ends
        let mut s = hooked("pre-tool-use:0", Some("checking command policy"));
        s.wait(150);
        assert!(named(&s, "checking command policy"), "{status}");
        s.feed(&turn_ended("turn-1", status));
        s.assert_absent("checking command policy");
        s.wait(2000);
        s.assert_absent("checking command policy");

        // held back when the turn ends: the reveal that was due never comes
        let mut s = hooked("pre-tool-use:0", Some("checking command policy"));
        s.feed(&turn_ended("turn-1", status));
        s.wait(2000);
        s.assert_absent("checking command policy");
    }
}

#[test]
fn a_new_turn_starts_without_the_hook_the_last_one_left() {
    let mut s = hooked("pre-tool-use:0", Some("checking command policy"));
    s.wait(150);
    s.feed(&turn_ended("turn-1", "completed"));
    s.feed(&turn_started("turn-2"));
    s.wait(2000);
    s.assert_contains("Working");
    s.assert_absent("checking command policy");
}

#[test]
fn hooks_that_run_together_say_so_in_one_row() {
    let mut s = hooked("pre-tool-use:0", Some("checking command policy"));
    s.feed(&hook_started("post-tool-use:1", Some("scanning for secrets")));
    s.wait(400);
    assert!(named(&s, "Running hooks"), "{}", s.text());
    s.assert_absent("checking command policy");
    s.assert_absent("scanning for secrets");
    // the older one ends: the other speaks for itself
    s.feed(&hook_quiet("pre-tool-use:0"));
    assert!(named(&s, "scanning for secrets"), "{}", s.text());
    s.feed(&hook_quiet("post-tool-use:1"));
    s.assert_absent("scanning for secrets");
    s.assert_absent("Running hook");
}

#[test]
fn a_hook_with_no_turn_is_a_row_of_its_own() {
    let mut s = chat();
    s.feed(&hook_started("session-end", Some("saving session summary")));
    s.assert_absent("Saving session summary");
    s.wait(150);
    s.assert_contains("Saving session summary");
    s.feed(&hook_quiet("session-end"));
    s.assert_absent("Saving session summary");
}
