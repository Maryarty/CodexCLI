//! `/export` as a feature slice (ADR-012; oracle `chatwidget/transcript_export.rs`
//! `show_transcript_export_popup`, `app/transcript_export.rs` `export_transcript`): the
//! conversation in use as Markdown, copied to the clipboard or written to a file, and the line
//! saying how it went. The destination picker of a bare `/export` and its filename prompt are the
//! UI half's (`src/features/export/`, opened as `Target::Feature(FeatureTarget::Export(Sheet))`); what
//! they decide arrives as the verb `Intent::Export`, as `/export <path>` does. Words:
//! `src/features/export/`.
//!
//! The Markdown is rendered from the cells' text, which is the UI's: it hands the app its
//! renderer once (`CodexApp::with_transcript_text`), a pure function of a thread's model that
//! the feature reads through `AppView::transcript_text`. The asks go out with the text in them
//! (`Ask::Copy`, `Ask::Export`) and the host's answer is worded as a note of the conversation in
//! use: a copy answers twice (pending, then how it ended) and says both.

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never, Slash};
use crate::app::effects::{Ask, CopyDestination, CopyFormat, CopyStatus, Done};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

/// The marker type of the feature.
pub struct Export;

/// What `/export` names (`FeatureTarget`): the feature's own sheet, opened by its UI half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet;

/// Where an exported transcript goes (oracle `TranscriptExportDestination`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    /// "Copy to clipboard": the complete Markdown transcript is copied.
    Clipboard,
    /// A path as typed: `~/` is the home directory, a relative path is under the thread's
    /// directory; a file that exists is never overwritten.
    File(String),
}

/// What the user asks of `/export`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/export <path>`: the conversation in use is written to the path as typed.
    Command { path: String },
    /// Export the conversation in use to `destination` (a row of the destination sheet, or the
    /// filename typed in its prompt).
    Export { destination: Destination },
}

/// What an ask of the host was for: the export of the conversation in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctx {
    Conversation,
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

impl Feature for Export {
    const NAME: &'static str = "export";
    const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::Export, Sheet));
    const RUNS: &'static [SlashCommand] = &[SlashCommand::Export];

    type State = ();
    type ThreadPart = ();
    type Target = Sheet;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    /// `/export <path>`: the conversation in use is written to the path as typed. A bare `/export`
    /// is the feature's sheet (`OPENS`) and never gets here.
    fn slash(state: &mut (), slash: Slash, cx: &mut AppCx<'_>) {
        if let Some(path) = slash.into_args() {
            Self::intent(state, Intent::Command { path }, cx);
        }
    }

    fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) {
        let destination = match intent {
            Intent::Command { path } => Destination::File(path),
            Intent::Export { destination } => destination,
        };
        run(cx, destination);
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
