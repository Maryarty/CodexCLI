#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S23 (small dialogs): the warnings viewer's pages, keep and
//! dismiss decisions (`bottom_pane/warnings_view_tests.rs`), the shortcut
//! overlay's two pages (`shortcut_overlay_tests.rs`) and the feedback flow
//! (`feedback_view.rs`: `feedback_selection_params`, `feedback_consent_params`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::dialogs::{
    Feedback, FeedbackCategory, Intent as DialogIntent, NoteRequest, Open, PageIntent,
    ShortcutView, Warning,
};
use pretty_assertions::assert_eq;

/// `warnings_view_tests.rs::entries()` plus the third page of
/// `warnings_dismiss_only_drawn_pages_after_navigation_skips_a_frame`.
const WARNINGS: &str = r#"{"step":"screen","dir":"dialogs","ev":{"kind":"warnings","entries":[{"source":"MCP · example","details":"MCP example could not connect\nSign in again using codex mcp login example"},{"source":"Startup","details":"Unknown setting `old_option`\nRemove it from config.toml"},{"source":"Startup","details":"Not yet viewed"}]}}"#;

fn send(sandbox: &mut Sandbox, intent: DialogIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Dialogs(intent))));
}

fn page(sandbox: &mut Sandbox, intent: PageIntent) {
    send(sandbox, DialogIntent::Page(intent));
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

fn open_warnings(sandbox: &Sandbox) -> codex_tui_core::screens::dialogs::Warnings {
    match &sandbox.app().screens().dialogs.open {
        Some(Open::Warnings(warnings)) => warnings.clone(),
        other => panic!("warnings expected, got {other:?}"),
    }
}

#[test]
fn warning_pages_copy_only_the_current_diagnostic() {
    let mut sandbox = Sandbox::play(WARNINGS).expect("plays");
    let want = entries();
    for (navigation, index) in [(PageIntent::Next, 1), (PageIntent::Previous, 0)] {
        page(&mut sandbox, navigation);
        assert_eq!(open_warnings(&sandbox).current, index);
        page(&mut sandbox, PageIntent::Copy);
        assert_eq!(
            sandbox.app().screens().dialogs.copied.last(),
            Some(&want[index].details)
        );
    }
    send(&mut sandbox, DialogIntent::Close);
    assert!(sandbox.app().screens().dialogs.open.is_none());
}

#[test]
fn navigation_stops_at_the_first_and_the_last_page() {
    let mut sandbox = Sandbox::play(WARNINGS).expect("plays");
    page(&mut sandbox, PageIntent::Previous);
    assert_eq!(open_warnings(&sandbox).current, 0);
    for _ in 0..5 {
        page(&mut sandbox, PageIntent::Next);
    }
    assert_eq!(open_warnings(&sandbox).current, 2);
}

#[test]
fn warnings_dismiss_the_pages_that_were_shown_on_close() {
    let mut sandbox = Sandbox::play(WARNINGS).expect("plays");
    page(&mut sandbox, PageIntent::Next);
    page(&mut sandbox, PageIntent::Next);
    page(&mut sandbox, PageIntent::Previous);
    send(&mut sandbox, DialogIntent::Close);
    let want = entries();
    let decided = &sandbox.app().screens().dialogs.decided;
    assert_eq!(decided.len(), 1);
    assert_eq!(decided[0].dismissed, want);
    assert!(decided[0].kept.is_empty());
}

#[test]
fn a_page_never_shown_is_not_dismissed() {
    let mut sandbox = Sandbox::play(WARNINGS).expect("plays");
    send(&mut sandbox, DialogIntent::Close);
    let want = entries();
    let decided = &sandbox.app().screens().dialogs.decided;
    assert_eq!(decided[0].dismissed, vec![want[0].clone()]);
}

#[test]
fn warnings_keep_current_and_next_without_dismissing_unvisited_pages() {
    let mut sandbox = Sandbox::play(WARNINGS).expect("plays");
    page(&mut sandbox, PageIntent::Next);
    page(&mut sandbox, PageIntent::KeepAndNext);
    assert_eq!(open_warnings(&sandbox).current, 2);
    page(&mut sandbox, PageIntent::Previous);
    send(&mut sandbox, DialogIntent::Close);
    // The oracle test keeps page 2 of 3 and dismisses page 1 only: here every
    // page reached was shown, so page 3 is dismissed too (delta N03).
    let want = entries();
    let decided = &sandbox.app().screens().dialogs.decided;
    assert_eq!(decided[0].kept, vec![want[1].clone()]);
    assert_eq!(decided[0].dismissed, vec![want[0].clone(), want[2].clone()]);
}

#[test]
fn keep_on_the_last_page_closes_the_viewer() {
    let mut sandbox = Sandbox::play(WARNINGS).expect("plays");
    page(&mut sandbox, PageIntent::Next);
    page(&mut sandbox, PageIntent::Next);
    page(&mut sandbox, PageIntent::KeepAndNext);
    assert!(sandbox.app().screens().dialogs.open.is_none());
    let decided = &sandbox.app().screens().dialogs.decided;
    assert_eq!(decided[0].kept, vec![entries()[2].clone()]);
}

#[test]
fn no_decision_is_recorded_when_nothing_was_shown_or_kept() {
    let mut sandbox =
        Sandbox::play(r#"{"step":"screen","dir":"dialogs","ev":{"kind":"warnings","entries":[]}}"#)
            .expect("plays");
    send(&mut sandbox, DialogIntent::Close);
    assert!(sandbox.app().screens().dialogs.decided.is_empty());
}

#[test]
fn the_shortcut_overlay_swaps_between_the_list_and_the_keyboard() {
    let mut sandbox =
        Sandbox::play(r#"{"step":"screen","dir":"dialogs","ev":{"kind":"shortcuts"}}"#)
            .expect("plays");
    assert_eq!(
        sandbox.app().screens().dialogs.open,
        Some(Open::Shortcuts(ShortcutView::List))
    );
    send(&mut sandbox, DialogIntent::ToggleKeyboard);
    assert_eq!(
        sandbox.app().screens().dialogs.open,
        Some(Open::Shortcuts(ShortcutView::Keyboard))
    );
    send(&mut sandbox, DialogIntent::ToggleKeyboard);
    assert_eq!(
        sandbox.app().screens().dialogs.open,
        Some(Open::Shortcuts(ShortcutView::List))
    );
    send(&mut sandbox, DialogIntent::Close);
    assert!(sandbox.app().screens().dialogs.open.is_none());
}

#[test]
fn a_category_leads_to_the_consent_step_and_the_answer_to_the_note() {
    let mut sandbox =
        Sandbox::play(r#"{"step":"screen","dir":"dialogs","ev":{"kind":"feedback_categories"}}"#)
            .expect("plays");
    send(&mut sandbox, DialogIntent::PickCategory(2));
    let Some(Open::Feedback(Feedback::Consent { category, .. })) =
        sandbox.app().screens().dialogs.open.clone()
    else {
        panic!("the consent step");
    };
    assert_eq!(category, FeedbackCategory::GoodResult);
    send(&mut sandbox, DialogIntent::Consent(false));
    assert!(sandbox.app().screens().dialogs.open.is_none());
    assert_eq!(
        sandbox.app().screens().dialogs.note,
        Some(NoteRequest {
            category: FeedbackCategory::GoodResult,
            include_logs: false
        })
    );
}

#[test]
fn a_category_past_the_rows_changes_nothing() {
    let mut sandbox =
        Sandbox::play(r#"{"step":"screen","dir":"dialogs","ev":{"kind":"feedback_categories"}}"#)
            .expect("plays");
    send(&mut sandbox, DialogIntent::PickCategory(9));
    assert_eq!(
        sandbox.app().screens().dialogs.open,
        Some(Open::Feedback(Feedback::Category))
    );
}

#[test]
fn closing_the_consent_step_asks_for_no_note() {
    let mut sandbox = Sandbox::play(
        r#"{"step":"screen","dir":"dialogs","ev":{"kind":"feedback_consent","category":"bug","can_save":true,"rollout":"rollout.jsonl"}}"#,
    )
    .expect("plays");
    send(&mut sandbox, DialogIntent::Close);
    assert!(sandbox.app().screens().dialogs.open.is_none());
    assert!(sandbox.app().screens().dialogs.note.is_none());
}

#[test]
fn an_unknown_seed_kind_is_a_bad_seed() {
    let err = Sandbox::play(r#"{"step":"screen","dir":"dialogs","ev":{"kind":"nope"}}"#)
        .err()
        .expect("fails");
    assert_eq!(err.line, 1);
}
