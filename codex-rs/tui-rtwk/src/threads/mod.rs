//! `ThreadModel` and `apply` (ADR-003 R3-R5, R9): one thread's data, changed
//! only here. A thread RPC is requested through `ThreadFx`, which stamps the
//! thread id.

use std::collections::HashMap;
use std::rc::Rc;

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;

use crate::app::AppFx;
use crate::app::Command;
use crate::ids::ItemId;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TicketMint;
use crate::ids::TurnId;
use crate::rpc::Answer;
use crate::rpc::Pending;
use crate::rpc::Reply;
use crate::screens;
use crate::screens::ScreenCommand;
use crate::screens::ScreenCtx;
use crate::screens::ScreenThreadEv;
use crate::screens::ThreadSlots;

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
    Screen(ScreenCtx),
}

/// An event of one thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    /// The user sent `text`; `apply` refuses it while a turn runs or starts.
    Submit {
        text: String,
    },
    /// An event of a screen slot's per-thread part.
    Screen(ScreenThreadEv),
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
    pub outgoing: Pending<ThreadCtx>,
    /// The ticket of the last submit `apply` accepted: the composer clears
    /// when it changes, never before.
    pub last_submit: Option<Ticket>,
    /// Messages sent while a turn runs: one goes out per turn end, in order.
    pub queue: Vec<String>,
    /// The per-thread parts of the screen slots.
    pub slots: ThreadSlots,
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
            outgoing: Pending::default(),
            last_submit: None,
            queue: Vec::new(),
            slots: ThreadSlots::default(),
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
/// and the ticket stamped (R3). This is the only way a thread RPC is made.
pub struct ThreadFx<'a> {
    thread: &'a ThreadId,
    out: &'a mut AppFx,
    mint: &'a mut TicketMint,
    registered: Vec<(Ticket, ThreadCtx)>,
}

impl<'a> ThreadFx<'a> {
    pub fn new(thread: &'a ThreadId, out: &'a mut AppFx, mint: &'a mut TicketMint) -> Self {
        Self {
            thread,
            out,
            mint,
            registered: Vec::new(),
        }
    }

    /// A fresh ticket, for an acceptance that sends no RPC.
    pub fn mint(&mut self) -> Ticket {
        self.mint.next()
    }

    /// Asks the backend to start a turn with `text`.
    pub fn start_turn(&mut self, text: String) -> Ticket {
        let ticket = self.mint.next();
        self.registered
            .push((ticket, ThreadCtx::StartTurn { text: text.clone() }));
        self.out.commands.push(Command::StartTurn {
            ticket,
            thread: self.thread.clone(),
            text,
        });
        ticket
    }

    /// An RPC of a screen slot: `ctx` is kept with the ticket and comes back
    /// with the reply. (Unreachable while no slot defines a `Ctx` yet.)
    #[allow(unreachable_code, unused_variables)]
    pub fn request(
        &mut self,
        ctx: impl Into<ScreenCtx>,
        command: impl Into<ScreenCommand>,
    ) -> Ticket {
        let ticket = self.mint.next();
        let ctx = ctx.into();
        self.registered.push((ticket, ThreadCtx::Screen(ctx)));
        self.out.commands.push(Command::Screen {
            ticket,
            thread: self.thread.clone(),
            command: command.into(),
        });
        ticket
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
    apply_ev(model, ev, fx);
    for (ticket, ctx) in std::mem::take(&mut fx.registered) {
        model.outgoing.insert(ticket, ctx);
    }
}

fn apply_ev(model: &mut ThreadModel, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    match ev {
        ThreadEv::Submit { text } => {
            // The decision is here (R10): while a turn runs or starts the
            // message is queued, otherwise it starts a turn.
            if model.is_running() || model.outgoing.any(is_start_turn) {
                model.queue.push(text);
                model.last_submit = Some(fx.mint());
                return;
            }
            model.last_error = None;
            model.last_submit = Some(fx.start_turn(text));
        }
        ThreadEv::Reply { ticket, reply } => {
            // A reply whose ticket is not ours any more is stale.
            let Some((ctx, answer)) = model.outgoing.resolve(ticket, reply) else {
                return;
            };
            match (ctx, answer) {
                (ThreadCtx::StartTurn { .. }, Answer::TurnStarted { turn, .. }) => {
                    start_turn(model, turn);
                }
                (ThreadCtx::StartTurn { .. }, Answer::Failed(message)) => {
                    model.turn = TurnState::Idle;
                    model.last_error = Some(message);
                    send_next(model, fx);
                }
                (ThreadCtx::Screen(ctx), Answer::Screen(reply)) => {
                    screens::apply_reply(&mut model.slots, ctx, Ok(reply), fx);
                }
                (ThreadCtx::Screen(ctx), Answer::Failed(message)) => {
                    screens::apply_reply(&mut model.slots, ctx, Err(message), fx);
                }
                _ => {}
            }
        }
        ThreadEv::Screen(ev) => screens::apply_thread_ev(&mut model.slots, ev, fx),
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
                send_next(model, fx);
            }
        }
    }
}

/// The next queued message becomes the next turn (one per turn end).
fn send_next(model: &mut ThreadModel, fx: &mut ThreadFx<'_>) {
    if model.queue.is_empty() {
        return;
    }
    let text = model.queue.remove(0);
    fx.start_turn(text);
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
