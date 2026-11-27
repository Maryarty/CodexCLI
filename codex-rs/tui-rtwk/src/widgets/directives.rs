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

use std::cell::RefCell;
use std::collections::BTreeSet;

use rtwk::layout::SizedBox;
use rtwk::navigation::Route;
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Handler, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{Session, SessionSlice};
use codex_tui_core::ids::{ThreadId, TurnId};
use codex_tui_core::threads::{
    DueCommand, LiveTurnEnd, Restore, ThreadModel, UserMessage, startup,
};

/// What changed in the model that the page acts on once.
#[derive(Clone, Debug, PartialEq)]
pub enum Edge {
    /// The model accepted a submit in this thread (the composer clears).
    Submitted,
    /// The thread's replayed submissions are new to the page (a resume, older turns in front):
    /// the composer's history is seeded from them.
    Replayed,
    /// A message went out for the user without the composer (the launch's prompt): the
    /// composer's recall keeps it.
    Recorded(UserMessage),
    /// The messages the model gave back to the composer (an interrupt, a failed steer, the
    /// edit-queued key).
    Restore {
        thread: ThreadId,
        restore: Restore,
        rejected_start: Option<u64>,
    },
    /// The turn's async questions ended, after any queued messages were restored.
    TurnEnded(ThreadId, LiveTurnEnd),
    /// The prompt an edit of the backtrack gives back to the composer.
    Draft(UserMessage),
    /// The turn ended with a proposed plan: "Implement this plan?".
    Plan,
    /// The queue handed the page a command it waits for (`ThreadModel::due_command`): the page
    /// runs the line as if typed now and tells the thread what it opened.
    Due(DueCommand),
    /// The page is the route in front again after it was not (a sheet over it closed): what the
    /// queue waited for may be over.
    Front,
    /// The session is configured and its thread takes input (a turn pending or running, a
    /// model known to be empty, a held queue are over): what waited for the session may go on.
    Ready,
    /// A thread was archived or deleted and the page is in front.
    Removed,
    /// The server refused a fork (the copy never opens): the page stops waiting on it.
    ForkFailed,
}

/// The thread's delivered question ends while core still holds their acknowledgement open.
#[derive(Default)]
pub struct AppliedQuestionEnds(RefCell<BTreeSet<TurnId>>);

impl AppliedQuestionEnds {
    pub fn take(&self, thread: &ThreadModel, turn: &TurnId) -> bool {
        let ends = question_ends(thread);
        let held = |turn: &TurnId| ends.iter().any(|end| &end.turn == turn);
        let mut seen = self.0.borrow_mut();
        seen.retain(held);
        held(turn) && seen.insert(turn.clone())
    }
}

fn question_ends(thread: &ThreadModel) -> &[LiveTurnEnd] {
    thread
        .view()
        .parts()
        .map_or(&[], |parts| parts.async_questions.ends())
}

/// The numbers of a thread the listener compares from one change to the next.
#[derive(Clone, Debug, PartialEq)]
struct Counters {
    thread: ThreadId,
    submitted: Option<u64>,
    replayed: u64,
    recorded: Option<u64>,
    restore: Option<u64>,
    live_ends: Vec<LiveTurnEnd>,
    draft: Option<u64>,
    plan: Option<u64>,
    due: Option<u64>,
}

