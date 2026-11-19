//! `ThreadModel` and `apply` (ADR-003 R3-R5, R9): one thread's data, changed
//! only here. A thread RPC is requested through `ThreadFx`, which stamps the
//! thread id.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::rc::Rc;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::SortDirection;
use codex_app_server_protocol::ThreadForkParams;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadTurnsListParams;
use codex_app_server_protocol::ThreadUnsubscribeParams;
use codex_app_server_protocol::TurnInterruptParams;
use codex_app_server_protocol::TurnItemsView;
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
use crate::rpc::RpcError;
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

    /// Puts an older page of history before every entry the transcript has
    /// (oldest first); an entry it already holds keeps its place.
    pub fn prepend_page(&mut self, page: Vec<(ItemKey, Item)>) {
        let mut older = Vec::new();
        for (key, item) in page {
            if self.items.insert(key.clone(), Rc::new(item)).is_none() {
                older.push(key);
            }
        }
        older.append(&mut self.order);
        self.order = older;
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
    /// A page of the thread's history (`thread/turns/list`, newest page
    /// first): the entries go before what the transcript has.
    HistoryPage {
        items: Vec<(ItemKey, Item)>,
        next: Option<String>,
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
    /// The server's `error` notification (not one it will retry): why a
    /// turn goes wrong, kept for the status line.
    TurnError {
        message: String,
    },
}

/// Why a turn, a start or the session went wrong. A server's message is
/// shown as it is; every other failure is a typed value the UI's string
/// tables word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Message(String),
    TurnFailed,
    /// The backend dropped this many events (a `Lagged` gap).
    Lagged(usize),
    /// The backend stopped.
    Stopped,
    /// A reply of another screen than its ticket's.
    ReplyMismatch,
}

impl From<RpcError> for Failure {
    fn from(error: RpcError) -> Self {
        match error {
            RpcError::Remote(message) | RpcError::Malformed(message) => Failure::Message(message),
            RpcError::Mismatch => Failure::ReplyMismatch,
        }
    }
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
    pub last_error: Option<Failure>,
    /// The last turn that ended: a late start of it must not revive it.
    pub last_ended: Option<TurnId>,
    /// The cursor of the next older page of history; none once the oldest
    /// page was read.
    pub history_next: Option<String>,
    /// Another process writes this thread: it is shown, not edited.
    pub read_only: bool,
    /// Keeps the struct literal inside this module: a `ThreadModel` is made
    /// only by `ThreadSet::open` (ADR-003).
    seal: Seal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Seal;

/// How a thread's model comes to be held (ADR-003: one model per thread).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A thread this process just started: a fresh model.
    Started,
    /// A thread this process resumed or forked: its model, kept if held.
    Resumed,
    /// A thread another process writes (`is_active_writer`): shown read-only.
    ForeignWriter,
}

/// The models of the threads one process holds, the only place a
/// `ThreadModel` is created (ADR-003). A thread has one model; `open` is the
/// one door.
#[derive(Clone, Debug)]
pub struct ThreadSet {
    held: BTreeMap<ThreadId, Rc<ThreadModel>>,
    /// The projection of "no thread yet": the provider list stays fixed.
    blank: Rc<ThreadModel>,
}

impl Default for ThreadSet {
    fn default() -> Self {
        Self {
            held: BTreeMap::new(),
            blank: Rc::new(ThreadModel::new(ThreadId::default())),
        }
    }
}

impl ThreadSet {
    /// The model of `id`, made when it is not held (`Started` always makes a
    /// fresh one; `ForeignWriter` marks it read-only).
    pub fn open(&mut self, id: &ThreadId, origin: Origin) -> &mut ThreadModel {
        if origin == Origin::Started {
            self.held
                .insert(id.clone(), Rc::new(ThreadModel::new(id.clone())));
        }
        let model = Rc::make_mut(
            self.held
                .entry(id.clone())
                .or_insert_with(|| Rc::new(ThreadModel::new(id.clone()))),
        );
        if origin == Origin::ForeignWriter {
            model.read_only = true;
        }
        model
    }

    pub fn get(&self, id: &ThreadId) -> Option<&Rc<ThreadModel>> {
        self.held.get(id)
    }

    pub fn get_mut(&mut self, id: &ThreadId) -> Option<&mut ThreadModel> {
        self.held.get_mut(id).map(Rc::make_mut)
    }

    pub fn contains(&self, id: &ThreadId) -> bool {
        self.held.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.held.len()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ThreadId, &Rc<ThreadModel>)> {
        self.held.iter()
    }

    /// The thread whose model holds this ticket.
    pub fn owner_of(&self, ticket: Ticket) -> Option<ThreadId> {
        self.held
            .iter()
            .find(|(_, model)| model.outgoing.contains(ticket))
            .map(|(id, _)| id.clone())
    }

