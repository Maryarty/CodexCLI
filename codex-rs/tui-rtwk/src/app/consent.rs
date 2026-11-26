//! Push the startup trust step over the current page when session navigation
//! reaches a folder that needs consent. Each question gets one route.

use rtwk::navigation::Navigator;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::onboarding::consent;

use crate::pages::startup::onboarding::{self, Step};

/// The home with the trust question's gate: the page is pushed over `child` for each question
/// the model opens.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsentGate {
    pub child: AnyWidget,
}

/// Which question the gate pushed the page for last (R14: widget state).
pub struct ConsentGateState {
    pushed: Option<u64>,
}

impl StatefulWidget for ConsentGate {
    type State = ConsentGateState;

    fn create_state(&self) -> ConsentGateState {
        ConsentGateState { pushed: None }
    }
}

impl_widget!(stateful eq ConsentGate);

impl State for ConsentGateState {
    type Widget = ConsentGate;
    type Msg = ();

    fn did_change_dependencies(&mut self, _: &ConsentGate, cx: &mut Cx<'_, Self>) {
        let open = select::<ScreenStates, Option<u64>, _>(cx, |states| {
            consent::open_number(&states.onboarding)
        });
        if let Some(number) = open.filter(|number| self.pushed != Some(*number)) {
            self.pushed = Some(number);
            let push = cx.send(());
            cx.emit(&push, ());
        }
    }

    fn update(&mut self, (): (), _: &ConsentGate, cx: &mut Cx<'_, Self>) -> Rebuild {
        drop(Navigator::of(cx).push::<()>(onboarding::route(Step::Consent)));
        Rebuild::No
    }

    fn build(&self, w: &ConsentGate, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.child.clone()
    }
}
