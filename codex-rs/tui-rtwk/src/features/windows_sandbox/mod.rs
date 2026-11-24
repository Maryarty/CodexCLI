//! The view half of the Windows sandbox prompts (ADR-013): the words of its notes. The feature is
//! `codex_tui_core::features::windows_sandbox` (the sandbox facts, the prompt that is due and its
//! `Page` data, the setup that runs, the verbs). UI LEFT: the two prompt sheets (enable, fallback:
//! header with the learn-more link, the rows of `State::page`, Esc coming back while
//! `choice_required`), the `Setting up sandbox...` status with the composer disabled while
//! `State::setting_up`, the `(non-admin sandbox)` row label and the `/setup-default-sandbox`
//! hint of the permissions page (`State::degraded`, `State::elevate_hint`).

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::windows_sandbox::{Intent, Note, Sheet, WindowsSandbox};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for WindowsSandbox {
    /// `/setup-default-sandbox` opens no sheet: it is the verb.
    type Args = ();

    fn args(_target: Sheet, _cwd: Cwd, _thread: Option<ThreadId>) {}

    fn open<S: 'static>(cx: &mut Cx<'_, S>, _args: ()) {
        super::verb(cx, Intent::Elevate);
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}
