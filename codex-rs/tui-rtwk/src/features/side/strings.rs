//! The words of `/side` and `/btw` (oracle `app/side.rs`): the lines it says in the transcript when
//! a side conversation cannot be started, prepared or closed. The wording is the oracle's English
//! until the catalog has these entries.

use codex_tui_core::features::side::Note;

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::MainUnavailable => {
            "'/side' is unavailable until the main thread is ready.".to_owned()
        }
        Note::BeforeSession(command) => {
            format!("'/{command}' is unavailable before the session starts.")
        }
        Note::NoStartedConversation => concat!(
            "'/side' is unavailable until the current conversation has started. ",
            "Send a message first, then try /side again."
        )
        .to_owned(),
        Note::AlreadyOpen => {
            "A side conversation is already open. Press ctrl + c to return before starting another."
                .to_owned()
        }
        Note::StartFailed(error) => format!("Failed to start side conversation: {error}"),
        Note::PrepareFailed { thread, error } => {
            format!("Failed to prepare side conversation {thread}: {error}")
        }
        Note::CloseFailed { thread, error } => {
            format!("Failed to close side conversation {thread}; it is still open: {error}")
        }
    }
}
