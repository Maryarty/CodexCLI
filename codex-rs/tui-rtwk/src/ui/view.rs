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

use codex_tui_core::app::AppEv;
use codex_tui_core::app::Intent;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::screens::ScreenIntent;

use crate::app::UiApp;
use crate::screens;
use crate::ui::debug_menu::DebugMenu;
use crate::ui::presenter::home_node;
use crate::ui::start_page::{Start, StartPage};

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

/// The view of the app.
pub fn view(app: &UiApp, cx: &mut ViewCx<'_, '_, UiApp>) -> AnyWidget {
    let hs = Hs {
        submit: cx.on_with(|app: &mut UiApp, text: String, _cx| {
            let thread = app.core.session.active.clone();
            app.send(AppEv::Intent(Intent::Submit { thread, text }));
        }),
        dismiss: cx.on_with(|app: &mut UiApp, key: RequestKey, _cx| {
            app.send(AppEv::Intent(Intent::DismissRequest(key)));
        }),
        screen: cx.on_with(|app: &mut UiApp, intent: ScreenIntent, _cx| {
            app.send(AppEv::Intent(Intent::Screen(intent)));
        }),
        quit: cx.on(|_, cx| cx.quit()),
    };
    // The home is a memo node that leaves its handle for the request
    // presenter, which shows a request's dialog from it as a route of this
    // navigator, under the providers (ADR-002); whichever home it is.
    let child: AnyWidget = if app.ui.debug {
        DebugMenu.into()
    } else if app.ui.start_agents {
        StartPage {
            chat: screens::chat::home(app),
            start: Start::Agents,
        }
        .into()
    } else if app.ui.start_resume {
        StartPage {
            chat: screens::chat::home(app),
            start: Start::Resume,
        }
        .into()
    } else {
        screens::chat::home(app)
    };
    let home = cx.memo((app.ui.presenter.place(), child), home_node);
    MultiProvider::new()
        .with(Provider::value_shared(app.core.session.clone()))
        .with(Provider::value_shared(app.core.active_thread()))
        .with(Provider::value_shared(app.core.requests.clone()))
        .with(Provider::value_shared(app.core.screens.clone()))
        .with(Provider::value_shared(Rc::new(hs)).update_should_notify(|_, _| false))
        .child(AnyWidget::from(Navigator::new().home(home)).with_key(app.ui.nav_key.clone()))
        .into()
}
