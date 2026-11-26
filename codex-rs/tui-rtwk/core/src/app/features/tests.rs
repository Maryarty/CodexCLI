//! The door of a feature to the host (`AppCx::effect`, ADR-012), end to end through the app. The
//! ctx of a real feature is built here because `Out` and `Effects` are typed by the list's roots
//! (the rules let this file and the framework's probes name one: `ROOT_TESTS`).

use super::*;
use crate::app::effects::{Ask, CopyStatus, is_feature_request};
use crate::app::{AppEv, Intent, apply_app};
use crate::features::{FeatureNote, goal, transcript_links};
use crate::threads::{Item, Origin};

fn ctx() -> FeatureCtx {
    FeatureCtx::Goal(goal::Ctx {
        thread: ThreadId::new("t1"),
        flow: goal::Flow::Show,
    })
}

/// What `AppCx::effect` makes of an ask: the command the host runs, with its request number.
fn ask_host(app: &mut CodexApp, ask: Ask) -> (u64, Ask) {
    let mut fx = AppFx::default();
    carry(app, Out::Effect { ask, ctx: ctx() }, &mut fx);
    let [Command::Effect { request, ask }] = fx.commands() else {
        panic!("one ask of the host: {:?}", fx.commands());
    };
    (*request, ask.clone())
}

fn answer(app: &mut CodexApp, request: u64, done: Done) {
    apply_app(app, AppEv::Effect { request, done }, &mut AppFx::default());
}

/// The app chooses the request number of a feature's ask (never a screen's), the host runs the
/// ask as it is, and the answers are the feature's: a copy's `Pending` does not end the ask, the
/// last answer does, and a late one is dropped.
#[test]
fn a_features_host_ask_goes_out_under_the_apps_number_and_its_answers_are_the_features() {
    let mut app = CodexApp::new();
    let (first, ask) = ask_host(&mut app, Ask::ClipboardText);
    let (second, _) = ask_host(&mut app, Ask::ClipboardText);
    assert_eq!(
        ask,
        Ask::ClipboardText,
        "the host's own capability, unchanged"
    );
    assert!(is_feature_request(first) && is_feature_request(second));
    assert_ne!(first, second);
    assert_eq!(app.effects.feature_asks_waiting(), 2);

    answer(&mut app, first, Done::Copy(Ok(CopyStatus::Pending(7))));
    assert_eq!(
        app.effects.feature_asks_waiting(),
        2,
        "a pending copy waits"
    );
    answer(&mut app, first, Done::Copy(Ok(CopyStatus::Confirmed)));
    assert_eq!(
        app.effects.feature_asks_waiting(),
        1,
        "its last answer ends it"
    );
    answer(&mut app, first, Done::ClipboardText(Ok("late".to_owned())));
    assert_eq!(
        app.effects.feature_asks_waiting(),
        1,
        "a late one is dropped"
    );
    assert!(
        app.effect(first).is_none()
            && app.effect(second).is_none()
            && app.latest_effect().is_none(),
        "no screen reads a feature's answer"
    );
}

/// The asks of a screen keep their own path: the answer is kept for the screen to read.
#[test]
fn a_screens_host_ask_is_still_kept_for_the_screen() {
    let mut app = CodexApp::new();
    answer(&mut app, 5, Done::ClipboardText(Ok("text".to_owned())));
    assert_eq!(
        app.effect(5),
        Some(&Done::ClipboardText(Ok("text".to_owned())))
    );
}

#[test]
fn browser_failure_belongs_to_the_source_thread_even_after_switching() {
    let first = ThreadId::new("first");
    let next = ThreadId::new("next");
    let url = "https://example.org/guide";
    let mut app = CodexApp::new();
    app.threads.open(&first, Origin::Resumed);
    app.threads.open(&next, Origin::Resumed);

    let click = |app: &mut CodexApp| {
        let mut fx = AppFx::default();
        apply_app(
            app,
            AppEv::Intent(Intent::Feature(
                transcript_links::Intent {
                    thread: first.clone(),
                    url: url.into(),
                }
                .into(),
            )),
            &mut fx,
        );
        let [
            Command::Effect {
                request,
                ask: Ask::OpenUrl { url: opened },
            },
        ] = fx.commands()
        else {
            panic!("expected one browser effect");
        };
        assert_eq!(opened, url);
        *request
    };
    let has_failure = |app: &CodexApp, id: &ThreadId| {
        app.thread(id).is_some_and(|thread| {
            thread.transcript().iter().any(|(_, item)| {
                matches!(
                    item,
                    Item::Note(note)
                        if matches!(&note.key, NoteKey::Feature(FeatureNote::TranscriptLinks(failure))
                            if failure.url == url && failure.error == "browser unavailable")
                )
            })
        })
    };
    let fail = |app: &mut CodexApp, request| {
        answer(
            app,
            request,
            Done::Opened(Err("browser unavailable".into())),
        );
    };

    let show_thread = |app: &mut CodexApp, id: &ThreadId| {
        apply_app(
            app,
            AppEv::Intent(Intent::OpenThread(id.clone())),
            &mut AppFx::default(),
        );
        assert_eq!(app.session().active.as_ref(), Some(id));
    };

    show_thread(&mut app, &first);
    let switched = click(&mut app);
    let closed = click(&mut app);
    show_thread(&mut app, &next);
    fail(&mut app, switched);
    assert!(has_failure(&app, &first));
    assert!(!has_failure(&app, &next));

    app.threads.forget(&first);
    let chat_notes = app.screens().chat.notes.clone();
    fail(&mut app, closed);
    assert_eq!(app.screens().chat.notes, chat_notes);
    assert!(!has_failure(&app, &next));
}
