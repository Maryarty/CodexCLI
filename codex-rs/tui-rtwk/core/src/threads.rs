//! `ThreadModel` and `apply` (ADR-003 R3-R5, R9): one thread's data, changed
//! only here. A thread RPC is requested through `ThreadFx`, which stamps the
//! thread id.

mod previews;

pub use previews::{ThreadPreview, ThreadPreviews};

mod activity;

pub use activity::{ToolActivity, ToolKind, ToolStatus};

use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::SortDirection;
use codex_app_server_protocol::ThreadCompactStartParams;
use codex_app_server_protocol::ThreadForkParams;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadSetNameParams;
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
use crate::screens::ScreenThreadIntent;
use crate::screens::ThreadSlots;

/// The key of a transcript entry (R4).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemKey {
    pub turn: TurnId,
    pub item: ItemId,
}

/// A transcript entry keyed by the server item ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    User { text: String },
    Agent { text: String },
    Tool(std::sync::Arc<ToolActivity>),
}

/// The ordered transcript: entries keyed `(TurnId, ItemId)`, kept in the
/// order they first arrived, changed only by upsert (R4). There is no `push`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    order: Rc<Vec<ItemKey>>,
    items: Rc<HashMap<ItemKey, Rc<Item>>>,
    timings: Rc<HashMap<TurnId, TimedTurn>>,
    // Keep streaming rows separate so updating a preview never copies history
    // held by the provider. There is still one ordered index for the transcript.
    streaming: HashMap<ItemKey, StreamingItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StreamingItem {
    item: Rc<Item>,
    committed: usize,
}

/// Timing data is separate from item IDs so a summary cannot collide with a server item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnTiming {
    pub duration_ms: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimedTurn {
    last_item: ItemId,
    timing: TurnTiming,
}

