//! The sheet of `/plugins` (ADR-013 O1): a modal bottom sheet whose body is a nested `Navigator`
//! as tall as the sheet's cap, its pages at the bottom of it (`pages::bottom`: `IntrinsicHeight`
//! cannot size a navigator whose page is built by a `LayoutBuilder`, `tests/rtwk_o1.rs`). The
//! routes of the navigator are the pages (`pages.rs`);
//! Esc pops one, and the browser under them keeps its picker (cursor, query, tab) because it is the
//! first route and is never rebuilt from scratch. The sheet says it wants the plugins of its
//! directory (`Interest`), so the feature loads them for as long as the sheet is open, and closes
//! when it has no reason to stay (`Sheet::alive`: the feature was turned off).
//!
//! In Flutter this is `showModalBottomSheet` with a `Navigator` as the body (the nested-navigator
//! recipe for a flow inside a sheet), `PopScope` per page, and a provider the pages select from.

use rtwk::navigation::Navigator;
use rtwk::overlay::close_dialog;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::plugins::{Interest as Wanted, PluginKey};

use super::pages::{Home, bottom};
use crate::widgets::interest::Interest;
use crate::widgets::sheet::Sheet;

/// Where the sheet opens.
#[derive(Clone, Debug, PartialEq)]
pub enum Start {
    /// The browser: `/plugins`.
    Browser,
    /// The setup of the apps of an installed plugin, over the browser (the feature's `Wish`).
    Auth(PluginKey),
}

/// The sheet of the plugins of `cwd`.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginsSheet {
    pub cwd: String,
    pub start: Start,
}

pub struct PluginsSheetState;

impl StatefulWidget for PluginsSheet {
    type State = PluginsSheetState;

    fn create_state(&self) -> PluginsSheetState {
        PluginsSheetState
    }
}

impl_widget!(stateful eq PluginsSheet);

impl Sheet for PluginsSheet {
    /// Whether the plugins feature is on.
    type Seen = bool;

    fn alive(&self, enabled: &bool) -> bool {
        *enabled
    }
}

impl State for PluginsSheetState {
    type Widget = PluginsSheet;
    type Msg = ();

    fn did_change_dependencies(&mut self, w: &PluginsSheet, cx: &mut Cx<'_, Self>) {
        let enabled = select::<FeatureStates, bool, _>(cx, |features| features.plugins.enabled());
        if !w.alive(&enabled) {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn build(&self, w: &PluginsSheet, _: &mut Cx<'_, Self>) -> AnyWidget {
        let browser = Home {
            cwd: w.cwd.clone(),
            start: w.start.clone(),
        };
        let pages = Navigator::new().home(bottom(browser)).back_key(None);
        Interest::new(Wanted::Listing { cwd: w.cwd.clone() }, pages).into()
    }
}
