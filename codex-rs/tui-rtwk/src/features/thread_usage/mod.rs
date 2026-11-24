//! The view half of the thread-usage slice (ADR-013): it has no sheet, wish or note of its own.
//! The figure is read from the slice's state by what shows it (the status line, the terminal
//! title, the `/status` card), which raise `UsageInterest` while they are drawn. The feature
//! itself is `codex_tui_core::features::thread_usage`.

use codex_tui_core::features::Feature;
use codex_tui_core::features::thread_usage::ThreadUsageCost;

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for ThreadUsageCost {
    fn note_words(note: &<ThreadUsageCost as Feature>::Note, _words: &SessionStrings) -> String {
        note.absurd()
    }
}