impl TurnTiming {
    fn from_turn(turn: &codex_app_server_protocol::Turn) -> Option<Self> {
        let duration_ms = turn.duration_ms.filter(|&millis| millis >= 0);
        (turn.status == TurnStatus::Completed
            && (duration_ms.is_some() || turn.completed_at.is_some()))
        .then_some(Self {
            duration_ms,
            completed_at: turn.completed_at,
        })
    }
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
        self.item(key).map(|item| (key, item))
    }

    /// A finished turn's timing belongs after its last retained item.
    pub fn timing_after(&self, index: usize) -> Option<TurnTiming> {
        let key = self.order.get(index)?;
        let entry = self.timings.get(&key.turn)?;
        (entry.last_item == key.item).then_some(entry.timing)
    }

    fn remember_timings(&mut self, turns: impl IntoIterator<Item = (TurnId, Option<TurnTiming>)>) {
        let mut pending: HashMap<_, _> = turns
            .into_iter()
            .filter_map(|(turn, update)| {
                let old = self.timings.get(&turn).map(|entry| entry.timing);
                let timing = match (update, old) {
                    (Some(new), Some(old)) => TurnTiming {
                        duration_ms: new.duration_ms.or(old.duration_ms),
                        completed_at: new.completed_at.or(old.completed_at),
                    },
                    (Some(timing), None) | (None, Some(timing)) => timing,
                    (None, None) => return None,
                };
                Some((turn, timing))
            })
            .collect();
        if pending.is_empty() {
            return;
        }
        for key in self.order.iter().rev() {
            if let Some(timing) = pending.remove(&key.turn) {
                let entry = TimedTurn {
                    last_item: key.item.clone(),
                    timing,
                };
                if self.timings.get(&key.turn) != Some(&entry) {
                    Rc::make_mut(&mut self.timings).insert(key.turn.clone(), entry);
                }
            }
            if pending.is_empty() {
                break;
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ItemKey, &Item)> {
        self.order
            .iter()
            .filter_map(|key| self.item(key).map(|item| (key, item)))
    }

    fn item(&self, key: &ItemKey) -> Option<&Item> {
        self.streaming
            .get(key)
            .map(|live| live.item.as_ref())
            .or_else(|| self.items.get(key).map(Rc::as_ref))
    }

    /// Inserts or replaces the entry; a replaced entry keeps its place.
    pub fn upsert(&mut self, key: ItemKey, item: Item) {
        let was_live = self.streaming.remove(&key).is_some();
        let before = Rc::make_mut(&mut self.items).insert(key.clone(), Rc::new(item));
        if !was_live && before.is_none() {
            Rc::make_mut(&mut self.order).push(key);
        }
    }

    /// The source eligible for a stable commit. The raw item retains its
    /// partial line for rtwk's live preview.
    pub fn committed_agent_text(&self, key: &ItemKey) -> Option<&str> {
        let Item::Agent { text } = self.item(key)? else {
            return None;
        };
        Some(
            &text[..self
                .streaming
                .get(key)
                .map_or(text.len(), |live| live.committed)],
        )
    }

    /// Appends a delta in wire order, scanning only its new bytes for a line boundary.
    pub fn append_agent(&mut self, key: ItemKey, delta: &str) {
        if delta.is_empty() {
            return;
        }
        use std::collections::hash_map::Entry;
        let live = match self.streaming.entry(key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let (item, committed) = match self.items.get(entry.key()) {
                    Some(item) => match item.as_ref() {
                        Item::Agent { text } => (item.clone(), text.len()),
                        Item::User { .. } | Item::Tool(_) => return,
                    },
                    None => {
                        Rc::make_mut(&mut self.order).push(entry.key().clone());
                        (
                            Rc::new(Item::Agent {
                                text: String::new(),
                            }),
                            0,
                        )
                    }
                };
                entry.insert(StreamingItem { item, committed })
            }
        };
        if let Item::Agent { text } = Rc::make_mut(&mut live.item) {
            if let Some(last) = delta.rfind('\n') {
                live.committed = text.len() + last + 1;
            }
            text.push_str(delta);
        }
    }

    fn finish_streams(&mut self, turn: &TurnId) {
        for (key, live) in self.streaming.extract_if(|key, _| &key.turn == turn) {
            Rc::make_mut(&mut self.items).insert(key, live.item);
        }
    }

    /// Installs a page from persisted history. Its full turns replace any
    /// live rows for those turns, including rows with synthesized ids.
    fn prepend_snapshots(&mut self, snapshots: Vec<HistoryTurn>) {
        if snapshots.is_empty() {
            return;
        }
        let full: HashSet<_> = snapshots
            .iter()
            .filter_map(|snapshot| {
                matches!(snapshot.items, TurnSnapshot::Full(_)).then_some(&snapshot.turn)
            })
            .collect();
        let stored = Rc::make_mut(&mut self.items);
        let order = Rc::make_mut(&mut self.order);
        order.retain(|key| {
            if full.contains(&key.turn) {
                stored.remove(key);
                false
            } else {
                true
            }
        });
        self.streaming.retain(|key, _| !full.contains(&key.turn));
        let mut older = Vec::new();
        for HistoryTurn { turn, items, .. } in snapshots {
            let items = match items {
                TurnSnapshot::Full(items) | TurnSnapshot::Summary(items) => items,
            };
            for (item, value) in items {
                let key = ItemKey {
                    turn: turn.clone(),
                    item,
                };
                if !self.streaming.contains_key(&key)
                    && let std::collections::hash_map::Entry::Vacant(entry) =
                        stored.entry(key.clone())
                {
                    entry.insert(Rc::new(value));
                    older.push(key);
                }
            }
        }
        if !older.is_empty() {
            older.append(order);
            *order = older;
        }
    }

    /// Replaces every entry of `turn` with `items`, in their order; a
    /// snapshot wins, it is never merged by id (R4).
    pub fn replace_turn(&mut self, turn: &TurnId, items: Vec<(ItemId, Item)>) {
        self.streaming.retain(|key, _| &key.turn != turn);
        let history = Rc::make_mut(&mut self.items);
        let order = Rc::make_mut(&mut self.order);
        let first = order
            .iter()
            .position(|key| &key.turn == turn)
            .unwrap_or(order.len());
        order.retain(|key| {
            if &key.turn == turn {
                history.remove(key);
                false
            } else {
                true
            }
        });
        let mut keys = Vec::with_capacity(items.len());
        for (item, entry) in items {
            let key = ItemKey {
                turn: turn.clone(),
                item,
            };
            if history.insert(key.clone(), Rc::new(entry)).is_none() {
                keys.push(key);
            }
        }
        order.splice(first..first, keys);
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
    pub started_at: Option<i64>,
    interrupt_sent: bool,
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
    StartTurn { interrupt: bool },
    Interrupt { turn: TurnId, retried: bool },
    Screen(ScreenCtx),
}

/// An event of one thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    /// A startup draft was accepted while an attached thread is pending.
    DeferredSubmit,
    ConnectionLost,
    /// An app-scoped error for this thread.
    Failure(Failure),
    /// The user sent `text`; `apply` refuses it while a turn runs or starts.
    Submit {
        text: String,
    },
    Interrupt,
    ScreenIntent(ScreenThreadIntent),
    /// An event of a screen slot's per-thread part.
    Screen(ScreenThreadEv),
    Reply {
        ticket: Ticket,
        reply: Reply,
    },
    /// A page of the thread's history (`thread/turns/list`, newest page
    /// first): the entries go before what the transcript has.
    HistoryPage {
        turns: Vec<HistoryTurn>,
        next: Option<String>,
    },
    TurnStarted {
        turn: TurnId,
        started_at: Option<i64>,
    },
    ItemStarted {
        key: ItemKey,
        item: Item,
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
    TurnCompleted {
        turn: TurnId,
        end: TurnEnd,
        snapshot: Option<TurnSnapshot>,
        timing: Option<TurnTiming>,
        error: Option<String>,
    },
    /// The server's `error` notification (not one it will retry): why a
    /// turn goes wrong, kept for the status line.
    TurnError {
        message: String,
    },
}

/// The payload of a terminal notification may be a full history or a summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnSnapshot {
    Full(Vec<(ItemId, Item)>),
    Summary(Vec<(ItemId, Item)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryTurn {
    turn: TurnId,
    end: Option<TurnEnd>,
    started_at: Option<i64>,
    timing: Option<TurnTiming>,
    error: Option<String>,
    items: TurnSnapshot,
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
    id: ThreadId,
    transcript: Transcript,
    turn: TurnState,
    /// Our RPCs of this thread, keyed `(ThreadId, Ticket)` by being in the
    /// thread's model (R3).
    outgoing: Pending<ThreadCtx>,
    /// The ticket of the last submit `apply` accepted: the composer clears
    /// when it changes, never before.
    last_submit: Option<Ticket>,
    /// Messages sent while a turn runs: one goes out per turn end, in order.
    queue: Vec<String>,
    /// The per-thread parts of the screen slots.
    slots: ThreadSlots,
    last_error: Option<Failure>,
    /// The last turn that ended: a late start of it must not revive it.
    last_ended: Option<TurnId>,
    finished_turns: Rc<HashSet<TurnId>>,
    authoritative_turns: Rc<HashSet<TurnId>>,
    completed_items: HashSet<ItemKey>,
    /// The cursor of the next older page of history; none once the oldest
    /// page was read.
    history_next: Option<String>,
    writer: Writer,
    /// Keeps the struct literal inside this module: a `ThreadModel` is made
    /// only by `ThreadSet::open` (ADR-003).
    seal: Seal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Writer {
    Attached,
    Detached,
    Foreign,
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
    previews: ThreadPreviews,
    /// The projection of "no thread yet": the provider list stays fixed.
    blank: Rc<ThreadModel>,
}

impl Default for ThreadSet {
    fn default() -> Self {
        Self {
            held: BTreeMap::new(),
            previews: ThreadPreviews::default(),
            blank: Rc::new(ThreadModel::new(ThreadId::default())),
        }
    }
}

impl ThreadSet {
    pub(crate) fn previews(&self) -> &ThreadPreviews {
        &self.previews
    }

    pub(crate) fn previews_mut(&mut self) -> &mut ThreadPreviews {
        &mut self.previews
    }

    /// The model of `id`, made when it is not held (`Started` always makes a
    /// fresh one; `ForeignWriter` marks it read-only).
    pub(crate) fn open(&mut self, id: &ThreadId, origin: Origin) -> &mut ThreadModel {
        if origin == Origin::Started {
            self.held
                .insert(id.clone(), Rc::new(ThreadModel::new(id.clone())));
        }
        let model = Rc::make_mut(
            self.held
                .entry(id.clone())
                .or_insert_with(|| Rc::new(ThreadModel::new(id.clone()))),
        );
        model.writer = match origin {
            Origin::ForeignWriter => Writer::Foreign,
            Origin::Started | Origin::Resumed => Writer::Attached,
        };
        model
    }

    pub fn get(&self, id: &ThreadId) -> Option<&Rc<ThreadModel>> {
        self.held.get(id)
    }

    pub(crate) fn get_mut(&mut self, id: &ThreadId) -> Option<&mut ThreadModel> {
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
            finished_turns: Rc::default(),
            authoritative_turns: Rc::default(),
            completed_items: HashSet::new(),
            history_next: None,
            writer: Writer::Detached,
        }
    }

    pub fn id(&self) -> &ThreadId {
        &self.id
    }

    pub(crate) fn history_cursor(&self) -> Option<&str> {
        self.history_next.as_deref()
    }

    pub fn can_load_history(&self) -> bool {
        self.history_next.is_some()
    }

    pub fn transcript(&self) -> &Transcript {
        &self.transcript
    }

    pub fn turn(&self) -> &TurnState {
        &self.turn
    }

    pub fn outgoing(&self) -> &Pending<ThreadCtx> {
        &self.outgoing
    }

    pub fn last_submit(&self) -> Option<Ticket> {
        self.last_submit
    }

    pub fn queue(&self) -> &[String] {
        &self.queue
    }

    pub fn screen_slots(&self) -> &ThreadSlots {
        &self.slots
    }

    pub fn last_error(&self) -> Option<&Failure> {
        self.last_error.as_ref()
    }

    pub fn last_ended(&self) -> Option<&TurnId> {
        self.last_ended.as_ref()
    }

    pub fn read_only(&self) -> bool {
        self.writer == Writer::Foreign
    }

    pub(crate) fn has_writer(&self) -> bool {
        self.writer == Writer::Attached
    }

    pub fn can_interrupt(&self) -> bool {
        self.has_writer() && (self.is_running() || self.outgoing.any(is_start_turn))
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
    Resume {
        thread: String,
    },
    /// `thread/fork`.
    Fork {
        thread: String,
    },
    /// `thread/start` in `cwd` (a new task).
    Start {
        cwd: Option<String>,
    },
    /// `thread/read` without turns.
    Read {
        thread: String,
    },
    /// `thread/read` with turns: the turn in progress, half of stopping it.
    ActiveTurn {
        thread: String,
    },
    /// The newest turn of a thread (`thread/turns/list`, limit 1).
    Turns {
        thread: String,
    },
    /// One page of a thread's history, newest first.
    History {
        thread: String,
        cursor: Option<String>,
    },
    /// `turn/interrupt`.
    Interrupt {
        thread: String,
        turn: String,
    },
    Compact {
        thread: String,
    },
    Rename {
        thread: String,
        name: String,
    },
    /// `thread/unsubscribe`: a thread started and not used.
    Unsubscribe {
        thread: String,
    },
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
            | ThreadAsk::Compact { thread }
            | ThreadAsk::Rename { thread, .. }
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
            ThreadAsk::Compact { thread } => ClientRequest::ThreadCompactStart {
                request_id,
                params: ThreadCompactStartParams {
                    thread_id: thread.clone(),
                },
            },
            ThreadAsk::Rename { thread, name } => ClientRequest::ThreadSetName {
                request_id,
                params: ThreadSetNameParams {
                    thread_id: thread.clone(),
                    name: name.clone(),
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
    pub(crate) fn new(thread: &'a ThreadId, out: &'a mut AppFx, mint: &'a mut TicketMint) -> Self {
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
            .push((ticket, ThreadCtx::StartTurn { interrupt: false }));
        self.out.commands.push(Command::StartTurn {
            ticket,
            thread: self.thread.clone(),
            text,
        });
        ticket
    }

    fn interrupt(&mut self, turn: TurnId, retried: bool) {
        let ticket = self.mint.next();
        self.registered.push((
            ticket,
            ThreadCtx::Interrupt {
                turn: turn.clone(),
                retried,
            },
        ));
        self.out.commands.push(Command::InterruptTurn {
            ticket,
            thread: self.thread.clone(),
            turn,
        });
    }

    /// An RPC of a screen slot: `ctx` is kept with the ticket and comes back
    /// with the reply.
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
    let transcript = match notification {
        ServerNotification::TurnStarted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnStarted {
                turn: TurnId::new(&n.turn.id),
                started_at: n.turn.started_at,
            },
        )),
        ServerNotification::TurnCompleted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnCompleted {
                turn: TurnId::new(&n.turn.id),
                end: match n.turn.status {
                    TurnStatus::Interrupted => TurnEnd::Interrupted,
                    TurnStatus::Failed => TurnEnd::Failed,
                    TurnStatus::Completed | TurnStatus::InProgress => TurnEnd::Completed,
                },
                snapshot: turn_snapshot(&n.turn),
                timing: TurnTiming::from_turn(&n.turn),
                error: n.turn.error.as_ref().map(|error| error.message.clone()),
            },
        )),
        ServerNotification::Error(n) if !n.will_retry => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnError {
                message: n.error.message.clone(),
            },
        )),
        ServerNotification::ItemStarted(n) => {
            item_upsert(&n.thread_id, &n.turn_id, &n.item).map(|(thread, ev)| match ev {
                ThreadEv::ItemUpsert { key, item } => (thread, ThreadEv::ItemStarted { key, item }),
                _ => (thread, ev),
            })
        }
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
    };
    transcript.or_else(|| {
        screens::project_thread(notification).map(|(thread, ev)| (thread, ThreadEv::Screen(ev)))
    })
}

