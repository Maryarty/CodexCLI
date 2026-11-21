//! The page the command line starts on (`codex-rtwk agents`, `codex-rtwk
//! resume`): the session is the navigator's home as ever, and the page is
//! pushed over it at once with nowhere to go back to but the session it
//! reveals when it ends.

use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::watch;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenStates;

use crate::pages::startup::onboarding;

/// The page the command line starts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    Agents,
    Resume,
}

impl Start {
    fn route(self) -> PageRoute<()> {
        match self {
            Start::Agents => crate::pages::agents::start_route(),
            Start::Resume => crate::pages::resume::route(),
        }
    }
}

/// The session `chat` with the `start` page pushed over it at mount.
#[derive(Clone, Debug, PartialEq)]
pub struct StartPage {
    pub chat: AnyWidget,
    pub start: Start,
}

pub struct StartPageState;

impl StatefulWidget for StartPage {
    type State = StartPageState;

    fn create_state(&self) -> StartPageState {
        StartPageState
    }
}

impl_widget!(stateful eq StartPage);

impl State for StartPageState {
    type Widget = StartPage;
    type Msg = ();

    fn init_state(&mut self, _: &StartPage, cx: &mut Cx<'_, Self>) {
        let push = cx.send(());
        cx.emit(&push, ());
    }

    fn update(&mut self, _: (), w: &StartPage, cx: &mut Cx<'_, Self>) -> Rebuild {
        drop(Navigator::of(cx).push::<()>(w.start.route()));
        Rebuild::No
    }

    fn build(&self, w: &StartPage, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.chat.clone()
    }
}

/// The home under the startup page's gate: the page is pushed over `child`
/// once its model has a step due (oracle: the update prompt and the
/// onboarding screens run before the chat). The facts come from the host
/// after the first frame, so the gate watches the model; it pushes once.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupGate {
    pub child: AnyWidget,
}

/// Whether the gate has pushed the page (R14: widget state).
pub struct StartupGateState {
    pushed: bool,
}

impl StatefulWidget for StartupGate {
    type State = StartupGateState;

    fn create_state(&self) -> StartupGateState {
        StartupGateState { pushed: false }
    }
}

impl_widget!(stateful eq StartupGate);

impl State for StartupGateState {
    type Widget = StartupGate;
    type Msg = ();

    fn did_change_dependencies(&mut self, _: &StartupGate, cx: &mut Cx<'_, Self>) {
        if !self.pushed && watch::<ScreenStates, _>(cx).onboarding.due() {
            self.pushed = true;
            let push = cx.send(());
            cx.emit(&push, ());
        }
    }

    fn update(&mut self, _: (), _: &StartupGate, cx: &mut Cx<'_, Self>) -> Rebuild {
        drop(Navigator::of(cx).push::<()>(onboarding::route(onboarding::Step::Update)));
        Rebuild::No
    }

    fn build(&self, w: &StartupGate, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.child.clone()
    }
}
