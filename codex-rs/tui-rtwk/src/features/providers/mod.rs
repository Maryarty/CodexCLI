//! The view half of the model provider list and editor (ADR-013): the settings panel's `Model
//! providers` row opens the sheet (`open`), a nested `Navigator` over the list the server
//! answered with (`list.rs`), the details pager of a provider (the oracle's static overlay) and the
//! form that edits a provider or creates a gateway (`form.rs`). The feature itself is
//! `codex_tui_core::features::providers` (the read of the list, the write under way, the outcome
//! of a write); nothing here keeps a copy of it but what the user is typing.

mod debug;
mod form;
mod list;
mod sheet;
pub mod strings;

use rtwk::Cx;
use rtwk::overlay::show_dialog;

use codex_tui_core::features::Feature;
use codex_tui_core::features::providers::{Note, Providers};

use super::FeatureView;
use crate::app::debug::Dir;
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Providers {
    fn note_words(note: &Note, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    fn debug_dir() -> Option<Dir> {
        Some(Dir {
            name: <Providers as Feature>::NAME,
            id: debug::ID,
            kind: debug::KIND,
            title: strings::title,
            debug_entries: debug::entries,
            open_empty: debug::open_empty,
        })
    }
}

/// The sheet of the providers, from the widget that asks (the settings panel's `Model providers`
/// row): it says it wants the list (`Interest::List`) and the feature reads it.
pub fn open<S: 'static>(cx: &mut Cx<'_, S>) {
    drop(show_dialog::<(), S>(
        cx,
        |_| sheet::sheet(),
        sheet::options(),
    ));
}
