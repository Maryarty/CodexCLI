#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/feedback` (oracle `bottom_pane/feedback_view.rs`, `feedback_note_view.rs`,
//! `app/feedback_upload.rs`): the verb `Submit` (what the note step says) becomes the slice's
//! own RPC (`feedback/upload`, or `fs/createDirectory` then `feedback/save` for a ZIP), the
//! answer is a note of the thread the report was composed in, and the facts the host gave
//! (uploads on, remote workspace, home, audience) decide what may go out. The steps and the
//! words are the UI's: these tests read data.

use codex_app_server_protocol::{ClientRequest, RequestId};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command as AppCommand, Intent, apply_app};
use codex_tui_core::features::feedback::{
    Audience, Command, Destination, Fact, Intent as FeedbackIntent, Note as FeedbackNote,
    Reply as FeedbackReply, classification, reason_of, shows_connectivity,
};
use codex_tui_core::features::{
    FeatureCommand, FeatureFact, FeatureIntent, FeatureNote, FeatureReply, seed_fact,
};
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
            AppCommand::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn tell(app: &mut CodexApp, fact: Fact) {
    run(app, AppEv::Feature(FeatureFact::Feedback(fact)));
}

const HOME: &str = "/home/x/.codex";
const FILE: &str = "codex-diagnostics-2026-11-24T10-00-00Z-1a2b3c4d.zip";

fn submit(app: &mut CodexApp, note: &str, include_logs: bool, destination: Destination) -> AppFx {
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
                file_name: FILE.to_owned(),
            },
        ))),
    )
}

fn asks(fx: &AppFx) -> Vec<(Ticket, Command)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            AppCommand::Feature {
                ticket,
                command: FeatureCommand::Feedback(command),
                ..
            } => Some((*ticket, command.clone())),
            _ => None,
        })
        .collect()
}

fn one(fx: &AppFx) -> (Ticket, Command) {
    let asks = asks(fx);
    assert_eq!(asks.len(), 1, "{:?}", fx.commands());
    asks[0].clone()
}

fn answer(app: &mut CodexApp, ticket: Ticket, reply: FeedbackReply) -> AppFx {
    run(
        app,
        AppEv::Reply(ticket, Reply::feature(FeatureReply::Feedback(reply))),
    )
}

fn fail(app: &mut CodexApp, ticket: Ticket, message: &str) -> AppFx {
    run(app, AppEv::Reply(ticket, Reply::failed(message)))
}

