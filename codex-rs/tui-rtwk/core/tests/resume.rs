//! State tests of S18 (the resume picker): the listing events, the clock as a
//! model input (R11) and the resume intent. Oracle data:
//! `resume_picker.rs` `resume_picker_table` (rows, ages) and
//! `format_relative_time`-style inputs; the oracle keeps them in `PickerState`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::resume::{Intent as ResumeIntent, Load};
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn play(steps: &[&str]) -> Sandbox {
    Sandbox::play(&format!("{SESSION}\n{}\n", steps.join("\n"))).expect("plays")
}

const LISTED: &str = r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":[
{"id":"a","preview":"Fix resume picker timestamps","cloud":true,"created":1,"updated":2},
{"id":"b","preview":"Investigate lazy pagination cap","unread":true,"thread_name":"Pagination"}]}}}"#;

#[test]
fn a_listing_fills_the_rows_and_ends_the_loading() {
    let loading = r#"{"step":"screen","dir":"resume","ev":{"loading":null}}"#;
    let sandbox = play(&[loading]);
    assert_eq!(sandbox.app().screens.resume.load, Load::Loading);
    let one_line = LISTED.replace('\n', "");
    let sandbox = play(&[loading, &one_line]);
    let model = &sandbox.app().screens.resume;
    assert_eq!(model.load, Load::Idle);
    assert_eq!(model.rows.len(), 2);
    assert!(model.rows[0].cloud);
    assert!(model.rows[1].unread);
}

#[test]
fn a_thread_name_replaces_the_preview() {
    // Oracle: `Row::display_preview`.
    let one_line = LISTED.replace('\n', "");
    let sandbox = play(&[&one_line]);
    let rows = &sandbox.app().screens.resume.rows;
    assert_eq!(rows[0].display_preview(), "Fix resume picker timestamps");
    assert_eq!(rows[1].display_preview(), "Pagination");
}

#[test]
fn a_failure_keeps_its_message_and_the_next_listing_clears_it() {
    let failed = r#"{"step":"screen","dir":"resume","ev":{"failed":"Failed to read session metadata from /tmp/missing.jsonl"}}"#;
    let sandbox = play(&[failed]);
    assert_eq!(
        sandbox.app().screens.resume.load,
        Load::Failed("Failed to read session metadata from /tmp/missing.jsonl".into())
    );
    let listed = r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":[]}}}"#;
    let sandbox = play(&[failed, listed]);
    assert_eq!(sandbox.app().screens.resume.load, Load::Idle);
}

#[test]
fn the_clock_is_a_model_input() {
    let sandbox = play(&[r#"{"step":"screen","dir":"resume","ev":{"clock":1800000000}}"#]);
    assert_eq!(sandbox.app().screens.resume.clock, 1_800_000_000);
}

#[test]
fn the_resume_intent_names_the_thread() {
    let mut sandbox = play(&[]);
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Resume(
        ResumeIntent::Resume(ThreadId::new("b")),
    ))));
    assert_eq!(
        sandbox.app().screens.resume.resuming,
        Some(ThreadId::new("b"))
    );
}

/// The skeleton's seed form (three fake rollouts) still reads: the widened
/// `Row` defaults its new fields.
#[test]
fn the_skeleton_fake_rollout_form_still_seeds() {
    let sandbox = play(&[
        r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":[{"id":"x","preview":"Hello","model_provider":"mock_provider","timestamp":"2025-01-02T12:00:00Z"}]}}}"#,
    ]);
    let row = &sandbox.app().screens.resume.rows[0];
    assert_eq!(row.model_provider, "mock_provider");
    assert_eq!(row.created, None);
}
