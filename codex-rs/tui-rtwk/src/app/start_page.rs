//! The page the command line starts on (`codex-rtwk agents`, `codex-rtwk
//! resume`): the session is the navigator's home as ever, and the page is
//! pushed over it at once with nowhere to go back to but the session it
//! reveals when it ends.

use rtwk::layout::SizedBox;
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
/// With `awaits_facts` the home shows nothing until the facts are read, and
/// then the chat or, with a step due, the page alone: the oracle draws no
/// chat before its prompt.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupGate {
    pub child: AnyWidget,
    pub awaits_facts: bool,
}

/// Whether the gate has pushed the page and whether the home may draw its
/// child (R14: widget state).
pub struct StartupGateState {
    pushed: bool,
    open: bool,
}

impl StatefulWidget for StartupGate {
    type State = StartupGateState;

    fn create_state(&self) -> StartupGateState {
        StartupGateState {
            pushed: false,
            open: !self.awaits_facts,
        }
    }
}

impl_widget!(stateful eq StartupGate);

impl State for StartupGateState {
    type Widget = StartupGate;
    type Msg = ();

    fn did_change_dependencies(&mut self, _: &StartupGate, cx: &mut Cx<'_, Self>) {
        let onboarding = &watch::<ScreenStates, _>(cx).onboarding;
        if !self.pushed && onboarding.due() {
            self.pushed = true;
            let push = cx.send(());
            cx.emit(&push, ());
        } else if !self.pushed && onboarding.settled() {
            self.open = true;
        }
    }

    fn update(&mut self, _: (), _: &StartupGate, cx: &mut Cx<'_, Self>) -> Rebuild {
        drop(Navigator::of(cx).push::<()>(onboarding::route(onboarding::Step::Update)));
        self.open = true;
        Rebuild::Yes
    }

    fn build(&self, w: &StartupGate, _: &mut Cx<'_, Self>) -> AnyWidget {
        if self.open {
            w.child.clone()
        } else {
            SizedBox::shrink().into()
        }
    }
}
