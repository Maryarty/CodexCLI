//! `settings`: the view half of S15 (settings dialogs): modal sheets over the
//! session page, each drawn without a frame at the bottom edge, as the
//! oracle draws its bottom-pane views (architecture §1, amendment
//! 2026-11-19b). `Screen` is the one widget the app opens, by the kind of
//! dialog a slash command names (`open`, `Args`); the feature builds the
//! dialog's catalogue from its facts (`Intent::Open`) and the sheet draws
//! it when it is there. What each dialog is before it is saved (its checks,
//! its order, its page) is its sheet's draft (`draft.rs`); the feature keeps
//! what it is offered and what it asked of the app (`Edit`, ADR-013). The
//! feature is `codex_tui_core::features::settings`.

use codex_tui_core::features::settings::{ID, KIND, Settings};

use super::FeatureView;
use crate::app::debug::Dir;
use crate::pages::session::strings::Strings as SessionStrings;

pub mod config_panel;
pub mod draft;
pub mod features;
pub mod hooks;
pub mod items;
mod keys;
pub mod memories;
mod open;
pub mod setup;
mod shown;
pub mod strings;
pub mod theme;
mod which;

pub use open::{debug_entries, open, open_empty};
pub use shown::{Screen, ScreenState};
pub use which::Args;
use open::request;

impl FeatureView for Settings {
    fn note_words(
        note: &codex_tui_core::features::settings::Note,
        _words: &SessionStrings,
    ) -> String {
        strings::note(note)
    }

    /// The slot of the dialogs in the debug menu: each dialog on a state worth opening.
    fn debug_dir() -> Option<Dir> {
        Some(Dir {
            name: <Settings as codex_tui_core::features::Feature>::NAME,
            id: ID,
            kind: KIND,
            title: strings::title,
            debug_entries,
            open_empty,
        })
    }
}

