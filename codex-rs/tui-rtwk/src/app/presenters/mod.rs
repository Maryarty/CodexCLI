//! The surface presenters (ADR-013 Ж): where the one-shots the features leave (`SurfaceWish`)
//! are shown. The presenter is the only place that says a wish was shown (`Intent::WishShown`,
//! rules test `adr013_wishes_shown_by_the_presenter`), so a wish is shown once, and the feature
//! never learns how. How a wish looks is the feature's view (`FeatureView::shown`): a sheet that
//! is open for the wish's scope shows it from its own slice of the feature's state, else a sheet
//! is opened that does; this file only does what is the same for every feature. `present` runs
//! after each ingested event.
//!
//! Whether a sheet is open for the scope is read where core keeps it: the counter of interests
//! (`Shown::held_by`), which a sheet raises while it is mounted. No registry of open sheets is
//! kept here, so a sheet open for another scope is never taken for this one and a sheet that
//! closes leaves nothing behind. (Two wishes for a scope made in one flush, before its sheet is
//! mounted, open two sheets; the sheets then show the same page twice, which is harmless.)

use rtwk::app::AppCx;

use codex_tui_core::app::{AppEv, Intent};

use crate::app::UiApp;

/// What the presenter keeps: plain data, UI-only (`UiApp.ui`).
#[derive(Default)]
pub struct Presenters {
    /// The number of the last wish taken: a wish still queued while its `WishShown` is on its
    /// way is not shown again.
    taken: u64,
}

/// Shows the wishes of the features that are waiting and can be shown now.
pub fn present(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let wishes: Vec<_> = app
        .core
        .wishes()
        .pending()
        .filter(|wish| wish.seq > app.ui.presenters.taken)
        .map(|wish| (wish.seq, crate::features::shown(&wish.wish, &wish.scope)))
        .collect();
    for (seq, shown) in wishes {
        if let Some(shown) = shown
            && app.core.interests().count(&shown.held_by) == 0
        {
            let Some(place) = app.ui.presenter.place().get() else {
                // no home to show from yet: the wish stays queued for the next event
                return;
            };
            let open = shown.open.clone();
            let dialog = cx.show_dialog_from::<()>(
                place,
                move |_| open(),
                shown.options,
                |_app, _answer, _cx| {},
            );
            if dialog.is_closed() {
                return;
            }
        }
        app.ui.presenters.taken = seq;
        app.send(AppEv::Intent(Intent::WishShown(seq)));
    }
}
