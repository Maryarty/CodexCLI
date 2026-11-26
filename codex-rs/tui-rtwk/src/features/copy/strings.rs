//! The words of `/copy` (oracle `chatwidget/interaction.rs` `show_copy_result`,
//! `prepare_last_response_copy`, `clipboard_copy.rs` `CopyStatus::message`): its transcript notes
//! and the picker's title. The wording is the oracle's English until the catalog has these entries.

use codex_tui_core::app::effects::CopyStatus;
use codex_tui_core::features::copy::Note;

/// The title of the copy picker (oracle `show_copy_picker`).
pub const PICKER_TITLE: &str = "Copy to clipboard";

/// The row of the whole response (oracle `show_copy_picker`).
pub const WHOLE_RESPONSE: &str = "Whole response";

pub const WHOLE_STATUS: &str = "Whole status";
pub const STATUS_MODEL: &str = "Model";
pub const STATUS_DIRECTORY: &str = "Directory";
pub const STATUS_THREAD_NAME: &str = "Thread name";
pub const STATUS_SESSION_ID: &str = "Session ID";

/// The row of a code block that names no language.
pub const CODE_BLOCK: &str = "Code block";

/// The row of a quote.
pub const BLOCKQUOTE: &str = "Blockquote";

/// The row of a code block in `language`.
pub fn code_in(language: &str) -> String {
    format!("{language} code")
}

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::NoResponse => "No agent response to copy".to_owned(),
        Note::NothingToCopy => "Nothing to copy".to_owned(),
        Note::Copied { label, result } => match result {
            Ok(CopyStatus::Confirmed) => format!("Copied {label} to clipboard"),
            Ok(CopyStatus::Unconfirmed) => "Copy unconfirmed; /export saves chat".to_owned(),
            Ok(CopyStatus::Pending(_)) => format!("Copying {label}…"),
            Ok(CopyStatus::Busy) => "Copy already in progress".to_owned(),
            Err(error) => format!("Copy failed: {error}"),
        },
    }
}
