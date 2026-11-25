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
use codex_tui_core::ids::{BackendId, RequestKey, ThreadId, Ticket};
use codex_tui_core::screens::{ScreenIntent, ScreenThreadIntent};
use codex_tui_core::threads::UserMessage;

use crate::app::UiApp;
use crate::app::debug::menu::DebugMenu;
use crate::app::start_page::{Start, StartPage, StartupGate};
use crate::pages;
use crate::pages::session::transcript::copy::CopyJob;
use crate::sheets::presenter::home_node;

/// The app's handlers, provided to the tree.
pub struct Hs {
    /// Send `text` to the active thread (to the startup draft before the
    /// session is ready).
    pub submit: Handler<UserMessage>,
    /// Close a request without answering it.
    pub dismiss: Handler<RequestKey>,
    pub dismiss_banner: Handler<(BackendId, Ticket)>,
    /// A row of a backend banner was chosen (a number key, a tap).
    pub pick_banner: Handler<(BackendId, Ticket, usize)>,
    /// Any intent of the app: a verb of a feature (`features::verb`), an interest
    /// (`widgets::interest`), a presented wish (`app::presenters`).
    pub intent: Handler<Intent>,
    /// An intent of a screen slot.
    pub screen: Handler<ScreenIntent>,
    /// An intent of a screen slot of one thread (the queue, Esc with
    /// steers, the edit of a queued message): `apply` owns the outcome.
    pub thread_screen: Handler<(ThreadId, ScreenThreadIntent)>,
    pub open_thread: Handler<ThreadId>,
    pub load_history: Handler<ThreadId>,
    pub interrupt: Handler<ThreadId>,
    pub quit: Handler<()>,
    /// Move the empty conversation to the next connected backend.
    pub cycle: Handler<()>,
    /// The reader copies text out of the transcript: the app is asked to put it on the clipboard
    /// (`Intent::Effect`), and the answer is the feedback line.
    pub copy: Handler<CopyJob>,
}

/// The view of the app.
pub fn view(app: &UiApp, cx: &mut ViewCx<'_, '_, UiApp>) -> AnyWidget {
    let hs = Hs {
        submit: cx.on_with(|app: &mut UiApp, message: UserMessage, _cx| {
            let thread = app.core.session().active.clone();
            app.send(AppEv::Intent(Intent::Submit { thread, message }));
        }),
        dismiss: cx.on_with(|app: &mut UiApp, key: RequestKey, _cx| {
            app.send(AppEv::Intent(Intent::DismissRequest(key)));
        }),
        dismiss_banner: cx.on_with(|app: &mut UiApp, (backend, ticket), _cx| {
            app.send(AppEv::Intent(Intent::DismissBackendBanner {
                backend,
                ticket,
            }));
        }),
        pick_banner: cx.on_with(
            |app: &mut UiApp, (backend, ticket, row): (BackendId, Ticket, usize), _cx| {
                app.send(AppEv::Intent(Intent::PickBannerCta {
                    backend,
                    ticket,
                    row,
                }));
            },
        ),
        intent: cx.on_with(|app: &mut UiApp, intent: Intent, _cx| {
            app.send(AppEv::Intent(intent));
        }),
        screen: cx.on_with(|app: &mut UiApp, intent: ScreenIntent, _cx| {
            app.send(AppEv::Intent(Intent::Screen(intent)));
        }),
        thread_screen: cx.on_with(
            |app: &mut UiApp, (thread, intent): (ThreadId, ScreenThreadIntent), _cx| {
                app.send(AppEv::Intent(Intent::ThreadScreen { thread, intent }));
            },
        ),
        open_thread: cx.on_with(|app: &mut UiApp, id: ThreadId, _cx| {
            app.send(AppEv::Intent(Intent::OpenThread(id)));
        }),
        load_history: cx.on_with(|app: &mut UiApp, id: ThreadId, _cx| {
            app.send(AppEv::Intent(Intent::LoadOlderHistory(id)));
        }),
        interrupt: cx.on_with(|app: &mut UiApp, id: ThreadId, _cx| {
            app.send(AppEv::Intent(Intent::Interrupt(id)));
        }),
        quit: cx.on(|_, cx| cx.quit()),
        cycle: cx.on(|app: &mut UiApp, _cx| app.send(AppEv::Intent(Intent::CycleBackend))),
        copy: cx.on_with(|app: &mut UiApp, job: CopyJob, _cx| {
            let (request, ask) = app.ui.copies.ask(&job);
            app.send(AppEv::Intent(Intent::Effect { request, ask }));
        }),
    };
    // The home is a memo node that leaves its handle for the request
    // presenter, which shows a request's dialog from it as a route of this
    // navigator, under the providers (ADR-002); whichever home it is.
    let child: AnyWidget = if app.ui.debug {
        DebugMenu.into()
    } else if app.ui.start_agents {
        StartPage {
            chat: pages::session::home(app),
            start: Start::Agents,
        }
        .into()
    } else if app.ui.start_resume {
        StartPage {
            chat: pages::session::home(app),
            start: Start::Resume,
        }
        .into()
    } else {
        pages::session::home(app)
    };
    let child: AnyWidget = if app.ui.debug {
        child
    } else {
        StartupGate {
            child,
            awaits_facts: app.ui.awaits_facts,
        }
        .into()
    };
    let home = cx.memo(
        (app.ui.presenter.place(), app.ui.presenter.delay(), child),
        home_node,
    );
    let tree = MultiProvider::new()
        .with(Provider::value_shared(app.core.session().clone()))
        .with(Provider::value_shared(app.core.active_thread()))
        .with(Provider::value_shared(app.core.requests().clone()))
        .with(Provider::value_shared(app.core.screens().clone()))
        .with(Provider::value_shared(app.core.features().clone()))
        .with(
            Provider::value_shared(app.core.keymap().clone())
                .update_should_notify(|old, new| old != new),
        )
        .with(
            Provider::value(app.core.agents_threads()).update_should_notify(|old, new| old != new),
        )
        .with(Provider::value(app.core.tui()).update_should_notify(|old, new| old != new))
        .with(Provider::value(app.ui.copies.shown()).update_should_notify(|old, new| old != new))
        .with(Provider::value_shared(Rc::new(hs)).update_should_notify(|_, _| false))
        .child(AnyWidget::from(Navigator::new().home(home)).with_key(app.ui.nav_key.clone()))
        .into();
    super::terminal_effects::TerminalEffects {
        child: tree,
        notice: app.ui.desktop_notice.clone(),
        always: app.core.screens().chat.status.notification_policy.always,
    }
    .into()
}
