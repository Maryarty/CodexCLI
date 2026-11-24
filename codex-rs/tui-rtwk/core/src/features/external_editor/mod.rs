//! The external editor (`Ctrl+G`) as a feature slice (ADR-012; oracle `external_editor.rs`,
//! `app/input.rs` `launch_external_editor`): the draft goes to a temporary file, the user's
//! `$VISUAL` / `$EDITOR` edits it, and what it left is handed back to the composer.
//!
//! The draft is the composer's, which is UI state (ADR-013): the view says the verb
//! `Intent::Open` with the text as it stands (the oracle's `composer_text_with_pending`), and core
//! only keeps what IO needs: whether an editor is running (`State::running`, the oracle's
//! `ExternalEditorState`: the key does nothing while one is, and the footer says so), the ask of
//! the host (`Ask::Editor`: it writes the file, runs the editor and reads the file back;
//! handing the terminal over is the host's) and the two ways it ends. An edited text is the
//! one-shot `State::edited`, which the thread's composer applies once (`apply_external_edit`) and
//! acknowledges with `Intent::Applied` (a wish is for a sheet a presenter opens, and this one is
//! no sheet); an editor that could not run is a line of the transcript. Words:
//! `src/features/external_editor/strings.rs`.

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never};
use crate::app::effects::{Ask, Done, EditorFailure};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

/// The marker type of the feature.
pub struct ExternalEditor;

/// What the user asks: the key (`Ctrl+G`) with the draft in the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Edit `draft`, the composer of `thread`, in the external editor. Nothing happens while an
    /// editor is already running or when this process does not write the thread.
    Open { thread: ThreadId, draft: String },
    /// The composer applied the edit numbered `seq` (`State::edited`): it is gone.
    Applied { seq: u64 },
}

/// The app-scoped state: whether an editor runs now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// The terminal belongs to an editor (`ExternalEditorState::Requested` until it ends); the
    /// key is refused and the footer hint stands meanwhile.
    pub running: bool,
    /// The text the last editor left, until the composer of its thread applied it.
    pub edited: Option<Edited>,
    /// Numbers the edits, so a composer applies each once.
    seq: u64,
}

/// What the editor left for a thread's composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edited {
    pub seq: u64,
    pub thread: ThreadId,
    /// The file's text, trailing whitespace trimmed: it replaces the draft
    /// (`apply_external_edit`).
    pub text: String,
}

/// What an ask of the host was for: the draft of this thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Draft(ThreadId),
}

/// What the feature says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Cannot open external editor: set $VISUAL or $EDITOR before starting Codex.`
    Missing,
    /// `Failed to open editor: {error}`: the host's reason (a command that does not parse, a file
    /// that could not be written or read, an editor that exited with a failure).
    Failed(String),
}

impl Feature for ExternalEditor {
    const NAME: &'static str = "external_editor";

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;
    type Event = Never<Self>;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let (thread, draft) = match intent {
            Intent::Open { thread, draft } => (thread, draft),
            Intent::Applied { seq } => {
                if state
                    .edited
                    .as_ref()
                    .is_some_and(|edited| edited.seq == seq)
                {
                    state.edited = None;
                }
                return;
            }
        };
        let view = cx.view();
        // `can_launch_external_editor`: a thread another process writes is only watched.
        if state.running
            || !view
                .thread(&thread)
                .is_some_and(|thread| thread.has_writer())
        {
            return;
        }
        state.running = true;
        let cwd = view.working_directory(Some(&thread));
        cx.effect(Ask::Editor { draft, cwd }, Ctx::Draft(thread));
    }

    /// The editor ended: its text goes back to the composer, or the failure is said.
    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        let Done::Editor(result) = done else {
            return;
        };
        state.running = false;
        let Ctx::Draft(thread) = ctx;
        match result {
            Ok(text) => {
                state.seq += 1;
                state.edited = Some(Edited {
                    seq: state.seq,
                    thread,
                    text: text.trim_end().to_owned(),
                });
            }
            Err(EditorFailure::Missing) => cx.note(Some(thread), Note::Missing),
            Err(EditorFailure::Failed(error)) => cx.note(Some(thread), Note::Failed(error)),
        }
    }

    fn note_tone(_note: &Note) -> Tone {
        Tone::Error
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
