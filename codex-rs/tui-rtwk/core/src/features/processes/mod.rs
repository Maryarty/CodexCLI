//! `/ps` as a feature slice (ADR-012; oracle `chatwidget.rs` `add_ps_output`,
//! `history_cell/exec.rs` `new_unified_exec_processes_output`): the background terminals of the
//! thread in use, said as one note of its transcript. The thread keeps them
//! (`threads::processes`: the commands that run as unified-exec sessions, the last lines each
//! printed); this feature reads them at the moment of the command, so the cell is a snapshot, as
//! the oracle's is. The cell's layout (the first 16 processes, a command cut to 80 graphemes and
//! to the width, the `... and N more running` line) and its words are the UI's
//! (`src/features/processes/`). `/stop` stays the thread's own intent.

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::{Terminal, Tone};

/// The marker type of the feature.
pub struct Processes;

/// What `/ps` names (`FeatureTarget`): the command runs the feature through its UI half's `open`,
/// which says the verb below for the thread in use (no sheet is drawn).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet;

/// What the user asks of `/ps`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/ps`, typed in the thread `source` (none before the session has started: no
    /// terminals).
    Show { source: Option<ThreadId> },
}

/// What `/ps` says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The terminals that ran when the command was typed, oldest first; none is
    /// `No background terminals running.`
    pub terminals: Vec<Terminal>,
}

impl Feature for Processes {
    const NAME: &'static str = "processes";
    const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::Ps, Sheet));

    type State = ();
    type ThreadPart = ();
    type Target = Sheet;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Never<Self>;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) {
        let Intent::Show { source } = intent;
        let terminals = source
            .as_ref()
            .and_then(|id| cx.view().thread(id))
            .map(|thread| thread.terminals().to_vec())
            .unwrap_or_default();
        cx.note(source, Note { terminals });
    }

    fn note_tone(_note: &Note) -> Tone {
        Tone::Info
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
