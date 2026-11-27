//! Saved side histories. The local host attests each result against the child's own rollout;
//! this state owns the parent, launch thread, and revision at which each operation was requested.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::app::AttachedHistory;
use crate::features::{AppCx, Refusal};
use crate::ids::ThreadId;
use crate::rpc::RpcError;

use super::{Command, Link, Reply, State as SideState, Storage, drop_side, failure};

pub use crate::app::SavedAttachment as SavedChat;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SavedChats {
    Unavailable,
    Loading,
    Ready(Vec<SavedChat>),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SavedReply {
    List(Option<Vec<SavedChat>>),
    Verified(bool),
    Archived(bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    List { parent: ThreadId, revision: u64 },
    Work { parent: ThreadId, revision: u64 },
    Restored { parent: ThreadId, revision: u64 },
    Returned { parent: ThreadId, revision: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Open,
    Archive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Action {
    revision: u64,
    child: ThreadId,
    anchor: Option<ThreadId>,
    kind: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Inventory {
    revision: u64,
    phase: SavedChats,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct State {
    sequence: u64,
    inventories: BTreeMap<ThreadId, Inventory>,
    actions: BTreeMap<ThreadId, Action>,
}

impl State {
    pub(super) fn get(&self, parent: &ThreadId) -> Option<&SavedChats> {
        self.inventories
            .get(parent)
            .map(|inventory| &inventory.phase)
    }

    pub(super) fn depart(&mut self, thread: &ThreadId) {
        for action in self.actions.values_mut() {
            if action.anchor.as_ref() != Some(thread) {
                action.anchor = None;
            }
        }
    }

    fn issue(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    fn contains(&self, parent: &ThreadId, child: &ThreadId) -> bool {
        matches!(self.get(parent), Some(SavedChats::Ready(rows)) if rows.iter().any(|row| &row.id == child))
    }

    fn complete(&mut self, parent: &ThreadId, error: Option<String>) {
        self.actions.remove(parent);
        if let Some(error) = error
            && let Some(inventory) = self.inventories.get_mut(parent)
        {
            inventory.phase = SavedChats::Failed(error);
        }
    }
}

fn family(state: &SideState, parent: &ThreadId, cx: &AppCx<'_>) -> bool {
    let active = cx.view().active();
    active == Some(parent)
        || active
            .and_then(|id| state.link(id))
            .is_some_and(|link| &link.parent == parent)
}

pub(super) fn refresh(state: &mut SideState, parent: ThreadId, cx: &mut AppCx<'_>) {
    if state.is_side(&parent)
        || !family(state, &parent, cx)
        || state.saved.actions.contains_key(&parent)
    {
        return;
    }
    let revision = state.saved.issue();
    state.saved.inventories.insert(
        parent.clone(),
        Inventory {
            revision,
            phase: SavedChats::Loading,
        },
    );
    let work = AttachedHistory::Discover;
    cx.ask_attachment(
        &parent,
        work.clone(),
        Command::Saved(work),
        super::Ctx::Saved(Ctx::List {
            parent: parent.clone(),
            revision,
        }),
    );
}

pub(super) fn open(state: &mut SideState, parent: ThreadId, child: ThreadId, cx: &mut AppCx<'_>) {
    begin(state, parent, child, Operation::Open, cx);
}

pub(super) fn archive(
    state: &mut SideState,
    parent: ThreadId,
    child: ThreadId,
    cx: &mut AppCx<'_>,
) {
    begin(state, parent, child, Operation::Archive, cx);
}

fn begin(
    state: &mut SideState,
    parent: ThreadId,
    child: ThreadId,
    kind: Operation,
    cx: &mut AppCx<'_>,
) {
    let attached = state
        .link(&child)
        .is_some_and(|link| link.parent == parent && link.storage == Storage::Attached);
    if child == parent
        || !family(state, &parent, cx)
        || (!state.saved.contains(&parent, &child) && !(attached && kind == Operation::Archive))
        || state.saved.actions.contains_key(&parent)
    {
        return;
    }
    let revision = state.saved.issue();
    let active = cx.view().active().cloned();
    state.saved.actions.insert(
        parent.clone(),
        Action {
            revision,
            child: child.clone(),
            anchor: active.clone(),
            kind,
        },
    );
    if kind == Operation::Archive && active.as_ref() == Some(&child) {
        cx.open_thread(
            &parent,
            super::Ctx::Saved(Ctx::Returned {
                parent: parent.clone(),
                revision,
            }),
        );
    } else {
        ask(&parent, child, revision, kind, cx);
    }
}

fn ask(parent: &ThreadId, child: ThreadId, revision: u64, kind: Operation, cx: &mut AppCx<'_>) {
    let work = match kind {
        Operation::Open => AttachedHistory::Verify { child },
        Operation::Archive => AttachedHistory::Archive { child },
    };
    cx.ask_attachment(
        parent,
        work.clone(),
        Command::Saved(work),
        super::Ctx::Saved(Ctx::Work {
            parent: parent.clone(),
            revision,
        }),
    );
}

pub(super) fn answered(
    state: &mut SideState,
    ctx: Ctx,
    reply: Result<Reply, RpcError>,
    cx: &mut AppCx<'_>,
) {
    match ctx {
        Ctx::List { parent, revision } => {
            let Some(inventory) = state
                .saved
                .inventories
                .get_mut(&parent)
                .filter(|state| state.revision == revision)
            else {
                return;
            };
            inventory.phase = match reply {
                Ok(Reply::Saved(SavedReply::List(Some(rows)))) => SavedChats::Ready(rows),
                Ok(Reply::Saved(SavedReply::List(None))) => SavedChats::Unavailable,
                Err(error) => SavedChats::Failed(failure(&error)),
                _ => SavedChats::Failed(failure(&RpcError::Mismatch)),
            };
        }
        Ctx::Work { parent, revision } => {
            let Some(action) = state
                .saved
                .actions
                .get(&parent)
                .filter(|action| action.revision == revision)
                .cloned()
            else {
                return;
            };
            match (action.kind, reply) {
                (Operation::Open, Ok(Reply::Saved(SavedReply::Verified(true)))) => {
                    if state.link(&action.child).is_some_and(|link| {
                        link.parent == parent && link.storage == Storage::Attached
                    }) && cx
                        .view()
                        .thread(&action.child)
                        .is_some_and(|thread| thread.has_writer() && !thread.stream_lost())
                    {
                        restored(state, parent, action, cx);
                    } else {
                        cx.restore_attachment(
                            &parent,
                            &action.child,
                            super::Ctx::Saved(Ctx::Restored {
                                parent: parent.clone(),
                                revision,
                            }),
                        );
                    }
                }
                (Operation::Archive, Ok(Reply::Saved(SavedReply::Archived(true)))) => {
                    state.saved.complete(&parent, None);
                    if let Some(Inventory {
                        phase: SavedChats::Ready(rows),
                        ..
                    }) = state.saved.inventories.get_mut(&parent)
                    {
                        rows.retain(|row| row.id != action.child);
                    }
                    drop_side(state, &action.child, cx);
                }
                (_, result) => {
                    let error = result.err().unwrap_or(RpcError::Mismatch);
                    state.saved.complete(&parent, Some(failure(&error)));
                }
            }
        }
        Ctx::Restored { .. } | Ctx::Returned { .. } => {}
    }
}

pub(super) fn opened(
    state: &mut SideState,
    ctx: Ctx,
    result: Result<ThreadId, Refusal>,
    cx: &mut AppCx<'_>,
) {
    let (parent, revision, restored_child) = match ctx {
        Ctx::Restored { parent, revision } => (parent, revision, true),
        Ctx::Returned { parent, revision } => (parent, revision, false),
        _ => return,
    };
    let Some(action) = state
        .saved
        .actions
        .get(&parent)
        .filter(|action| action.revision == revision)
        .cloned()
    else {
        return;
    };
    match result {
        Ok(child) if restored_child && child == action.child => restored(state, parent, action, cx),
        Ok(shown) if !restored_child && shown == parent && cx.view().active() == Some(&parent) => {
            ask(&parent, action.child, revision, action.kind, cx);
        }
        result => {
            let error = match result {
                Err(Refusal::Failed(error)) => failure(&error),
                _ => failure(&RpcError::Mismatch),
            };
            state.saved.complete(&parent, Some(error));
        }
    }
}

fn restored(state: &mut SideState, parent: ThreadId, action: Action, cx: &mut AppCx<'_>) {
    state.saved.complete(&parent, None);
    let status = state
        .side_of(&parent)
        .and_then(|other| state.link(other))
        .and_then(|link| link.status);
    state.remember(
        action.child.clone(),
        Link {
            parent: parent.clone(),
            storage: Storage::Attached,
            status,
        },
    );
    cx.quiet_interrupts(&action.child, true);
    if action
        .anchor
        .as_ref()
        .is_some_and(|anchor| Some(anchor) == cx.view().active())
        && !cx.view().navigation_pending()
    {
        cx.open_thread(
            &action.child,
            super::Ctx::Reuse {
                parent,
                prompt: None,
            },
        );
    } else {
        cx.chat_facts(super::chat_facts(state, cx.view()));
    }
}

pub(super) fn decode(work: &AttachedHistory, value: Value) -> Result<SavedReply, RpcError> {
    let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
    match work {
        AttachedHistory::Discover => serde_json::from_value(value)
            .map(SavedReply::List)
            .map_err(malformed),
        AttachedHistory::Verify { .. } => serde_json::from_value::<Option<bool>>(value)
            .map(|answer| SavedReply::Verified(answer.unwrap_or(false)))
            .map_err(malformed),
        AttachedHistory::Archive { .. } => serde_json::from_value::<Option<bool>>(value)
            .map(|answer| SavedReply::Archived(answer.unwrap_or(false)))
            .map_err(malformed),
    }
}