    /// The projection of "no thread yet".
    pub fn blank(&self) -> &Rc<ThreadModel> {
        &self.blank
    }

    pub fn blank_mut(&mut self) -> &mut ThreadModel {
        Rc::make_mut(&mut self.blank)
    }
}

impl ThreadModel {
    fn new(id: ThreadId) -> Self {
        Self {
            seal: Seal,
            id,
            transcript: Transcript::default(),
            turn: TurnState::Idle,
            outgoing: Pending::default(),
            last_submit: None,
            queue: Vec::new(),
            slots: ThreadSlots::default(),
            last_error: None,
            last_ended: None,
            history_next: None,
            read_only: false,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.turn, TurnState::Running(_))
    }
}

/// A thread RPC a screen wants (ADR-003): data only. The wire request is made
/// here and nowhere else (`wire`); `apply_app` mints the ticket and sends it,
/// and decides before whether the thread is already held (`OpenThread`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadAsk {
    /// `thread/resume`: the thread into the chat.
    Resume { thread: String },
    /// `thread/fork`.
    Fork { thread: String },
    /// `thread/start` in `cwd` (a new task).
    Start { cwd: Option<String> },
    /// `thread/read` without turns.
    Read { thread: String },
    /// `thread/read` with turns: the turn in progress, half of stopping it.
    ActiveTurn { thread: String },
    /// The newest turn of a thread (`thread/turns/list`, limit 1).
    Turns { thread: String },
    /// One page of a thread's history, newest first.
    History {
        thread: String,
        cursor: Option<String>,
    },
    /// `turn/interrupt`.
    Interrupt { thread: String, turn: String },
    /// `thread/unsubscribe`: a thread started and not used.
    Unsubscribe { thread: String },
}

/// How many turns one page of a thread's history holds.
pub const HISTORY_PAGE: u32 = 20;

impl ThreadAsk {
    /// The thread this asks about, none for a start.
    pub fn thread(&self) -> Option<&str> {
        match self {
            ThreadAsk::Resume { thread }
            | ThreadAsk::Fork { thread }
            | ThreadAsk::Read { thread }
            | ThreadAsk::ActiveTurn { thread }
            | ThreadAsk::Turns { thread }
            | ThreadAsk::History { thread, .. }
            | ThreadAsk::Interrupt { thread, .. }
            | ThreadAsk::Unsubscribe { thread } => Some(thread),
            ThreadAsk::Start { .. } => None,
        }
    }

