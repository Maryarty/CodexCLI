//! The empty conversation moves to a Cloud backend (oracle `cloud_switch_wait.rs`): the draft
//! stays editable and nothing is submitted while Cloud prepares the destination, the footer says
//! `Connecting to Cloud…`, and a refused start says `Cloud conversation did not open · draft
//! kept` for the frame that heard it. The state is the app's (`SessionSlice::cloud_switch`,
//! derived from the start's ticket); the page words it, restricts its composer and lets the
//! refusal's hint run out.
//!
//! Flutter writes the refusal's hint as a `SnackBar` with a short `duration`: `Hinted` owns the
//! timer (as `footer::Timed` does for a flash) and draws the hint until it fires.

use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{CloudSwitch, SessionSlice};
use codex_tui_core::screens::composer::restricted::Restricted;

use crate::pages::session::footer::{Footer, Timed};
use crate::pages::session::strings::Strings;

/// How long a refusal's hint stays: the oracle draws it for the one frame of the failure, and
/// the error is on screen with the next.
const BRIEF: Duration = Duration::from_millis(50);

/// The footer's hint items while the switch is on (`show_cloud_switch_hint`): the label with no
/// action while connecting, the label and `draft kept` after a refusal.
pub(super) struct Hint {
    items: Vec<(String, String)>,
    /// A refusal: the hint runs out.
    brief: bool,
}

impl Hint {
    pub(super) fn of(session: &SessionSlice, strings: &Strings) -> Option<Hint> {
        let (label, action, brief) = match session.cloud_switch? {
            CloudSwitch::Connecting => (strings.cloud_connecting, "", false),
            CloudSwitch::Failed => (strings.cloud_failed, strings.cloud_failed_action, true),
        };
        Some(Hint {
            items: vec![(label.to_owned(), action.to_owned())],
            brief,
        })
    }
}

/// The composer edits and submits nothing while Cloud is asked (`handle_restricted_key` with
/// `RestrictedInputMode::DraftHandoff`): Enter, Tab and the queue key are swallowed. A refusal
/// restricts nothing: the draft is the user's again.
pub(super) fn restricted(session: &SessionSlice) -> Option<Restricted> {
    (session.cloud_switch == Some(CloudSwitch::Connecting)).then_some(Restricted::DraftHandoff)
}

/// The composer's footer with the switch's hint over it (`set_footer_hint_override`), the
/// refusal's for a moment only. Inside, the footer's own timed flash.
pub(super) fn footer(footer: Footer, hint: Option<Hint>) -> AnyWidget {
    match hint {
        Some(hint) => Hinted {
            footer,
            items: hint.items,
            brief: hint.brief,
        }
        .into(),
        None => Timed { footer }.into(),
    }
}

/// A footer under the switch's hint.
#[derive(Clone, PartialEq)]
struct Hinted {
    footer: Footer,
    items: Vec<(String, String)>,
    brief: bool,
}

impl_widget!(stateful eq Hinted);

/// Whether the refusal's hint ran out, and the timer that ends it.
struct HintedState {
    spent: bool,
    timer: Option<Timer>,
}

#[derive(Clone, PartialEq)]
enum Msg {
    Expired,
}

impl StatefulWidget for Hinted {
    type State = HintedState;

    fn create_state(&self) -> HintedState {
        HintedState {
            spent: false,
            timer: None,
        }
    }
}

impl HintedState {
    /// A refusal's hint starts its timer once; a hint that stays cancels it.
    fn arm(&mut self, w: &Hinted, cx: &mut Cx<'_, Self>) {
        if !w.brief {
            self.stop();
            return;
        }
        if self.spent || self.timer.is_some() {
            return;
        }
        if let Some(binding) = AppBinding::maybe_of(cx) {
            self.timer = Some(binding.start_timer(BRIEF, cx.map(|()| Msg::Expired)));
        }
    }

    fn stop(&mut self) {
        self.spent = false;
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
    }
}

impl State for HintedState {
    type Widget = Hinted;
    type Msg = Msg;

    fn init_state(&mut self, w: &Hinted, cx: &mut Cx<'_, Self>) {
        self.arm(w, cx);
    }

    fn did_update_widget(&mut self, _: &Hinted, w: &Hinted, cx: &mut Cx<'_, Self>) {
        self.arm(w, cx);
    }

    fn dispose(&mut self) {
        self.stop();
    }

    fn update(&mut self, msg: Msg, _: &Hinted, _: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Expired = msg;
        self.spent = true;
        self.timer = None;
        Rebuild::Yes
    }

    fn build(&self, w: &Hinted, _: &mut Cx<'_, Self>) -> AnyWidget {
        let mut footer = w.footer.clone();
        if !(w.brief && self.spent) && footer.hint_override.is_none() {
            footer.hint_override = Some(w.items.clone());
        }
        Timed { footer }.into()
    }
}
