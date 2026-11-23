//! The footer's timed flash (oracle `chat_composer/footer_state.rs`
//! `show_footer_flash`, `flash_visible`): a line that takes the footer's left
//! side for some seconds and then gives it back (to the hint override, else
//! the passive hints).
//!
//! Flutter writes this as a `SnackBar` with a `duration` under the
//! `ScaffoldMessenger`: the owner shows a message, the widget keeps the timer.
//! Here `Timed` wraps a `Footer` whose `flash` is the request; it starts one
//! timer per `epoch` (the owner bumps it for each new flash), draws the flash
//! until the timer fires and the plain footer after.

use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use super::Footer;

/// The style the flash line is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// The terminal's own text style (`Line::from("..")`).
    Plain,
    /// A refusal (`.red()`).
    Error,
}

/// A line the footer shows for `lasts`; `epoch` tells one showing from the next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flash {
    pub text: String,
    pub tone: Tone,
    pub epoch: u64,
    pub lasts: Duration,
}

/// A footer that lets its flash run out.
#[derive(Clone, PartialEq)]
pub struct Timed {
    pub footer: Footer,
}

impl_widget!(stateful eq Timed);

/// Which flash is on, and the timer that ends it.
pub struct TimedState {
    live: Option<u64>,
    spent: Option<u64>,
    timer: Option<Timer>,
}

/// The timer fired for the flash of this epoch.
#[derive(Clone, PartialEq)]
pub enum Msg {
    Expired(u64),
}

impl StatefulWidget for Timed {
    type State = TimedState;

    fn create_state(&self) -> TimedState {
        TimedState {
            live: None,
            spent: None,
            timer: None,
        }
    }
}

impl TimedState {
    /// Starts the timer of a flash not seen before.
    fn arm(&mut self, w: &Timed, cx: &mut Cx<'_, Self>) {
        let Some(flash) = &w.footer.flash else {
            return;
        };
        let epoch = flash.epoch;
        if self.live == Some(epoch) || self.spent == Some(epoch) {
            return;
        }
        if let Some(old) = self.timer.take() {
            old.cancel();
        }
        self.live = Some(epoch);
        if let Some(binding) = AppBinding::maybe_of(cx) {
            self.timer =
                Some(binding.start_timer(flash.lasts, cx.map(move |()| Msg::Expired(epoch))));
        }
    }
}

impl State for TimedState {
    type Widget = Timed;
    type Msg = Msg;

    fn init_state(&mut self, w: &Timed, cx: &mut Cx<'_, Self>) {
        self.arm(w, cx);
    }

    fn did_update_widget(&mut self, _: &Timed, w: &Timed, cx: &mut Cx<'_, Self>) {
        self.arm(w, cx);
    }

    fn dispose(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.cancel();
        }
    }

    fn update(&mut self, msg: Msg, _: &Timed, _: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Expired(epoch) = msg;
        if self.live == Some(epoch) {
            self.live = None;
            self.spent = Some(epoch);
            self.timer = None;
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Timed, _: &mut Cx<'_, Self>) -> AnyWidget {
        let mut footer = w.footer.clone();
        if footer.flash.as_ref().map(|flash| flash.epoch) != self.live {
            footer.flash = None;
        }
        footer.into()
    }
}
