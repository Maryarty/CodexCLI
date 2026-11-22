//! A local note in the transcript: what a command line answered with (the
//! oracle's `new_info_event` and `new_error_event` cells). The session leaves
//! it as an entry of the thread, so it stays where the conversation was when
//! it was said. Typed keys, the UI's string tables hold the words (ADR-010).

use crate::commands::Notice;

/// How a note reads: the oracle's `add_info_message` (a dim bullet, then the
/// message) or `add_error_message` (a red `■ `).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Error,
}

/// A part of the oracle the TUI does not carry (`add_app_server_stub_message`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stub {
    /// `/debug-m-drop`, `/debug-m-update`.
    MemoryMaintenance,
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
}

impl NoteKey {
    /// The oracle shows each answer as information or as an error.
    pub fn tone(&self) -> Tone {
        match self {
            NoteKey::Command(notice) if !notice.is_info() => Tone::Error,
            NoteKey::Stub(_) | NoteKey::ParentOwned => Tone::Error,
            NoteKey::Command(_)
            | NoteKey::WorkingDirectory(_)
            | NoteKey::Rollout(_)
            | NoteKey::DesktopOpened
            | NoteKey::StoppingTerminals
            | NoteKey::RawOutput(_) => Tone::Info,
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
