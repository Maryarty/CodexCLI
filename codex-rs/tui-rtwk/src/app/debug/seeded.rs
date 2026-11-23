//! A seeded screen for the debug menu: the core model a fixture script made
//! (`codex_tui_core::fixtures`), provided to one screen under the same
//! provider types the live view provides, so the screen under test is the
//! real widget reading a real model, never data handed to it.
//!
//! The scope owns its model and its own handlers (`Hs`): what the user does
//! on the screen (a submit, a dismissal, an intent) is applied to the seeded
//! copy through core's `Sandbox`, which drops the effects, and the screen
//! rebuilds from the result. Nothing reaches the live app or its backend.

use rtwk::AnyWidget;
use rtwk::provider::{MultiProvider, Provider};
use rtwk::{Cx, Rebuild, State, StatefulWidget, impl_widget};
use std::rc::Rc;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::CodexApp;
use codex_tui_core::app::Intent;
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{BackendId, RequestKey, ThreadId, Ticket};
use codex_tui_core::screens::{ScreenIntent, ScreenThreadIntent};
use codex_tui_core::threads::UserMessage;

use crate::app::view::Hs;
use crate::pages::session::transcript::copy::{self, CopyJob, Shown};
use crate::widgets::sheet::Door;
use crate::widgets::thread_ui::ThreadUiMap;

/// What a debug entry's builder sees of the seeded model.
pub struct Seeded<'a> {
    pub core: &'a CodexApp,
    pub thread_ui: &'a ThreadUiMap,
}

/// The scope of one seeded screen: plays `script` into its own model and
/// shows what `build` makes of it.
pub struct SeededScope {
    pub script: &'static str,
    pub build: fn(&Seeded<'_>) -> AnyWidget,
}

pub struct SeededScopeState {
    /// `None` when the script did not play; a test plays every registered one.
    sandbox: Option<Sandbox>,
    thread_ui: ThreadUiMap,
}

/// An intent of the app carried as a message of the seeded page: `Intent` has no equality, so
/// two are never equal (a message is compared only to tell a handler from another).
#[derive(Clone, Debug)]
pub struct Sent(pub Intent);

impl PartialEq for Sent {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SeededMsg {
    Submit(UserMessage),
    Dismiss(RequestKey),
    DismissBanner(BackendId, Ticket),
    PickBanner(BackendId, Ticket, usize),
    Intent(Sent),
    Screen(ScreenIntent),
    ThreadScreen(ThreadId, ScreenThreadIntent),
    OpenThread(ThreadId),
    LoadHistory(ThreadId),
    Interrupt(ThreadId),
    Cycle,
    Copy(CopyJob),
    Quit,
}

impl StatefulWidget for SeededScope {
    type State = SeededScopeState;

    fn create_state(&self) -> SeededScopeState {
        let sandbox = Sandbox::play(self.script).ok();
        let mut thread_ui = ThreadUiMap::default();
        if let Some(active) = sandbox
            .as_ref()
            .and_then(|sandbox| sandbox.app().session().active.clone())
        {
            thread_ui.ensure(&active);
        }
        SeededScopeState { sandbox, thread_ui }
    }
}

impl_widget!(stateful SeededScope);

impl State for SeededScopeState {
    type Widget = SeededScope;
    type Msg = SeededMsg;

    fn update(&mut self, msg: SeededMsg, _: &SeededScope, _: &mut Cx<'_, Self>) -> Rebuild {
        let Some(sandbox) = self.sandbox.as_mut() else {
            return Rebuild::No;
        };
        let intent = match msg {
            SeededMsg::Submit(message) => Intent::Submit {
                thread: sandbox.app().session().active.clone(),
                message,
            },
            SeededMsg::Dismiss(key) => Intent::DismissRequest(key),
            SeededMsg::DismissBanner(backend, ticket) => {
                Intent::DismissBackendBanner { backend, ticket }
            }
            SeededMsg::PickBanner(backend, ticket, row) => Intent::PickBannerCta {
                backend,
                ticket,
                row,
            },
            SeededMsg::Intent(Sent(intent)) => intent,
            SeededMsg::Screen(intent) => Intent::Screen(intent),
            SeededMsg::ThreadScreen(thread, intent) => Intent::ThreadScreen { thread, intent },
            SeededMsg::OpenThread(id) => Intent::OpenThread(id),
            SeededMsg::LoadHistory(id) => Intent::LoadOlderHistory(id),
            SeededMsg::Interrupt(id) => Intent::Interrupt(id),
            SeededMsg::Cycle => Intent::CycleBackend,
            // the seeded page keeps no log of its copies: the host answers under the one number
            SeededMsg::Copy(job) => Intent::Effect {
                request: copy::REQUEST_BIT,
                ask: job.ask(),
            },
            // The seeded screen is a page of the debug menu: leaving it is the
            // menu's own back key.
            SeededMsg::Quit => return Rebuild::No,
        };
        sandbox.send(AppEv::Intent(intent));
        if let Some(active) = sandbox.app().session().active.clone() {
            self.thread_ui.ensure(&active);
        }
        Rebuild::Yes
    }

    fn build(&self, w: &SeededScope, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let Some(sandbox) = &self.sandbox else {
            return AnyWidget::default();
        };
        let core = sandbox.app();
        let hs = Hs {
            submit: cx.map(SeededMsg::Submit),
            dismiss: cx.map(SeededMsg::Dismiss),
            dismiss_banner: cx.map(|(backend, ticket)| SeededMsg::DismissBanner(backend, ticket)),
            pick_banner: cx
                .map(|(backend, ticket, row)| SeededMsg::PickBanner(backend, ticket, row)),
            intent: cx.map(|intent| SeededMsg::Intent(Sent(intent))),
            screen: cx.map(SeededMsg::Screen),
            thread_screen: cx.map(|(thread, intent)| SeededMsg::ThreadScreen(thread, intent)),
            open_thread: cx.map(SeededMsg::OpenThread),
            load_history: cx.map(SeededMsg::LoadHistory),
            interrupt: cx.map(SeededMsg::Interrupt),
            quit: cx.map(|()| SeededMsg::Quit),
            cycle: cx.map(|()| SeededMsg::Cycle),
            copy: cx.map(SeededMsg::Copy),
        };
        let child = (w.build)(&Seeded {
            core,
            thread_ui: &self.thread_ui,
        });
        MultiProvider::new()
            .with(Provider::value_shared(core.session().clone()))
            .with(Provider::value_shared(core.active_thread()))
            .with(Provider::value_shared(core.requests().clone()))
            .with(Provider::value_shared(core.screens().clone()))
            .with(Provider::value_shared(core.features().clone()))
            .with(
                Provider::value(core.agents_threads()).update_should_notify(|old, new| old != new),
            )
            .with(Provider::value(core.tui()).update_should_notify(|old, new| old != new))
            .with(Provider::value(Shown::default()).update_should_notify(|_, _| false))
            .with(Provider::value_shared(Rc::new(hs)).update_should_notify(|_, _| false))
            .with(Door::provider(Rc::default()))
            .child(child)
            .into()
    }
}
