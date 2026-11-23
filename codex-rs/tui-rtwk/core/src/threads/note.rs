//! A local note in the transcript: what a command line answered with (the
//! oracle's `new_info_event` and `new_error_event` cells). The session leaves
//! it as an entry of the thread, so it stays where the conversation was when
//! it was said. Typed keys, the UI's string tables hold the words (ADR-010).

use crate::app::effects::CopyStatus;
use crate::commands::Notice;
use crate::rpc::RpcError;
use crate::screens::agents::Action;
use crate::threads::Goal;

/// How a note reads: the oracle's `add_info_message` (a dim bullet, then the
/// message) or `add_error_message` (a red `■ `).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    /// A green `✓ ` and the message (a finished recovery).
    Done,
    Error,
}

/// A part of the oracle the TUI does not carry (`add_app_server_stub_message`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stub {
    /// `/debug-m-drop`, `/debug-m-update`.
    MemoryMaintenance,
}

/// Why `/cd` did not change the directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CdRefusal {
    /// The session has not started (`request_working_directory_change`).
    NotStarted,
    /// Not an idle primary session without queued input
    /// (`can_change_working_directory`).
    NotIdle,
    /// The conversation has no saved history to carry over.
    HistoryNotSaved,
}

/// Why `/archive` or `/delete` did not remove the thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoveNote {
    /// No thread has started (`archive_current_thread`).
    NotStarted,
    /// The thread is a side conversation of another.
    SideConversation,
    /// The server refused or the request failed.
    Failed,
}

/// What a note says, before the UI words it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteKey {
    /// A command line was refused, or its arguments are wrong (dispatch).
    Command(Notice),
    /// `/pwd`: the directory the session works in.
    WorkingDirectory(String),
    /// `/rollout`: the file the session's history is written to, none until
    /// the server has named it.
    Rollout(Option<String>),
    /// `/app`: the thread was handed to the Desktop app.
    DesktopOpened,
    /// `/stop`: the background terminals are being stopped.
    StoppingTerminals,
    /// A feature the TUI does not carry.
    Stub(Stub),
    /// `/raw`: the transcript is shown as text for selection, or as before.
    RawOutput(bool),
    /// The thread is controlled by its parent: it takes no direct input.
    ParentOwned,
    /// A switch of the collaboration mode while a turn runs
    /// (`Cannot switch collaboration mode while a turn is running.`).
    ModeBusy,
    /// `/cd` was refused.
    CdRefused(CdRefusal),
    /// `/archive` or `/delete` did not remove the thread.
    Remove(Action, RemoveNote),
    /// What a picker (`/model`, `/permissions`, `/apps`) said after a pick or
    /// an answer (`select::Note`).
    Select(crate::screens::select::Note),
    /// What the server said while it worked, in its own words (`modelProvider/authRecoveryStarted`).
    Server(String),
    /// What the server said when its work finished, in its own words
    /// (`modelProvider/authRecoveryCompleted`).
    ServerDone(String),
    /// How a copy went (`show_copy_result`): the delivery status, or the clipboard's failure.
    Copy {
        label: CopyLabel,
        result: Result<CopyStatus, String>,
    },
    /// What `/export` answered with.
    Export(ExportNote),
    /// What `/goal` answered with.
    Goal(GoalNote),
    /// What a switch of the collaboration mode said.
    Mode(ModeNote),
    /// A line of a feature slice (ADR-012); its words are `src/features/<f>/strings.rs`.
    Feature(crate::features::FeatureNote),
}

/// What `/plan` and the mode switches said (`apply_plan_slash_command`,
/// `set_collaboration_mask`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModeNote {
    /// `Collaboration modes are disabled.` with `Enable collaboration modes to use /plan.`
    /// under it.
    Disabled,
    /// `Plan mode unavailable right now.`: the server's catalog has no Plan preset.
    PlanUnavailable,
    /// `Model changed to {model} {effort} for {mode} mode.`
    ModelChanged(crate::screens::select::collaboration::ModelChange),
}

