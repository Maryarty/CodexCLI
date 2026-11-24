//! The view half of the external editor (ADR-013): the words of its notes. The feature itself is
//! `codex_tui_core::features::external_editor` (the draft goes to the host's editor runner and
//! the text it left comes back as `State::edited`); it has no sheet of its own.

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::Never;
use codex_tui_core::features::external_editor::{ExternalEditor, Note};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for ExternalEditor {
    /// A verb, not a sheet: the key says it.
    type Args = Never<Self>;

    fn args(target: Never<Self>, _cwd: Cwd, _thread: Option<ThreadId>) -> Never<Self> {
        target.absurd()
    }

    fn open<S: 'static>(_cx: &mut Cx<'_, S>, args: Never<Self>) {
        args.absurd()
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
