#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of live context compaction (task fw1-c6, oracle `chatwidget/compaction.rs`): the
//! running compaction and its start are thread facts, a live completion that matches it gives
//! the cell a duration, a completion that comes with history gives none, and the end of the turn
//! clears what is left. The words ("Compacting context", "Context compacted · 1m 3s") are the UI's.

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{ItemId, ThreadId};
use codex_tui_core::threads::{Item, Notice};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const T: &str = "thread-1";
const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn notify(message: Value) -> String {
    format!(r#"{{"step":"notify","message":{message}}}"#)
}

fn clock(now: i64) -> String {
    format!(r#"{{"step":"screen","dir":"agents","ev":{{"clock":{now}}}}}"#)
}

fn turn_started() -> String {
    notify(
        json!({"method": "turn/started", "params": {"threadId": T, "turn": {
        "id": "turn-1", "items": [], "itemsView": "full", "status": "inProgress",
        "error": null}}}),
    )
}

fn turn_completed() -> String {
    notify(
        json!({"method": "turn/completed", "params": {"threadId": T, "turn": {
        "id": "turn-1", "items": [], "itemsView": "full", "status": "completed",
        "error": null}}}),
    )
}

fn compaction_started(id: &str, started_at_ms: i64) -> String {
    notify(
        json!({"method": "item/started", "params": {"threadId": T, "turnId": "turn-1",
        "startedAtMs": started_at_ms, "item": {"type": "contextCompaction", "id": id}}}),
    )
}

fn compaction_completed(id: &str) -> String {
    notify(
        json!({"method": "item/completed", "params": {"threadId": T, "turnId": "turn-1",
        "completedAtMs": 0, "item": {"type": "contextCompaction", "id": id}}}),
    )
}

fn play(lines: &[String]) -> Sandbox {
    let mut script = vec![SESSION.to_string()];
    script.extend(lines.iter().cloned());
    Sandbox::play(&script.join("\n")).expect("plays")
}

fn rows(sandbox: &Sandbox) -> Vec<Item> {
    let thread = sandbox.app().thread(&ThreadId::new(T)).expect("the thread");
    let transcript = thread.transcript();
    (0..transcript.len())
        .filter_map(|index| transcript.get(index).map(|(_, item)| item.clone()))
        .collect()
}

fn active(sandbox: &Sandbox) -> Option<(String, i64)> {
    let thread = sandbox.app().thread(&ThreadId::new(T)).expect("the thread");
    thread
        .compaction()
        .active()
        .map(|active| (active.id.to_string(), active.started_at))
}

fn took(sandbox: &Sandbox, id: &str) -> Option<u64> {
    let thread = sandbox.app().thread(&ThreadId::new(T)).expect("the thread");
    thread.compaction().took(&ItemId::new(id))
}

/// A start is the running compaction with the server's own start time; the row of the cell does
/// not exist until it completes.
#[test]
fn a_started_compaction_runs_from_the_servers_start_and_has_no_row_yet() {
    let sandbox = play(&[
        clock(2_000),
        turn_started(),
        compaction_started("c1", 1_990_000),
    ]);
    assert_eq!(active(&sandbox), Some(("c1".to_owned(), 1_990)));
    assert_eq!(rows(&sandbox), vec![]);
}

/// A start the server dates in the future (or a start without a date) counts from now.
#[test]
fn a_start_in_the_future_counts_from_now() {
    let sandbox = play(&[
        clock(2_000),
        turn_started(),
        compaction_started("c1", 9_999_000),
    ]);
    assert_eq!(active(&sandbox), Some(("c1".to_owned(), 2_000)));
    let sandbox = play(&[clock(2_000), turn_started(), compaction_started("c1", 0)]);
    assert_eq!(active(&sandbox), Some(("c1".to_owned(), 2_000)));
}

/// The same start again does not restart the timer.
#[test]
fn a_repeated_start_keeps_the_first_start() {
    let sandbox = play(&[
        clock(2_000),
        turn_started(),
        compaction_started("c1", 1_900_000),
        clock(2_010),
        compaction_started("c1", 2_005_000),
    ]);
    assert_eq!(active(&sandbox), Some(("c1".to_owned(), 1_900)));
}

/// A live completion of the running compaction keeps how long it took, ends the status and puts
/// the cell in the transcript.
#[test]
fn a_live_completion_keeps_its_duration_and_adds_the_cell() {
    let sandbox = play(&[
        clock(2_000),
        turn_started(),
        compaction_started("c1", 1_900_000),
        clock(2_060),
        compaction_completed("c1"),
    ]);
    assert_eq!(active(&sandbox), None);
    assert_eq!(took(&sandbox, "c1"), Some(160));
    assert_eq!(rows(&sandbox), vec![Item::Notice(Notice::ContextCompacted)]);
}

/// A completion that comes with history (no start seen here) has a cell and no duration, and does
/// not end the compaction that runs.
#[test]
fn a_completion_that_came_with_history_has_no_duration() {
    let sandbox = play(&[
        clock(2_000),
        turn_started(),
        compaction_started("c2", 1_990_000),
        compaction_completed("c1"),
    ]);
    assert_eq!(took(&sandbox, "c1"), None);
    assert_eq!(active(&sandbox), Some(("c2".to_owned(), 1_990)));
    assert_eq!(rows(&sandbox), vec![Item::Notice(Notice::ContextCompacted)]);
}

/// The end of the turn clears a compaction whose completion never came.
#[test]
fn the_end_of_the_turn_clears_the_running_compaction() {
    let sandbox = play(&[
        clock(2_000),
        turn_started(),
        compaction_started("c1", 1_990_000),
        turn_completed(),
    ]);
    assert_eq!(active(&sandbox), None);
}

/// A start that arrives after its turn ended leaves no timer behind.
#[test]
fn a_start_after_the_end_of_its_turn_is_ignored() {
    let sandbox = play(&[
        clock(2_000),
        turn_started(),
        turn_completed(),
        compaction_started("c1", 1_990_000),
    ]);
    assert_eq!(active(&sandbox), None);
}
