//! The one-shots a feature leaves for a surface (ADR-013 Ж `Directive`): a restore, a scroll to
//! a place, a focus. The listener reads one by its `seq`, once, so a rebuild or a second reader
//! never carries it out twice; it is the only place that does (rules test
//! `adr013_directives_read_in_one_place`). The chat's composer takes the thread's pending restore
//! (`ThreadModel::restore`) and the backtrack's draft through [`Listener`]; the pilot of `/plugins`
//! has none.

use rtwk::Cx;
use rtwk::provider::{read, select};

use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::{Restore, ThreadModel, UserMessage};

/// What the listener has carried out: the `seq` of the last directive it took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seen(u64);

impl Seen {
    /// Whether the directive numbered `seq` is new to this listener; it is then marked taken.
    pub fn take(&mut self, seq: u64) -> bool {
        if seq > self.0 {
            self.0 = seq;
            true
        } else {
            false
        }
    }
}

/// The listener of a thread's one-shot: the page asks `poll` at each change of dependencies and
/// gets the directive's payload once, the first time its `seq` is seen to change within the same
/// thread (the first look at a thread, or a switch of thread, only takes note). Flutter's
/// `ref.listen` on a `seq` with the payload read when it fires.
#[derive(Debug, Default)]
pub struct Listener {
    seen: Option<(ThreadId, Option<u64>)>,
}

fn restore_seq(thread: &ThreadModel) -> Option<u64> {
    thread.restore().map(Restore::seq)
}

fn draft_seq(thread: &ThreadModel) -> Option<u64> {
    let restore = thread.screen_slots().backtrack.restore.as_ref();
    restore.map(|restore| restore.seq)
}

/// The `seq` of a directive of the thread, selected: what keeps the page a dependent of the
/// thread for its `did_change_dependencies` (a build selects anew, so each build asks it too).
fn numbered<S: 'static>(
    cx: &mut Cx<'_, S>,
    seq_of: fn(&ThreadModel) -> Option<u64>,
) -> (ThreadId, Option<u64>) {
    select::<ThreadModel, (ThreadId, Option<u64>), _>(cx, move |thread| {
        (thread.id().clone(), seq_of(thread))
    })
}

impl Listener {
    /// The messages the model gave back to the composer (an interrupt, a failed steer, the
    /// edit-queued key): the thread's pending restore.
    pub fn restore<S: 'static>(&mut self, cx: &mut Cx<'_, S>) -> Option<Restore> {
        self.poll(cx, restore_seq, |thread| thread.restore().cloned())
    }

    /// The prompt an edit of the backtrack gives back to the composer.
    pub fn backtrack_draft<S: 'static>(&mut self, cx: &mut Cx<'_, S>) -> Option<UserMessage> {
        self.poll(cx, draft_seq, |thread| {
            let restore = thread.screen_slots().backtrack.restore.as_ref();
            restore.map(|restore| restore.draft.message())
        })
    }

    /// The build's own look at the restore's number (`restore` is asked in
    /// `did_change_dependencies`; the build keeps it a dependency).
    pub fn keep_restore<S: 'static>(cx: &mut Cx<'_, S>) {
        numbered(cx, restore_seq);
    }

    /// The same for the backtrack's draft.
    pub fn keep_backtrack_draft<S: 'static>(cx: &mut Cx<'_, S>) {
        numbered(cx, draft_seq);
    }

    fn poll<S: 'static, T>(
        &mut self,
        cx: &mut Cx<'_, S>,
        seq_of: fn(&ThreadModel) -> Option<u64>,
        payload: fn(&ThreadModel) -> Option<T>,
    ) -> Option<T> {
        let now = numbered(cx, seq_of);
        let fresh = self.seen.as_ref().is_some_and(|(thread, before)| {
            *thread == now.0 && *before != now.1 && now.1.is_some()
        });
        self.seen = Some(now);
        if fresh {
            payload(&read::<ThreadModel, _>(cx))
        } else {
            None
        }
    }
}
