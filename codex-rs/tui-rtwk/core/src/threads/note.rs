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

/// What a note says, before the UI words it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteKey {
    /// A command line was refused, or its arguments are wrong (dispatch).
    Command(Notice),
}

/// One note of the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub tone: Tone,
    pub key: NoteKey,
}

impl Note {
    pub fn info(key: NoteKey) -> Self {
        Self {
            tone: Tone::Info,
            key,
        }
    }

    pub fn error(key: NoteKey) -> Self {
        Self {
            tone: Tone::Error,
            key,
        }
    }
}

impl From<Notice> for Note {
    /// The oracle shows `Notice::is_info` as information and every other as an
    /// error.
    fn from(notice: Notice) -> Self {
        let tone = if notice.is_info() {
            Tone::Info
        } else {
            Tone::Error
        };
        Self {
            tone,
            key: NoteKey::Command(notice),
        }
    }
}
