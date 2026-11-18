//! A seeded model for the debug menu: the core model a fixture script made
//! (`codex_tui_rtwk_core::fixtures::script`), provided to one screen under
//! the same provider types the live view provides, so the screen under test
//! is the real widget reading a real model, never data handed to it. The
//! handlers (`Hs`) are the app's own: they cannot be made without the app's
//! context, so a submit from a seeded screen goes to the real model.

use rtwk::AnyWidget;
use rtwk::provider::{MultiProvider, Provider};

use codex_tui_rtwk_core::app::CodexApp;

use crate::ui::thread_ui::ThreadUiMap;

/// The model a debug entry shows, with the controllers of its active thread.
pub struct Seeded {
    pub core: CodexApp,
    pub thread_ui: ThreadUiMap,
}

impl Seeded {
    pub fn new(core: CodexApp) -> Self {
        let mut thread_ui = ThreadUiMap::default();
        if let Some(active) = core.session.active.clone() {
            thread_ui.ensure(&active);
        }
        Self { core, thread_ui }
    }

    /// `child` under this model's providers: they shadow the app's own.
    pub fn scope(&self, child: AnyWidget) -> AnyWidget {
        MultiProvider::new()
            .with(Provider::value_shared(self.core.session.clone()))
            .with(Provider::value_shared(self.core.active_thread()))
            .with(Provider::value_shared(self.core.requests.clone()))
            .with(Provider::value_shared(self.core.screens.clone()))
            .child(child)
            .into()
    }
}
