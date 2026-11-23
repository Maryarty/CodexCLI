//! The view half of `selection` (ADR-012): the transcript's selection area, which hands what
//! the reader selected to the app (`transcript.rs`), and the result of the copy in the gap above
//! the composer (`feedback.rs`, words in `strings.rs`). The flow itself is
//! `codex_tui_core::features::selection`.

mod feedback;
pub mod strings;
mod transcript;

use rtwk::AnyWidget;

use codex_tui_core::features::Never;
use codex_tui_core::features::selection::{Selection, State};
use codex_tui_core::screens::select::list::List;

use super::{FeatureView, PromptWords};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

impl FeatureView for Selection {
    fn note_words(note: &Never<Self>, _words: &SessionStrings) -> String {
        note.absurd()
    }

    fn menu_list(menu: &Never<Self>, _state: &State, _words: &SelectStrings) -> List {
        menu.absurd()
    }

    fn prompt_words(prompt: &Never<Self>, _words: &DialogStrings) -> PromptWords {
        prompt.absurd()
    }

    fn around_transcript(child: AnyWidget) -> AnyWidget {
        transcript::SelectionHost { child }.into()
    }

    fn composer_gap(state: &State, gap: AnyWidget, lead: bool) -> AnyWidget {
        match &state.feedback {
            Some(feedback) => feedback::CopyFeedback {
                feedback: feedback.clone(),
                lead,
                fallback: gap,
            }
            .into(),
            None => gap,
        }
    }
}
