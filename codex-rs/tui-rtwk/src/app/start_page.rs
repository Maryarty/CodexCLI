//! The page the command line starts on (`codex-rtwk agents`, `codex-rtwk
//! resume`): the session is the navigator's home as ever, and the page is
//! pushed over it at once with nowhere to go back to but the session it
//! reveals when it ends. The resume picker stands in for the first thread,
//! so the chat is not built under it: it mounts once the picker has popped
//! (Flutter: `await Navigator.push(picker)`, then `setState`), and its
//! composer takes the focus at mount (`autofocus`), with whatever thread the
//! pick opened.

use rtwk::Handler;
use rtwk::layout::SizedBox;
use rtwk::navigation::{Navigator, PageRoute, PopInvoked, PopScope};
use rtwk::provider::{read, watch};
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
    /// The route of the page. The resume picker stands in for the first
    /// thread: its `PopScope` (Flutter: `onPopInvokedWithResult`) tells
    /// `left` that it is gone, and the app starts a fresh thread unless the
    /// picker opened one (oracle: `SessionSelection::StartFresh`).
    fn route(self, left: Handler<PopInvoked>) -> PageRoute<()> {
        match self {
            Start::Agents => crate::pages::agents::start_route(),
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

/// Whether the chat is built: the resume picker is the page until it pops.
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
        StartPageState {
            shown: self.start != Start::Resume,
        }
    }
}

impl_widget!(stateful eq StartPage);

impl State for StartPageState {
    type Widget = StartPage;
    type Msg = StartMsg;

    fn init_state(&mut self, _: &StartPage, cx: &mut Cx<'_, Self>) {
        let push = cx.send(StartMsg::Push);
        cx.emit(&push, ());
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
/// chat before its prompt. A start page of the command line (`--agents`,
/// `resume`) pushes its route as it mounts, so with `holds_child` the home
/// mounts only once the startup page has popped (Flutter: `await
/// Navigator.push(..)`, then `setState`): the prompts go first.
#[derive(Clone, Debug, PartialEq)]
pub struct StartupGate {
    pub child: AnyWidget,
    pub awaits_facts: bool,
    pub holds_child: bool,
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
        let onboarding = &watch::<ScreenStates, _>(cx).onboarding;
        if !self.pushed && onboarding.due() {
            self.pushed = true;
            let push = cx.send(GateMsg::Push);
            cx.emit(&push, ());
        } else if !self.pushed && onboarding.settled() {
            self.open = true;
        }
    }

    fn update(&mut self, msg: GateMsg, w: &StartupGate, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            GateMsg::Push => {
                let step = onboarding::Step::Update;
                if w.holds_child {
                    let left = cx.send(GateMsg::Popped);
                    drop(Navigator::of(cx).push::<()>(onboarding::route_then(step, left)));
                    return Rebuild::No;
                }
                drop(Navigator::of(cx).push::<()>(onboarding::route(step)));
            }
            GateMsg::Popped => {}
        }
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
