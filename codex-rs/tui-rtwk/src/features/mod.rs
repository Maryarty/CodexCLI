//! The view half of the feature slices (ADR-012): the words and the menu rows of each
//! feature of `codex_tui_core::features`, one directory per feature (`src/features/<f>/`,
//! its `strings.rs` and views). The directories are the ones of core's `feature_list!`, which
//! this file invokes with its own macros, so the two lists cannot differ. A feature implements
//! `FeatureView` for its core marker type; views of the whole is a page, a sheet or a widget of
//! its own, which a menu row or a note leads to and which are not routed here.
//!
//! The generic places call in: `Strings::note` words a `NoteKey::Feature` through
//! `note_words`, the select sheet turns a `Menu::Feature` into its list through `menu_list`
//! (from the feature's state, which the app provides to the views like the screens'),
//! the prompt dialog words a `PromptKind::Feature` through `prompt_words`.

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::app::Intent;
use codex_tui_core::feature_list;
use codex_tui_core::features::{
    Feature, FeatureIntent, FeatureMenu, FeatureNote, FeaturePrompt, FeatureStates,
};
use codex_tui_core::screens::select::list::List;

use crate::app::view::Hs;

use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

/// What a text prompt of a feature shows (oracle `bottom_pane/custom_prompt_view.rs`): the
/// title over the field and the placeholder of an empty field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptWords {
    pub title: String,
    pub placeholder: String,
}

/// What the view of a feature says: the words of its notes, the rows of its menus and the
/// words of its text prompt.
pub trait FeatureView: Feature {
    /// The words of one of the feature's notes (the oracle's `add_info_message` text).
    fn note_words(note: &Self::Note, words: &SessionStrings) -> String;

    /// The generic list one of the feature's menus is shown as. The menu says which page it is;
    /// the rows come from the feature's `state`, read again at every build, so a menu whose rows
    /// change while it is open (install progress, a stale answer) shows them without being
    /// opened again: the picker keeps its cursor, tab and query.
    fn menu_list(menu: &Self::Menu, state: &Self::State, words: &SelectStrings) -> List;

    /// The title and placeholder of one of the feature's text prompts.
    fn prompt_words(prompt: &Self::Prompt, words: &DialogStrings) -> PromptWords;
}

/// A verb of a feature, from the widget that has the user's decision: the app applies it
/// (`Intent::Feature`). The arguments are ids (a directory, a plugin's row), never what was
/// pressed.
pub fn verb<S: 'static>(cx: &mut Cx<'_, S>, intent: impl Into<FeatureIntent>) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.intent, Intent::Feature(intent.into()));
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

        /// The title and placeholder of a feature's text prompt.
        #[allow(unused_variables)]
        pub fn prompt_words(prompt: &FeaturePrompt, words: &DialogStrings) -> PromptWords {
            match *prompt {
                $(FeaturePrompt::$variant(ref prompt) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::prompt_words(prompt, words)
                })*
            }
        }

        /// The generic list a feature's menu is shown as, from the feature's state.
        #[allow(unused_variables)]
        pub fn menu_list(
            menu: &FeatureMenu,
            states: &FeatureStates,
            words: &SelectStrings,
        ) -> List {
            match *menu {
                $(FeatureMenu::$variant(ref menu) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::menu_list(
                        menu,
                        &states.$name,
                        words,
                    )
                })*
            }
        }
    };
}

feature_list!(view_mods);
feature_list!(view_dispatch);