/// What `/goal` answered with (`thread_goal_actions.rs`, `goal_menu.rs`, `slash_dispatch.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoalNote {
    /// The usage line (`Usage: /goal [<objective>|clear|edit|pause|resume]`) with the hint
    /// under it.
    Usage(GoalHint),
    /// The bare `/goal`: the goal as a summary (`goal_summary_lines`).
    Summary(Goal),
    /// `Goal {status}` with the usage summary under it: a goal was set or its status changed.
    Status(Goal),
    /// `Goal cleared`.
    Cleared,
    /// `No goal to clear` with `This thread does not currently have a goal.`
    NothingToClear,
    /// `No goal is currently set.` as an error (`/goal edit` over no goal).
    NoGoalToEdit,
    /// An ask failed: `Failed to {action} thread goal: {error}`, or the saved-session
    /// explanation for a temporary thread.
    Failed {
        action: GoalAction,
        failure: GoalFailure,
    },
}

/// The hint under the usage line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalHint {
    /// `Example: /goal improve benchmark coverage` (the bare `/goal`, no thread yet).
    Example,
    /// `No goal is currently set.`
    NoGoal,
    /// `The session must start before you can change a goal.`
    StartToChange,
    /// `The session must start before you can set a goal.`
    StartToSet,
    /// `Create a goal before editing it.`
    CreateFirst,
}

/// What the failed ask was doing (the verb of `Failed to {action} thread goal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalAction {
    Read,
    Replace,
    Set,
    Update,
    Clear,
}

/// Why a goal ask failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoalFailure {
    /// The thread is temporary: goals need a saved session (`is_ephemeral_thread_goal_error`).
    Ephemeral,
    /// The server's own words.
    Other(String),
}

impl GoalFailure {
    pub fn of(error: &RpcError) -> Self {
        let text = match error {
            RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => {
                text.clone()
            }
            RpcError::Provider(error) => error.to_string(),
            RpcError::Mismatch => String::new(),
        };
        if text.contains("ephemeral thread does not support goals")
            || text.contains("thread goals require a persisted thread; this thread is ephemeral")
        {
            GoalFailure::Ephemeral
        } else {
            GoalFailure::Other(text)
        }
    }
}

/// What a copy put on the clipboard: the word in the line saying how it went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyLabel {
    /// `/export`, "Copy to clipboard".
    Conversation,
}

/// What `/export` answered with (`export_transcript`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportNote {
    /// `Saved conversation to {path}`: the path written.
    Saved(String),
    /// `Export failed: {error}`: the host's reason the file was not written.
    Failed(String),
    /// `Export failed: No active conversation to export.`
    NoConversation,
    /// `Export failed: No conversation content to export.`
    NoContent,
}

impl NoteKey {
    /// The oracle shows each answer as information or as an error.
    pub fn tone(&self) -> Tone {
        match self {
            NoteKey::Command(notice) if !notice.is_info() => Tone::Error,
            NoteKey::Select(note) => note.tone(),
            NoteKey::Feature(note) => crate::features::note_tone(note),
            NoteKey::ServerDone(_) => Tone::Done,
            NoteKey::Copy { result: Err(_), .. }
            | NoteKey::Export(
                ExportNote::Failed(_) | ExportNote::NoConversation | ExportNote::NoContent,
            )
            | NoteKey::Goal(GoalNote::NoGoalToEdit | GoalNote::Failed { .. }) => Tone::Error,
            NoteKey::Stub(_)
            | NoteKey::ParentOwned
            | NoteKey::ModeBusy
            | NoteKey::CdRefused(_)
            | NoteKey::Remove(..) => Tone::Error,
            NoteKey::Command(_)
            | NoteKey::WorkingDirectory(_)
            | NoteKey::Rollout(_)
            | NoteKey::DesktopOpened
            | NoteKey::StoppingTerminals
            | NoteKey::RawOutput(_)
            | NoteKey::Copy { .. }
            | NoteKey::Export(ExportNote::Saved(_))
            | NoteKey::Goal(_)
            | NoteKey::Mode(_)
            | NoteKey::Server(_) => Tone::Info,
        }
    }
}

/// One note of the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub tone: Tone,
    pub key: NoteKey,
}

impl From<NoteKey> for Note {
    fn from(key: NoteKey) -> Self {
        Self {
            tone: key.tone(),
            key,
        }
    }
}

impl From<Notice> for Note {
    /// The oracle shows `Notice::is_info` as information and every other as an
    /// error.
    fn from(notice: Notice) -> Self {
        Self::from(NoteKey::Command(notice))
    }
}