fn turn_end(status: &TurnStatus) -> Option<TurnEnd> {
    match status {
        TurnStatus::InProgress => None,
        TurnStatus::Interrupted => Some(TurnEnd::Interrupted),
        TurnStatus::Failed => Some(TurnEnd::Failed),
        TurnStatus::Completed => Some(TurnEnd::Completed),
    }
}

fn turn_snapshot(turn: &codex_app_server_protocol::Turn) -> Option<TurnSnapshot> {
    let items = || turn.item_payloads().filter_map(item_entry).collect();
    match turn.items_view {
        TurnItemsView::Full => Some(TurnSnapshot::Full(items())),
        TurnItemsView::Summary => Some(TurnSnapshot::Summary(items())),
        TurnItemsView::NotLoaded => None,
    }
}

/// The server lists newest turns first. Project them in transcript order.
pub fn history_page(turns: &[codex_app_server_protocol::Turn], next: Option<String>) -> ThreadEv {
    let turns = turns
        .iter()
        .rev()
        .filter_map(|turn| {
            Some(HistoryTurn {
                turn: TurnId::new(&turn.id),
                end: turn_end(&turn.status),
                started_at: turn.started_at,
                timing: TurnTiming::from_turn(turn),
                error: turn.error.as_ref().map(|error| error.message.clone()),
                items: turn_snapshot(turn)?,
            })
        })
        .collect();
    ThreadEv::HistoryPage { turns, next }
}