    /// The wire request (oracle: `agents_overview_actions.rs`).
    pub(crate) fn wire(&self, request_id: RequestId) -> ClientRequest {
        match self {
            ThreadAsk::Resume { thread } => ClientRequest::ThreadResume {
                request_id,
                params: ThreadResumeParams {
                    thread_id: thread.clone(),
                    // The center keeps the thread's row, not its turns, and the
                    // chat reads its transcript page by page: the whole history
                    // over the wire is the cost of opening a long session.
                    exclude_turns: true,
                    ..ThreadResumeParams::default()
                },
            },
            ThreadAsk::Fork { thread } => ClientRequest::ThreadFork {
                request_id,
                params: ThreadForkParams {
                    thread_id: thread.clone(),
                    ..ThreadForkParams::default()
                },
            },
            ThreadAsk::Start { cwd } => ClientRequest::ThreadStart {
                request_id,
                params: ThreadStartParams {
                    cwd: cwd.clone(),
                    ..ThreadStartParams::default()
                },
            },
            ThreadAsk::Read { thread } => ClientRequest::ThreadRead {
                request_id,
                params: ThreadReadParams {
                    thread_id: thread.clone(),
                    include_turns: false,
                },
            },
            ThreadAsk::ActiveTurn { thread } => ClientRequest::ThreadRead {
                request_id,
                params: ThreadReadParams {
                    thread_id: thread.clone(),
                    include_turns: true,
                },
            },
            ThreadAsk::Turns { thread } => ClientRequest::ThreadTurnsList {
                request_id,
                params: ThreadTurnsListParams {
                    thread_id: thread.clone(),
                    cursor: None,
                    limit: Some(1),
                    sort_direction: None,
                    items_view: None,
                },
            },
            ThreadAsk::History { thread, cursor } => ClientRequest::ThreadTurnsList {
                request_id,
                params: ThreadTurnsListParams {
                    thread_id: thread.clone(),
                    cursor: cursor.clone(),
                    limit: Some(HISTORY_PAGE),
                    sort_direction: Some(SortDirection::Desc),
                    items_view: Some(TurnItemsView::Full),
                },
            },
            ThreadAsk::Interrupt { thread, turn } => ClientRequest::TurnInterrupt {
                request_id,
                params: TurnInterruptParams {
                    thread_id: thread.clone(),
                    turn_id: turn.clone(),
                },
            },
            ThreadAsk::Unsubscribe { thread } => ClientRequest::ThreadUnsubscribe {
                request_id,
                params: ThreadUnsubscribeParams {
                    thread_id: thread.clone(),
                },
            },
        }
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
        ServerNotification::Error(n) if !n.will_retry => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnError {
                message: n.error.message.clone(),
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

/// The entries of a page of turns, newest page first as the server sends
/// it, in transcript order (oldest first). Kinds the transcript does not
/// show yet are left out.
pub fn history_items(turns: &[codex_app_server_protocol::Turn]) -> Vec<(ItemKey, Item)> {
    turns
        .iter()
        .rev()
        .flat_map(|turn| {
            turn.items.iter().filter_map(|item| {
                let (id, entry) = item_entry(&item.payload)?;
                Some((
                    ItemKey {
                        turn: TurnId::new(&turn.id),
                        item: ItemId::new(&id),
                    },
                    entry,
                ))
            })
        })
        .collect()
}

fn item_entry(payload: &ThreadItemPayload) -> Option<(String, Item)> {
    Some(match payload {
        ThreadItemPayload::UserMessage { id, content, .. } => {
            let text = content
                .iter()
                .filter_map(|input| match input {
                    UserInput::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            (id.clone(), Item::User { text })
        }
        ThreadItemPayload::AgentMessage { id, text, .. } => {
            (id.clone(), Item::Agent { text: text.clone() })
        }
        _ => return None,
    })
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
        ThreadEv::HistoryPage { items, next } => {
            model.transcript.prepend_page(items);
            model.history_next = next;
        }
        ThreadEv::Submit { text } => {
            // The decision is here (R10): while a turn runs or starts the
            // message is queued, otherwise it starts a turn.
            // An older queued message goes first, whatever the turn state.
            let busy = model.is_running() || model.outgoing.any(is_start_turn);
            if busy || !model.queue.is_empty() {
                model.queue.push(text);
                model.last_submit = Some(fx.mint());
                if !busy {
                    send_next(model, fx);
                }
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
                    // After a `Lagged` gap the end of this turn can have been
                    // seen before this reply: nothing will end it again, so
                    // the next queued message goes out now.
                    let over = model.last_ended.as_ref() == Some(&turn);
                    start_turn(model, turn);
                    if over {
                        send_next(model, fx);
                    }
                }
                (ThreadCtx::StartTurn { .. }, Answer::Failed(error)) => {
                    model.turn = TurnState::Idle;
                    model.last_error = Some(Failure::from(error));
                    send_next(model, fx);
                }
                (ThreadCtx::Screen(ctx), Answer::Screen(reply)) => {
                    screens::apply_reply(&mut model.slots, ctx, Ok(reply), fx);
                }
                (ThreadCtx::Screen(ctx), Answer::Failed(error)) => {
                    screens::apply_reply(&mut model.slots, ctx, Err(error), fx);
                }
                _ => {}
            }
        }
        ThreadEv::Screen(ev) => screens::apply_thread_ev(&mut model.slots, ev, fx),
        ThreadEv::TurnError { message } => model.last_error = Some(Failure::Message(message)),
        ThreadEv::TurnStarted { turn } => start_turn(model, turn),
        ThreadEv::ItemUpsert { key, item } => model.transcript.upsert(key, item),
        ThreadEv::AgentDelta { key, delta } => model.transcript.append_agent(key, &delta),
        ThreadEv::TurnEnded { turn, end } => {
            let ends_running =
                matches!(&model.turn, TurnState::Running(running) if running.turn == turn);
            model.last_ended = Some(turn);
            if ends_running {
                model.turn = TurnState::Idle;
                if end == TurnEnd::Failed && model.last_error.is_none() {
                    model.last_error = Some(Failure::TurnFailed);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Opening a task from the center must not pull its whole history over the
    /// wire (perf-open-session): the resume asks for the thread's state only;
    /// the transcript is read page by page (`thread/turns/list`) when the chat
    /// shows it, as the oracle's paginated sessions do.
    #[test]
    fn resume_asks_for_no_turns() {
        let ask = ThreadAsk::Resume {
            thread: "t1".to_owned(),
        };
        let ClientRequest::ThreadResume { params, .. } = ask.wire(RequestId::Integer(1)) else {
            panic!("resume is thread/resume");
        };
        assert_eq!(params.thread_id, "t1");
        assert!(params.exclude_turns, "thread/resume must exclude the turns");
    }
}
