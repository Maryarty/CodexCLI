//! The surface presenters (ADR-013 Ж): where the one-shots the features leave (`SurfaceWish`)
//! are shown. The presenter is the only place that says a wish was shown (`Intent::WishShown`,
//! rules test `adr013_wishes_shown_by_the_presenter`), so a wish is shown once, and the feature
//! never learns how. How a wish looks is the feature's view (`FeatureView::shown`): a page of
//! the open sheet of its kind and scope, else a sheet opened at it; this file only does what is
//! the same for every feature. `present` runs after each ingested event.

use std::rc::Rc;

use rtwk::app::AppCx;

use codex_tui_core::app::{AppEv, Intent};

use crate::app::UiApp;
use crate::widgets::sheet::Door;

/// What the presenter keeps: plain data, UI-only (`UiApp.ui`).
#[derive(Default)]
pub struct Presenters {
    /// The sheets that are open, to show a wish in (the view provides it to the sheets).
    door: Rc<Door>,
    /// The number of the last wish taken: a wish still queued while its `WishShown` is on its
    /// way is not shown again.
    taken: u64,
}

impl Presenters {
    /// The door the open sheets leave their navigator in.
    pub fn door(&self) -> Rc<Door> {
        self.door.clone()
    }
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
        if let Some(shown) = shown {
            match app.ui.presenters.door.nav(shown.sheet, &shown.scope) {
                Some(nav) => (shown.push)(&nav),
                None => {
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
            }
        }
        app.ui.presenters.taken = seq;
        app.send(AppEv::Intent(Intent::WishShown(seq)));
    }
}
