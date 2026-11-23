//! The page the command line starts on (`codex-rtwk agents`, `codex-rtwk
//! resume`): the session is the navigator's home as ever, and the page is
//! pushed over it at once with nowhere to go back to but the session it
//! reveals when it ends. The page stands in for the first thread (the resume
//! picker, the command center), so the chat is not built under it: it mounts
//! once the page has popped (Flutter: `await Navigator.push(page)`, then
//! `setState`), and its composer takes the focus at mount (`autofocus`), with
//! whatever thread the page opened. The trust prompt cancelled at launch
//! opens the command center in place of the startup page: it is the start page
//! then, whatever the command line said.

use rtwk::Handler;
use rtwk::layout::SizedBox;
use rtwk::navigation::{Navigator, PageRoute, PopInvoked, PopScope};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::onboarding::Intent as StartupIntent;
use codex_tui_core::screens::{ScreenIntent, ScreenStates};

use crate::app::view::Hs;
use crate::pages::startup::onboarding;

/// The page the command line starts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    Agents,
    Resume,
}

impl Start {
    /// The route of the page. The page stands in for the first thread: its
    /// `PopScope` (Flutter: `onPopInvokedWithResult`) tells `left` that it is
    /// gone, and the app starts a fresh thread unless the page opened one
    /// (oracle: `SessionSelection::StartFresh`).
    fn route(self, left: Handler<PopInvoked>) -> PageRoute<()> {
        match self {
            Start::Agents => crate::pages::agents::start_route_then(left),
            Start::Resume => PageRoute::new(move |_| {
                PopScope::new(crate::pages::resume::Screen { live: true })
                    .on_pop_invoked_with_result(left.clone())
                    .into()
            }),
        }
    }
}

/// The session `chat` with the `start` page pushed over it at mount.
#[derive(Clone, Debug, PartialEq)]
pub struct StartPage {
    pub chat: AnyWidget,
    pub start: Start,
}

/// Whether the chat is built: the start page is the page until it pops.
pub struct StartPageState {
    shown: bool,
}

/// What the start page does next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartMsg {
    /// Push the page of the command line.
    Push,
    /// The page was left (popped).
    Left,
}

impl StatefulWidget for StartPage {
    type State = StartPageState;

    fn create_state(&self) -> StartPageState {
        StartPageState { shown: false }
    }
}

impl_widget!(stateful eq StartPage);

impl State for StartPageState {
    type Widget = StartPage;
    type Msg = StartMsg;

    fn init_state(&mut self, _: &StartPage, cx: &mut Cx<'_, Self>) {
        // The command center the cancelled trust prompt opened was the page
        // and is gone: the chat is mounted at once, as after any start page.
        let overview = onboarding::overview(&read::<ScreenStates, _>(cx).onboarding);
        let msg = if overview {
            self.shown = true;
            StartMsg::Left
        } else {
            StartMsg::Push
        };
        let next = cx.send(msg);
        cx.emit(&next, ());
    }

    fn update(&mut self, msg: StartMsg, w: &StartPage, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            StartMsg::Push => {
                let left = cx.map(|_: PopInvoked| StartMsg::Left);
                drop(Navigator::of(cx).push::<()>(w.start.route(left)));
            }
            StartMsg::Left => {
                let hs = read::<Hs, _>(cx);
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Onboarding(StartupIntent::StartPageLeft),
                );
                if !self.shown {
                    self.shown = true;
                    return Rebuild::Yes;
                }
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &StartPage, _: &mut Cx<'_, Self>) -> AnyWidget {
        if self.shown {
            w.chat.clone()
        } else {
            SizedBox::shrink().into()
        }
    }
}

/// The home under the startup page's gate: the page is pushed over `child`
/// once its model has a step due (oracle: the update prompt and the
/// onboarding screens run before the chat). The facts come from the host
/// after the first frame, so the gate watches the model; it pushes once.
/// With `awaits_facts` the home shows nothing until the facts are read, and
/// then the chat or, with a step due, the page alone: the oracle draws no
/// chat before its prompt. The home mounts only once the startup page is gone
/// (Flutter: `await Navigator.push(..)`, then `setState`): the prompts go
/// first, a start page of the command line (`--agents`, `resume`) pushes its
/// route as it mounts, and the composer takes the focus at mount, not under a
/// route that is still on top (the trust prompt cancelled at launch replaces
/// the page with the command center: the page is gone when that pops).
#[derive(Clone, Debug, PartialEq)]
pub struct StartupGate {
    pub child: AnyWidget,
    pub awaits_facts: bool,
}

/// What the gate does next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateMsg {
    /// Push the startup page.
    Push,
    /// The startup page popped.
    Popped,
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
    type Msg = GateMsg;

    fn did_change_dependencies(&mut self, _: &StartupGate, cx: &mut Cx<'_, Self>) {
        let (due, settled) = select::<ScreenStates, (bool, bool), _>(cx, |states| {
            (states.onboarding.due(), states.onboarding.settled())
        });
        if !self.pushed && due {
            self.pushed = true;
            let push = cx.send(GateMsg::Push);
            cx.emit(&push, ());
        } else if !self.pushed && settled {
            self.open = true;
        }
    }

    fn update(&mut self, msg: GateMsg, _: &StartupGate, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            GateMsg::Push => {
                let left = onboarding::Left {
                    page: cx.send(GateMsg::Popped),
                    center: cx.map(|_: PopInvoked| GateMsg::Popped),
                };
                let step = onboarding::Step::Update;
                drop(Navigator::of(cx).push::<()>(onboarding::route_then(step, left)));
                Rebuild::No
            }
            GateMsg::Popped => {
                self.open = true;
                Rebuild::Yes
            }
        }
    }

    fn build(&self, w: &StartupGate, _: &mut Cx<'_, Self>) -> AnyWidget {
        if self.open {
            w.child.clone()
        } else {
            SizedBox::shrink().into()
        }
    }
}
