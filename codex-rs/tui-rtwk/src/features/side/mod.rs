//! The view half of `/side` and `/btw` (ADR-013): the words of its notes. The feature itself is
//! `codex_tui_core::features::side` (the fork, the links between a parent and its side
//! conversation, the footer label); it has no sheet of its own: `Ctrl+/` is a key action of the
//! session page, `Ctrl+C` and `Ctrl+D` over an empty composer are the keys the feature takes
//! before the page does (`FeatureView::key`).

pub mod strings;

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::features::side::{Intent, Note, Side};
use codex_tui_core::features::{FeatureIntent, FeatureStates};
use codex_tui_core::ids::ThreadId;

use super::{FeatureView, PageKey};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Side {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    /// Ctrl+C and Ctrl+D over an empty composer in a side conversation return to its parent
    /// (`maybe_return_from_side`); anywhere else they are the page's.
    fn key<S: 'static>(
        _key: PageKey,
        thread: &ThreadId,
        draft: &str,
        cx: &mut Cx<'_, S>,
    ) -> Option<FeatureIntent> {
        let is_side = draft.is_empty() && read::<FeatureStates, _>(cx).side.is_side(thread);
        is_side.then(|| {
            Intent::Return {
                child: thread.clone(),
            }
            .into()
        })
    }
}
