//! A surface that shows a feature's data says so (ADR-013 И): `Interest` raises the feature's
//! counter when it mounts and lowers it when it is gone, so a surface that forgot to close, or was
//! closed by a route popped under it, can never leave the counter up. It is the only place that
//! sends `Intent::Interest` (rules test `adr013_interest_made_in_one_place`). In Flutter it is
//! the `ref.watch` that subscribes a provider for as long as a widget listens.

use std::rc::Rc;

use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{FeatureInterest, Intent, InterestStep};

use crate::app::view::Hs;

/// `child`, for as long as it is in the tree, wants `what` loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct Interest {
    pub what: FeatureInterest,
    pub child: AnyWidget,
}

impl Interest {
    pub fn new(what: impl Into<FeatureInterest>, child: impl Into<AnyWidget>) -> Interest {
        Interest {
            what: what.into(),
            child: child.into(),
        }
    }
}

pub struct InterestState {
    /// The door to the app, kept: the unmounting element cannot read it (`dispose_with`).
    hs: Option<Rc<Hs>>,
    /// What this element raised and has not lowered.
    held: Option<FeatureInterest>,
}

impl StatefulWidget for Interest {
    type State = InterestState;

    fn create_state(&self) -> InterestState {
        InterestState {
            hs: None,
            held: None,
        }
    }
}

impl_widget!(stateful eq Interest);

impl InterestState {
    fn say(&self, cx: &mut Cx<'_, Self>, step: InterestStep) {
        if let Some(hs) = &self.hs {
            cx.emit(&hs.intent, Intent::Interest(step));
        }
    }
}

impl State for InterestState {
    type Widget = Interest;
    type Msg = ();

    // Not `init_state`: the door is read from above, which is allowed from here on.
    fn did_change_dependencies(&mut self, w: &Interest, cx: &mut Cx<'_, Self>) {
        if self.held.is_none() {
            self.hs = Some(read::<Hs, _>(cx));
            self.held = Some(w.what.clone());
            self.say(cx, InterestStep::Open(w.what.clone()));
        }
    }

    fn did_update_widget(&mut self, old: &Interest, w: &Interest, cx: &mut Cx<'_, Self>) {
        if old.what != w.what {
            self.say(cx, InterestStep::Close(old.what.clone()));
            self.held = Some(w.what.clone());
            self.say(cx, InterestStep::Open(w.what.clone()));
        }
    }

    fn dispose_with(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(what) = self.held.take() {
            self.say(cx, InterestStep::Close(what));
        }
    }

    fn build(&self, w: &Interest, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.child.clone()
    }
}
