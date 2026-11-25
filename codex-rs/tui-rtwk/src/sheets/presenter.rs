//! The request presenter: the one place that opens a dialog for the head of
//! the model's request queue, in the oracle's order (ADR-002). The dialog is shown with rtwk's
//! `AppCx::show_dialog_from` from the kept handle of the `Navigator`'s home
//! (a memo node, see `home_node`), so it is a route of that navigator under
//! the providers; the decision which request is next is made on the model,
//! in `closed` and after each ingested event, never from a provider read in
//! the same flush (G14). The dialog itself is `sheets::approvals`'s.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use rtwk::app::AppCx;
use rtwk::app::MemoCx;
use rtwk::app::ViewRoot;
use rtwk::binding::{AppBinding, Timer};
use rtwk::{AnyWidget, Cx, Handler, StatelessWidget, impl_widget};

use codex_tui_core::ids::RequestKey;
use codex_tui_core::screens::approvals::order::{Family, to_show};

use crate::app::UiApp;
use crate::sheets;

/// How long an approval waits after the last keystroke in the composer (the oracle's
/// `APPROVAL_PROMPT_TYPING_IDLE_DELAY`): the user who is typing is not handed a prompt a stray
/// `y` could answer.
pub const TYPING_IDLE_DELAY: Duration = Duration::from_secs(1);

/// Where the home left its handle: a request is shown from it, and the next
/// one from the handle in the cell then (a replaced home writes its own).
pub type Place = Rc<Cell<Option<ViewRoot<UiApp>>>>;

/// The clock and the timer of the typing delay: the app's binding (left by the home, which is
/// built inside the app) and the handler that presents again when the delay is over.
#[derive(Default)]
pub struct Delay {
    binding: RefCell<Option<AppBinding>>,
    due: RefCell<Option<Handler<()>>>,
    timer: RefCell<Option<Timer>>,
}

impl Delay {
    /// The app clock's time (none while the home has not been built).
    fn now(&self) -> Option<Duration> {
        self.binding
            .borrow()
            .as_ref()
            .map(|binding| binding.clock().now())
    }

    /// Presents again after `left`, instead of any timer armed before.
    fn arm(&self, left: Duration) {
        let (binding, due) = (self.binding.borrow(), self.due.borrow());
        if let (Some(binding), Some(due)) = (binding.as_ref(), due.as_ref()) {
            if let Some(old) = self
                .timer
                .borrow_mut()
                .replace(binding.start_timer(left, due.clone()))
            {
                old.cancel();
            }
        }
    }
}

/// The handle the home carries (equal by identity, so the memo keeps its node).
#[derive(Clone, Default)]
pub struct DelayHandle(Rc<Delay>);

impl PartialEq for DelayHandle {
    fn eq(&self, other: &DelayHandle) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// What the presenter keeps: plain data, UI-only (`UiApp.ui`).
pub struct Presenter {
    place: Place,
    delay: DelayHandle,
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
    /// The app clock at the last composer keystroke (`typed`).
    last_typed: Option<Duration>,
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
            delay: DelayHandle::default(),
            shown: None,
            last: None,
            parked: false,
            last_typed: None,
        }
    }

    /// The cell the home leaves its handle in.
    pub fn place(&self) -> Place {
        self.place.clone()
    }

    /// The clock and timer of the typing delay, which the home fills.
    pub fn delay(&self) -> DelayHandle {
        self.delay.clone()
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

    /// A key reached the composer at `now` (the app clock): approvals wait for the idle delay
    /// after it. The composer owner calls this from the keys the oracle counts as activity
    /// (characters, Backspace, Delete, Enter, Tab without ctrl or alt).
    pub fn typed(&mut self, now: Duration) {
        self.last_typed = self.last_typed.max(Some(now));
    }

    /// How long a request of `family` still waits at `now`, `None` when it may be shown: only an
    /// approval waits (a question or a form is not delayed), and only until the delay since the
    /// last keystroke has passed; the caller arms one timer for the remainder.
    pub fn wait(&self, family: Family, now: Duration) -> Option<Duration> {
        if family != Family::Approval {
            return None;
        }
        let at = self.last_typed?.checked_add(TYPING_IDLE_DELAY)?;
        at.checked_sub(now).filter(|left| !left.is_zero())
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
    let requests = app.core.requests();
    // The view on top by the oracle's order (approvals newest first, the rest oldest first, a
    // kind that is not the last one's under it): `approvals::order`.
    let queue: Vec<(RequestKey, Family)> = requests
        .entries
        .iter()
        .map(|request| (request.key.clone(), Family::of(request.prompt.as_deref())))
        .collect();
    let head = to_show(&queue).cloned();
    // An approval waits for the idle delay after the last key of the composer (the oracle's
    // `delayed_approval_requests`): presented again by a timer when it is over.
    let delay = app.ui.presenter.delay.clone();
    if app.ui.presenter.shown.is_none()
        && let Some(family) = head
            .as_ref()
            .and_then(|key| queue.iter().find(|(queued, _)| queued == key))
            .map(|(_, family)| *family)
        && let Some(now) = delay.0.now()
    {
        let active = app.core.session().active.clone();
        if let Some(keyed) = app.ui.thread_ui.get(active.as_ref()).draft.last_keyed() {
            app.ui.presenter.typed(keyed);
        }
        if let Some(left) = app.ui.presenter.wait(family, now) {
            delay.0.arm(left);
            return;
        }
    }
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
        move |_| sheets::approvals::request_dialog(shown.clone()),
        // A bottom pane without a frame, as the oracle draws a request.
        sheets::sheet_options(),
        move |app, answer, cx| {
            let requests = app.core.requests();
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
pub fn home_node(
    deps: &(Place, DelayHandle, AnyWidget),
    cx: &mut MemoCx<'_, '_, UiApp>,
) -> AnyWidget {
    deps.0.set(Some(cx.root()));
    *deps.1.0.due.borrow_mut() = Some(cx.on(present));
    Home {
        delay: deps.1.clone(),
        child: deps.2.clone(),
    }
    .into()
}

/// The home's child, which leaves the app's binding where the delay reads its clock.
#[derive(Clone, PartialEq)]
struct Home {
    delay: DelayHandle,
    child: AnyWidget,
}

impl_widget!(stateless eq Home);

impl StatelessWidget for Home {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        *self.delay.0.binding.borrow_mut() = AppBinding::maybe_of(cx);
        self.child.clone()
    }
}
