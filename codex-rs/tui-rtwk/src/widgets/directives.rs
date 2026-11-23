//! The one-shots the model leaves for a surface (ADR-013 Ж `Directive`: a restore, a prompt, a
//! number that grew) and the one listener that carries each out once. Flutter's `BlocListener`:
//! a leaf widget that is a dependent of the model's counters and tells its page, as an [`Edge`] on
//! the page's [`Handler`], when one changed; the page acts on it in its own `update`. It is the
//! only place that reads a counter (`seq`, `submit_seq`) or a directive's payload (rules test
//! `adr013_directives_read_in_one_place`).
//!
//! Why a widget of its own and not the page's `did_change_dependencies`: rtwk does not tell a
//! dependent that is already to be rebuilt that a part it selects changed (it selects anew in the
//! build), so an edge that lands in the same flush as the page's own rebuild would be lost to the
//! page. The listener has no reason to be rebuilt but its own selects (its widget is equal on each
//! rebuild of the page), so each edge reaches it. Its `deps` is the one list of what it selects:
//! asked in `did_change_dependencies` and again in `build`, because a build forgets the selects of
//! the build before.

use rtwk::layout::SizedBox;
use rtwk::navigation::Route;
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Handler, State, StatefulWidget, impl_widget};

use codex_tui_core::app::SessionSlice;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::agents::Notice;
use codex_tui_core::threads::{Restore, ThreadModel, UserMessage};

/// What changed in the model that the page acts on once.
#[derive(Clone, Debug, PartialEq)]
pub enum Edge {
    /// The model accepted a submit in this thread (the composer clears).
    Submitted,
    /// The thread's replayed submissions are new to the page (a resume, older turns in front):
    /// the composer's history is seeded from them.
    Replayed,
    /// The messages the model gave back to the composer (an interrupt, a failed steer, the
    /// edit-queued key).
    Restore(Restore),
    /// The prompt an edit of the backtrack gives back to the composer.
    Draft(UserMessage),
    /// The turn ended with a proposed plan: "Implement this plan?".
    Plan,
    /// A thread was archived or deleted and the page is in front.
    Removed,
    /// The error the command center's notice shows changed (a fork ends with an error of it):
    /// its text, none when the notice is no error.
    Error(Option<String>),
}

/// The numbers of a thread the listener compares from one change to the next.
#[derive(Clone, Debug, PartialEq)]
struct Counters {
    thread: ThreadId,
    submitted: Option<u64>,
    replayed: u64,
    restore: Option<u64>,
    draft: Option<u64>,
    plan: Option<u64>,
}

/// Everything the listener selects.
#[derive(Clone, Debug, PartialEq)]
struct Deps {
    counters: Counters,
    removed: u32,
    error: Option<String>,
    /// The page is the route in front: a removal waits until it is.
    current: bool,
}

/// The text of the error a notice shows (what a fork that failed leaves in the command center).
pub fn error_of(notice: &Option<Notice>) -> Option<String> {
    match notice {
        Some(Notice::Error(text)) => Some(text.clone()),
        _ => None,
    }
}

/// The one list of what the listener selects (asked by its `did_change_dependencies` and its
/// `build`).
fn deps<S: 'static>(cx: &mut Cx<'_, S>) -> Deps {
    let counters = select::<ThreadModel, Counters, _>(cx, |thread| {
        let slots = thread.screen_slots();
        Counters {
            thread: thread.id().clone(),
            submitted: thread.submit_seq(),
            replayed: thread.replayed_submissions().generation(),
            restore: thread.restore().map(Restore::seq),
            draft: slots.backtrack.restore.as_ref().map(|restore| restore.seq),
            plan: slots.chat.plan.prompt().map(|prompt| prompt.seq),
        }
    });
    let removed = select::<SessionSlice, u32, _>(cx, |session| session.removed);
    let error = select::<ScreenStates, _, _>(cx, |states| error_of(&states.agents.notice));
    Deps {
        counters,
        removed,
        error,
        current: Route::is_current_of(cx),
    }
}

/// The listener: put it once in the page's tree; `on_edge` is the page's handler (made once, so
/// the widget stays equal).
#[derive(Clone, PartialEq)]
pub struct Listener {
    pub on_edge: Handler<Edge>,
}

/// What the listener has seen: the last numbers, and the removals the page has answered.
#[derive(Default)]
pub struct ListenerState {
    seen: Option<Counters>,
    error: Option<Option<String>>,
    removed: Option<u32>,
}

impl StatefulWidget for Listener {
    type State = ListenerState;

    fn create_state(&self) -> ListenerState {
        ListenerState::default()
    }
}

impl_widget!(stateful eq Listener);

impl ListenerState {
    /// A number of the same thread that moved to a value: each number is carried out once, in the
    /// thread that made it (the first look at a thread, or a switch of thread, only takes note).
    fn moved(
        before: Option<&Counters>,
        now: &Counters,
        pick: fn(&Counters) -> Option<u64>,
    ) -> bool {
        before.is_some_and(|before| {
            before.thread == now.thread && pick(before) != pick(now) && pick(now).is_some()
        })
    }
}

impl State for ListenerState {
    type Widget = Listener;
    type Msg = ();

    fn did_change_dependencies(&mut self, w: &Listener, cx: &mut Cx<'_, Self>) {
        let Deps {
            counters: now,
            removed,
            error,
            current,
        } = deps(cx);
        let before = self.seen.replace(now.clone());
        if Self::moved(before.as_ref(), &now, |c| c.submitted) {
            cx.emit(&w.on_edge, Edge::Submitted);
        }
        if Self::moved(before.as_ref(), &now, |c| c.restore)
            && let Some(restore) = read::<ThreadModel, _>(cx).restore().cloned()
        {
            cx.emit(&w.on_edge, Edge::Restore(restore));
        }
        // the replayed submissions of a thread seed the history once per resume, and again from
        // the whole list when older turns came in front: a first look and a switch count
        let replayed = |c: &Counters| (c.thread.clone(), c.replayed);
        if before.as_ref().map(replayed) != Some(replayed(&now)) && now.replayed > 0 {
            cx.emit(&w.on_edge, Edge::Replayed);
        }
        if Self::moved(before.as_ref(), &now, |c| c.draft) {
            let thread = read::<ThreadModel, _>(cx);
            let restore = thread.screen_slots().backtrack.restore.as_ref();
            if let Some(restore) = restore {
                cx.emit(&w.on_edge, Edge::Draft(restore.draft.message()));
            }
        }
        if Self::moved(before.as_ref(), &now, |c| c.plan) {
            cx.emit(&w.on_edge, Edge::Plan);
        }
        // a removal under a sheet waits until the page is in front
        match self.removed {
            None => self.removed = Some(removed),
            Some(seen) if removed > seen && current => {
                self.removed = Some(removed);
                cx.emit(&w.on_edge, Edge::Removed);
            }
            Some(_) => {}
        }
        if self
            .error
            .replace(error.clone())
            .is_some_and(|seen| seen != error)
        {
            cx.emit(&w.on_edge, Edge::Error(error));
        }
    }

    fn build(&self, _: &Listener, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // a build forgets what the build before selected: ask again to stay a dependent
        deps(cx);
        SizedBox::shrink().into()
    }
}
