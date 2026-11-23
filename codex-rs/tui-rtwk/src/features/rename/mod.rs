//! The view half of `/rename` (ADR-012): the prompt's words and the words of its notes
//! (`strings.rs`). The flow itself is `codex_tui_core::features::rename`.

pub mod strings;

use codex_tui_core::features::Never;
use codex_tui_core::features::rename::{Note, Prompt, Rename};
use crate::widgets::picker::list::List;

use super::{FeatureView, PromptWords};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

impl FeatureView for Rename {
    fn note_words(note: &Note, words: &SessionStrings) -> String {
        strings::note(note, words)
    }

    fn menu_list(menu: &Never<Self>, _state: &(), _words: &SelectStrings) -> List {
        menu.absurd()
    }

    fn prompt_words(prompt: &Prompt, _words: &DialogStrings) -> PromptWords {
        strings::prompt(prompt)
    }
}
