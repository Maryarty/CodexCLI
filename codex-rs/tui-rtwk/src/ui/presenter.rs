//! The request presenter: the one place that opens a dialog for the head of
//! the model's request queue (ADR-002). It wraps the `Navigator`'s home (see
//! `view`), whichever home that is, so it is mounted once and the dialog
//! finds the providers. The dialog itself is `screens::approvals`'s.

use std::time::Duration;

use rtwk::overlay::{DialogOptions, show_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::app::RequestQueue;
use codex_tui_rtwk_core::ids::RequestKey;

use crate::screens;

/// How the presenter awaits its dialog's result: given by the root (the
/// shell's `dialog_closed`), because spawning is the shell's (R1).
pub type CloseWatcher =
    fn(&mut Cx<'_, PresenterState>, rtwk::overlay::DialogResult<()>, PresenterMsg);

/// Shows the dialog of the queue's head; its child is what it wraps.
pub struct Presenter {
    pub watch: CloseWatcher,
    pub child: AnyWidget,
}

pub struct PresenterState {
    watch: CloseWatcher,
    shown: Option<RequestKey>,
    /// The last request shown: a snapshot that still holds it (the view
    /// has not rerun yet) is not a new request.
    last: Option<RequestKey>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PresenterMsg {
    Closed(RequestKey),
}

impl StatefulWidget for Presenter {
    type State = PresenterState;

    fn create_state(&self) -> PresenterState {
        PresenterState {
            watch: self.watch,
            shown: None,
            last: None,
        }
    }
}

impl_widget!(stateful Presenter);

impl PresenterState {
    fn sync(&mut self, head: Option<RequestKey>, cx: &mut Cx<'_, Self>) {
        let (None, Some(key)) = (&self.shown, head) else {
            return;
        };
        if self.last.as_ref() == Some(&key) {
            return;
        }
        self.shown = Some(key.clone());
        self.last = Some(key.clone());
        let shown = key.clone();
        let result = show_dialog::<(), _>(
            cx,
            move |_| screens::approvals::request_dialog(shown.clone()),
            DialogOptions {
                barrier_dismissible: false,
                fade_in: Duration::ZERO,
                ..DialogOptions::default()
            },
        );
        (self.watch)(cx, result, PresenterMsg::Closed(key));
    }
}

impl State for PresenterState {
    type Widget = Presenter;
    type Msg = PresenterMsg;

    fn did_change_dependencies(&mut self, _: &Presenter, cx: &mut Cx<'_, Self>) {
        let head = select::<RequestQueue, Option<RequestKey>, _>(cx, |queue| {
            queue.head().map(|request| request.key.clone())
        });
        self.sync(head, cx);
    }

    fn update(&mut self, msg: PresenterMsg, _: &Presenter, cx: &mut Cx<'_, Self>) -> Rebuild {
        let PresenterMsg::Closed(key) = msg;
        if self.shown.as_ref() == Some(&key) {
            self.shown = None;
        }
        // A request that came while the dialog was closing.
        let head = read::<RequestQueue, _>(cx)
            .head()
            .map(|request| request.key.clone());
        self.sync(head, cx);
        Rebuild::No
    }

    fn build(&self, w: &Presenter, _: &mut Cx<'_, Self>) -> AnyWidget {
        w.child.clone()
    }
}
