//! The view half of the external editor (ADR-013): the words of its notes. The feature itself is
//! `codex_tui_core::features::external_editor` (the draft goes to the host's editor runner and
//! the text it left comes back as `State::edited`); it has no sheet of its own.

pub mod strings;

use rtwk::Cx;
use rtwk::provider::{Provider, select};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::external_editor::{ExternalEditor, Intent, Note};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::keymap::actions::GLOBAL_OPEN_EXTERNAL_EDITOR;

use super::{FeatureAction, FeatureView};
use crate::app::view::Hs;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for ExternalEditor {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    /// The text the editor left for the composer of `thread`, numbered, until the composer
    /// applied it (`State::edited`). Where the app's features are not provided (a composer shown
    /// alone) there is none.
    fn handed<S: 'static>(thread: &ThreadId, cx: &mut Cx<'_, S>) -> Option<(u64, String)> {
        Provider::<FeatureStates>::maybe_of(cx, false)?;
        let thread = thread.clone();
        select::<FeatureStates, _, _>(cx, move |states| {
            let edited = states.external_editor.edited.as_ref()?;
            (edited.thread == thread).then(|| (edited.seq, edited.text.clone()))
        })
    }

    /// The composer applied the edit numbered `number`: the feature forgets it.
    fn taken<S: 'static>(number: u64, hs: &Hs, cx: &mut Cx<'_, S>) {
        super::verb_by(cx, hs, Intent::Applied { seq: number });
    }

    /// `Ctrl+G`: the draft as it stands goes to the editor (`launch_external_editor`).
    fn actions() -> Vec<FeatureAction> {
        vec![FeatureAction {
            id: GLOBAL_OPEN_EXTERNAL_EDITOR,
            also: &[],
            run: |cx| {
                let (thread, draft) = (cx.thread.clone(), cx.draft.to_owned());
                cx.say(Intent::Open { thread, draft });
            },
        }]
    }
}
