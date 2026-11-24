#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/export` (oracle `chatwidget/transcript_export.rs`, `app/transcript_export.rs`): the verb
//! `Export { destination }` (what `/export <path>` and the UI's destination sheet say) goes to the
//! clipboard or writes a file, the Markdown is what the UI's renderer made of the thread, and
//! the host's answers are the notes of the conversation. The picker and the filename prompt are
//! the UI's (`src/features/export/`). The words are the view's: these tests read data.

use codex_tui_core::app::effects::{
    Ask, CopyDestination, CopyFormat, CopyStatus, Done, FEATURE_REQUEST_BIT,
};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, TranscriptText, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::features::export::{Destination, Intent as ExportIntent, Note as ExportNote};
use codex_tui_core::features::{FeatureIntent, FeatureNote};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::{Item as Entry, Note, NoteKey, Tone};
use pretty_assertions::assert_eq;

const T: &str = "thread-1";
const MARKDOWN: &str = "# Codex conversation\n\n## User\n\nhello\n";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn start(mut app: CodexApp) -> CodexApp {
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

fn ready() -> CodexApp {
    start(CodexApp::new().with_transcript_text(TranscriptText::new(|_| Some(MARKDOWN.to_owned()))))
}

fn export_command(app: &mut CodexApp, path: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Export,
            intent: SessionIntent::Feature {
                args: path.to_owned(),
            },
        }))),
    )
}

/// What the destination sheet of the UI says: the verb, with where the Markdown goes.
fn export_to(app: &mut CodexApp, destination: Destination) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Export(
            ExportIntent::Export { destination },
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

fn exports(app: &CodexApp) -> Vec<ExportNote> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: NoteKey::Feature(FeatureNote::Export(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_path_writes_the_conversation_there() {
    let mut app = ready();
    let fx = export_command(&mut app, "notes/talk.md");
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1);
    let (request, ask) = &asks[0];
    assert!(*request & FEATURE_REQUEST_BIT != 0);
    let Ask::Export {
        thread,
        path,
        cwd: _,
        markdown,
    } = ask
    else {
        panic!("not an export: {ask:?}");
    };
    assert_eq!(thread, &ThreadId::new(T));
    assert_eq!(path, "notes/talk.md");
    assert_eq!(markdown, MARKDOWN);
}

#[test]
fn the_clipboard_destination_copies_the_markdown_as_plain_text() {
    let mut app = ready();
    let fx = export_to(&mut app, Destination::Clipboard);
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert_eq!(
        asks[0].1,
        Ask::Copy {
            text: MARKDOWN.to_owned(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
        }
    );
}

#[test]
fn a_file_destination_writes_the_conversation_under_that_name() {
    let mut app = ready();
    let fx = export_to(&mut app, Destination::File("mine.md".to_owned()));
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1, "{asks:?}");
    let Ask::Export { path, markdown, .. } = &asks[0].1 else {
        panic!("not an export: {:?}", asks[0]);
    };
    assert_eq!(path, "mine.md");
    assert_eq!(markdown, MARKDOWN);
}

#[test]
fn a_saved_file_is_said_with_its_path_and_a_failure_with_its_reason() {
    let mut app = ready();
    for done in [
        Done::Export(Ok("/work/a.md".to_owned())),
        Done::Export(Err("could not create /work/a.md: exists".to_owned())),
    ] {
        let request = asks(&export_command(&mut app, "a.md"))[0].0;
        run(&mut app, AppEv::Effect { request, done });
    }
    assert_eq!(
        exports(&app),
        vec![
            ExportNote::Saved("/work/a.md".to_owned()),
            ExportNote::Failed("could not create /work/a.md: exists".to_owned()),
        ]
    );
    let tones = app
        .active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(note) if matches!(note.key, NoteKey::Feature(FeatureNote::Export(_))) => {
                Some(note.tone)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tones, vec![Tone::Info, Tone::Error]);
}

#[test]
fn a_copy_says_pending_first_and_its_status_when_settled() {
    let mut app = ready();
    let answers = |app: &mut CodexApp, results: Vec<Result<CopyStatus, String>>| {
        let request = asks(&export_to(app, Destination::Clipboard))[0].0;
        for result in results {
            run(
                app,
                AppEv::Effect {
                    request,
                    done: Done::Copy(result),
                },
            );
        }
    };
    answers(
        &mut app,
        vec![Ok(CopyStatus::Pending(7)), Ok(CopyStatus::Confirmed)],
    );
    answers(&mut app, vec![Err("no clipboard".to_owned())]);
    let copy = ExportNote::Copy;
    assert_eq!(
        exports(&app),
        vec![
            copy(Ok(CopyStatus::Pending(7))),
            copy(Ok(CopyStatus::Confirmed)),
            copy(Err("no clipboard".to_owned())),
        ]
    );
}

#[test]
fn an_answer_after_the_last_one_of_an_ask_is_dropped() {
    let mut app = ready();
    let request = asks(&export_to(&mut app, Destination::Clipboard))[0].0;
    for result in [Ok(CopyStatus::Confirmed), Ok(CopyStatus::Unconfirmed)] {
        run(
            &mut app,
            AppEv::Effect {
                request,
                done: Done::Copy(result),
            },
        );
    }
    assert_eq!(
        exports(&app),
        vec![ExportNote::Copy(Ok(CopyStatus::Confirmed))]
    );
}

#[test]
fn another_requests_copy_is_not_a_note() {
    let mut app = ready();
    run(
        &mut app,
        AppEv::Effect {
            request: 5,
            done: Done::Copy(Ok(CopyStatus::Confirmed)),
        },
    );
    assert!(exports(&app).is_empty());
}

#[test]
fn nothing_to_export_is_the_failure_line_and_no_ask() {
    let mut app = start(CodexApp::new().with_transcript_text(TranscriptText::new(|_| None)));
    let fx = export_command(&mut app, "a.md");
    assert!(asks(&fx).is_empty());
    assert_eq!(exports(&app), vec![ExportNote::NoContent]);
}

#[test]
fn without_a_thread_the_failure_is_a_note_of_the_chat() {
    let mut app =
        CodexApp::new().with_transcript_text(TranscriptText::new(|_| Some(MARKDOWN.to_owned())));
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: None,
            command: SlashCommand::Export,
            intent: SessionIntent::Feature {
                args: "a.md".to_owned(),
            },
        }))),
    );
    assert!(asks(&fx).is_empty());
    let notes = &app.screens().chat.notes;
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert_eq!(
        notes[0].key,
        NoteKey::Feature(FeatureNote::Export(ExportNote::NoConversation))
    );
    assert_eq!(notes[0].tone, Tone::Error);
}
