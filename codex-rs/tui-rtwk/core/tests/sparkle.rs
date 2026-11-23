#![allow(clippy::expect_used)]

//! State tests of the composer sparkle's eligibility (core-pass for chunk-status-sparkle):
//! only a fresh primary thread of an Astra model with animations and the starfield effect on is
//! offered the sparkle (oracle `mark_fresh_task_for_sparkle`, `stop_ineligible_sparkle`,
//! `is_sparkle_model`).

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::chat::is_sparkle_model;
use serde_json::{Value, json};

fn facts(model: &str, extra: Value) -> String {
    let mut status = json!({"model": model, "animations": true, "starfield_effect": true});
    for (key, value) in extra.as_object().expect("an object") {
        status[key] = value.clone();
    }
    format!(
        r#"{{"step":"screen","dir":"chat","ev":{}}}"#,
        json!({"status": status})
    )
}

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn eligible(lines: &[String]) -> bool {
    let mut script = vec![SESSION.to_string()];
    script.extend(lines.iter().cloned());
    let sandbox = Sandbox::play(&script.join("\n")).expect("plays");
    let app = sandbox.app();
    let thread = app.active_thread();
    let part = &thread.screen_slots().chat;
    part.sparkle_eligible(&app.screens().chat.status)
}

fn turn_started() -> String {
    r#"{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","items":[],"itemsView":"full","status":"inProgress","error":null}}}}"#.to_string()
}

#[test]
fn the_astra_model_names() {
    assert!(is_sparkle_model("gpt-6-astra"));
    assert!(is_sparkle_model("ASTRA"));
    // an underscore joins words: one word, not `astra`
    assert!(!is_sparkle_model("astra_mini"));
    assert!(!is_sparkle_model("astral"));
    assert!(!is_sparkle_model("gpt-5.6"));
}

#[test]
fn a_fresh_thread_of_an_astra_model_is_offered_the_sparkle() {
    assert!(eligible(&[facts("gpt-6-astra", json!({}))]));
}

#[test]
fn another_model_or_a_switched_off_effect_is_not() {
    assert!(!eligible(&[facts("gpt-5.6", json!({}))]));
    assert!(!eligible(&[facts(
        "gpt-6-astra",
        json!({"starfield_effect": false})
    )]));
    assert!(!eligible(&[facts(
        "gpt-6-astra",
        json!({"animations": false})
    )]));
}

#[test]
fn a_thread_that_ran_a_turn_is_not_fresh() {
    assert!(!eligible(&[
        facts("gpt-6-astra", json!({})),
        turn_started()
    ]));
}
