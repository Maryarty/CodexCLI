//! The words of `/export` (oracle `chatwidget/transcript_export.rs`, `app/transcript_export.rs`,
//! `show_copy_result`): its transcript notes, the rows of its destination picker and its
//! filename prompt. The wording is the oracle's English until the catalog has these entries.

use codex_tui_core::app::effects::CopyStatus;
use codex_tui_core::features::export::{CLIPBOARD, FILE, Menu, Note, Prompt};
use codex_tui_core::screens::select::list::{Columns, Item, List};

use crate::features::PromptWords;

/// The words of one of the feature's notes.
pub fn note(note: &Note) -> String {
    match note {
        Note::Saved(path) => format!("Saved conversation to {path}"),
        Note::Failed(error) => format!("Export failed: {error}"),
        Note::NoConversation => "Export failed: No active conversation to export.".to_owned(),
        Note::NoContent => "Export failed: No conversation content to export.".to_owned(),
        Note::Copy(Ok(CopyStatus::Confirmed)) => "Copied conversation to clipboard".to_owned(),
        Note::Copy(Ok(CopyStatus::Unconfirmed)) => "Copy unconfirmed; /export saves chat".to_owned(),
        Note::Copy(Ok(CopyStatus::Pending(_))) => "Copying conversation…".to_owned(),
        Note::Copy(Ok(CopyStatus::Busy)) => "Copy already in progress".to_owned(),
        Note::Copy(Err(error)) => format!("Copy failed: {error}"),
    }
}

/// The filename prompt (oracle `show_transcript_export_file_prompt`).
pub fn prompt(prompt: &Prompt) -> PromptWords {
    match prompt {
        Prompt::File => PromptWords {
            title: "Save conversation".into(),
            placeholder: "Type a filename and press Enter".into(),
        },
    }
}

/// The destination picker (oracle `show_transcript_export_popup`), shaped as
/// `SelectionViewParams` with the standard hint.
pub fn menu(menu: &Menu) -> List {
    let Menu::Destination = menu;
    let row = |id: &str, name: &str, description: &str| Item {
        id: id.to_owned(),
        name: name.to_owned(),
        description: Some(description.to_owned()),
        ..Item::default()
    };
    List {
        title: Some("Export conversation".to_owned()),
        subtitle: Some("Save the complete conversation as Markdown".to_owned()),
        standard_hint: true,
        items: vec![
            row(
                CLIPBOARD,
                "Copy to clipboard",
                "Copy the complete Markdown transcript",
            ),
            row(FILE, "Save to file", "Choose a Markdown filename"),
        ],
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
