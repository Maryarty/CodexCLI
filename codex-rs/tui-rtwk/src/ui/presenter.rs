//! The request presenter: the one place that opens a dialog for the head of
//! the model's request queue (ADR-002). The dialog is shown with rtwk's
//! `AppCx::show_dialog_from` from the kept handle of the `Navigator`'s home
//! (a memo node, see `home_node`), so it is a route of that navigator under
//! the providers; the decision which request is next is made on the model,
//! in `closed` and after each ingested event, never from a provider read in
//! the same flush (G14). The dialog itself is `screens::approvals`'s.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::AnyWidget;
use rtwk::app::AppCx;
use rtwk::app::MemoCx;
use rtwk::app::ViewRoot;
use rtwk::overlay::DialogOptions;

use codex_tui_core::ids::RequestKey;

use crate::app::UiApp;
use crate::screens;

/// Where the home left its handle: a request is shown from it, and the next
/// one from the handle in the cell then (a replaced home writes its own).
pub type Place = Rc<Cell<Option<ViewRoot<UiApp>>>>;

/// What the presenter keeps: plain data, UI-only (`UiApp.ui`).
pub struct Presenter {
    place: Place,
    /// The request whose dialog is open.
    shown: Option<RequestKey>,
    /// The last request shown: while it is queued it is not shown again (its
    /// dismissal is still on its way to the model, or its dialog went with a
    /// home that was replaced).
    last: Option<RequestKey>,
    /// `last` closed with `None` while still queued: not retried by
    /// `closed` (that would run at every flush while the place is gone),
    /// but by the next ingested event.
    parked: bool,
}

impl Default for Presenter {
    fn default() -> Self {
        Self::new()
    }
}

impl Presenter {
    pub fn new() -> Self {
        Self {
            place: Place::default(),
            shown: None,
            last: None,
            parked: false,
        }
    }

    /// The cell the home leaves its handle in.
    pub fn place(&self) -> Place {
        self.place.clone()
    }

    /// The request whose dialog is open.
    pub fn shown(&self) -> Option<&RequestKey> {
        self.shown.as_ref()
    }

    /// The request to show now, given the queue's `head` and what is
    /// `queued`: none while a dialog is open or when the head is the one
    /// shown last and still queued. The key returned counts as shown.
    pub fn next(
        &mut self,
        head: Option<RequestKey>,
        queued: impl Fn(&RequestKey) -> bool,
    ) -> Option<RequestKey> {
        if self.shown.is_some() {
            return None;
        }
        if self.last.as_ref().is_some_and(|last| !queued(last)) {
            self.last = None;
        }
        let key = head?;
        if self.last.as_ref() == Some(&key) {
            return None;
        }
        self.shown = Some(key.clone());
        self.last = Some(key.clone());
        Some(key)
    }

    /// The dialog of `key` closed with `answer` (`None`: the request left the
    /// queue, the home went, or no dialog could be shown).
    pub fn closed(
        &mut self,
        key: &RequestKey,
        answer: Option<()>,
        queued: impl Fn(&RequestKey) -> bool,
    ) {
        if self.shown.as_ref() == Some(key) {
            self.shown = None;
        }
        if answer.is_none() && queued(key) {
            self.parked = true;
        }
    }

    /// An event was ingested: a request parked by a `None` may be tried
    /// again, once.
    pub fn event(&mut self) {
        if std::mem::take(&mut self.parked) {
            self.last = None;
        }
    }
}

/// Shows the head of the queue if no request is open and the home's handle
/// is there; otherwise the request waits in the queue. Called after each
/// ingested event and from `closed`, with the model at hand.
pub fn present(app: &mut UiApp, cx: &mut AppCx<'_, '_, UiApp>) {
    let Some(place) = app.ui.presenter.place.get() else {
        return;
    };
    let requests = &app.core.requests;
    let head = requests.head().map(|request| request.key.clone());
    let Some(key) = app
        .ui
        .presenter
        .next(head, |queued| requests.contains(queued))
    else {
        return;
    };
    let shown = key.clone();
    let dialog = cx.show_dialog_from::<()>(
        place,
        move |_| screens::approvals::request_dialog(shown.clone()),
        // A bottom pane without a frame, as the oracle draws a request:
        // no dimming, shown at once (the pane aligns itself to the bottom).
        DialogOptions {
            barrier_alpha: Some(0),
            barrier_dismissible: false,
            fade_in: Duration::ZERO,
            ..DialogOptions::default()
        },
        move |app, answer, cx| {
            let requests = &app.core.requests;
            app.ui
                .presenter
                .closed(&key, answer, |queued| requests.contains(queued));
            present(app, cx);
        },
    );
    if dialog.is_closed() {
        // The handle's memo node is gone (rtwk showed nothing and `closed`
        // follows with `None`): drop the handle, requests wait for the next
        // home, which leaves its own.
        app.ui.presenter.place.set(None);
    }
}

/// The `Navigator`'s home: a memo node that leaves its handle in `place`
/// and is `child`. Its handle is the origin the request dialogs are shown
/// from.
pub fn home_node(deps: &(Place, AnyWidget), cx: &mut MemoCx<'_, '_, UiApp>) -> AnyWidget {
    deps.0.set(Some(cx.root()));
    deps.1.clone()
}
