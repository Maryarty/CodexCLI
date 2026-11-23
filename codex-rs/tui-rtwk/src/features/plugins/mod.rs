//! The view half of `/plugins` (ADR-013): the sheet over the chat and the pages in it. The feature
//! itself is `codex_tui_core::features::plugins` (the domain: the catalog and detail of a
//! directory as `Remote`s, the changes the user asked for as ops by key, the setup of an
//! installed plugin's apps as a wish); nothing here keeps a copy of it.
//!
//! The template of a feature's UI half: `mod.rs` (this: `open`, with ids as its arguments, and the
//! `FeatureView` words), `sheet.rs` (the modal sheet: a nested `Navigator` in an
//! `IntrinsicHeight`, the `Interest` that has the feature load, `alive`), `pages.rs` (the routes
//! of the navigator: one `StatefulWidget` per page, each selecting the feature's part of the app
//! and sending verbs), `lists.rs` (what each page shows, as the shared picker's lists),
//! `strings.rs` (the words).

pub mod lists;
pub mod pages;
pub mod sheet;
pub mod strings;

use rtwk::Cx;
use rtwk::overlay::{DialogOptions, show_dialog};
use rtwk::provider::read;

use codex_tui_core::features::plugins::{Intent, Plugins, Say, State};
use codex_tui_core::features::{FeatureStates, Never};
use codex_tui_core::screens::select::list::List;

use self::sheet::{PluginsSheet, Start};
use super::{FeatureView, PromptWords, verb};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

impl FeatureView for Plugins {
    fn note_words(note: &Say, _words: &SessionStrings) -> String {
        strings::note(note)
    }

    // The feature has no menu and no prompt of its own: its pages are the sheet's routes.
    fn menu_list(menu: &Never<Plugins>, _state: &State, _words: &SelectStrings) -> List {
        menu.absurd()
    }

    fn prompt_words(prompt: &Never<Plugins>, _words: &DialogStrings) -> PromptWords {
        prompt.absurd()
    }
}

/// What opens `/plugins`: the directory the plugins are listed for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub cwd: String,
}

/// `/plugins` from the widget that asks: the feature is told (it says so in the history when it
/// is off), and the sheet opens when it is on. The sheet shows the feature's state and asks for
/// nothing itself: it says it is interested (`Interest`) and the feature loads.
pub fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Args) {
    let on = read::<FeatureStates, _>(cx).plugins.enabled();
    let cwd = args.cwd;
    verb(cx, Intent::Run { cwd: cwd.clone() });
    if on {
        drop(show_dialog::<(), S>(
            cx,
            move |_| {
                PluginsSheet {
                    cwd: cwd.clone(),
                    start: Start::Browser,
                }
                .into()
            },
            sheet_options(),
        ));
    }
}

/// The options the sheet is shown with: the bottom pane, built under the providers of the page
/// that asks.
pub fn sheet_options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        ..crate::sheets::sheet_options()
    }
}
