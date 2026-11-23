//! The surface presenters (ADR-013 Ж): where the one-shots the features leave (`SurfaceWish`)
//! are shown. A presenter is the only place that says a wish was shown (`Intent::WishShown`, rules
//! test `adr013_wishes_shown_by_the_presenter`), so a wish is shown once, and the feature never
//! learns how. One file per family of wishes; `present` runs after each ingested event.

use rtwk::app::AppCx;

use crate::app::UiApp;

pub mod plugins;

/// What the presenters keep: plain data, UI-only (`UiApp.ui`).
#[derive(Default)]
pub struct Presenters {
    pub plugins: plugins::PluginsPresenter,
}

/// Shows the wishes of the features that are waiting and can be shown now.
pub fn present(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    plugins::present(app, cx);
}
