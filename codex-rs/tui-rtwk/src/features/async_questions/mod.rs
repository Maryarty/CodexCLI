//! The view half of the questions the model asks while it works (ADR-013): the slice itself is
//! `codex_tui_core::features::async_questions` (which questions of which thread wait). It has no
//! note and no sheet of its own: the editor is `sheets::approvals::questions`, shown by the chat
//! page in place of the composer.

use codex_tui_core::features::Feature;
use codex_tui_core::features::async_questions::AsyncQuestions;

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for AsyncQuestions {
    fn note_words(note: &<AsyncQuestions as Feature>::Note, _words: &SessionStrings) -> String {
        note.absurd()
    }
}
