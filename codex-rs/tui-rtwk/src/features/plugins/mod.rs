//! The view half of `/plugins` (ADR-013): the sheet over the chat and the pages in it. The feature
//! itself is `codex_tui_core::features::plugins` (the domain: the catalog and detail of a
//! directory as `Remote`s, the changes the user asked for as ops by key, the setup of an
//! installed plugin's apps as a wish); nothing here keeps a copy of it.
//!
//! The template of a feature's UI half: `mod.rs` (this: `open`, with ids as its arguments, and the
//! `FeatureView` words), `sheet.rs` (the modal sheet: a nested `Navigator` as tall as the sheet's
//! cap, the `Interest` that has the feature load, `alive`), `pages.rs` (the routes
//! of the navigator: one `StatefulWidget` per page, each selecting the feature's part of the app
//! and sending verbs), `lists.rs` (what each page shows, as the shared picker's lists),
//! `strings.rs` (the words).

pub mod lists;
pub mod pages;
pub mod sheet;
pub mod strings;

use std::rc::Rc;

use rtwk::overlay::{DialogOptions, show_dialog};
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::features::plugins::{Intent, Interest, Plugins, Say, Sheet, Wish};
use codex_tui_core::features::{FeatureInterest, FeatureStates, Scope};
use codex_tui_core::ids::{Cwd, ThreadId};

use self::sheet::PluginsSheet;
use super::{FeatureView, Shown, verb};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for Plugins {
    type Args = Args;

    fn args(_target: Sheet, cwd: Cwd, _thread: Option<ThreadId>) -> Args {
        Args { cwd }
    }

    fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Args) {
        open(cx, args)
    }

    /// The setup of the apps of an installed plugin: the sheet of its directory shows the page
    /// from the feature's state (`State::setups_in`); the wish only has one opened when none
    /// is open for the directory, which the sheet's interest tells.
    fn shown(wish: &Wish, scope: &Scope) -> Option<Shown> {
        let Wish::Auth(_) = wish;
        let Scope::Cwd(cwd) = scope else {
            return None;
        };
        let held_by = Some(FeatureInterest::from(Interest::Listing {
            cwd: cwd.clone(),
        }));
        let cwd = cwd.clone();
        Some(Shown {
            held_by,
            open: Rc::new(move || -> AnyWidget { PluginsSheet { cwd: cwd.clone() }.into() }),
            options: sheet_options(),
        })
    }

    fn note_words(note: &Say, _words: &SessionStrings) -> String {
        strings::note(note)
    }
}

/// What opens `/plugins`: the directory the plugins are listed for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub cwd: Cwd,
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
            move |_| PluginsSheet { cwd: cwd.clone() }.into(),
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
