//! The view half of `/goal` (ADR-012): the words of its notes and the rows of its two
//! confirmations (`strings.rs`). The flow itself is `codex_tui_core::features::goal`.

pub mod strings;

use codex_tui_core::features::goal::{Goal, Menu, Note, Prompt};
use crate::widgets::picker::list::List;

use super::{FeatureView, PromptWords};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

impl FeatureView for Goal {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    fn menu_list(menu: &Menu, _state: &(), _words: &SelectStrings) -> List {
        strings::menu(menu)
    }

    fn prompt_words(prompt: &Prompt, _words: &DialogStrings) -> PromptWords {
        strings::prompt(prompt)
    }
}
