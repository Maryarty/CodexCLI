//! The transcript view alone (`Open::view_only`): the oracle's `transcript_view` tests drive the
//! view with no bottom pane (`TranscriptView::render(area, ..)` of 32x6 or 20x2), where the full
//! chat gives the list one row or none (the composer and the footer take the rest).
//!
//! The screen is the live app (its session, its fake backend, its `feed`, history pages and
//! effects all work as for `live()`), with one page changed: the home is the transcript of the
//! active thread alone, the sheet the owned full-screen Ctrl+T shows (`Chrome::Bare`: the list
//! and the pager's keys, no title, no status row, no hints, no composer). A test reads the
//! kit's `Screen` as for any other seed.
//!
//! Flutter's counterpart: a test `pumpWidget`s the one widget under test inside the app's
//! providers. The app is not built twice: `AppView` keeps the state it has when it is given a
//! new view, so the harness's own app stays and only its `view` is this file's, which is
//! `app::view::view` with the home replaced (the request presenter and the terminal effects are
//! not part of it: nothing here asks a dialog or rings a bell).

use std::rc::Rc;

use rtwk::AnyWidget;
use rtwk::app::{AppView, ViewCx};
use rtwk::navigation::Navigator;
use rtwk::provider::{MultiProvider, Provider};

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::ids::{BackendId, RequestKey, ThreadId, Ticket};
use codex_tui_core::screens::{ScreenIntent, ScreenThreadIntent};
use codex_tui_core::threads::UserMessage;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::app::debug::seeded::Seeded;
use codex_tui_rtwk::app::run::{localized, root};
use codex_tui_rtwk::app::view::Hs;
use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::pages::session::transcript::sheet;
use codex_tui_rtwk::services::backend::fake::FakeServer;
use codex_tui_rtwk::services::shell::Backend;

use crate::common::{Harness, ms};

/// Puts the transcript alone in place of the harness app's home. The app (its core, its
/// controllers, its backend link) is the one running; the spare given to `AppView` is dropped
/// unused, as its state is only the initial one.
pub(super) fn mount(h: &mut Harness, compact: bool) {
    // The page the app showed goes first, and is let go of in a frame of its own: a controller the
    // new transcript shares with it (the thread's selection) is detached by the old area's disposal,
    // which Flutter runs at the end of the frame that inflated the new one, and no Find match could
    // be selected after it.
    let (server, _) = FakeServer::new();
    let spare = UiApp::new(Backend::Fake(server));
    h.t.pump_widget(localized(
        root(AppView::new(spare, |_, _| AnyWidget::default())),
        Lang::En,
    ));
    h.t.pump_for(ms(20));
    let (server, _) = FakeServer::new();
    let spare = UiApp::new(Backend::Fake(server));
    h.t.pump_widget(localized(
        root(AppView::new(
            spare,
            if compact { compact_view } else { view },
        )),
        Lang::En,
    ));
    h.t.pump_for(ms(20));
}

fn view(app: &UiApp, cx: &mut ViewCx<'_, '_, UiApp>) -> AnyWidget {
    home(app, cx, sheet::seeded_bare)
}

/// The same with the compact transcript (`Chrome::Compact`).
fn compact_view(app: &UiApp, cx: &mut ViewCx<'_, '_, UiApp>) -> AnyWidget {
    home(app, cx, sheet::seeded_bare_compact)
}

/// `app::view::view` with the home replaced by the bare transcript sheet of the active thread.
fn home(
    app: &UiApp,
    cx: &mut ViewCx<'_, '_, UiApp>,
    make: fn(&Seeded<'_>) -> AnyWidget,
) -> AnyWidget {
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
        copy: cx.on_with(|app: &mut UiApp, job, _cx| {
            let (request, ask) = app.ui.copies.ask(&job);
            app.send(AppEv::Intent(Intent::Effect { request, ask }));
        }),
    };
    let home = make(&Seeded {
        core: &app.core,
        thread_ui: &app.ui.thread_ui,
    });
    MultiProvider::new()
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
        .into()
}
