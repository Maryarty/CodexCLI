//! A local note in the transcript: what a command line answered with (the
//! oracle's `new_info_event` and `new_error_event` cells). The session leaves
//! it as an entry of the thread, so it stays where the conversation was when
//! it was said. Typed keys, the UI's string tables hold the words (ADR-010).

use crate::commands::Notice;
use crate::screens::agents::Action;

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

/// A setting a command turns on or off, and says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    /// `/raw`: the transcript shown as text for selection.
    RawOutput,
    /// `/vim` (or the toggle key): modal editing of the draft.
    Vim,
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
    /// The edit of a past prompt reverted the thread (oracle `emit_prompt_edit_thread_event`).
    PromptReverted,
    /// A feature the TUI does not carry.
    Stub(Stub),
    /// `/raw`, `/vim`: the setting is on or off now.
    Toggled(Toggle, bool),
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
    /// `Automatically switched to {model} {effort} due to usage limits.` or `... back to ...
    /// because ordinary usage is available again.` (`apply_backend_banner_fallback`).
    AutomaticSwitch(crate::screens::chat::AutomaticSwitch),
}

impl NoteKey {
    /// The oracle shows each answer as information or as an error.
    pub fn tone(&self) -> Tone {
        match self {
            NoteKey::Command(notice) if !notice.is_info() => Tone::Error,
            NoteKey::Select(note) => note.tone(),
            NoteKey::Feature(note) => crate::features::note_tone(note),
            NoteKey::ServerDone(_) => Tone::Done,
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
            | NoteKey::PromptReverted
            | NoteKey::Toggled(..)
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
