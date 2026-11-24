//! The view half of `/diff` (ADR-013): `/diff` is `Target::Feature(Diff)`, whose `open` says the
//! verb `Intent::Run`; the host computes the diff and core asks the surface to show it
//! (`Wish::Pager`). The presenter (`FeatureView::shown`) opens the static pager over the document
//! (the diff as git coloured it, or the words of its note, `strings::note`): a dialog the size of
//! the screen over the chat, so the chat below keeps its place; `q` closes it, as the oracle's
//! `StaticOverlay` does (Esc is not one of its keys).

pub mod strings;

use std::rc::Rc;
use std::time::Duration;

use rtwk::layout::BoxWidget;
use rtwk::overlay::DialogOptions;
use rtwk::style::Style;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::Scope;
use codex_tui_core::features::diff::{Diff, Intent, Sheet, Wish};
use codex_tui_core::ids::{Cwd, ThreadId};
use codex_tui_core::screens::transcript::PagerDoc;

use super::{FeatureSheet, FeatureView, Shown};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::pages::session::transcript::pager::StaticPager;

impl FeatureView for Diff {
    /// The pager opens once for the wish (`Overlay::new_static_with_lines(.., "D I F F")`); no
    /// sheet of a scope keeps it.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let Wish::Pager { text, note } = wish;
        let doc = PagerDoc {
            title: strings::TITLE.to_owned(),
            text: note.as_ref().map_or_else(|| text.clone(), strings::note),
            action: None,
        };
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                // the alternate screen is cleared (`enter_alt_screen`): nothing of the chat
                // shows through the cells the pager leaves blank
                BoxWidget::new()
                    .style(Style::new())
                    .child(StaticPager::over(doc.clone()))
                    .into()
            }),
            options: DialogOptions {
                barrier_dismissible: false,
                barrier_alpha: Some(0),
                fade_in: Duration::ZERO,
                ..DialogOptions::default()
            },
        })
    }

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
