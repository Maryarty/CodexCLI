//! The words of `/rename` (oracle `chatwidget/interaction.rs` `show_rename_prompt`): the prompt's
//! title and placeholder, and the three lines it says in the transcript, read from the session
//! catalog's entries (`thread_name_empty`, `side_rename`, `session_starting`).

use codex_tui_core::features::rename::{Note, Prompt};
use rtwk::l10n::{Arg, Fmt};

use crate::features::PromptWords;
use crate::pages::session::strings::Strings as SessionStrings;

/// The prompt (`Rename thread` over a thread that has a name, `Name thread` over one that has
/// none).
pub fn prompt(prompt: &Prompt) -> PromptWords {
    let Prompt::Name { renaming, .. } = prompt;
    PromptWords {
        title: if *renaming {
            "Rename thread"
        } else {
            "Name thread"
        }
        .into(),
        placeholder: "Type a name and press Enter".into(),
    }
}

/// The words of one of the feature's notes.
pub fn note(note: &Note, words: &SessionStrings) -> String {
    match note {
        Note::SessionStarting => {
            Fmt::new(words.session_starting, [Arg::Text("rename")], &words.num).to_string()
        }
        Note::SideRename => words.side_rename.to_string(),
        Note::NameEmpty => words.thread_name_empty.to_string(),
    }
}
