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

use std::rc::Rc;

use rtwk::overlay::{DialogOptions, show_dialog};
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::commands::SlashCommand;
use codex_tui_core::features::plugins::{Intent, Plugins, Say, State, Wish};
use codex_tui_core::features::{FeatureStates, Never};
use codex_tui_core::screens::select::list::List;

use self::sheet::{PluginsSheet, Start};
use super::{FeatureView, PromptWords, Shown, verb};
use crate::pages::session::strings::Strings as SessionStrings;
use crate::sheets::dialogs::strings::Strings as DialogStrings;
use crate::sheets::select::strings::Strings as SelectStrings;

/// The kind of sheet the plugins feature shows (what its open sheet leaves in the `Door`).
pub const KIND: &str = "plugins";

impl FeatureView for Plugins {
    const SHEET: Option<SlashCommand> = Some(SlashCommand::Plugins);

    fn open_sheet<S: 'static>(cx: &mut Cx<'_, S>, scope: &str) {
        open(
            cx,
            Args {
                cwd: scope.to_owned(),
            },
        );
    }

    /// The setup of the apps of an installed plugin: a page of the open sheet of its directory,
    /// else the sheet opens at it.
    fn shown(wish: &Wish, scope: &str) -> Option<Shown> {
        let Wish::Auth(key) = wish;
        let (page, start) = (key.clone(), key.clone());
        let cwd = scope.to_owned();
        Some(Shown {
            sheet: KIND,
            scope: cwd.clone(),
            push: Box::new(move |nav| drop(nav.push::<()>(pages::auth_route(page.clone())))),
            open: Rc::new(move || -> AnyWidget {
                PluginsSheet {
                    cwd: cwd.clone(),
                    start: Start::Auth(start.clone()),
                }
                .into()
            }),
            options: sheet_options(),
        })
    }

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
