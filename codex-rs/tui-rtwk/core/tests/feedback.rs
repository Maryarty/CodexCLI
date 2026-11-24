#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/feedback` (oracle `bottom_pane/feedback_view.rs`, `feedback_note_view.rs`,
//! `app/feedback_upload.rs`): the verb `Submit` (what the note step says) becomes one ask of the
//! host, the host's answer is a note of the thread the report was composed in, and the facts the
//! host gave (uploads on, remote workspace, audience) decide what may go out. The steps and the
//! words are the UI's: these tests read data.

use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::feedback::{
    Audience, Destination, Environment, Intent as FeedbackIntent, Note as FeedbackNote, Receipt,
    Upload, classification, reason_of, shows_connectivity,
};
use codex_tui_core::features::{FeatureFact, FeatureIntent, FeatureNote, seed_fact};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::dialogs::FeedbackCategory;
use codex_tui_core::threads::{Item as Entry, Note, NoteKey, Tone};
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

fn tell(app: &mut CodexApp, environment: Environment) {
    run(
        app,
        AppEv::Feature(FeatureFact::Feedback(environment)),
    );
}

fn submit(
    app: &mut CodexApp,
    note: &str,
    include_logs: bool,
    destination: Destination,
) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Feedback(
            FeedbackIntent::Submit {
                category: FeedbackCategory::Bug,
                note: note.to_owned(),
                turn_id: Some("turn-3".to_owned()),
                rollout: Some("/home/.codex/sessions/rollout-1.jsonl".to_owned()),
                include_logs,
                destination,
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

fn one_upload(fx: &AppFx) -> (u64, Upload) {
    let asks = asks(fx);
    assert_eq!(asks.len(), 1, "{asks:?}");
    let (request, Ask::FeedbackUpload(upload)) = asks[0].clone() else {
        panic!("not a feedback upload: {:?}", asks[0]);
    };
    (request, upload)
}

fn said(app: &CodexApp) -> Vec<(FeedbackNote, Tone)> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: NoteKey::Feature(FeatureNote::Feedback(note)),
                tone,
            }) => Some((note.clone(), *tone)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_submit_is_one_ask_with_the_note_trimmed_and_the_thread_in_use() {
    let mut app = ready();
    let (_, upload) = one_upload(&submit(&mut app, "  it broke \n", true, Destination::OpenAi));
    assert_eq!(
        upload,
        Upload {
            category: FeedbackCategory::Bug,
            reason: Some("it broke".to_owned()),
            thread: Some(ThreadId::new(T)),
            turn_id: Some("turn-3".to_owned()),
            include_logs: true,
            rollout: Some("/home/.codex/sessions/rollout-1.jsonl".to_owned()),
            destination: Destination::OpenAi,
        }
    );
}

#[test]
fn an_empty_note_is_no_reason() {
    let mut app = ready();
    let (_, upload) = one_upload(&submit(&mut app, " \n ", false, Destination::OpenAi));
    assert_eq!(upload.reason, None);
    assert_eq!(reason_of("x"), Some("x".to_owned()));
}

#[test]
fn an_uploaded_report_says_who_it_is_for_in_the_thread_it_came_from() {
    let mut app = ready();
    tell(
        &mut app,
        Environment {
            audience: Audience::OpenAiEmployee,
            ..Environment::default()
        },
    );
    let (request, _) = one_upload(&submit(&mut app, "", true, Destination::OpenAi));
    run(
        &mut app,
        AppEv::Effect {
            request,
            done: Done::Feedback(Ok(Receipt::Uploaded {
                thread_id: "filed-1".to_owned(),
            })),
        },
    );
    assert_eq!(
        said(&app),
        vec![(
            FeedbackNote::Uploaded {
                category: FeedbackCategory::Bug,
                include_logs: true,
                thread_id: "filed-1".to_owned(),
                audience: Audience::OpenAiEmployee,
            },
            Tone::Info
        )]
    );
}

#[test]
fn a_saved_zip_and_a_failure_are_lines_of_the_thread() {
    let mut app = ready();
    for done in [
        Done::Feedback(Ok(Receipt::Saved {
            path: "/home/.codex/feedback/a.zip".to_owned(),
        })),
        Done::Feedback(Err("Failed to upload feedback: offline".to_owned())),
    ] {
        let (request, _) = one_upload(&submit(&mut app, "", false, Destination::OpenAi));
        run(&mut app, AppEv::Effect { request, done });
    }
    assert_eq!(
        said(&app),
        vec![
            (
                FeedbackNote::Saved("/home/.codex/feedback/a.zip".to_owned()),
                Tone::Info
            ),
            (
                FeedbackNote::Failed("Failed to upload feedback: offline".to_owned()),
                Tone::Error
            ),
        ]
    );
}

#[test]
fn a_zip_needs_a_local_server_and_goes_out_no_ask() {
    let mut app = ready();
    tell(
        &mut app,
        Environment {
            remote: true,
            ..Environment::default()
        },
    );
    let fx = submit(&mut app, "", false, Destination::LocalZip);
    assert!(asks(&fx).is_empty());
    assert_eq!(
        said(&app),
        vec![(FeedbackNote::ZipNeedsLocalServer, Tone::Info)]
    );
}

#[test]
fn disabled_uploads_leave_only_the_zip() {
    let mut app = ready();
    tell(
        &mut app,
        Environment {
            uploads: false,
            ..Environment::default()
        },
    );
    assert!(asks(&submit(&mut app, "", true, Destination::OpenAi)).is_empty());
    let (_, upload) = one_upload(&submit(&mut app, "", true, Destination::LocalZip));
    assert_eq!(upload.destination, Destination::LocalZip);
}

#[test]
fn a_remote_workspace_without_uploads_says_nothing_can_be_delivered() {
    let mut app = ready();
    let open = |app: &mut CodexApp| {
        run(
            app,
            AppEv::Intent(Intent::Feature(FeatureIntent::Feedback(
                FeedbackIntent::Open {
                    source: Some(ThreadId::new(T)),
                },
            ))),
        )
    };
    open(&mut app);
    assert!(said(&app).is_empty());
    tell(
        &mut app,
        Environment {
            remote: true,
            uploads: false,
            ..Environment::default()
        },
    );
    open(&mut app);
    assert_eq!(said(&app), vec![(FeedbackNote::Unavailable, Tone::Info)]);
}

#[test]
fn another_ask_answer_is_not_a_feedback_note() {
    let mut app = ready();
    run(
        &mut app,
        AppEv::Effect {
            request: 5,
            done: Done::Feedback(Err("x".to_owned())),
        },
    );
    assert!(said(&app).is_empty());
}

#[test]
fn the_wire_names_and_the_connectivity_rule_are_the_oracles() {
    let names: Vec<_> = FeedbackCategory::ALL.map(classification).into();
    assert_eq!(
        names,
        ["bug", "bad_result", "good_result", "safety_check", "other"]
    );
    assert!(shows_connectivity(FeedbackCategory::Bug, 1));
    assert!(!shows_connectivity(FeedbackCategory::Bug, 0));
    assert!(!shows_connectivity(FeedbackCategory::GoodResult, 3));
}

#[test]
fn a_script_seeds_the_environment() {
    let fact = seed_fact(
        "feedback",
        serde_json::json!({"audience": "open_ai_employee", "uploads": false, "remote": true}),
    )
    .unwrap();
    assert_eq!(
        fact,
        FeatureFact::Feedback(Environment {
            audience: Audience::OpenAiEmployee,
            uploads: false,
            remote: true,
        })
    );
    let bare = seed_fact("feedback", serde_json::json!({})).unwrap();
    assert_eq!(bare, FeatureFact::Feedback(Environment::default()));
}
