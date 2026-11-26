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
use rtwk::provider::{Provider, read, select};
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

/// The home waits for the host's startup facts and pushes the startup page if a step is due.
/// Start pages and likely first sign-in wait for this decision before they mount. During an
/// established local launch the draft can mount at once, and its state survives a prompt
/// pushed over it. A cancelled trust prompt can replace that prompt with the command center.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupGate {
    pub child: AnyWidget,
    pub awaits_facts: bool,
    pub provisional_draft: bool,
}

/// The chat is mounted for local draft input while the startup page's gate is closed.
#[derive(PartialEq)]
pub(crate) struct ProvisionalDraft(pub bool);

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
        if w.provisional_draft {
            Provider::value(ProvisionalDraft(!self.open))
                .child(w.child.clone())
                .into()
        } else if self.open {
            w.child.clone()
        } else {
            SizedBox::shrink().into()
        }
    }
}
