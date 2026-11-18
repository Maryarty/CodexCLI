//! `ThreadModel` and `apply` (ADR-003 R3-R5, R9): one thread's data, changed
//! only here. A thread RPC is requested through `ThreadFx`, which stamps the
//! thread id.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::rc::Rc;

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;

use crate::app::AppFx;
use crate::app::Command;
use crate::app::Reply;
use crate::ids::ItemId;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TurnId;

/// The key of a transcript entry (R4).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemKey {
    pub turn: TurnId,
    pub item: ItemId,
}

/// A transcript entry. The skeleton shows plain text only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    User { text: String },
    Agent { text: String },
}

/// The ordered transcript: entries keyed `(TurnId, ItemId)`, kept in the
/// order they first arrived, changed only by upsert (R4). There is no `push`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    order: Vec<ItemKey>,
    items: HashMap<ItemKey, Rc<Item>>,
}

impl Transcript {
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<(&ItemKey, &Item)> {
        let key = self.order.get(index)?;
        self.items.get(key).map(|item| (key, item.as_ref()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ItemKey, &Item)> {
        self.order
            .iter()
            .filter_map(|key| self.items.get(key).map(|item| (key, item.as_ref())))
    }

    /// Inserts or replaces the entry; a replaced entry keeps its place.
    pub fn upsert(&mut self, key: ItemKey, item: Item) {
        if self.items.insert(key.clone(), Rc::new(item)).is_none() {
            self.order.push(key);
        }
    }

    /// Appends `delta` to an agent entry, creating it if the delta came
    /// before the item (R9).
    pub fn append_agent(&mut self, key: ItemKey, delta: &str) {
        match self.items.get_mut(&key).map(Rc::make_mut) {
            Some(Item::Agent { text }) => text.push_str(delta),
            Some(Item::User { .. }) => {}
            None => self.upsert(
                key,
                Item::Agent {
                    text: delta.to_string(),
                },
            ),
        }
    }

    /// Replaces every entry of `turn` with `items`, in their order; a
    /// snapshot wins, it is never merged by id (R4).
    pub fn replace_turn(&mut self, turn: &TurnId, items: Vec<(ItemId, Item)>) {
        self.order.retain(|key| &key.turn != turn);
        self.items.retain(|key, _| &key.turn != turn);
        for (item, entry) in items {
            self.upsert(
                ItemKey {
                    turn: turn.clone(),
                    item,
                },
                entry,
            );
        }
    }
}

/// The turn lifecycle (R5): transient state lives inside the variant, the
/// end transition consumes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnState {
    Idle,
    Running(RunningTurn),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningTurn {
    pub turn: TurnId,
}

/// How a turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnEnd {
    Completed,
    Interrupted,
    Failed,
}

/// What an outgoing thread RPC was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadCtx {
    StartTurn { text: String },
}

/// An event of one thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    /// The user sent `text`; the RPC goes out with `ticket`.
    Submit {
        ticket: Ticket,
        text: String,
    },
    Reply {
        ticket: Ticket,
        reply: Reply,
    },
    TurnStarted {
        turn: TurnId,
    },
    ItemUpsert {
        key: ItemKey,
        item: Item,
    },
    AgentDelta {
        key: ItemKey,
        delta: String,
    },
    TurnEnded {
        turn: TurnId,
        end: TurnEnd,
    },
}

/// One thread's model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadModel {
    pub id: ThreadId,
    pub transcript: Transcript,
    pub turn: TurnState,
    /// Our RPCs of this thread, keyed `(ThreadId, Ticket)` by being in the
    /// thread's model (R3).
    pub outgoing: BTreeMap<Ticket, ThreadCtx>,
    pub last_error: Option<String>,
    /// The last turn that ended: a late start of it must not revive it.
    pub last_ended: Option<TurnId>,
}

impl ThreadModel {
    pub fn new(id: ThreadId) -> Self {
        Self {
            id,
            transcript: Transcript::default(),
            turn: TurnState::Idle,
            outgoing: BTreeMap::new(),
            last_error: None,
            last_ended: None,
        }
    }

    /// The projection of "no thread yet".
    pub fn blank() -> Self {
        Self::new(ThreadId::default())
    }

    pub fn is_running(&self) -> bool {
        matches!(self.turn, TurnState::Running(_))
    }
}

/// The effects of one thread's `apply`: thread RPCs, with the thread id
/// stamped (R3).
pub struct ThreadFx<'a> {
    thread: &'a ThreadId,
    out: &'a mut AppFx,
}