fn item_entry(payload: &ThreadItemPayload) -> Option<(ItemId, Item)> {
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
            (ItemId::new(id), Item::User { text })
        }
        ThreadItemPayload::AgentMessage { id, text, .. } => {
            (ItemId::new(id), Item::Agent { text: text.clone() })
        }
        _ => {
            return activity::tool(payload)
                .map(|(id, tool)| (id, Item::Tool(std::sync::Arc::new(tool))));
        }
    })
}

fn item_upsert(thread: &str, turn: &str, item: &ThreadItem) -> Option<(ThreadId, ThreadEv)> {
    let (id, entry) = item_entry(&item.payload)?;
    Some((
        ThreadId::new(thread),
        ThreadEv::ItemUpsert {
            key: ItemKey {
                turn: TurnId::new(turn),
                item: id,
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
        ThreadEv::DeferredSubmit => model.last_submit = Some(fx.mint()),
        ThreadEv::ConnectionLost if model.has_writer() => {
            model.writer = Writer::Detached;
            model.outgoing = Pending::default();
            model.turn = TurnState::Idle;
            model.slots.chat.connection_lost();
        }
        ThreadEv::ConnectionLost => {}
        ThreadEv::Failure(error) => model.last_error = Some(error),
        ThreadEv::HistoryPage { turns, next } => {
            for snapshot in &turns {
                if let Some(end) = snapshot.end {
                    if matches!(&model.turn, TurnState::Running(running) if running.turn == snapshot.turn)
                    {
                        end_turn(
                            model,
                            snapshot.turn.clone(),
                            end,
                            snapshot.error.clone(),
                            fx,
                        );
                    } else {
                        Rc::make_mut(&mut model.finished_turns).insert(snapshot.turn.clone());
                    }
                    if matches!(snapshot.items, TurnSnapshot::Full(_)) {
                        Rc::make_mut(&mut model.authoritative_turns).insert(snapshot.turn.clone());
                    }
                } else {
                    start_turn(model, snapshot.turn.clone());
                    if let TurnState::Running(running) = &mut model.turn
                        && running.turn == snapshot.turn
                        && running.started_at.is_none()
                    {
                        running.started_at = snapshot.started_at;
                    }
                }
            }
            let timings: Vec<_> = turns
                .iter()
                .map(|entry| (entry.turn.clone(), entry.timing))
                .collect();
            model.transcript.prepend_snapshots(turns);
            model.transcript.remember_timings(timings);
            model.history_next = next.filter(|cursor| Some(cursor) != model.history_next.as_ref());
        }
        ThreadEv::Submit { text } if model.has_writer() => {
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
        ThreadEv::Submit { .. } => {}
        ThreadEv::Interrupt if model.can_interrupt() => {
            if let TurnState::Running(running) = &mut model.turn {
                if !running.interrupt_sent {
                    running.interrupt_sent = true;
                    fx.interrupt(running.turn.clone(), false);
                }
            } else if let Some(ThreadCtx::StartTurn { interrupt }) =
                model.outgoing.find_mut(is_start_turn)
            {
                *interrupt = true;
            }
        }
        ThreadEv::Interrupt => {}
        ThreadEv::Reply { ticket, reply } => {
            // A reply whose ticket is not ours any more is stale.
            let Some((ctx, answer)) = model.outgoing.resolve(ticket, reply) else {
                return;
            };
            match (ctx, answer) {
                (ThreadCtx::StartTurn { interrupt }, Answer::TurnStarted { turn, .. }) => {
                    // After a `Lagged` gap the end of this turn can have been
                    // seen before this reply: nothing will end it again, so
                    // the next queued message goes out now.
                    let over = model.finished_turns.contains(&turn);
                    start_turn(model, turn);
                    if over {
                        send_next(model, fx);
                    } else if interrupt {
                        apply_ev(model, ThreadEv::Interrupt, fx);
                    }
                }
                (ThreadCtx::StartTurn { .. }, Answer::Failed(error)) => {
                    if !model.is_running() {
                        model.last_error = Some(Failure::from(error));
                        send_next(model, fx);
                    }
                }
                (
                    ThreadCtx::Interrupt {
                        turn,
                        retried: false,
                    },
                    Answer::InterruptTurnChanged(actual),
                ) => {
                    if let TurnState::Running(running) = &mut model.turn
                        && running.turn == turn
                        && !model.finished_turns.contains(&actual)
                    {
                        running.turn = actual.clone();
                        fx.interrupt(actual, true);
                    }
                }
                (ThreadCtx::Interrupt { turn, .. }, Answer::InterruptTurnChanged(_)) => {
                    if let TurnState::Running(running) = &mut model.turn
                        && running.turn == turn
                    {
                        running.interrupt_sent = false;
                        model.last_error = Some(Failure::ReplyMismatch);
                    }
                }
                (ThreadCtx::Interrupt { turn, .. }, Answer::Failed(error)) => {
                    if let TurnState::Running(running) = &mut model.turn
                        && running.turn == turn
                    {
                        running.interrupt_sent = false;
                        model.last_error = Some(Failure::from(error));
                    }
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
        ThreadEv::ScreenIntent(intent) if model.has_writer() => {
            let busy = model.is_running() || model.outgoing.any(is_start_turn);
            screens::apply_thread_intent(&mut model.slots, intent, busy, fx);
        }
        ThreadEv::ScreenIntent(_) => {}
        ThreadEv::Screen(ev) => {
            if let ScreenThreadEv::Chat(ref ev) = ev
                && let Some(turn) = ev.turn()
                && !matches!(&model.turn, TurnState::Running(running) if &running.turn == turn)
            {
                return;
            }
            screens::apply_thread_ev(&mut model.slots, ev, fx);
        }
        ThreadEv::TurnError { message } => model.last_error = Some(Failure::Message(message)),
        ThreadEv::TurnStarted { turn, started_at } => {
            start_turn(model, turn.clone());
            if let TurnState::Running(running) = &mut model.turn
                && running.turn == turn
                && started_at.is_some()
            {
                running.started_at = started_at;
            }
            if model
                .outgoing
                .any(|ctx| matches!(ctx, ThreadCtx::StartTurn { interrupt: true }))
            {
                apply_ev(model, ThreadEv::Interrupt, fx);
            }
        }
        ThreadEv::ItemStarted { key, item } => {
            if !model.finished_turns.contains(&key.turn) && model.transcript.item(&key).is_none() {
                model.transcript.upsert(key, item);
            }
        }
        ThreadEv::ItemUpsert { key, item } => {
            if !model.authoritative_turns.contains(&key.turn) {
                if !model.finished_turns.contains(&key.turn) && matches!(item, Item::Agent { .. }) {
                    model.completed_items.insert(key.clone());
                }
                model.transcript.upsert(key, item);
            }
        }
        ThreadEv::AgentDelta { key, delta } => {
            if !model.finished_turns.contains(&key.turn) && !model.completed_items.contains(&key) {
                model.transcript.append_agent(key, &delta);
            }
        }
        ThreadEv::TurnEnded { turn, end } => end_turn(model, turn, end, None, fx),
        ThreadEv::TurnCompleted {
            turn,
            end,
            snapshot,
            timing,
            error,
        } => {
            match snapshot {
                Some(TurnSnapshot::Full(items)) => {
                    Rc::make_mut(&mut model.authoritative_turns).insert(turn.clone());
                    model.transcript.replace_turn(&turn, items);
                }
                Some(TurnSnapshot::Summary(items))
                    if !model.authoritative_turns.contains(&turn) =>
                {
                    for (item, entry) in items {
                        model.transcript.upsert(
                            ItemKey {
                                turn: turn.clone(),
                                item,
                            },
                            entry,
                        );
                    }
                }
                _ => {}
            }
            model.transcript.remember_timings([(turn.clone(), timing)]);
            end_turn(model, turn, end, error, fx);
        }
    }
}

fn end_turn(
    model: &mut ThreadModel,
    turn: TurnId,
    end: TurnEnd,
    error: Option<String>,
    fx: &mut ThreadFx<'_>,
) {
    Rc::make_mut(&mut model.finished_turns).insert(turn.clone());
    model.transcript.finish_streams(&turn);
    model.completed_items.retain(|key| key.turn != turn);
    let ends_running = matches!(&model.turn, TurnState::Running(running) if running.turn == turn);
    if ends_running || !model.is_running() {
        model.last_ended = Some(turn.clone());
    }
    model
        .outgoing
        .retain(|ctx| !matches!(ctx, ThreadCtx::Interrupt { turn: target, .. } if *target == turn));
    if ends_running {
        model.turn = TurnState::Idle;
        model.slots.chat.end_turn();
        if let Some(message) = error {
            model.last_error = Some(Failure::Message(message));
        } else if end == TurnEnd::Failed && model.last_error.is_none() {
            model.last_error = Some(Failure::TurnFailed);
        }
        if !model.outgoing.any(is_start_turn) {
            send_next(model, fx);
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
    if model.finished_turns.contains(&turn) {
        return;
    }
    match &model.turn {
        TurnState::Idle => {
            model.slots.chat.begin_turn();
            model.turn = TurnState::Running(RunningTurn {
                turn,
                started_at: None,
                interrupt_sent: false,
            })
        }
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
