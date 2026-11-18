//! The view: `MultiProvider` (fixed list, ADR-002) over a `Navigator` whose
//! home is the chat screen (or the debug menu with `--debug-screens`).
//! Handlers are provided as one `Hs`; the provider never notifies, readers
//! read the current handlers when they emit.

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::Handler;
use rtwk::app::ViewCx;
use rtwk::navigation::Navigator;
use rtwk::provider::{MultiProvider, Provider};

use crate::app::AppEv;
use crate::app::CodexApp;
use crate::app::Intent;
use crate::ids::RequestKey;
use crate::screens::ScreenIntent;
use crate::ui::chat::Chat;
use crate::ui::debug_menu::DebugMenu;

/// The app's handlers, provided to the tree.
pub struct Hs {
    /// Send `text` to the active thread (to the startup draft before the
    /// session is ready).
    pub submit: Handler<String>,
    /// Close a request without answering it.
    pub dismiss: Handler<RequestKey>,
    /// An intent of a screen slot.
    pub screen: Handler<ScreenIntent>,
    pub quit: Handler<()>,
}

pub fn view(app: &CodexApp, cx: &mut ViewCx<'_, '_, CodexApp>) -> AnyWidget {
    let hs = Hs {
        submit: cx.on_with(|app: &mut CodexApp, text: String, _cx| {
            let thread = app.session.active.clone();
            app.send(AppEv::Intent(Intent::Submit { thread, text }));
        }),
        dismiss: cx.on_with(|app: &mut CodexApp, key: RequestKey, _cx| {
            app.send(AppEv::Intent(Intent::DismissRequest(key)));
        }),
        screen: cx.on_with(|app: &mut CodexApp, intent: ScreenIntent, _cx| {
            app.send(AppEv::Intent(Intent::Screen(intent)));
        }),
        quit: cx.on(|_, cx| cx.quit()),
    };
    let home: AnyWidget = if app.debug_screens {
        DebugMenu.into()
    } else {
        Chat {
            ui: app.thread_ui.get(app.session.active.as_ref()),
        }
        .into()
    };
    MultiProvider::new()
        .with(Provider::value_shared(app.session.clone()))
        .with(Provider::value_shared(app.active_thread()))
        .with(Provider::value_shared(app.requests.clone()))
        .with(Provider::value_shared(app.screens.clone()))
        .with(Provider::value_shared(Rc::new(hs)).update_should_notify(|_, _| false))
        .child(AnyWidget::from(Navigator::new().home(home)).with_key(app.nav_key.clone()))
        .into()
}
