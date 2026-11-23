//! The view half of `/export` (ADR-012): the words of its notes, the rows of its destination
//! picker and its filename prompt (`strings.rs`). The flow itself is
//! `codex_tui_core::features::export`.

pub mod strings;

use codex_tui_core::features::export::{Export, Menu, Note, Prompt};
use crate::widgets::picker::list::List;

use super::{FeatureView, PromptWords};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

impl FeatureView for Export {
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