fn wire(command: &Command) -> ClientRequest {
    codex_tui_core::features::wire(
        &FeatureCommand::Feedback(command.clone()),
        &ThreadId::new(T),
        RequestId::Integer(1),
    )
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
fn a_submit_is_one_upload_with_the_note_trimmed_and_the_thread_in_use() {
    let mut app = ready();
    let (_, command) = one(&submit(
        &mut app,
        "  it broke \n",
        true,
        Destination::OpenAi,
    ));
    let ClientRequest::FeedbackUpload { params, .. } = wire(&command) else {
        panic!("not an upload: {command:?}");
    };
    assert_eq!(params.classification, "bug");
    assert_eq!(params.reason.as_deref(), Some("it broke"));
    assert_eq!(params.thread_id.as_deref(), Some(T));
    assert!(params.include_logs);
    assert_eq!(
        params.extra_log_files,
        Some(vec!["/home/.codex/sessions/rollout-1.jsonl".into()])
    );
    assert_eq!(
        params.tags,
        Some([("turn_id".to_owned(), "turn-3".to_owned())].into())
    );
}

#[test]
fn an_empty_note_is_no_reason_and_no_logs_attach_no_rollout() {
    let mut app = ready();
    let (_, command) = one(&submit(&mut app, " \n ", false, Destination::OpenAi));
    let ClientRequest::FeedbackUpload { params, .. } = wire(&command) else {
        panic!("not an upload");
    };
    assert_eq!((params.reason, params.extra_log_files), (None, None));
    assert_eq!(reason_of("x"), Some("x".to_owned()));
}

#[test]
fn an_uploaded_report_says_who_it_is_for_in_the_thread_it_came_from() {
    let mut app = ready();
    tell(&mut app, Fact::account(Some("dev@openai.com")));
    let (ticket, _) = one(&submit(&mut app, "", true, Destination::OpenAi));
    answer(
        &mut app,
        ticket,
        FeedbackReply::Uploaded {
            report: "filed-1".to_owned(),
        },
    );
    assert_eq!(
        said(&app),
        vec![(
            FeedbackNote::Uploaded {
                category: FeedbackCategory::Bug,
                include_logs: true,
                report: "filed-1".to_owned(),
                audience: Audience::OpenAiEmployee,
            },
            Tone::Info
        )]
    );
    // An account of another domain, or an API key, is external.
    tell(&mut app, Fact::account(Some("dev@example.com")));
    tell(&mut app, Fact::account(None));
    let (ticket, _) = one(&submit(&mut app, "", false, Destination::OpenAi));
    answer(
        &mut app,
        ticket,
        FeedbackReply::Uploaded {
            report: "filed-2".to_owned(),
        },
    );
    assert!(matches!(
        said(&app).last(),
        Some((
            FeedbackNote::Uploaded {
                audience: Audience::External,
                ..
            },
            _
        ))
    ));
}

#[test]
fn a_failed_upload_is_an_error_line_with_the_oracles_prefix() {
    let mut app = ready();
    let (ticket, _) = one(&submit(&mut app, "", false, Destination::OpenAi));
    fail(&mut app, ticket, "offline");
    assert_eq!(
        said(&app),
        vec![(
            FeedbackNote::Failed("Failed to upload feedback: offline".to_owned()),
            Tone::Error
        )]
    );
}

#[test]
fn a_zip_makes_the_folder_then_saves_into_a_fresh_file() {
    let mut app = ready();
    tell(&mut app, Fact::config(true, false, Some(HOME.to_owned())));
    let (ticket, first) = one(&submit(&mut app, "x", true, Destination::LocalZip));
    let folder = format!("{HOME}/feedback");
    assert_eq!(
        first,
        Command::MakeDirectory {
            path: folder.clone()
        }
    );
    assert!(matches!(
        wire(&first),
        ClientRequest::FsCreateDirectory { .. }
    ));
    let (ticket, save) = one(&answer(&mut app, ticket, FeedbackReply::Directory));
    let file = format!("{folder}/{FILE}");
    let Command::Save { upload, path } = &save else {
        panic!("not a save: {save:?}");
    };
    assert_eq!(path, &file);
    assert_eq!(upload.destination, Destination::LocalZip);
    let ClientRequest::FeedbackSave { params, .. } = wire(&save) else {
        panic!("not a save request");
    };
    assert_eq!(params.save_path.as_str(), file);
    // A save selects the thread's transcripts by its id: no rollout is attached.
    assert_eq!(params.feedback.extra_log_files, None);
    assert_eq!(params.feedback.thread_id.as_deref(), Some(T));
    answer(
        &mut app,
        ticket,
        FeedbackReply::Saved { path: file.clone() },
    );
    assert_eq!(said(&app), vec![(FeedbackNote::Saved(file), Tone::Info)]);
}

#[test]
fn a_zip_that_cannot_be_made_or_saved_is_an_error_line() {
    let mut app = ready();
    tell(&mut app, Fact::config(true, false, Some(HOME.to_owned())));
    let (ticket, _) = one(&submit(&mut app, "", false, Destination::LocalZip));
    fail(&mut app, ticket, "denied");
    let (ticket, _) = one(&submit(&mut app, "", false, Destination::LocalZip));
    let (ticket, _) = one(&answer(&mut app, ticket, FeedbackReply::Directory));
    fail(&mut app, ticket, "disk full");
    assert_eq!(
        said(&app),
        vec![
            (
                FeedbackNote::Failed(format!(
                    "Failed to save feedback ZIP: Could not create the feedback directory at {HOME}/feedback: denied"
                )),
                Tone::Error
            ),
            (
                FeedbackNote::Failed("Failed to save feedback ZIP: disk full".to_owned()),
                Tone::Error
            ),
        ]
    );
}

#[test]
fn a_zip_without_a_known_home_is_said_not_asked() {
    let mut app = ready();
    let fx = submit(&mut app, "", false, Destination::LocalZip);
    assert!(asks(&fx).is_empty());
    assert_eq!(
        said(&app),
        vec![(
            FeedbackNote::Failed(
                "Failed to save feedback ZIP: The app server did not report its Codex home directory"
                    .to_owned()
            ),
            Tone::Error
        )]
    );
}

#[test]
fn a_zip_needs_a_local_server_and_goes_out_no_ask() {
    let mut app = ready();
    tell(&mut app, Fact::config(true, true, Some(HOME.to_owned())));
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
    tell(&mut app, Fact::config(false, false, Some(HOME.to_owned())));
    assert!(asks(&submit(&mut app, "", true, Destination::OpenAi)).is_empty());
    let (_, command) = one(&submit(&mut app, "", true, Destination::LocalZip));
    assert!(matches!(command, Command::MakeDirectory { .. }));
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
    tell(&mut app, Fact::config(false, true, None));
    open(&mut app);
    assert_eq!(said(&app), vec![(FeedbackNote::Unavailable, Tone::Info)]);
}

#[test]
fn the_answers_decode_and_a_malformed_one_is_a_failure() {
    let decode = |command: &Command, value: serde_json::Value| {
        codex_tui_core::features::decode(&FeatureCommand::Feedback(command.clone()), value)
    };
    let mut app = ready();
    let (_, upload) = one(&submit(&mut app, "", false, Destination::OpenAi));
    assert_eq!(
        decode(&upload, serde_json::json!({"threadId": "filed-9"})).ok(),
        Some(FeatureReply::Feedback(FeedbackReply::Uploaded {
            report: "filed-9".to_owned()
        }))
    );
    assert!(decode(&upload, serde_json::json!({"nope": 1})).is_err());
    let directory = Command::MakeDirectory {
        path: HOME.to_owned(),
    };
    assert!(decode(&directory, serde_json::json!({})).is_ok());
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
        serde_json::json!({"audience": "open_ai_employee", "uploads": false, "remote": true, "home": HOME}),
    )
    .unwrap();
    let FeatureFact::Feedback(seeded) = fact else {
        panic!("not a feedback fact");
    };
    let mut app = ready();
    tell(&mut app, seeded);
    // Remote and no uploads: nothing can be delivered.
    run(
        &mut app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Feedback(
            FeedbackIntent::Open { source: None },
        ))),
    );
    assert_eq!(said(&app), vec![(FeedbackNote::Unavailable, Tone::Info)]);
    let bare = seed_fact("feedback", serde_json::json!({})).unwrap();
    assert_eq!(bare, FeatureFact::Feedback(Fact::default()));
}
