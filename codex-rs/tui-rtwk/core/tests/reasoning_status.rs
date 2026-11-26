#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the status row's reasoning header (oracle `chatwidget/streaming.rs`
//! `latest_summary_line`, `chatwidget/tests/reasoning_status_tests.rs`): the latest usable line of
//! the running turn's reasoning, through the transcript's items and the streamed deltas.

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{ThreadId, TurnId};
use codex_tui_core::screens::chat::reasoning::latest_summary_line;
use pretty_assertions::assert_eq;
use serde_json::json;

const TURN: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":1700000000,"completedAt":null,"durationMs":null}}}}"#;

fn notify(method: &str, params: serde_json::Value) -> String {
    json!({"step": "notify", "message": {"method": method, "params": params}}).to_string()
}

fn lifecycle(turn: &str, id: &str, summary: &[&str], done: bool) -> String {
    let (method, at) = if done {
        ("item/completed", "completedAtMs")
    } else {
        ("item/started", "startedAtMs")
    };
    notify(
        method,
        json!({"threadId": "thread-1", "turnId": turn, at: 0,
            "item": {"type": "reasoning", "id": id, "summary": summary, "content": []}}),
    )
}

fn delta(item: &str, text: &str) -> String {
    notify(
        "item/reasoning/summaryTextDelta",
        json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": item,
            "delta": text, "summaryIndex": 0}),
    )
}

fn raw_delta(item: &str, text: &str) -> String {
    notify(
        "item/reasoning/textDelta",
        json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": item,
            "delta": text, "contentIndex": 0}),
    )
}

fn part(item: &str) -> String {
    notify(
        "item/reasoning/summaryPartAdded",
        json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": item, "summaryIndex": 1}),
    )
}

/// The header after the script `steps` ran on a running turn.
fn header_of(steps: &[String], show_raw: bool) -> Option<String> {
    let mut script = TURN.to_string();
    for step in steps {
        script.push('\n');
        script.push_str(step);
    }
    let sandbox = Sandbox::play(&script).expect("plays");
    let thread = sandbox.app().thread(&ThreadId::new("thread-1")).unwrap();
    thread.screen_slots().chat.reasoning_header(
        thread.transcript(),
        &TurnId::new("turn-1"),
        show_raw,
    )
}

#[test]
fn latest_line_skips_blank_comment_and_unclosed_bold() {
    assert_eq!(
        latest_summary_line("Plain\n\n<!-- note -->\n  \n"),
        Some("Plain".to_string())
    );
    // an unclosed `**` is no line yet: the earlier ones are
    assert_eq!(
        latest_summary_line("**Closed**\n**Open"),
        Some("Closed".to_string())
    );
    assert_eq!(latest_summary_line("**Open"), None);
    assert_eq!(latest_summary_line(""), None);
}

#[test]
fn bold_with_plain_suffix_joins() {
    assert_eq!(
        latest_summary_line("**Checking tests**: running suite"),
        Some("Checking tests: running suite".to_string())
    );
}

#[test]
fn heading_marker_hashes_are_stripped() {
    assert_eq!(
        latest_summary_line("## Mapping the app"),
        Some("Mapping the app".to_string())
    );
    assert_eq!(latest_summary_line("###"), None);
}

#[test]
fn no_reasoning_gives_no_header() {
    assert_eq!(header_of(&[], false), None);
    assert_eq!(
        header_of(&[lifecycle("turn-1", "a", &[], false)], false),
        None
    );
}

#[test]
fn last_useful_line_survives_empty_items() {
    let steps = [
        lifecycle("turn-1", "first", &[], false),
        delta("first", "**Researching backend freshness**"),
        // an empty completion keeps the deltas the item streamed
        lifecycle("turn-1", "first", &[], true),
        lifecycle("turn-1", "second", &[], false),
    ];
    assert_eq!(
        header_of(&steps, false),
        Some("Researching backend freshness".to_string())
    );
}

#[test]
fn deltas_extend_the_item_in_flight_only() {
    let mut steps = vec![
        lifecycle("turn-1", "reasoning", &["**Checking tests"], false),
        // before the bold closes it is no header
    ];
    assert_eq!(header_of(&steps, false), None);
    steps.push(delta("reasoning", "**: running suite"));
    assert_eq!(
        header_of(&steps, false),
        Some("Checking tests: running suite".to_string())
    );
    // the next item's own words win over the earlier item's
    steps.push(lifecycle("turn-1", "next", &[], false));
    steps.push(delta("next", "No summary heading is available."));
    assert_eq!(
        header_of(&steps, false),
        Some("No summary heading is available.".to_string())
    );
    steps.push(delta("next", "\n**Preparing evidence report**"));
    assert_eq!(
        header_of(&steps, false),
        Some("Preparing evidence report".to_string())
    );
}

#[test]
fn a_completed_item_that_holds_its_streamed_text_adds_nothing() {
    let steps = [
        lifecycle("turn-1", "a", &[], false),
        delta("a", "**Done heading**"),
        lifecycle("turn-1", "a", &["**Done heading**"], true),
    ];
    assert_eq!(header_of(&steps, false), Some("Done heading".to_string()));
}

#[test]
fn a_part_break_starts_a_new_part() {
    let steps = [
        lifecycle("turn-1", "a", &[], false),
        delta("a", "**First part**"),
        part("a"),
    ];
    // the new part has no line yet: the earlier part's line stays
    assert_eq!(header_of(&steps, false), Some("First part".to_string()));
    let mut steps = steps.to_vec();
    steps.push(delta("a", "**Second"));
    assert_eq!(header_of(&steps, false), Some("First part".to_string()));
    steps.push(delta("a", " part**"));
    assert_eq!(header_of(&steps, false), Some("Second part".to_string()));
}

#[test]
fn another_turns_items_do_not_count() {
    let steps = [
        lifecycle("turn-0", "old", &["Old turn's words"], true),
        lifecycle("turn-1", "a", &[], false),
    ];
    assert_eq!(header_of(&steps, false), None);
}

#[test]
fn raw_deltas_count_only_with_the_raw_flag() {
    let steps = [
        lifecycle("turn-1", "a", &[], false),
        raw_delta("a", "Private thought"),
    ];
    assert_eq!(header_of(&steps, false), None);
    assert_eq!(header_of(&steps, true), Some("Private thought".to_string()));
}