impl<'a> ThreadFx<'a> {
    pub fn new(thread: &'a ThreadId, out: &'a mut AppFx) -> Self {
        Self { thread, out }
    }

    pub fn start_turn(&mut self, ticket: Ticket, text: String) {
        self.out.commands.push(Command::StartTurn {
            ticket,
            thread: self.thread.clone(),
            text,
        });
    }
}

/// The one projection from protocol notifications to thread events (R4).
/// Everything the skeleton does not show maps to `None`.
pub fn project(notification: &ServerNotification) -> Option<(ThreadId, ThreadEv)> {
    match notification {
        ServerNotification::TurnStarted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnStarted {
                turn: TurnId::new(&n.turn.id),
            },
        )),
        ServerNotification::TurnCompleted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnEnded {
                turn: TurnId::new(&n.turn.id),
                end: match n.turn.status {
                    TurnStatus::Interrupted => TurnEnd::Interrupted,
                    TurnStatus::Failed => TurnEnd::Failed,
                    TurnStatus::Completed | TurnStatus::InProgress => TurnEnd::Completed,
                },
            },
        )),
        ServerNotification::ItemStarted(n) => item_upsert(&n.thread_id, &n.turn_id, &n.item),
        ServerNotification::ItemCompleted(n) => item_upsert(&n.thread_id, &n.turn_id, &n.item),
        ServerNotification::AgentMessageDelta(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::AgentDelta {
                key: ItemKey {
                    turn: TurnId::new(&n.turn_id),
                    item: ItemId::new(&n.item_id),
                },
                delta: n.delta.clone(),
            },
        )),
        _ => None,
    }
}

fn item_upsert(thread: &str, turn: &str, item: &ThreadItem) -> Option<(ThreadId, ThreadEv)> {
    let (id, entry) = match &item.payload {
        ThreadItemPayload::UserMessage { id, content, .. } => {
            let text = content
                .iter()
                .filter_map(|input| match input {
                    UserInput::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            (id, Item::User { text })
        }
        ThreadItemPayload::AgentMessage { id, text, .. } => {
            (id, Item::Agent { text: text.clone() })
        }
        _ => return None,
    };
    Some((
        ThreadId::new(thread),
        ThreadEv::ItemUpsert {
            key: ItemKey {
                turn: TurnId::new(turn),
                item: ItemId::new(id),
            },
            item: entry,
        },
    ))
}

/// Applies one event to one thread's model. Events are tolerant of order
/// (R9): a delta may come before its item, a started turn twice.
pub fn apply(model: &mut ThreadModel, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    match ev {
        ThreadEv::Submit { ticket, text } => {
            if model.is_running() || model.outgoing.values().any(is_start_turn) {
                return;
            }
            model.last_error = None;
            model
                .outgoing
                .insert(ticket, ThreadCtx::StartTurn { text: text.clone() });
            fx.start_turn(ticket, text);
        }
        ThreadEv::Reply { ticket, reply } => {
            // A reply whose ticket is not ours any more is stale.
            let Some(ThreadCtx::StartTurn { .. }) = model.outgoing.remove(&ticket) else {
                return;
            };
            match reply {
                Reply::TurnStarted { turn, .. } => start_turn(model, turn),
                Reply::Failed(message) => {
                    model.turn = TurnState::Idle;
                    model.last_error = Some(message);
                }
                Reply::ThreadStarted { .. } => {}
            }
        }
        ThreadEv::TurnStarted { turn } => start_turn(model, turn),
        ThreadEv::ItemUpsert { key, item } => model.transcript.upsert(key, item),
        ThreadEv::AgentDelta { key, delta } => model.transcript.append_agent(key, &delta),
        ThreadEv::TurnEnded { turn, end } => {
            let ends_running =
                matches!(&model.turn, TurnState::Running(running) if running.turn == turn);
            model.last_ended = Some(turn);
            if ends_running {
                model.turn = TurnState::Idle;
                if end == TurnEnd::Failed {
                    model.last_error = Some("the turn failed".to_string());
                }
            }
        }
    }
}

fn is_start_turn(ctx: &ThreadCtx) -> bool {
    matches!(ctx, ThreadCtx::StartTurn { .. })
}

fn start_turn(model: &mut ThreadModel, turn: TurnId) {
    // A turn that already ended is never revived by a late start.
    if model.last_ended.as_ref() == Some(&turn) {
        return;
    }
    match &model.turn {
        TurnState::Idle => model.turn = TurnState::Running(RunningTurn { turn }),
        TurnState::Running(_) => {}
    }
}
