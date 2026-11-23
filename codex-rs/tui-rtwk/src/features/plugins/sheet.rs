//! The sheet of `/plugins` (ADR-013 O1): a modal bottom sheet whose body is a nested `Navigator`
//! in an `IntrinsicHeight`, so the sheet is as tall as the page on top and the sheet's own cap
//! (the options' max height) bounds it. The routes of the navigator are the pages (`pages.rs`);
//! Esc pops one, and the browser under them keeps its picker (cursor, query, tab) because it is the
//! first route and is never rebuilt from scratch. The sheet says it wants the plugins of its
//! directory (`Interest`), so the feature loads them for as long as the sheet is open, and closes
//! when it has no reason to stay (`Sheet::alive`: the feature was turned off).
//!
//! In Flutter this is `showModalBottomSheet` with a `Navigator` as the body (the nested-navigator
//! recipe for a flow inside a sheet), `PopScope` per page, and a provider the pages select from.

use std::cell::RefCell;
use std::rc::Rc;

use rtwk::layout::IntrinsicHeight;
use rtwk::navigation::{Navigator, NavigatorState};
use rtwk::overlay::close_dialog;
use rtwk::provider::{InheritedProvider, Provider, select};
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::plugins::{Interest as Wanted, PluginKey};

use super::pages::{Home, auth_route};
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

/// What an open sheet leaves for whoever wants a page shown in it: where it is and how to reach
/// its navigator. Written by the browser, the sheet's first route, and cleared when it goes.
#[derive(Clone)]
pub struct Opened {
    pub cwd: String,
    pub nav: NavigatorState,
}

/// The place an open sheet leaves its `Opened` in. The presenter keeps the door and the view
/// provides it (never notifying: a reader looks when it needs to).
#[derive(Default)]
pub struct Door(RefCell<Option<Opened>>);

impl Door {
    pub fn set(&self, opened: Opened) {
        *self.0.borrow_mut() = Some(opened);
    }

    pub fn clear(&self) {
        self.0.borrow_mut().take();
    }

    pub fn get(&self) -> Option<Opened> {
        self.0.borrow().clone()
    }

    /// Shows the setup page of `key` in the sheet that is open for `cwd`; false when none is.
    pub fn show_auth(&self, cwd: &str, key: &PluginKey) -> bool {
        let Some(opened) = self.get().filter(|opened| opened.cwd == cwd) else {
            return false;
        };
        drop(opened.nav.push::<()>(auth_route(key.clone())));
        true
    }

    /// The provider of the door, for the view's `MultiProvider`.
    pub fn provider(door: Rc<Door>) -> InheritedProvider<Door> {
        Provider::value_shared(door).update_should_notify(|_, _| false)
    }
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
        let pages = Navigator::new().home(browser).back_key(None);
        Interest::new(
            Wanted::Listing { cwd: w.cwd.clone() },
            IntrinsicHeight::new(pages),
        )
        .into()
    }
}
