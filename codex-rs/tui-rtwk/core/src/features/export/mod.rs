//! `/export` as a feature slice (ADR-012; oracle `chatwidget/transcript_export.rs`
//! `show_transcript_export_popup`, `app/transcript_export.rs` `export_transcript`): the
//! conversation in use as Markdown, copied to the clipboard or written to a file, and the line
//! saying how it went. A bare `/export` opens the destination picker (`Menu::Destination`), whose
//! "Copy to clipboard" row copies at once and whose "Save to file" row opens the filename prompt
//! (`Prompt::File`); `/export <path>` writes at once. Words and rows: `src/features/export/`.
//!
//! The Markdown is rendered from the cells' text, which is the UI's: it hands the app its
//! renderer once (`CodexApp::with_transcript_text`), a pure function of a thread's model that
//! the feature reads through `AppView::transcript_text`. The asks go out with the text in them
//! (`Ask::Copy`, `Ask::Export`) and the host's answer is worded as a note of the conversation in
//! use: a copy answers twice (pending, then how it ended) and says both.

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Choice, Feature, Never};
use crate::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

/// The marker type of the feature.
pub struct Export;

/// The row ids of the destination picker, in order (oracle: "Copy to clipboard", "Save to
/// file").
pub const CLIPBOARD: &str = "export.clipboard";
pub const FILE: &str = "export.file";

/// What the user asks of `/export`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/export` (no path: the destination picker) and `/export <path>` (written at once).
    Command { path: Option<String> },
}

/// What an ask of the host was for: the export of the conversation in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctx {
    Conversation,
}

/// The destination picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    /// `Export conversation`: `Copy to clipboard`, `Save to file`.
    Destination,
}

/// A row of the destination picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// "Copy to clipboard": the complete Markdown transcript is copied.
    Clipboard,
    /// "Save to file": a filename is asked for (`Prompt::File`).
    File,
}

/// The filename prompt (oracle `show_transcript_export_file_prompt`, `Save conversation`). The
/// conversation is the one in use when it is submitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prompt {
    File,
}

/// What `/export` says in the transcript (`export_transcript`, `show_copy_result`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Saved conversation to {path}`: the path written.
    Saved(String),
    /// `Export failed: {error}`: the host's reason the file was not written.
    Failed(String),
    /// `Export failed: No active conversation to export.`
    NoConversation,
    /// `Export failed: No conversation content to export.`
    NoContent,
    /// How the copy of the conversation went: the delivery status, or the clipboard's failure.
    Copy(Result<CopyStatus, String>),
}

/// Where an exported transcript goes (oracle `TranscriptExportDestination`).
enum Destination {
    Clipboard,
    /// A path as typed: `~/` is the home directory, a relative path is under the thread's
    /// directory; a file that exists is never overwritten.
    File(String),
}

/// `ExportTranscript`: renders the conversation in use and asks the host to put it on the
/// clipboard or write it; what cannot be exported is the `Export failed:` line.
fn run(cx: &mut AppCx<'_>, destination: Destination) {
    let view = cx.view();
    let Some(thread) = view
        .active()
        .filter(|id| view.thread(id).is_some())
        .cloned()
    else {
        return cx.note(None, Note::NoConversation);
    };
    let Some(markdown) = view.transcript_text(&thread) else {
        return cx.note(Some(thread), Note::NoContent);
    };
    let ask = match destination {
        Destination::Clipboard => Ask::Copy {
            text: markdown,
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
        },
        // The directory a relative path is under: the thread's own, else the one the status
        // line reads.
        Destination::File(path) => Ask::Export {
            cwd: view.working_directory(Some(&thread)),
            thread,
            path,
            markdown,
        },
    };
    cx.effect(ask, Ctx::Conversation);
}

/// The filename the prompt starts with, for the conversation in use (`codex-session-{thread}.md`,
/// `codex-session.md` before a thread exists): a filename, not prose, so the oracle's own.
fn filename(thread: Option<&ThreadId>) -> String {
    thread.map_or_else(
        || "codex-session.md".to_owned(),
        |thread| ["codex-session-", thread.as_str(), ".md"].concat(),
    )
}

impl Feature for Export {
    const NAME: &'static str = "export";

    type State = ();
    type ThreadPart = ();
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Menu = Menu;
    type Pick = Pick;
    type Prompt = Prompt;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Command { path: Some(path) } => run(cx, Destination::File(path)),
            Intent::Command { path: None } => cx.open(Menu::Destination),
        }
    }

    /// The host's answer to the export's ask, said in the conversation in use: the copy's status
    /// (a pending copy says so first, and again when it is settled), the path written, or the
    /// failure.
    fn effect_done(_state: &mut (), _ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        let note = match done {
            Done::Copy(result) => Note::Copy(result),
            Done::Export(Ok(path)) => Note::Saved(path),
            Done::Export(Err(error)) => Note::Failed(error),
            _ => return,
        };
        cx.note(None, note);
    }

    fn choose(_menu: &Menu, row: &str) -> Choice<Menu, Pick> {
        match row {
            CLIPBOARD => Choice::Pick(Pick::Clipboard),
            FILE => Choice::Pick(Pick::File),
            _ => Choice::Stay,
        }
    }

    fn picked(_state: &mut (), pick: Pick, cx: &mut AppCx<'_>) {
        match pick {
            Pick::Clipboard => run(cx, Destination::Clipboard),
            Pick::File => {
                let initial = filename(cx.view().active());
                cx.prompt(Prompt::File, initial);
            }
        }
    }

    /// The typed filename is the destination.
    fn prompt_submitted(_state: &mut (), prompt: Prompt, text: String, cx: &mut AppCx<'_>) {
        let Prompt::File = prompt;
        run(cx, Destination::File(text));
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::Saved(_) | Note::Copy(Ok(_)) => Tone::Info,
            Note::Failed(_) | Note::NoConversation | Note::NoContent | Note::Copy(Err(_)) => {
                Tone::Error
            }
        }
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
