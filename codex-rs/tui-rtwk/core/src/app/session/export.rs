//! `/export` (oracle `app/transcript_export.rs` `export_transcript`, `chatwidget/
//! transcript_export.rs`): the conversation in use as Markdown, copied to the clipboard or written
//! to a file, and the line saying how it went. The destination picker is the view's list
//! (`select::export` rows), its "Save to file" row opens the filename prompt
//! (`PromptKind::ExportFile`); `/export <path>` writes at once.
//!
//! The Markdown is rendered from the cells' text, which is the UI's: it hands the app its
//! renderer once (`CodexApp::with_transcript_text`), a pure function of a thread's model, so
//! the flow stays one decision of the app (ADR-003): the asks go out with the text in them
//! (`Ask::Copy`, `Ask::Export`), the host's answer is worded as a note of the thread.

use std::rc::Rc;

use super::session_note;
use crate::app::effects::{Ask, CopyFormat, Done, command};
use crate::app::{AppFx, CodexApp};
use crate::ids::ThreadId;
use crate::screens::dialogs::{Open, Prompt};
use crate::screens::select::export::{Destination, REQUEST_BIT, Row};
use crate::threads::{CopyLabel, ExportNote, Note, NoteKey, ThreadModel};

/// What renders a thread's conversation as the Markdown of an export: `# Codex conversation`,
/// then a `## User` / `## Assistant` / `## Plan` / `## Reasoning` / `## Activity` section per
/// cell. `None` is a conversation with nothing in it to export.
#[derive(Clone)]
pub struct TranscriptText(Rc<Render>);

type Render = dyn Fn(&ThreadModel) -> Option<String>;

impl TranscriptText {
    pub fn new(render: impl Fn(&ThreadModel) -> Option<String> + 'static) -> Self {
        Self(Rc::new(render))
    }

    fn render(&self, model: &ThreadModel) -> Option<String> {
        (self.0)(model)
    }
}

impl std::fmt::Debug for TranscriptText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TranscriptText")
    }
}

/// The row the user picked in the destination picker.
pub(in crate::app) fn picked(app: &mut CodexApp, row: Row, fx: &mut AppFx) {
    match row {
        Row::Clipboard => run(app, Destination::Clipboard, fx),
        Row::File => {
            let thread = app.session.active.clone();
            Rc::make_mut(&mut app.screens).dialogs.open =
                Some(Open::Prompt(Prompt::export_file(thread.as_ref())));
        }
    }
}

/// `ExportTranscript`: renders the conversation in use and asks the host to put it on the
/// clipboard or write it; a failure is the `Export failed:` line of the thread.
pub(in crate::app) fn run(app: &mut CodexApp, destination: Destination, fx: &mut AppFx) {
    let Some(thread) = app
        .session
        .active
        .clone()
        .filter(|id| app.threads.get(id).is_some())
    else {
        return said(app, None, ExportNote::NoConversation, fx);
    };
    let markdown = app
        .transcript_text
        .as_ref()
        .zip(app.threads.get(&thread))
        .and_then(|(render, model)| render.render(model));
    let Some(markdown) = markdown else {
        return said(app, Some(thread), ExportNote::NoContent, fx);
    };
    let ask = match destination {
        Destination::Clipboard => Ask::Copy {
            text: markdown,
            format: CopyFormat::PlainText,
        },
        Destination::File(path) => Ask::Export {
            cwd: directory(app, &thread),
            thread,
            path,
            markdown,
        },
    };
    let request = REQUEST_BIT | Rc::make_mut(&mut app.screens).dialogs.next_stamp();
    fx.commands.push(command(request, ask));
}

/// The directory a relative path is under: the thread's own, else the one the status line reads.
fn directory(app: &CodexApp, thread: &ThreadId) -> String {
    super::working_directory(app, Some(thread))
}

fn said(app: &mut CodexApp, thread: Option<ThreadId>, note: ExportNote, fx: &mut AppFx) {
    session_note(app, thread, Note::from(NoteKey::Export(note)), fx);
}

/// The host's answer to an ask of the export, said in the conversation in use: the copy's
/// status (a pending copy says so first, and again when it is settled), the path written, or
/// the failure. Other requests are none of its business.
pub(in crate::app) fn settled(app: &mut CodexApp, request: u64, done: &Done, fx: &mut AppFx) {
    if request & REQUEST_BIT == 0 {
        return;
    }
    let thread = app.session.active.clone();
    let key = match done {
        Done::Copy(result) => NoteKey::Copy {
            label: CopyLabel::Conversation,
            result: result.clone(),
        },
        Done::Export(Ok(path)) => NoteKey::Export(ExportNote::Saved(path.clone())),
        Done::Export(Err(error)) => NoteKey::Export(ExportNote::Failed(error.clone())),
        _ => return,
    };
    session_note(app, thread, Note::from(key), fx);
}

/// A prompt of the export, submitted: the typed filename is the destination.
pub(in crate::app) fn submitted(app: &mut CodexApp, text: String, fx: &mut AppFx) {
    run(app, Destination::File(text), fx);
}
