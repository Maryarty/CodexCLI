//! The view half of `/export` (ADR-013): the words of its notes, the destination picker and the
//! filename prompt. The flow itself is `codex_tui_core::features::export` (the export of the
//! conversation in use to a destination); a bare `/export` is `Target::Feature(Export)`, whose
//! `open` shows the picker, and what the user decides goes back as `Intent::Export`.

pub mod sheet;
pub mod strings;

use rtwk::overlay::show_dialog;
use rtwk::{Cx};

use codex_tui_core::features::export::{Export, Note};
use codex_tui_core::ids::ThreadId;

use self::sheet::Destinations;
use super::FeatureView;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Export {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

/// What opens `/export`: the thread in use, whose id names the file the prompt starts with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub thread: Option<ThreadId>,
}

/// A bare `/export` from the widget that asks: the destination picker over the chat.
pub fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Args) {
    drop(show_dialog::<(), S>(
        cx,
        move |_| {
            Destinations {
                thread: args.thread.clone(),
            }
            .into()
        },
        crate::sheets::sheet_options(),
    ));
}
