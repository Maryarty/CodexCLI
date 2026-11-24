//! The words of the small commands (oracle `app/event_dispatch.rs` `Logout`,
//! `chatwidget/slash_dispatch.rs` `Daybreak`, `app/daybreak.rs`, `chatwidget/ide_context.rs`):
//! their transcript notes. An info line with a hint is the line, a newline, then the hint. The
//! wording is the oracle's English until the catalog has these entries.

use codex_tui_core::features::small_cmds::{DaybreakFailure, DefaultFailure, Note};

const DAYBREAK_NOT_AVAILABLE: &str = "Daybreak is not available for this account. Apply at https://openai.com/form/enterprise-trusted-access-for-cyber/. Learn more at https://help.openai.com/en/articles/20001326.";

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::LogoutFailed(error) => format!("Logout failed: {error}"),
        Note::DaybreakNeedsProvider => "Daybreak requires the OpenAI provider with either ChatGPT sign-in or an API key with Daybreak support enabled.".to_owned(),
        Note::DaybreakNotAvailable => DAYBREAK_NOT_AVAILABLE.to_owned(),
        Note::DaybreakUndetermined => "Daybreak availability could not be determined from the connected host's model catalog.".to_owned(),
        Note::DaybreakNoThread => "Daybreak is unavailable until the thread starts.".to_owned(),
        Note::DaybreakFailed(DaybreakFailure::Save(error)) => {
            format!("Failed to save Daybreak for this thread: {error}")
        }
        Note::DaybreakFailed(DaybreakFailure::Read(error)) => {
            format!("Failed to read the thread to save Daybreak: {error}")
        }
        Note::DaybreakDefault(DefaultFailure::Save(error)) => {
            format!("Failed to save the default Daybreak preference: {error}")
        }
        Note::DaybreakDefault(DefaultFailure::Read(error)) => {
            format!("Failed to read the default Daybreak preference: {error}")
        }
        Note::DaybreakSet(on) => format!(
            "Daybreak {}. Applies to new turns.",
            if *on { "on" } else { "off" }
        ),
        Note::IdeOff => "IDE context is off.".to_owned(),
        Note::IdeOn { has_context: true } => {
            "IDE context is on.\nFuture messages will include your current IDE selection and open tabs.".to_owned()
        }
        Note::IdeOn { has_context: false } => "IDE context is on.\nConnected to your IDE.".to_owned(),
        Note::IdeCouldNotEnable { hint } => format!("IDE context could not be enabled.\n{hint}"),
        Note::IdeUsage => "Usage: /ide [on|off|status]".to_owned(),
        Note::DebugConfig(rows) => rows
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
