//! The words of `/side` and `/btw` (oracle `app/side.rs`): the lines it says in the transcript when
//! a side conversation cannot be started, prepared or closed. The wording is the oracle's English
//! until the catalog has these entries.

use codex_tui_core::features::side::{Footer, Note};

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::MainUnavailable => "'/side' is unavailable until the main thread is ready.".to_owned(),
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

/// The footer's words (`sync_side_thread_ui`): `Side from main thread · main finished · ctrl + /
/// to switch · ctrl + c to close` over a side conversation, `ctrl + / for side` over its parent.
pub fn footer(footer: &Footer) -> String {
    match footer {
        Footer::Parent { switch } => format!("{switch} for side"),
        Footer::Side {
            parent_is_main,
            parent_label,
            status,
            switch,
        } => {
            let mut parts = Vec::new();
            if *parent_is_main {
                parts.push("from main thread".to_owned());
            } else {
                parts.push(format!("from parent thread ({parent_label})"));
            }
            if let Some(status) = status {
                parts.push(status.label(*parent_is_main).to_owned());
            }
            if let Some(key) = switch {
                parts.push(format!("{key} to switch"));
            }
            parts.push("ctrl + c to close".to_owned());
            format!("Side {}", parts.join(" · "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::features::side::ParentStatus;

    #[test]
    fn the_footer_words_the_label() {
        let side = Footer::Side {
            parent_is_main: true,
            parent_label: String::new(),
            status: Some(ParentStatus::Finished),
            switch: Some("ctrl + /".to_owned()),
        };
        assert_eq!(
            footer(&side),
            "Side from main thread · main finished · ctrl + / to switch · ctrl + c to close"
        );
        let parent = Footer::Side {
            parent_is_main: false,
            parent_label: "agent".to_owned(),
            status: None,
            switch: None,
        };
        assert_eq!(
            footer(&parent),
            "Side from parent thread (agent) · ctrl + c to close"
        );
        let over = Footer::Parent {
            switch: "ctrl + /".to_owned(),
        };
        assert_eq!(footer(&over), "ctrl + / for side");
    }
}
