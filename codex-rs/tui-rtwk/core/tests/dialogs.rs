#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S23 (small dialogs): what the domain keeps of them (ADR-013). The warnings
//! viewer's pages, the shortcut overlay's two pages and the feedback steps are the sheet's own
//! (`src/sheets/dialogs`, its unit tests and `tests/sheets/dialogs.rs`); the model takes the
//! verbs of the user's decisions (`bottom_pane/warnings_view_tests.rs`: the dismissed set,
//! `feedback_view.rs`: the note).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::dialogs::{
    Attachments, Decision, FeedbackCategory, Intent as DialogIntent, NoteRequest, Warning,
};
use pretty_assertions::assert_eq;

const RETAINED: &str = r#"{"step":"screen","dir":"dialogs","ev":{"kind":"retained","entries":[{"source":"MCP · example","details":"MCP example could not connect\nSign in again using codex mcp login example"},{"source":"Startup","details":"Unknown setting `old_option`\nRemove it from config.toml"},{"source":"Startup","details":"Not yet viewed"}]}}"#;

fn send(sandbox: &mut Sandbox, intent: DialogIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Dialogs(intent))));
}

fn warning(source: &str, details: &str) -> Warning {
    Warning {
        source: source.into(),
        details: details.into(),
    }
}

fn entries() -> Vec<Warning> {
    vec![
        warning(
            "MCP · example",
            "MCP example could not connect\nSign in again using codex mcp login example",
        ),
        warning(
            "Startup",
            "Unknown setting `old_option`\nRemove it from config.toml",
        ),
        warning("Startup", "Not yet viewed"),
    ]
}

#[test]
fn the_retained_warnings_are_the_seeded_ones_each_text_once() {
    let mut sandbox = Sandbox::play(RETAINED).expect("plays");
    assert_eq!(sandbox.app().screens().dialogs.retained, entries());
    // The same text again is the same warning (`warning_entries` joins duplicates).
    sandbox.send(AppEv::Screen(codex_tui_core::screens::ScreenEv::Dialogs(
        codex_tui_core::screens::dialogs::Ev::Retained(vec![entries()[1].clone()]),
    )));
    assert_eq!(sandbox.app().screens().dialogs.retained.len(), 3);
}

#[test]
fn a_copied_page_is_recorded_for_the_clipboard() {
    let mut sandbox = Sandbox::play(RETAINED).expect("plays");
    send(
        &mut sandbox,
        DialogIntent::CopyWarning(entries()[1].details.clone()),
    );
    assert_eq!(
        sandbox.app().screens().dialogs.copied.last(),
        Some(&entries()[1].details)
    );
}

#[test]
fn a_decision_dismisses_its_pages_and_keeps_the_saved_ones_retained() {
    let mut sandbox = Sandbox::play(RETAINED).expect("plays");
    let want = entries();
    let decision = Decision {
        dismissed: vec![want[0].clone(), want[2].clone()],
        saved: vec![want[1].clone()],
    };
    send(&mut sandbox, DialogIntent::Decide(decision.clone()));
    let dialogs = &sandbox.app().screens().dialogs;
    assert_eq!(dialogs.decided, vec![decision]);
    assert_eq!(dialogs.retained, vec![want[1].clone()]);
}

#[test]
fn no_decision_is_recorded_for_nothing_shown_or_saved() {
    let mut sandbox = Sandbox::play(RETAINED).expect("plays");
    send(
        &mut sandbox,
        DialogIntent::Decide(Decision {
            dismissed: Vec::new(),
            saved: Vec::new(),
        }),
    );
    assert!(sandbox.app().screens().dialogs.decided.is_empty());
    assert_eq!(sandbox.app().screens().dialogs.retained.len(), 3);
}

#[test]
fn the_consent_answer_is_the_note_the_flow_ends_in() {
    let mut sandbox = Sandbox::play("# nothing").expect("plays");
    let note = NoteRequest {
        category: FeedbackCategory::GoodResult,
        include_logs: false,
    };
    send(&mut sandbox, DialogIntent::RequestNote(note.clone()));
    assert_eq!(sandbox.app().screens().dialogs.note, Some(note));
}

#[test]
fn the_hosts_attachments_are_a_fact_of_the_model() {
    let sandbox = Sandbox::play(
        r#"{"step":"screen","dir":"dialogs","ev":{"kind":"attachments","can_save":true,"rollout":"rollout.jsonl"}}"#,
    )
    .expect("plays");
    assert_eq!(
        sandbox.app().screens().dialogs.attachments,
        Attachments {
            can_save: true,
            windows_sandbox_log: false,
            rollout: Some("rollout.jsonl".into()),
            auto_review: None,
        }
    );
}

#[test]
fn an_unknown_seed_kind_is_a_bad_seed() {
    let err = Sandbox::play(r#"{"step":"screen","dir":"dialogs","ev":{"kind":"nope"}}"#)
        .err()
        .expect("fails");
    assert_eq!(err.line, 1);
}

#[test]
fn the_views_own_pages_are_no_seed_kinds_any_more() {
    for kind in ["shortcuts", "feedback_categories"] {
        let line = format!(r#"{{"step":"screen","dir":"dialogs","ev":{{"kind":"{kind}"}}}}"#);
        assert!(Sandbox::play(&line).is_err(), "{kind}");
    }
}