/// Everything the listener selects.
#[derive(Clone, Debug, PartialEq)]
struct Deps {
    counters: Counters,
    removed: u32,
    fork_failed: u32,
    /// The page is the route in front: a removal waits until it is.
    current: bool,
    /// Only the selected pane can open a global surface.
    focused: bool,
    /// The session is configured and its thread idle (`startup::confirm_ready`).
    ready: bool,
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
            recorded: thread.recorded().map(|(seq, _)| seq),
            restore: thread.restore().map(Restore::seq),
            live_ends: question_ends(thread).to_vec(),
            draft: slots.backtrack.restore.as_ref().map(|restore| restore.seq),
            plan: slots.chat.plan.prompt().map(|prompt| prompt.seq),
            due: thread.due_command().map(DueCommand::seq),
        }
    });
    let removed = select::<SessionSlice, u32, _>(cx, |session| session.removed);
    let thread = counters.thread.clone();
    let addressed = thread.clone();
    let focused = select::<SessionSlice, bool, _>(cx, move |session| {
        session.active.as_ref().map_or_else(
            || addressed.as_str().is_empty(),
            |active| active == &addressed,
        )
    });
    let fork_failed = select::<SessionSlice, u32, _>(cx, move |session| {
        session.fork_failed.get(&thread).copied().unwrap_or(0)
    });
    let idle = select::<ThreadModel, bool, _>(cx, startup::thread_idle);
    let started = select::<SessionSlice, bool, _>(cx, |session| session.status == Session::Ready);
    Deps {
        counters,
        removed,
        fork_failed,
        current: focused && Route::is_current_of(cx),
        focused,
        ready: idle && started,
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
    fork_failed: Option<u32>,
    removed: Option<u32>,
    /// Whether the page was in front at the last look.
    front: bool,
    /// Whether the session was ready at the last look.
    ready: bool,
    due_shown: Option<(ThreadId, u64)>,
    plan_shown: Option<(ThreadId, u64)>,
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
            fork_failed,
            current,
            focused,
            ready,
        } = deps(cx);
        let before = self.seen.replace(now.clone());
        // false to true: what waited for the session goes on, once (a model that arrives later,
        // or a turn that ends, is such a change too)
        if ready && !std::mem::replace(&mut self.ready, ready) {
            cx.emit(&w.on_edge, Edge::Ready);
        }
        self.ready = ready;
        if Self::moved(before.as_ref(), &now, |c| c.submitted) {
            cx.emit(&w.on_edge, Edge::Submitted);
        }
        let owned_restore = now.live_ends.iter().any(|end| {
            end.restore
                .as_ref()
                .is_some_and(|restore| Some(restore.seq()) == now.restore)
        });
        let restore_changed = Self::moved(before.as_ref(), &now, |c| c.restore);
        let newly_shown = before
            .as_ref()
            .is_none_or(|before| before.thread != now.thread);
        let mut restore = (!owned_restore && (restore_changed || newly_shown))
            .then(|| {
                read::<ThreadModel, _>(cx)
                    .restore()
                    .filter(|restore| restore_changed || restore.preserves_editing())
                    .map(|restore| Edge::Restore {
                        thread: now.thread.clone(),
                        restore: restore.clone(),
                        rejected_start: restore.preserves_editing().then(|| restore.seq()),
                    })
            })
            .flatten();
        let before_ends = before
            .as_ref()
            .filter(|before| before.thread == now.thread)
            .map_or(&[][..], |before| before.live_ends.as_slice());
        if now.live_ends.iter().all(|end| end.restore.is_none())
            && let Some(restore) = restore.take()
        {
            cx.emit(&w.on_edge, restore);
        }
        for end in &now.live_ends {
            if !before_ends.contains(end) {
                cx.emit(&w.on_edge, Edge::TurnEnded(now.thread.clone(), end.clone()));
            }
        }
        if let Some(restore) = restore {
            cx.emit(&w.on_edge, restore);
        }
        // the replayed submissions of a thread seed the history once per resume, and again from
        // the whole list when older turns came in front: a first look and a switch count
        let replayed = |c: &Counters| (c.thread.clone(), c.replayed);
        if before.as_ref().map(replayed) != Some(replayed(&now)) && now.replayed > 0 {
            cx.emit(&w.on_edge, Edge::Replayed);
        }
        // a prompt the launch sent is on the thread when the page first looks at it: a first
        // look counts, as for the replayed submissions
        let recorded = |c: &Counters| (c.thread.clone(), c.recorded);
        if now.recorded.is_some()
            && before.as_ref().map(recorded) != Some(recorded(&now))
            && let Some((_, message)) = read::<ThreadModel, _>(cx).recorded()
        {
            cx.emit(&w.on_edge, Edge::Recorded(message.clone()));
        }
        if Self::moved(before.as_ref(), &now, |c| c.draft) {
            let thread = read::<ThreadModel, _>(cx);
            let restore = thread.screen_slots().backtrack.restore.as_ref();
            if let Some(restore) = restore {
                cx.emit(&w.on_edge, Edge::Draft(restore.draft.message()));
            }
        }
        let plan = now.plan.map(|seq| (now.thread.clone(), seq));
        if focused && plan.is_some() && plan != self.plan_shown {
            self.plan_shown = plan;
            cx.emit(&w.on_edge, Edge::Plan);
        }
        // a queued command waits for the page, a first look at the thread included: it is run
        // once, when the page is in front to show what it opens
        let due = now.due.map(|seq| (now.thread.clone(), seq));
        if current
            && due.is_some()
            && due != self.due_shown
            && let Some(command) = read::<ThreadModel, _>(cx).due_command().cloned()
        {
            self.due_shown = due;
            cx.emit(&w.on_edge, Edge::Due(command));
        }
        if current && !self.front {
            cx.emit(&w.on_edge, Edge::Front);
        }
        self.front = current;
        // a removal under a sheet waits until the page is in front
        match self.removed {
            None => self.removed = Some(removed),
            Some(seen) if removed > seen && current => {
                self.removed = Some(removed);
                cx.emit(&w.on_edge, Edge::Removed);
            }
            Some(_) => {}
        }
        // the count is of this page's thread: a switch of thread only takes note of the new one
        let same_thread = before
            .as_ref()
            .is_some_and(|before| before.thread == now.thread);
        if self
            .fork_failed
            .replace(fork_failed)
            .is_some_and(|seen| same_thread && seen < fork_failed)
        {
            cx.emit(&w.on_edge, Edge::ForkFailed);
        }
    }

    fn build(&self, _: &Listener, cx: &mut Cx<'_, Self>) -> AnyWidget {
        // a build forgets what the build before selected: ask again to stay a dependent
        deps(cx);
        SizedBox::shrink().into()
    }
}
