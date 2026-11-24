#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The external editor (oracle `external_editor.rs`, `app/input.rs` `launch_external_editor`):
//! the verb `Open { thread, draft }` asks the host to run `$VISUAL` / `$EDITOR` on the draft, one
//! editor at a time; what it leaves is the one-shot `State::edited` of the thread's composer, and
//! a failure is a line of the transcript. The words are the view's: these tests read data.

use codex_tui_core::app::effects::{Ask, Done, EditorFailure, FEATURE_REQUEST_BIT};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::external_editor::{Intent as EditorIntent, Note as EditorNote};
use codex_tui_core::features::{FeatureIntent, FeatureNote};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::threads::{Item as Entry, Note, NoteKey};
use pretty_assertions::assert_eq;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn open(app: &mut CodexApp, draft: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::ExternalEditor(
            EditorIntent::Open {
                thread: ThreadId::new(T),
                draft: draft.to_owned(),
            },
        ))),
    )
}

fn asks(fx: &AppFx) -> Vec<(u64, Ask)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
            _ => None,
        })
        .collect()
}

fn notes(app: &CodexApp) -> Vec<EditorNote> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: NoteKey::Feature(FeatureNote::ExternalEditor(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_draft_goes_to_the_host_with_the_threads_directory() {
    let mut app = ready();
    let fx = open(&mut app, "fix the\nbug");
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert!(asks[0].0 & FEATURE_REQUEST_BIT != 0);
    let Ask::ExternalEditor { draft, .. } = &asks[0].1 else {
        panic!("not an editor ask: {:?}", asks[0]);
    };
    assert_eq!(draft, "fix the\nbug");
    assert!(app.features().external_editor.running);
}

#[test]
fn a_second_open_waits_for_the_first_editor() {
    let mut app = ready();
    assert_eq!(asks(&open(&mut app, "one")).len(), 1);
    assert_eq!(asks(&open(&mut app, "two")).len(), 0);
}

#[test]
fn a_thread_that_is_not_held_has_no_editor() {
    let mut app = ready();
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Feature(FeatureIntent::ExternalEditor(
            EditorIntent::Open {
                thread: ThreadId::new("another"),
                draft: "x".to_owned(),
            },
        ))),
    );
    assert_eq!(asks(&fx).len(), 0);
    assert!(!app.features().external_editor.running);
}

#[test]
fn the_edited_text_is_trimmed_kept_once_and_the_key_is_free_again() {
    let mut app = ready();
    let request = asks(&open(&mut app, "draft"))[0].0;
    run(
        &mut app,
        AppEv::Effect {
            request,
            done: Done::ExternalEditor(Ok("better draft\n\n  ".to_owned())),
        },
    );
    let state = &app.features().external_editor;
    assert!(!state.running);
    let edited = state.edited.clone().expect("an edit");
    assert_eq!(edited.text, "better draft");
    assert_eq!(edited.thread, ThreadId::new(T));
    run(
        &mut app,
        AppEv::Intent(Intent::Feature(FeatureIntent::ExternalEditor(
            EditorIntent::Applied { seq: edited.seq },
        ))),
    );
    assert_eq!(app.features().external_editor.edited, None);
    assert_eq!(asks(&open(&mut app, "again")).len(), 1);
}

#[test]
fn a_missing_editor_and_a_failed_one_are_lines_of_the_transcript() {
    let mut app = ready();
    for failure in [
        EditorFailure::Missing,
        EditorFailure::Failed("editor exited with status 1".to_owned()),
    ] {
        let request = asks(&open(&mut app, "x"))[0].0;
        run(
            &mut app,
            AppEv::Effect {
                request,
                done: Done::ExternalEditor(Err(failure)),
            },
        );
    }
    assert_eq!(
        notes(&app),
        vec![
            EditorNote::Missing,
            EditorNote::Failed("editor exited with status 1".to_owned()),
        ]
    );
    assert_eq!(app.features().external_editor.edited, None);
}
