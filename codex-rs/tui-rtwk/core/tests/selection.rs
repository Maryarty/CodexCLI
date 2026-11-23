#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The copy of a transcript selection (oracle `transcript_view/selection.rs`
//! `copy_selected_text_with`, `composer_gap.rs` `show_copy_feedback`): the view's text goes to
//! the host as an `Ask::Copy`, and the host's answers are the feedback the composer gap shows.
//! The words are the view's: these tests read data.

use codex_tui_core::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::FeatureIntent;
use codex_tui_core::features::selection::{Feedback, Intent as Selection};
use pretty_assertions::assert_eq;

fn copy(app: &mut CodexApp, text: &str) -> u64 {
    let mut fx = AppFx::default();
    apply_app(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(Selection::Copy {
            text: text.to_owned(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
        }))),
        &mut fx,
    );
    let [Command::Effect { request, ask, .. }] = fx.commands() else {
        panic!("one ask of the host: {:?}", fx.commands());
    };
    assert_eq!(
        *ask,
        Ask::Copy {
            text: text.to_owned(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
        }
    );
    *request
}

fn answer(app: &mut CodexApp, request: u64, done: Done) {
    apply_app(app, AppEv::Effect { request, done }, &mut AppFx::default());
}

fn feedback(app: &CodexApp) -> Option<Feedback> {
    app.features().selection.feedback.clone()
}

#[test]
fn nothing_is_shown_until_the_host_answers() {
    let mut app = CodexApp::new();
    copy(&mut app, "row 03");
    assert_eq!(feedback(&app), None);
}

#[test]
fn a_confirmed_copy_is_counted_in_what_was_copied_not_in_bytes() {
    let mut app = CodexApp::new();
    let request = copy(&mut app, "größe ✓");
    answer(&mut app, request, Done::Copy(Ok(CopyStatus::Confirmed)));
    let got = feedback(&app).expect("feedback");
    assert_eq!(got.result, Ok(CopyStatus::Confirmed));
    assert_eq!(got.characters, 7);
}

#[test]
fn a_failure_keeps_only_that_it_failed() {
    let mut app = CodexApp::new();
    let request = copy(&mut app, "x");
    answer(&mut app, request, Done::Copy(Err("no tool".into())));
    assert_eq!(feedback(&app).expect("feedback").result, Err(()));
}

#[test]
fn a_pending_copy_that_ends_is_a_new_answer() {
    let mut app = CodexApp::new();
    let request = copy(&mut app, "abc");
    answer(&mut app, request, Done::Copy(Ok(CopyStatus::Pending(1))));
    let pending = feedback(&app).expect("feedback");
    assert_eq!(pending.result, Ok(CopyStatus::Pending(1)));
    answer(&mut app, request, Done::Copy(Ok(CopyStatus::Confirmed)));
    let done = feedback(&app).expect("feedback");
    assert_eq!(done.result, Ok(CopyStatus::Confirmed));
    assert!(done.version > pending.version);
}

#[test]
fn a_busy_answer_leaves_the_copy_under_way_showing() {
    let mut app = CodexApp::new();
    let first = copy(&mut app, "abc");
    answer(&mut app, first, Done::Copy(Ok(CopyStatus::Pending(1))));
    let under_way = feedback(&app);
    let second = copy(&mut app, "def");
    answer(&mut app, second, Done::Copy(Ok(CopyStatus::Busy)));
    assert_eq!(feedback(&app), under_way);
}

#[test]
fn busy_alone_is_shown() {
    let mut app = CodexApp::new();
    let request = copy(&mut app, "abc");
    answer(&mut app, request, Done::Copy(Ok(CopyStatus::Busy)));
    assert_eq!(
        feedback(&app).expect("feedback").result,
        Ok(CopyStatus::Busy)
    );
}
