//! The view half of `/diff` (ADR-013): `/diff` is `Target::Feature(Diff)`, whose `open` says the
//! verb `Intent::Run`; the host computes the diff and core asks the surface to show it
//! (`Wish::Pager`). The pager over the document, the diff's colours and the words of its notes
//! (`strings::note`) are drawn by the pager page (UI LEFT: P1 of the full-port map).

pub mod strings;

use rtwk::Cx;

use codex_tui_core::features::diff::{Diff, Intent, Sheet};
use codex_tui_core::ids::{Cwd, ThreadId};

use super::{FeatureSheet, FeatureView};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Diff {
    fn note_words(
        note: &<Diff as codex_tui_core::features::Feature>::Note,
        _words: &SessionStrings,
    ) -> String {
        note.absurd()
    }
}

impl FeatureSheet for Diff {
    type Args = Args;

    fn args(_target: Sheet, _cwd: Cwd, _thread: Option<ThreadId>) -> Args {
        Args
    }

    fn open<S: 'static>(cx: &mut Cx<'_, S>, _args: Args) {
        super::verb(cx, Intent::Run);
    }
}

/// What opens `/diff`: nothing, the verb needs no ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args;
