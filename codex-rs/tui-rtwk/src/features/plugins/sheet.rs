//! The sheet of `/plugins` (ADR-013 O1): a modal bottom sheet whose body is a nested `Navigator`.
//! A `Navigator` is an `Overlay`, so it always takes the largest size it is given: the sheet gives
//! it the height of the sheet's cap and each page sits at the bottom edge of it (`pages::bottom`,
//! `Align(BOTTOM_CENTER)`), which is where the oracle's pane is. That is the pattern for a nested
//! navigator in a sheet, not a workaround: `tests/rtwk_o1.rs` pins it. The routes of the
//! navigator are the pages (`pages.rs`). Esc does what the oracle's page does (`pages.rs`: it
//! closes the sheet, or goes back where the oracle goes back), and the browser under a page that
//! goes back keeps its picker (cursor, query, tab) because it is the first route and is never
//! rebuilt from scratch. The sheet says it wants the plugins of its directory (`Interest`), so
//! the feature loads them for as long as the sheet is open, and closes when it has no reason to
//! stay (`Sheet::alive`: the feature was turned off).
//!
//! In Flutter this is `showModalBottomSheet` with a `Navigator` as the body (the nested-navigator
//! recipe for a flow inside a sheet), `PopScope` per page, and a provider the pages select from.

use rtwk::navigation::Navigator;
use rtwk::overlay::close_dialog;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::plugins::Interest as Wanted;
use codex_tui_core::ids::Cwd;

use super::pages::Home;
use crate::widgets::interest::Interest;
use crate::widgets::sheet::{Sheet, bottom};

/// The sheet of the plugins of `cwd`.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginsSheet {
    pub cwd: Cwd,
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

impl PluginsSheetState {
    /// What the sheet watches: asked in `did_change_dependencies` and again in `build`, because
    /// a build forgets the selects of the build before.
    fn seen(cx: &mut Cx<'_, Self>) -> bool {
        select::<FeatureStates, bool, _>(cx, |features| features.plugins.enabled())
    }
}

impl State for PluginsSheetState {
    type Widget = PluginsSheet;
    type Msg = ();

    fn did_change_dependencies(&mut self, w: &PluginsSheet, cx: &mut Cx<'_, Self>) {
        if !w.alive(&Self::seen(cx)) {
            close_dialog::<_, ()>(cx, None);
        }
    }

    fn build(&self, w: &PluginsSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Self::seen(cx);
        let browser = Home { cwd: w.cwd.clone() };
        let pages = Navigator::new().home(bottom(browser)).back_key(None);
        Interest::new(Wanted::Listing { cwd: w.cwd.clone() }, pages).into()
    }
}
