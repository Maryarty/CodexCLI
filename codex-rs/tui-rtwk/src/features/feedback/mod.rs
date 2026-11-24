//! The view half of `/feedback` (ADR-013): the flow itself is
//! `codex_tui_core::features::feedback` (the report's checks, its delivery to the host and the
//! line that says how it went). The three steps the user takes before a report exists (the
//! category, the consent, the note) are this directory's sheet: `/feedback` says the verb
//! `Intent::Open`, the feature wishes the flow (`Wish::Flow`) and the presenter opens the sheet.
//! What the success cell looks like is `cell.rs`; the words are `strings.rs`.

mod cell;
mod debug;
mod note;
mod sheet;
mod steps;
pub mod strings;

use std::rc::Rc;

use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::Scope;
use codex_tui_core::features::feedback::{Feedback, Intent, Note, Sheet, Wish};
use codex_tui_core::ids::{Cwd, ThreadId};
use codex_tui_core::screens::{Kind, dialogs};

use self::sheet::FeedbackSheet;
use super::{FeatureSheet, FeatureView, Shown};
use crate::app::debug::Dir;
use crate::pages::session::strings::Strings as SessionStrings;

/// The screen map's slot of the flow: it was the dialogs' (S23) and stays on its id.
const ID: &str = dialogs::ID;
const KIND: Kind = Kind::Dialog;

impl FeatureView for Feedback {
    /// The flow opens once for the wish, over the chat: each step is the sheet's own.
    fn shown(wish: &Wish, _scope: &Scope) -> Option<Shown> {
        let Wish::Flow { turn_id } = wish;
        let turn_id = turn_id.clone();
        Some(Shown {
            held_by: None,
            open: Rc::new(move || -> AnyWidget {
                FeedbackSheet {
                    start: steps::Step::Category,
                    text: "",
                    turn_id: turn_id.clone(),
                }
                .into()
            }),
            options: crate::sheets::sheet_options(),
        })
    }

    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    fn cell(note: &Note) -> Option<AnyWidget> {
        cell::of(note)
    }

    fn debug_dir() -> Option<Dir> {
        Some(Dir {
            name: <Feedback as codex_tui_core::features::Feature>::NAME,
            id: ID,
            kind: KIND,
            title: strings::title,
            debug_entries: debug::entries,
            open_empty: debug::open_empty,
        })
    }
}

impl FeatureSheet for Feedback {
    type Args = Args;

    fn args(_target: Sheet, _cwd: Cwd, thread: Option<ThreadId>) -> Args {
        Args { thread }
    }

    /// The verb checks the session can deliver a report; the flow opens as the feature's wish.
    fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Args) {
        super::verb(cx, Intent::Open { source: args.thread });
    }
}

/// What opens `/feedback`: the thread the command was typed in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub thread: Option<ThreadId>,
}
