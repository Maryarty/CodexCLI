//! The view half of the feature slices (ADR-012): the words and the menu rows of each
//! feature of `codex_tui_core::features`, one directory per feature (`src/features/<f>/`,
//! its `strings.rs` and views). The directories are the ones of core's `feature_list!`, which
//! this file invokes with its own macros, so the two lists cannot differ. A feature implements
//! `FeatureView` for its core marker type; views of the whole is a page, a sheet or a widget of
//! its own, which a menu row or a note leads to and which are not routed here.
//!
//! The generic places call in: `Strings::note` words a `NoteKey::Feature` through
//! `note_words`, the select sheet turns a `Menu::Feature` into its list through `menu_list`.

use codex_tui_core::feature_list;
use codex_tui_core::features::{Feature, FeatureMenu, FeatureNote};
use codex_tui_core::screens::select::list::List;

use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

/// What the view of a feature says: the words of its notes and the rows of its menus.
pub trait FeatureView: Feature {
    /// The words of one of the feature's notes (the oracle's `add_info_message` text).
    fn note_words(note: &Self::Note, words: &SessionStrings) -> String;

    /// The generic list one of the feature's menus is shown as.
    fn menu_list(menu: &Self::Menu, words: &SelectStrings) -> List;
}

macro_rules! view_mods {
    ($($name:ident => $variant:ident;)*) => {
        $(pub mod $name;)*
    };
}

macro_rules! view_dispatch {
    ($($name:ident => $variant:ident;)*) => {
        /// The words of a feature's note.
        #[allow(unused_variables)]
        pub fn note_words(note: &FeatureNote, words: &SessionStrings) -> String {
            match *note {
                $(FeatureNote::$variant(ref note) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::note_words(note, words)
                })*
            }
        }

        /// The generic list a feature's menu is shown as.
        #[allow(unused_variables)]
        pub fn menu_list(menu: &FeatureMenu, words: &SelectStrings) -> List {
            match *menu {
                $(FeatureMenu::$variant(ref menu) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::menu_list(menu, words)
                })*
            }
        }
    };
}

feature_list!(view_mods);
feature_list!(view_dispatch);
