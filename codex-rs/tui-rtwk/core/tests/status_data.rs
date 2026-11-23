#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the session status line's data (`status-data`): the facts
//! the host reads (config, cwd, git) enter as one event; the items and their
//! texts are the oracle's (`chatwidget/status_surfaces.rs`:
//! `status_line_value_for_item`, `approval_mode_display`,
//! `status_line_reasoning_effort_label`, `configured_status_line_items`).
//! Warnings: the retained list behind `⚠ N warnings · f2 to view`.

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::chat::StatusFacts;
use codex_tui_core::screens::dialogs::{Decision, Intent as DialogIntent, Warning};
use pretty_assertions::assert_eq;

fn facts(json: &str) -> std::rc::Rc<StatusFacts> {
    let line = format!(r#"{{"step":"screen","dir":"chat","ev":{{"status":{json}}}}}"#);
    let sandbox = Sandbox::play(&line).expect("plays");
    sandbox.app().screens().chat.status.clone()
}

fn items(json: &str) -> Vec<(String, String)> {
    facts(json).items()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
        .collect()
}

// custom_model_display_name_in_status_line_and_fallback: the slug as the
// model is called, the reasoning label after it.
#[test]
fn custom_model_display_name_in_status_line_and_fallback() {
    let got = items(
        r#"{"model":"us.openai.gpt-5.6-luna","effort":"high","items":["model-name","model-with-reasoning"]}"#,
    );
    assert_eq!(
        got,
        pairs(&[
            ("model-name", "us.openai.gpt-5.6-luna"),
            ("model-with-reasoning", "us.openai.gpt-5.6-luna high"),
        ])
    );
    // `model_display_name`: the reserve model has its own name.
    let got = items(r#"{"model":"gpt-reserve","items":["model-name"]}"#);
    assert_eq!(got, pairs(&[("model-name", "Luna Reserve")]));
}

// status_line_model_with_reasoning_context_remaining_footer_snapshot: the
// items that need no session; context-remaining has no value yet and is
// left out, as an item without a value is.
#[test]
fn status_line_model_with_reasoning_context_remaining_footer() {
    let got = items(
        r#"{"model":"gpt-5.4","effort":"xhigh","cwd":"/tmp/project","items":["model-with-reasoning","context-remaining","current-dir"]}"#,
    );
    assert_eq!(
        got,
        pairs(&[
            ("model-with-reasoning", "gpt-5.4 xhigh"),
            ("current-dir", "/tmp/project"),
        ])
    );
}

// status_line_reasoning_effort_label: none and unset read `default`.
#[test]
fn reasoning_effort_label_is_default_without_an_effort() {
    for json in [
        r#"{"model":"m","items":["reasoning"]}"#,
        r#"{"model":"m","effort":"none","items":["reasoning"]}"#,
    ] {
        assert_eq!(items(json), pairs(&[("reasoning", "default")]));
    }
    assert_eq!(
        items(r#"{"model":"m","items":["model-with-reasoning"]}"#),
        pairs(&[("model-with-reasoning", "m default")])
    );
}

// approval_mode_display.
#[test]
fn approval_mode_display_follows_the_policy_and_the_reviewer() {
    let case = |policy: &str, auto: bool| {
        items(&format!(
            r#"{{"approval_policy":"{policy}","auto_review":{auto},"items":["approval-mode"]}}"#
        ))
    };
    assert_eq!(
        case("on-request", true),
        pairs(&[("approval-mode", "Approve for me")])
    );
    assert_eq!(
        case("on-request", false),
        pairs(&[("approval-mode", "Ask for approval")])
    );
    assert_eq!(case("never", false), pairs(&[("approval-mode", "never")]));
    assert_eq!(
        case("untrusted", true),
        pairs(&[("approval-mode", "untrusted")])
    );
}

// The person's screenshot: the order the config gives, project and branch
// from the shell, items with no data (context, limits) not drawn.
#[test]
fn items_keep_the_configured_order_and_skip_what_has_no_value() {
    let json = r#"{"model":"GPT-6.2-Sol","effort":"high","project_root":"rtwk","branch":"main","approval_policy":"on-request","auto_review":true,"items":["model-with-reasoning","project-root","git-branch","approval-mode","context-remaining","context-used","five-hour-limit","weekly-limit"]}"#;
    assert_eq!(
        items(json),
        pairs(&[
            ("model-with-reasoning", "GPT-6.2-Sol high"),
            ("project-root", "rtwk"),
            ("git-branch", "main"),
            ("approval-mode", "Approve for me"),
        ])
    );
    let reordered = r#"{"model":"GPT-6.2-Sol","effort":"high","project_root":"rtwk","branch":"main","items":["git-branch","project","model"]}"#;
    assert_eq!(
        items(reordered),
        pairs(&[
            ("git-branch", "main"),
            ("project", "rtwk"),
            ("model", "GPT-6.2-Sol"),
        ])
    );
}

// configured_status_line_items: no list configured is the default list
// (model-with-reasoning, current-dir, thread-name); an empty list is empty.
#[test]
fn no_configured_list_is_the_default_items() {
    let got = items(r#"{"model":"m","effort":"low","cwd":"~/p"}"#);
    assert_eq!(
        got,
        pairs(&[("model-with-reasoning", "m low"), ("current-dir", "~/p")])
    );
    assert_eq!(items(r#"{"model":"m","items":[]}"#), Vec::new());
}

// Unknown ids are not items (`parse_items_with_invalids`).
#[test]
fn unknown_item_ids_are_dropped() {
    let got = items(r#"{"model":"m","items":["nope","model-name"]}"#);
    assert_eq!(got, pairs(&[("model-name", "m")]));
}

// tui.status_line_use_colors.
#[test]
fn colours_default_on_and_follow_the_config() {
    assert!(facts(r#"{"model":"m"}"#).colors);
    assert!(!facts(r#"{"model":"m","colors":false}"#).colors);
}

// ---- warnings: `⚠ N warnings` counts the retained ones ----

const RETAINED: &str = r#"{"step":"screen","dir":"dialogs","ev":{"kind":"retained","entries":[{"source":"Startup","details":"one"},{"source":"Startup","details":"two"},{"source":"MCP","details":"three"}]}}"#;

fn send(sandbox: &mut Sandbox, intent: DialogIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Dialogs(intent))));
}

fn warning(source: &str, details: &str) -> Warning {
    Warning {
        source: source.into(),
        details: details.into(),
    }
}

fn left(sandbox: &Sandbox) -> Vec<String> {
    sandbox
        .app()
        .screens()
        .dialogs
        .retained
        .iter()
        .map(|w| w.details.clone())
        .collect()
}

#[test]
fn retained_warnings_are_counted() {
    let sandbox = Sandbox::play(RETAINED).expect("plays");
    assert_eq!(sandbox.app().screens().dialogs.retained.len(), 3);
    assert_eq!(left(&sandbox), ["one", "two", "three"]);
}

// A page shown and left is dismissed and no longer counts, a saved one stays;
// pages never shown stay (`WarningsView::close`, `AppEvent::UpdateWarnings`).
// Which pages were shown is the viewer's (`src/sheets/dialogs/warnings.rs`).
#[test]
fn dismissed_warnings_leave_the_retained_list() {
    let mut sandbox = Sandbox::play(RETAINED).expect("plays");
    send(
        &mut sandbox,
        DialogIntent::Decide(Decision {
            dismissed: vec![warning("Startup", "one")],
            saved: Vec::new(),
        }),
    );
    assert_eq!(left(&sandbox), ["two", "three"]);
    send(
        &mut sandbox,
        DialogIntent::Decide(Decision {
            dismissed: vec![warning("MCP", "three")],
            saved: vec![warning("Startup", "two")],
        }),
    );
    assert_eq!(left(&sandbox), ["two"]);
}
