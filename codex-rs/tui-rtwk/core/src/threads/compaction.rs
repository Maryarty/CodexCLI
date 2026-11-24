//! Live context compaction as a thread fact (oracle `chatwidget/compaction.rs`): which compaction
//! is running, when it started, and how long the ones that finished here took. Its wall clock is
//! separate from the turn's running time, and only a live completion that matches the running
//! compaction gives the transcript a duration; a completion that comes with history never does.
//!
//! ADR-013: what core keeps is the fact (`id`, the start in epoch seconds) and the completion
//! data of a cell (`took`). The header ("Compacting context"), its details, the timer drawn from
//! `started_at` and the cell's words are the UI's. The one writer is `threads::apply`
//! (`chat::ThreadEv::Compacting`, the chat's event door: the thread's facts need no variant of
//! the hub's `ThreadEv`).

use codex_app_server_protocol::{ServerNotification, ThreadItemPayload};

use super::{Item, ItemKey, Notice, ThreadEv, ThreadFx, ThreadModel};
use crate::ids::{ItemId, ThreadId, TurnId};
use crate::screens::{ScreenThreadEv, chat};

/// How many finished compactions keep their duration for the cells that show it.
const KEPT: usize = 8;

/// The compaction under way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Running {
    pub id: ItemId,
    /// When it started, epoch seconds: the status row's timer counts from here (`started_at`
    /// of the oracle's `ActiveCompaction`, restored from the server's own start time).
    pub started_at: i64,
}

/// The thread's compaction facts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compaction {
    running: Option<Running>,
    /// Seconds a finished live compaction took, by its item, oldest first: the completion
    /// cell reads it ("Context compacted · 1m 23s"); a compaction not in it is a plain cell.
    took: Vec<(ItemId, u64)>,
}

impl Compaction {
    /// The compaction running now.
    pub fn running(&self) -> Option<&Running> {
        self.running.as_ref()
    }

    /// Whether one runs (the status header says so and the follow-up input must not hide it).
    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// Seconds the live compaction `id` took; none for a replayed or unknown one.
    pub fn took(&self, id: &ItemId) -> Option<u64> {
        self.took
            .iter()
            .find_map(|(item, secs)| (item == id).then_some(*secs))
    }

    /// A repeated start of the same compaction changes nothing (the timer does not restart).
    fn start(&mut self, id: ItemId, started_at: i64) {
        if self.running.as_ref().is_some_and(|active| active.id == id) {
            return;
        }
        self.running = Some(Running { id, started_at });
    }

    /// The compaction `id` completed live: when it is the running one its duration is kept
    /// (`now` unknown, 0, keeps none) and the status goes back. Any other id is a completion that
    /// came with history: it neither contributes a duration nor ends the running one.
    fn finish(&mut self, id: &ItemId, now: i64) {
        let Some(active) = self.running.as_ref().filter(|active| &active.id == id) else {
            return;
        };
        if now > 0 {
            let secs = now.saturating_sub(active.started_at).max(0) as u64;
            self.took.retain(|(item, _)| item != id);
            self.took.push((id.clone(), secs));
            if self.took.len() > KEPT {
                self.took.remove(0);
            }
        }
        self.running = None;
    }

    /// The turn ended without the item's completion, or a new one began (`clear_context_
    /// compaction`).
    pub(super) fn clear(&mut self) {
        self.running = None;
    }
}

/// What happened to a compaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// `item/started` of a context compaction; `started_at_ms` is the server's clock.
    Started {
        turn: TurnId,
        id: ItemId,
        started_at_ms: i64,
    },
    /// `item/completed`: the cell goes into the transcript.
    Completed { key: ItemKey },
}

fn wrap(ev: Ev) -> ThreadEv {
    ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::Compacting(ev)))
}

/// The notifications of a context compaction (they never reach the generic item path: the cell is
/// made on completion, not on start).
pub(super) fn project(notification: &ServerNotification) -> Option<(ThreadId, ThreadEv)> {
    match notification {
        ServerNotification::ItemStarted(n) => match &n.item.payload {
            ThreadItemPayload::ContextCompaction { id } => Some((
                ThreadId::new(&n.thread_id),
                wrap(Ev::Started {
                    turn: TurnId::new(&n.turn_id),
                    id: ItemId::new(id),
                    started_at_ms: n.started_at_ms,
                }),
            )),
            _ => None,
        },
        ServerNotification::ItemCompleted(n) => match &n.item.payload {
            ThreadItemPayload::ContextCompaction { id } => Some((
                ThreadId::new(&n.thread_id),
                wrap(Ev::Completed {
                    key: ItemKey {
                        turn: TurnId::new(&n.turn_id),
                        item: ItemId::new(id),
                    },
                }),
            )),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn apply(model: &mut ThreadModel, ev: Ev, fx: &mut ThreadFx<'_>) {
    match ev {
        Ev::Started {
            turn,
            id,
            started_at_ms,
        } => {
            // A start that comes after its turn ended would leave a timer nobody stops.
            if model.finished_turns.contains(&turn) {
                return;
            }
            let now = fx.now();
            // The server's start time restores the elapsed time of a compaction that began
            // before we looked (a thread switch); one in the future, or a clock the app has
            // not read yet, counts from now.
            let started_ms = (started_at_ms > 0).then_some(started_at_ms / 1000);
            let started_at = match started_ms {
                Some(secs) if now == 0 || secs <= now => secs,
                _ => now,
            };
            model.core.compaction.start(id, started_at);
        }
        Ev::Completed { key } => {
            model.core.compaction.finish(&key.item, fx.now());
            super::apply_ev(
                model,
                ThreadEv::ItemUpsert {
                    key,
                    item: Item::Notice(Notice::ContextCompacted),
                },
                fx,
            );
        }
    }
}
