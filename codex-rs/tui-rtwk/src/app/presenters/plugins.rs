//! The wishes of `/plugins` (ADR-013 Ж): the setup of the apps an installed plugin needs signed
//! in. When the sheet of that directory is open the setup is pushed on its navigator (over
//! whatever page the user is at); when it is not, the sheet opens at the setup. Either way the
//! wish is said to be shown, once.

use std::rc::Rc;

use rtwk::app::AppCx;

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::features::FeatureWish;
use codex_tui_core::features::plugins::Wish;

use crate::app::UiApp;
use crate::features::plugins::sheet::{Door, PluginsSheet, Start};

/// What the presenter keeps: the door of the open sheet and the last wish it took.
#[derive(Default)]
pub struct PluginsPresenter {
    door: Rc<Door>,
    /// The number of the last wish taken: a wish still queued while its `WishShown` is on its
    /// way is not shown again.
    taken: u64,
}

impl PluginsPresenter {
    /// The door the open sheet leaves its navigator in (the view provides it to the sheet).
    pub fn door(&self) -> Rc<Door> {
        self.door.clone()
    }
}

pub fn present(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let wishes: Vec<_> = app
        .core
        .wishes()
        .pending()
        .filter(|wish| wish.seq > app.ui.presenters.plugins.taken)
        .filter_map(|wish| match &wish.wish {
            FeatureWish::Plugins(Wish::Auth(key)) => Some((wish.seq, wish.scope.clone(), key.clone())),
            _ => None,
        })
        .collect();
    for (seq, scope, key) in wishes {
        if !app.ui.presenters.plugins.door.show_auth(&scope, &key) {
            let Some(place) = app.ui.presenter.place().get() else {
                // no home to show from yet: the wish stays queued for the next event
                return;
            };
            let opened = key.clone();
            let dialog = cx.show_dialog_from::<()>(
                place,
                move |_| {
                    PluginsSheet {
                        cwd: scope.clone(),
                        start: Start::Auth(opened.clone()),
                    }
                    .into()
                },
                crate::features::plugins::sheet_options(),
                |_app, _answer, _cx| {},
            );
            if dialog.is_closed() {
                return;
            }
        }
        app.ui.presenters.plugins.taken = seq;
        app.send(AppEv::Intent(Intent::WishShown(seq)));
    }
}
