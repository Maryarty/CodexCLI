//! The hook runs under way, the thread's side (oracle `chatwidget/hook_lifecycle.rs`,
//! `history_cell/hook_cell.rs`): `hook/started` puts a run on the list, `hook/completed` takes it
//! off and, when the run has something to say (a failure, a block, a stop, output for the user),
//! leaves its row in the transcript at that point of the event order (`flush_completed_hook_output`):
//! a row that stands before any later answer, and a restart of the same id never drops it.
//! The edges of a turn drop the runs whose end never came (`clear_active_hook_cell`: a turn
//! starts, completes, is interrupted or fails), so a cancelled turn leaves no orphaned running
//! row behind.
//!
//! What the oracle holds back for 300 ms (`HOOK_RUN_REVEAL_DELAY`: a quiet hook that is over
//! sooner never shows) is the view's timer (`pages/session/hook_reveal.rs`, ADR-012: core reads
//! no clock): each run has a `seq`, a restart of the same id is a new run with a new delay.
//! The 600 ms a quiet run lingers after it has drawn (`QUIET_HOOK_MIN_VISIBLE`) only holds back
//! bookkeeping the oracle's rows never show (the summary clears on completion), and is not kept.

use super::{Item, ThreadModel, TurnId, apply_appended};

/// A hook run that started and has not completed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningHook {
    /// Told apart from a restart of the same id: that is a new run, with its own reveal delay.
    pub seq: u32,
    pub id: String,
    /// What the hook says of itself while it runs (`statusMessage`).
    pub message: Option<String>,
}

/// The runs under way, in the order they started.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Hooks {
    runs: Vec<RunningHook>,
    next: u32,
}

impl Hooks {
    fn start(&mut self, id: String, message: Option<String>) {
        // a second begin of an id replaces the run (the oracle resets its reveal delay)
        self.runs.retain(|run| run.id != id);
        let seq = self.next;
        self.next = self.next.wrapping_add(1);
        self.runs.push(RunningHook { seq, id, message });
    }
}

impl ThreadModel {
    /// The runs under way: the status row names the ones the view has revealed
    /// ([`summary`]).
    pub fn running_hooks(&self) -> &[RunningHook] {
        &self.hooks.runs
    }
}

/// `hook/started`.
pub(super) fn started(model: &mut ThreadModel, id: String, message: Option<String>) {
    model.hooks.start(id, message);
}

/// `hook/completed`: the run leaves the list, and what it has to say joins the transcript now.
pub(super) fn ended(model: &mut ThreadModel, id: &str, turn: Option<TurnId>, item: Option<Item>) {
    model.hooks.runs.retain(|run| run.id != id);
    if let Some(item) = item {
        apply_appended(model, turn, item);
    }
}

/// A turn started, completed, was interrupted or failed: no run is under way any more.
pub(super) fn clear(model: &mut ThreadModel) {
    model.hooks.runs.clear();
}

/// What the status row says of the runs it shows (oracle `HookCell::running_status_summary`):
/// their shared message, else `Running hooks` for several and `Running hook` for one.
pub fn summary<'a>(runs: impl IntoIterator<Item = &'a RunningHook>) -> Option<String> {
    let mut messages = runs
        .into_iter()
        .map(|run| {
            run.message
                .as_deref()
                .map(str::trim)
                .filter(|message| !message.is_empty())
        })
        .peekable();
    let first = messages.next()?;
    let multiple = messages.peek().is_some();
    Some(match first {
        Some(message) if messages.all(|other| other == Some(message)) => message.to_string(),
        _ if multiple => "Running hooks".to_string(),
        _ => "Running hook".to_string(),
    })
}
