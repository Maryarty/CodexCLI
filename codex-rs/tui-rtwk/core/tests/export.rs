#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/export` (oracle `chatwidget/transcript_export.rs`, `app/transcript_export.rs`): the
//! destination picker's rows go to the clipboard or to the filename prompt, `/export <path>`
//! writes at once, the Markdown is what the UI's renderer made of the thread, and the host's
//! answers are the notes of the conversation. The words are the view's: these tests read data.

use codex_tui_core::app::effects::{Ask, CopyFormat, CopyStatus, Done};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, TranscriptText, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::dialogs::{self, Open, PromptKind};
use codex_tui_core::screens::select::export::{self, Row};
use codex_tui_core::screens::select::list::{Item, List};
use codex_tui_core::screens::select::{Intent as SelectIntent, Menu};
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::{CopyLabel, ExportNote, Item as Entry, Note, NoteKey, Tone};
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

fn export_command(app: &mut CodexApp, path: Option<&str>) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Export,
            intent: SessionIntent::Export {
                path: path.map(str::to_owned),
            },
        }))),
    )
}

/// The picker as the view builds it: a generic list over the rows' ids.
fn pick(app: &mut CodexApp, row: Row) -> AppFx {
    let item = |row: Row| Item {
        id: row.id().to_owned(),
        name: row.id().to_owned(),
        ..Item::default()
    };
    let list = List {
        items: vec![item(Row::Clipboard), item(Row::File)],
        ..List::default()
    };
    let select = |app: &mut CodexApp, intent| {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
        )
    };
    select(app, SelectIntent::Open(Menu::List(Box::new(list))));
    select(app, SelectIntent::Choose(row.id().to_owned()))
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

fn exports(app: &CodexApp) -> Vec<NoteKey> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: key @ (NoteKey::Export(_) | NoteKey::Copy { .. }),
                ..
            }) => Some(key.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_path_writes_the_conversation_there() {
    let mut app = ready();
    let fx = export_command(&mut app, Some("notes/talk.md"));
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1);
    let (request, ask) = &asks[0];
    assert!(request & dialogs::prompt::REQUEST_BIT == 0);
    assert!(*request & export::REQUEST_BIT != 0);
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
fn the_clipboard_row_copies_the_markdown_as_plain_text() {
    let mut app = ready();
    let fx = pick(&mut app, Row::Clipboard);
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1, "{asks:?}");
    assert_eq!(
        asks[0].1,
        Ask::Copy {
            text: MARKDOWN.to_owned(),
            format: CopyFormat::PlainText,
        }
    );
    assert!(app.screens().dialogs.open.is_none());
}

#[test]
fn the_file_row_opens_the_filename_prompt_and_its_submit_writes() {
    let mut app = ready();
    let fx = pick(&mut app, Row::File);
    assert!(asks(&fx).is_empty(), "nothing is asked before the filename");
    let Some(Open::Prompt(prompt)) = &app.screens().dialogs.open else {
        panic!("no prompt: {:?}", app.screens().dialogs.open);
    };
    assert_eq!(prompt.kind, PromptKind::ExportFile);
    assert_eq!(prompt.initial, "codex-session-thread-1.md");

    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Dialogs(
            dialogs::Intent::Submit("mine.md".to_owned()),
        ))),
    );
    assert!(app.screens().dialogs.open.is_none());
    let asks = asks(&fx);
    assert_eq!(asks.len(), 1, "{asks:?}");
    let Ask::Export { path, markdown, .. } = &asks[0].1 else {
        panic!("not an export: {:?}", asks[0]);
    };
    assert_eq!(path, "mine.md");
    assert_eq!(markdown, MARKDOWN);
}

#[test]
fn the_prompt_filename_has_no_thread_before_one_exists() {
    let mut app = CodexApp::new();
    pick(&mut app, Row::File);
    let Some(Open::Prompt(prompt)) = &app.screens().dialogs.open else {
        panic!("no prompt");
    };
    assert_eq!(prompt.initial, "codex-session.md");
}

#[test]
fn a_saved_file_is_said_with_its_path_and_a_failure_with_its_reason() {
    let mut app = ready();
    let request = asks(&export_command(&mut app, Some("a.md")))[0].0;
    run(
        &mut app,
        AppEv::Effect {
            request,
            done: Done::Export(Ok("/work/a.md".to_owned())),
        },
    );
    run(
        &mut app,
        AppEv::Effect {
            request,
            done: Done::Export(Err("could not create /work/a.md: exists".to_owned())),
        },
    );
    assert_eq!(
        exports(&app),
        vec![
            NoteKey::Export(ExportNote::Saved("/work/a.md".to_owned())),
            NoteKey::Export(ExportNote::Failed(
                "could not create /work/a.md: exists".to_owned()
            )),
        ]
    );
    let tones = app
        .active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(note) if matches!(note.key, NoteKey::Export(_)) => Some(note.tone),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(tones, vec![Tone::Info, Tone::Error]);
}

#[test]
fn a_copy_says_pending_first_and_its_status_when_settled() {
    let mut app = ready();
    let request = asks(&pick(&mut app, Row::Clipboard))[0].0;
    for result in [
        Ok(CopyStatus::Pending(7)),
        Ok(CopyStatus::Confirmed),
        Err("no clipboard".to_owned()),
    ] {
        run(
            &mut app,
            AppEv::Effect {
                request,
                done: Done::Copy(result),
            },
        );
    }
    let copy = |result| NoteKey::Copy {
        label: CopyLabel::Conversation,
        result,
    };
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
    let fx = export_command(&mut app, Some("a.md"));
    assert!(asks(&fx).is_empty());
    assert_eq!(exports(&app), vec![NoteKey::Export(ExportNote::NoContent)]);
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
            intent: SessionIntent::Export {
                path: Some("a.md".to_owned()),
            },
        }))),
    );
    assert!(asks(&fx).is_empty());
    let notes = &app.screens().chat.notes;
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert_eq!(notes[0].key, NoteKey::Export(ExportNote::NoConversation));
    assert_eq!(notes[0].tone, Tone::Error);
}

#[test]
fn the_row_ids_are_the_pickers_and_nothing_else() {
    assert_eq!(Row::of_row(export::CLIPBOARD), Some(Row::Clipboard));
    assert_eq!(Row::of_row(export::FILE), Some(Row::File));
    assert_eq!(Row::of_row("plan.yes"), None);
}
