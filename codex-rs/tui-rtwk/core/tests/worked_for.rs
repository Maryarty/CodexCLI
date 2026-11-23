#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the "Worked for" fallback (core-pass for chunk-live-worked-for): a live turn
//! completion the server timed not at all falls back to the status timer (the clock since the
//! client saw the turn start) and the local clock, as the oracle's
//! `FinalMessageSeparator::from_turn_once`; a completion the server timed keeps its own numbers.

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::threads::TurnTiming;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const T: &str = "thread-1";

fn notify(message: Value) -> String {
    format!(r#"{{"step":"notify","message":{message}}}"#)
}

fn clock(now: i64) -> String {
    format!(r#"{{"step":"screen","dir":"agents","ev":{{"clock":{now}}}}}"#)
}

fn started() -> String {
    notify(
        json!({"method": "turn/started", "params": {"threadId": T, "turn": {
        "id": "turn-1", "items": [], "itemsView": "full", "status": "inProgress",
        "error": null}}}),
    )
}

fn answer() -> String {
    notify(
        json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
        "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "agentMessage", "id": "a1", "text": "Done.", "phase": null}}}),
    )
}

fn completed(extra: Value) -> String {
    let mut turn = json!({"id": "turn-1", "itemsView": "full", "status": "completed",
        "error": null, "items": [
            {"type": "agentMessage", "id": "a1", "text": "Done.", "phase": null}]});
    for (key, value) in extra.as_object().unwrap() {
        turn[key] = value.clone();
    }
    notify(json!({"method": "turn/completed", "params": {"threadId": T, "turn": turn}}))
}

fn timing(lines: &[String]) -> Option<TurnTiming> {
    let mut script = vec![r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_string()];
    script.extend(lines.iter().cloned());
    let sandbox = Sandbox::play(&script.join("\n")).expect("plays");
    let thread = sandbox.app().active_thread();
    let transcript = thread.transcript();
    transcript.timing_after(transcript.len().checked_sub(1)?)
}

#[test]
fn a_live_completion_without_timing_works_for_the_status_timer() {
    let got = timing(&[
        clock(1_000),
        started(),
        answer(),
        clock(1_090),
        completed(json!({})),
    ])
    .expect("a timing");
    assert_eq!(got.duration_ms, Some(90_000));
    assert_eq!(got.completed_at, Some(1_090));
}

#[test]
fn the_servers_own_numbers_win_over_the_fallback() {
    let got = timing(&[
        clock(1_000),
        started(),
        answer(),
        clock(1_090),
        completed(json!({"durationMs": 5_000, "completedAt": 1_070})),
    ])
    .expect("a timing");
    assert_eq!(got.duration_ms, Some(5_000));
    assert_eq!(got.completed_at, Some(1_070));
}

#[test]
fn a_completion_the_client_never_saw_start_has_no_elapsed_time() {
    // The oracle's restored completion: no status timer ran, so no duration is invented.
    let got = timing(&[clock(1_090), answer(), completed(json!({}))]);
    assert_eq!(got.and_then(|t| t.duration_ms), None);
}

#[test]
fn no_clock_yet_invents_nothing() {
    assert_eq!(timing(&[started(), answer(), completed(json!({}))]), None);
}
