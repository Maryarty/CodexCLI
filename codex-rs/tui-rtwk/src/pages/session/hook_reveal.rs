//! The 300 ms a hook run is held back before the status row names it (oracle
//! `history_cell/hook_cell.rs` `HOOK_RUN_REVEAL_DELAY`, `HookRunState::PendingReveal`): a hook
//! that is quiet and over sooner never draws, so activity does not flash for work that was
//! effectively instant.
//!
//! In Flutter this is a `Timer` the page's `State` starts for the next run that is due, and a
//! map from the run to the moment it was first built. The runs are the thread's
//! (`ThreadModel::running_hooks`, each with a `seq`, so a restart of the same id is a new run
//! with a new delay); the delay is a presentation matter, timed on the app clock (ADR-012:
//! core reads no clock). The oracle's 600 ms linger of a quiet run that has drawn shows nothing
//! (the summary clears on completion) and is not kept.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use rtwk::binding::{AppBinding, Timer};
use rtwk::{Cx, State};

use codex_tui_core::threads::hooks::{self, RunningHook};

/// How long a run runs before it is shown (`HOOK_RUN_REVEAL_DELAY`).
pub(super) const REVEAL_DELAY: Duration = Duration::from_millis(300);

/// When each run under way was first built, and the timer of the next one that is due.
#[derive(Default)]
pub(super) struct Reveal {
    seen: RefCell<HashMap<u32, Duration>>,
    /// The moment the timer is due on the app clock, and the timer.
    timer: RefCell<Option<(Duration, Timer)>>,
}

impl Reveal {
    /// What the status row says of the runs of `runs` that have run for the delay, none while
    /// all are held back. Every build asks: it notes the runs it has not seen, forgets the ones
    /// that left, and keeps one timer for the earliest run still held back, whose `due` message
    /// builds the page again.
    pub(super) fn shown<S: State>(
        &self,
        runs: &[RunningHook],
        cx: &mut Cx<'_, S>,
        due: S::Msg,
    ) -> Option<String>
    where
        S::Msg: Clone + PartialEq,
    {
        let binding = AppBinding::maybe_of(cx);
        let Some(binding) = binding else {
            // no app clock to time it on (a bare widget tree): nothing is held back
            return hooks::summary(runs);
        };
        let now = binding.clock().now();
        let mut seen = self.seen.borrow_mut();
        seen.retain(|seq, _| runs.iter().any(|run| run.seq == *seq));
        for run in runs {
            seen.entry(run.seq).or_insert(now);
        }
        let shown: Vec<&RunningHook> = runs
            .iter()
            .filter(|run| now.saturating_sub(seen[&run.seq]) >= REVEAL_DELAY)
            .collect();
        let next = runs
            .iter()
            .map(|run| seen[&run.seq] + REVEAL_DELAY)
            .filter(|at| *at > now)
            .min();
        drop(seen);
        self.arm(next, now, &binding, cx.send(due));
        hooks::summary(shown)
    }

    /// One timer for the moment `next` on the app clock, none when no run waits.
    fn arm(
        &self,
        next: Option<Duration>,
        now: Duration,
        binding: &AppBinding,
        wake: rtwk::Handler<()>,
    ) {
        let mut timer = self.timer.borrow_mut();
        if timer.as_ref().map(|(at, _)| *at) == next && next.is_some() {
            return;
        }
        if let Some((_, old)) = timer.take() {
            old.cancel();
        }
        if let Some(at) = next {
            *timer = Some((at, binding.start_timer(at.saturating_sub(now), wake)));
        }
    }

    /// The page goes: no timer is left to call it.
    pub(super) fn dispose(&self) {
        if let Some((_, timer)) = self.timer.borrow_mut().take() {
            timer.cancel();
        }
    }
}
