//! Browser results for links clicked in the transcript.

use codex_tui_core::features::transcript_links::{Note, TranscriptLinks};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for TranscriptLinks {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

mod strings;
