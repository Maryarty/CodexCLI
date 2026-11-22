//! `ThreadModel` and `apply` (ADR-003 R3-R5, R9): one thread's data, changed
//! only here. A thread RPC is requested through `ThreadFx`, which stamps the
//! thread id.

mod previews;

pub use previews::{ThreadPreview, ThreadPreviews};

mod activity;
mod cells;
pub mod citations;
mod collab;
pub mod directives;
pub mod followups;
pub mod inline_vis;
mod message;
mod note;
pub mod stream_preview;
mod user_prompt;

pub use activity::{ExplorationAction, ToolActivity, ToolKind, ToolMedia, ToolStatus};
pub use cells::{
    DelegatedMessage, GeneratedImage, HookEntry, HookEntryKind, HookRun, HookStatus, Notice,
    PatchActivity, PatchChange, PatchKind, PlanStep, PlanUpdate, StepStatus, WebAction,
    WebActivity,
};
pub use collab::{AgentOutcome, AgentRef, CollabEvent, SpawnRequest, SubAgentKind};
pub use message::{LocalImage, UserMessage};
pub use note::{Note, NoteKey, Stub, Tone};
pub use user_prompt::UserPrompt;

use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::SortDirection;
use codex_app_server_protocol::ThreadBackgroundTerminalsCleanParams;
use codex_app_server_protocol::ThreadCompactStartParams;
use codex_app_server_protocol::ThreadForkParams;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadRevertParams;
use codex_app_server_protocol::ThreadSetNameParams;
use codex_app_server_protocol::ThreadShellCommandParams;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadTurnsListParams;
use codex_app_server_protocol::ThreadUnsubscribeParams;
use codex_app_server_protocol::TurnInterruptParams;
use codex_app_server_protocol::TurnItemsView;
use codex_app_server_protocol::TurnStatus;

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
    User(UserPrompt),
    Agent {
        text: String,
    },
    Tool(Arc<ToolActivity>),
    /// The model's reasoning summary; shown only in the detailed transcript
    /// (oracle `ReasoningSummaryCell::transcript_only`).
    Reasoning {
        summary: String,
    },
    Patch(Arc<PatchActivity>),
    Plan {
        text: String,
    },
    Web(WebActivity),
    /// A viewed image: its path and the file name the main view shows.
    ImageView {
        path: String,
        name: String,
    },
    Notice(Notice),
    /// What a command line answered with (oracle `new_info_event` /
    /// `new_error_event`): a local entry the session leaves in the thread.
    Note(Note),
    /// An `update_plan` report: the server has no item for it, so the
    /// transcript mints its key (`Transcript::mint`).
    PlanUpdate(Arc<PlanUpdate>),
    /// A finished hook run with something to show.
    Hook(Arc<HookRun>),
    /// A multi-agent call or a sub-agent's activity.
    Collab(Arc<CollabEvent>),
    /// A message another task delegated to this thread.
    Delegated(Arc<DelegatedMessage>),
    /// A finished image generation call.
    Generated(Arc<GeneratedImage>),
    /// Why a turn went wrong (oracle `new_error_event`).
    Error {
        message: String,
    },
    /// A warning of the server; shown only in the detailed transcript
    /// (oracle `new_warning_event`: `visible_in_transcript: false`).
    Warning {
        message: String,
    },
    /// An overloaded server that said nothing (oracle `on_server_overloaded_error`):
    /// the error row the UI words as high load.
    HighLoad,
    /// A startup warning (an MCP server failed, a round settled incomplete):
    /// a `⚠` row of the main transcript (oracle `StartupWarningsCell`).
    StartupWarning(screens::chat::McpWarning),
    /// A deprecation notice; shown only in the detailed transcript (oracle
    /// `DeprecationNoticeCell`: no display lines, a `⚠ ` summary and details).
    Deprecation {
        summary: String,
        details: Option<String>,
    },
}

impl Item {
    /// Entries only the detailed transcript (Ctrl+T) shows: the reasoning
    /// summary, the server's warnings and its deprecation notices.
    pub fn is_detailed_only(&self) -> bool {
        matches!(
            self,
            Self::Reasoning { .. } | Self::Warning { .. } | Self::Deprecation { .. }
        )
    }

    /// An exploration can share one visible section with its neighbors.
    pub fn is_exploration(&self) -> bool {
        matches!(self, Self::Tool(tool) if matches!(
            &tool.kind, ToolKind::Shell { actions, .. } if !actions.is_empty()
        ))
    }
}

/// The ordered transcript: entries keyed `(TurnId, ItemId)`, kept in the
/// order they first arrived, changed only by upsert (R4). There is no `push`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    order: Rc<Vec<ItemKey>>,
    items: Rc<HashMap<ItemKey, Rc<Item>>>,
    layout_revision: u64,
    timings: Rc<HashMap<TurnId, TimedTurn>>,
    // Keep streaming rows separate so updating a preview never copies history
    // held by the provider. There is still one ordered index for the transcript.
    streaming: HashMap<ItemKey, StreamingItem>,
    /// Entries the transcript keyed itself (`mint`): the server sent no item.
    minted: u64,
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
    /// Changes when the ordered keys or their exploration classification change.
    /// A streamed text or command output delta leaves the layout index valid.
    pub fn layout_revision(&self) -> u64 {
        self.layout_revision
    }

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

    /// Inserts or replaces the entry; a replaced entry keeps its place. A web
    /// search that completes after its start does not replace it: the oracle's
    /// `item/completed` runs `on_web_search_begin`, which commits the cell
    /// `item/started` left (`• Browsing the web`), and then the end commits the
    /// completed one, so the completed search is an entry after the started one.
    pub fn upsert(&mut self, mut key: ItemKey, mut item: Item) {
        if matches!(&item, Item::Web(web) if web.completed)
            && matches!(self.item(&key), Some(Item::Web(started)) if !started.completed)
        {
            // The second entry of the item: its id and `#2`.
            key.item = ItemId::new(format!("{}#2", key.item.as_str()));
        }
        if self.item(&key).map(Item::is_exploration) != Some(item.is_exploration()) {
            self.layout_revision += 1;
        }
        if let Item::Tool(tool) = &mut item
            && tool.output.is_none()
            && let Some(Item::Tool(previous)) = self.item(&key)
            && let Some(output) = &previous.output
        {
            Arc::make_mut(tool).output = Some(output.clone());
        }
        let was_live = self.streaming.remove(&key).is_some();
        let before = Rc::make_mut(&mut self.items).insert(key.clone(), Rc::new(item));
        if !was_live && before.is_none() {
            Rc::make_mut(&mut self.order).push(key);
        }
    }

    /// The turn of the last entry.
    pub fn last_turn(&self) -> Option<&TurnId> {
        self.order.last().map(|key| &key.turn)
    }

    /// The key of an entry the server has no item for (a plan update): the
    /// next number, which no server item id (`item-…`, a UUID) can equal.
    fn mint(&mut self, turn: TurnId) -> ItemKey {
        self.minted += 1;
        ItemKey {
            turn,
            item: ItemId::new(format!("#{}", self.minted)),
        }
    }

    /// An entry the server has no item for carries a key `mint` made.
    fn is_minted(key: &ItemKey) -> bool {
        key.item.as_str().starts_with('#')
    }

    /// Whether `turn` already shows this error (oracle `last_non_retry_error`: the
    /// turn's own report of an error the `error` notification gave ends quietly).
    fn holds_error(&self, turn: &TurnId, fault: &Fault) -> bool {
        self.order
            .iter()
            .any(|key| &key.turn == turn && self.item(key).is_some_and(|held| fault.is(held)))
    }

    /// The error row of a turn, once per message.
    fn report_error(&mut self, turn: &TurnId, fault: &Fault) {
        if !self.holds_error(turn, fault) {
            let key = self.mint(turn.clone());
            self.upsert(key, fault.item());
        }
    }

    /// A partial answer still receiving text deltas.
    pub fn is_agent_streaming(&self, key: &ItemKey) -> bool {
        self.streaming.contains_key(key)
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
                        _ => return,
                    },
                    None => {
                        Rc::make_mut(&mut self.order).push(entry.key().clone());
                        self.layout_revision += 1;
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

    fn append_command_output(&mut self, key: ItemKey, delta: &str) {
        if delta.is_empty() {
            return;
        }
        use std::collections::hash_map::Entry;
        let live = match self.streaming.entry(key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let Some(item) = self.items.get(entry.key()) else {
                    return;
                };
                if !matches!(item.as_ref(), Item::Tool(tool) if tool.status == ToolStatus::Running)
                {
                    return;
                }
                entry.insert(StreamingItem {
                    item: item.clone(),
                    committed: 0,
                })
            }
        };
        if let Item::Tool(tool) = Rc::make_mut(&mut live.item)
            && tool.status == ToolStatus::Running
        {
            Arc::make_mut(tool)
                .output
                .get_or_insert_with(String::new)
                .push_str(delta);
        }
    }

    fn finish_streams(&mut self, turn: &TurnId) {
        for (key, live) in self.streaming.extract_if(|key, _| &key.turn == turn) {
            Rc::make_mut(&mut self.items).insert(key, live.item);
        }
    }

    /// The server's history became shorter (`thread/revert`): every row and
    /// every timing is dropped; the retained turns are read again.
    fn reset(&mut self) {
        *self = Transcript {
            layout_revision: self.layout_revision + 1,
            ..Transcript::default()
        };
    }

    /// Installs a page from persisted history. Its full turns replace any
    /// live rows for those turns, including rows with synthesized ids.
    fn prepend_snapshots(&mut self, snapshots: Vec<HistoryTurn>) {
        if snapshots.is_empty() {
            return;
        }
        self.layout_revision += 1;
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
        for HistoryTurn {
            turn, items, error, ..
        } in snapshots
        {
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
            // The turn's own error follows its items (oracle `thread_transcript`).
            if let Some(fault) = error
                && !order.iter().chain(older.iter()).any(|key| {
                    key.turn == turn && stored.get(key).is_some_and(|held| fault.is(held))
                })
            {
                self.minted += 1;
                let key = ItemKey {
                    turn: turn.clone(),
                    item: ItemId::new(format!("#{}", self.minted)),
                };
                stored.insert(key.clone(), Rc::new(fault.item()));
                older.push(key);
            }
        }
        if !older.is_empty() {
            older.append(order);
            *order = older;
        }
    }

    /// Replaces every server item of `turn` with `items`, in their order; a
    /// snapshot wins, it is never merged by id (R4).
    pub fn replace_turn(&mut self, turn: &TurnId, items: Vec<(ItemId, Item)>) {
        self.layout_revision += 1;
        self.streaming.retain(|key, _| &key.turn != turn);
        let history = Rc::make_mut(&mut self.items);
        let order = Rc::make_mut(&mut self.order);
        let first = order
            .iter()
            .position(|key| &key.turn == turn)
            .unwrap_or(order.len());
        // The server's items only: an entry it has no item for (an error, a hook run) is ours.
        order.retain(|key| {
            if &key.turn == turn && !Self::is_minted(key) {
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

/// A message sent to the running turn (`turn/steer`) that the server has not
/// yet echoed as the user's item (the oracle's `PendingSteer`): the pending-
/// input preview lists it until then.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingSteer {
    message: UserMessage,
    /// The turn the server said it joined.
    admitted: Option<TurnId>,
}

impl PendingSteer {
    pub fn message(&self) -> &UserMessage {
        &self.message
    }
}

/// Messages the model gives back to the composer (the oracle's
/// `restore_composer_state`): after an interrupt, a failed steer, or the
/// edit-queued key. The composer's controller applies each `seq` once; the
/// fact is the model's, the draft is the UI's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Restore {
    seq: u64,
    messages: Vec<UserMessage>,
    replace: bool,
}

impl Restore {
    /// Grows with every restore: the composer applies a number once.
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// In the order they were sent: rejected steers, pending steers, queued.
    pub fn messages(&self) -> &[UserMessage] {
        &self.messages
    }

    /// Whether the composer's draft is replaced (edit-queued) or the messages
    /// go before it (an interrupt or a failure).
    pub fn replaces(&self) -> bool {
        self.replace
    }

    /// The messages as one, joined by newlines, image placeholders renumbered.
    pub fn merged(&self) -> UserMessage {
        merge_messages(self.messages.clone())
    }

    /// What the composer holds after the restore, given the `draft` it holds
    /// now: the messages first, the draft after them (`drain_pending_
    /// messages_for_restore`); an edit-queued restore replaces the draft.
    pub fn onto(&self, draft: UserMessage) -> UserMessage {
        let mut messages = self.messages.clone();
        if !self.replace && !draft.is_empty() {
            messages.push(draft);
        }
        merge_messages(messages)
    }
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
    /// The user sent `message`; `apply` refuses it while a turn runs or starts.
    Submit {
        message: UserMessage,
    },
    Interrupt,
    /// `thread/reverted`: the server's history is now the prefix before a
    /// turn (the edit of a past prompt). Nothing held of the old history is
    /// trusted: the transcript is cleared and the app reads the retained
    /// turns again (oracle `AppEvent::PromptRevert`, `set_session`).
    Reverted,
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
    /// An `update_plan` report: a new entry of the turn, whatever came before.
    PlanUpdated {
        turn: TurnId,
        update: Arc<PlanUpdate>,
    },
    /// An entry the server has no item for (a hook run): after the last row,
    /// in the turn it names, else the running turn, else the last row's.
    Appended {
        turn: Option<TurnId>,
        item: Item,
    },
    AgentDelta {
        key: ItemKey,
        delta: String,
    },
    CommandOutputDelta {
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
        error: Option<Fault>,
    },
    /// The server's `error` notification (not one it will retry): why a
    /// turn goes wrong, a row of the turn.
    TurnError {
        turn: TurnId,
        fault: Fault,
    },
    /// An MCP server's start or outcome (`mcpServer/startupStatus/updated`).
    Mcp {
        server: String,
        status: screens::chat::McpStatus,
    },
    /// The app server's event stream lagged (`BackendEv::Lagged`).
    McpLagged,
}

impl ThreadEv {
    /// The thread's server notifications: any of them but a retrying error
    /// ends the retry header (oracle `restore_retry_status_header_if_present`).
    fn is_notification(&self) -> bool {
        match self {
            Self::Screen(ScreenThreadEv::Chat(screens::chat::ThreadEv::Retry { .. })) => false,
            Self::TurnStarted { .. }
            | Self::ItemStarted { .. }
            | Self::ItemUpsert { .. }
            | Self::PlanUpdated { .. }
            | Self::Appended { .. }
            | Self::AgentDelta { .. }
            | Self::CommandOutputDelta { .. }
            | Self::TurnEnded { .. }
            | Self::TurnCompleted { .. }
            | Self::TurnError { .. }
            | Self::Mcp { .. }
            | Self::Screen(_) => true,
            _ => false,
        }
    }
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
    error: Option<Fault>,
    items: TurnSnapshot,
}

/// Why a turn, a start or the session went wrong. A server's message is
/// shown as it is; every other failure is a typed value the UI's string
/// tables word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Message(String),
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
    queue: Vec<UserMessage>,
    /// Steers sent to the running turn and not yet echoed, oldest first.
    pending_steers: Vec<PendingSteer>,
    /// Steers the server refused (a review or compact turn): they go out first
    /// when the turn ends, merged into one message.
    rejected_steers: Vec<UserMessage>,
    /// Esc with steers pending: the interrupt that follows sends them as one
    /// message (`submit_pending_steers_after_interrupt`).
    steers_after_interrupt: bool,
    /// The last messages given back to the composer.
    restore: Option<Restore>,
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
            pending_steers: Vec::new(),
            rejected_steers: Vec::new(),
            steers_after_interrupt: false,
            restore: None,
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

    pub fn queue(&self) -> &[UserMessage] {
        &self.queue
    }

    pub fn pending_steers(&self) -> &[PendingSteer] {
        &self.pending_steers
    }

    pub fn rejected_steers(&self) -> &[UserMessage] {
        &self.rejected_steers
    }

    pub fn restore(&self) -> Option<&Restore> {
        self.restore.as_ref()
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
    /// `thread/resume` in the directory the user chose at launch (`codex
    /// resume <id>`, the oracle's `config.cwd` after the cwd prompt); `None`
    /// is the session's own.
    ResumeAt {
        thread: String,
        cwd: Option<String>,
    },
    /// `thread/fork` in the directory the user chose at launch (`codex fork
    /// <id>`).
    ForkAt {
        thread: String,
        cwd: Option<String>,
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
    /// `thread/read` with turns: the persisted history of a thread. The edit
    /// of a past prompt (the transcript's backtrack) resolves the prompt and
    /// the turn to revert before from it; the caller decodes the response.
    ReadTurns {
        thread: String,
    },
    /// `thread/revert`: drop `before_turn` and every later turn from the
    /// thread's persisted history; the server answers with `ThreadReverted`.
    Revert {
        thread: String,
        before_turn: TurnId,
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
    /// `turn/steer`: `message` joins the running turn `turn`.
    Steer {
        thread: String,
        turn: String,
        message: UserMessage,
    },
    Compact {
        thread: String,
    },
    /// `thread/backgroundTerminals/clean`: stop the thread's background
    /// terminals (`/stop`).
    CleanTerminals {
        thread: String,
    },
    /// `thread/shellCommand`: a user shell command (`!cmd` in the composer),
    /// run by the thread's shell and shown as its own transcript item.
    Shell {
        thread: String,
        command: String,
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
            | ThreadAsk::ResumeAt { thread, .. }
            | ThreadAsk::Fork { thread }
            | ThreadAsk::ForkAt { thread, .. }
            | ThreadAsk::Read { thread }
            | ThreadAsk::ActiveTurn { thread }
            | ThreadAsk::ReadTurns { thread }
            | ThreadAsk::Revert { thread, .. }
            | ThreadAsk::Turns { thread }
            | ThreadAsk::History { thread, .. }
            | ThreadAsk::Interrupt { thread, .. }
            | ThreadAsk::Steer { thread, .. }
            | ThreadAsk::Compact { thread }
            | ThreadAsk::CleanTerminals { thread }
            | ThreadAsk::Shell { thread, .. }
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
            ThreadAsk::ResumeAt { thread, cwd } => ClientRequest::ThreadResume {
                request_id,
                params: ThreadResumeParams {
                    thread_id: thread.clone(),
                    exclude_turns: true,
                    cwd: cwd.clone(),
                    ..ThreadResumeParams::default()
                },
            },
            ThreadAsk::ForkAt { thread, cwd } => ClientRequest::ThreadFork {
                request_id,
                params: ThreadForkParams {
                    thread_id: thread.clone(),
                    cwd: cwd.clone(),
                    ..ThreadForkParams::default()
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
            ThreadAsk::ReadTurns { thread } => ClientRequest::ThreadRead {
                request_id,
                params: ThreadReadParams {
                    thread_id: thread.clone(),
                    include_turns: true,
                },
            },
            ThreadAsk::Revert {
                thread,
                before_turn,
            } => ClientRequest::ThreadRevert {
                request_id,
                params: ThreadRevertParams {
                    thread_id: thread.clone(),
                    before_turn_id: before_turn.as_str().to_owned(),
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
            ThreadAsk::Steer {
                thread,
                turn,
                message,
            } => ClientRequest::TurnSteer {
                request_id,
                params: codex_app_server_protocol::TurnSteerParams {
                    thread_id: thread.clone(),
                    input: crate::protocol::turn_input(message),
                    expected_turn_id: turn.clone(),
                    ..Default::default()
                },
            },
            ThreadAsk::Compact { thread } => ClientRequest::ThreadCompactStart {
                request_id,
                params: ThreadCompactStartParams {
                    thread_id: thread.clone(),
                },
            },
            ThreadAsk::CleanTerminals { thread } => ClientRequest::ThreadBackgroundTerminalsClean {
                request_id,
                params: ThreadBackgroundTerminalsCleanParams {
                    thread_id: thread.clone(),
                },
            },
            ThreadAsk::Shell { thread, command } => ClientRequest::ThreadShellCommand {
                request_id,
                params: ThreadShellCommandParams {
                    thread_id: thread.clone(),
                    command: command.clone(),
                    timeout_ms: None,
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

    /// Asks the backend to start a turn with `message`.
    pub fn start_turn(&mut self, message: UserMessage) -> Ticket {
        let ticket = self.mint.next();
        self.registered
            .push((ticket, ThreadCtx::StartTurn { interrupt: false }));
        self.out.commands.push(Command::StartTurn {
            ticket,
            thread: self.thread.clone(),
            message,
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
                error: n.turn.error.as_ref().map(Fault::of),
            },
        )),
        ServerNotification::Error(n) if !n.will_retry => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnError {
                turn: TurnId::new(&n.turn_id),
                fault: Fault::of(&n.error),
            },
        )),
        ServerNotification::ThreadReverted(n) => {
            Some((ThreadId::new(&n.thread_id), ThreadEv::Reverted))
        }
        ServerNotification::McpServerStatusUpdated(n) => n.thread_id.as_deref().map(|thread| {
            (
                ThreadId::new(thread),
                ThreadEv::Mcp {
                    server: n.name.clone(),
                    status: screens::chat::McpStatus::of(n),
                },
            )
        }),
        ServerNotification::Warning(n) => n.thread_id.as_deref().map(|thread| {
            (
                ThreadId::new(thread),
                ThreadEv::Appended {
                    turn: None,
                    item: Item::Warning {
                        message: n.message.clone(),
                    },
                },
            )
        }),
        ServerNotification::ItemStarted(n) => item_upsert(&n.thread_id, &n.turn_id, &n.item, false)
            .map(|(thread, ev)| match ev {
                ThreadEv::ItemUpsert { key, item } => (thread, ThreadEv::ItemStarted { key, item }),
                _ => (thread, ev),
            }),
        ServerNotification::ItemCompleted(n) => {
            item_upsert(&n.thread_id, &n.turn_id, &n.item, true)
        }
        ServerNotification::TurnPlanUpdated(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::PlanUpdated {
                turn: TurnId::new(&n.turn_id),
                update: Arc::new(PlanUpdate::from_notification(n)),
            },
        )),
        ServerNotification::HookCompleted(n) => HookRun::from_summary(&n.run).map(|run| {
            (
                ThreadId::new(&n.thread_id),
                ThreadEv::Appended {
                    turn: n.turn_id.as_deref().map(TurnId::new),
                    item: Item::Hook(Arc::new(run)),
                },
            )
        }),
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
        ServerNotification::CommandExecutionOutputDelta(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::CommandOutputDelta {
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

/// What the server sends without a thread and the chat shows in the thread
/// the app displays (oracle: the chat widget's `on_warning` and
/// `on_deprecation_notice`). The app picks the thread and applies the event.
pub fn project_unrouted(notification: &ServerNotification) -> Option<ThreadEv> {
    let item = match notification {
        ServerNotification::Warning(n) if n.thread_id.is_none() => Item::Warning {
            message: n.message.clone(),
        },
        ServerNotification::DeprecationNotice(n) => Item::Deprecation {
            summary: n.summary.clone(),
            details: n.details.clone(),
        },
        // The server tells no thread: the one on screen starts its servers.
        ServerNotification::McpServerStatusUpdated(n) if n.thread_id.is_none() => {
            return Some(ThreadEv::Mcp {
                server: n.name.clone(),
                status: screens::chat::McpStatus::of(n),
            });
        }
        _ => return None,
    };
    Some(ThreadEv::Appended { turn: None, item })
}

/// Why a turn went wrong, before the UI words it (oracle `new_error_event`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The server's own words.
    Message(String),
    /// An overloaded server that said nothing (oracle `on_server_overloaded_error`).
    HighLoad,
}

impl Fault {
    fn of(error: &codex_app_server_protocol::TurnError) -> Fault {
        use codex_app_server_protocol::CodexErrorInfo;
        if matches!(
            error.codex_error_info,
            Some(CodexErrorInfo::ServerOverloaded | CodexErrorInfo::PolicyCapacityUnavailable)
        ) && error.message.trim().is_empty()
        {
            Fault::HighLoad
        } else {
            Fault::Message(error.message.clone())
        }
    }

    fn item(&self) -> Item {
        match self {
            Fault::Message(message) => Item::Error {
                message: message.clone(),
            },
            Fault::HighLoad => Item::HighLoad,
        }
    }

    /// Whether `item` is this fault's row.
    fn is(&self, item: &Item) -> bool {
        match (self, item) {
            (Fault::Message(message), Item::Error { message: held }) => message == held,
            (Fault::HighLoad, Item::HighLoad) => true,
            _ => false,
        }
    }
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
    let items = || {
        turn.item_payloads()
            .filter_map(|payload| item_entry(payload, true))
            .collect()
    };
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
                error: turn
                    .error
                    .as_ref()
                    .map(|error| Fault::Message(error.message.clone())),
                items: turn_snapshot(turn)?,
            })
        })
        .collect();
    ThreadEv::HistoryPage { turns, next }
}

fn item_entry(payload: &ThreadItemPayload, completed: bool) -> Option<(ItemId, Item)> {
    Some(match payload {
        ThreadItemPayload::UserMessage { id, content, .. } => {
            (ItemId::new(id), Item::User(UserPrompt::from_input(content)))
        }
        ThreadItemPayload::AgentMessage { id, text, .. } => {
            (ItemId::new(id), Item::Agent { text: text.clone() })
        }
        _ => {
            return activity::tool(payload)
                .map(|(id, tool)| (id, Item::Tool(Arc::new(tool))))
                .or_else(|| cells::entry(payload, completed))
                .or_else(|| collab::entry(payload));
        }
    })
}

fn item_upsert(
    thread: &str,
    turn: &str,
    item: &ThreadItem,
    completed: bool,
) -> Option<(ThreadId, ThreadEv)> {
    let (id, entry) = item_entry(&item.payload, completed)?;
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
    if ev.is_notification() {
        model.slots.chat.clear_retry();
    }
    match ev {
        ThreadEv::Mcp { server, status } => {
            let warnings = model.slots.chat.mcp.update(server, status);
            add_startup_warnings(model, warnings, fx);
        }
        ThreadEv::McpLagged => {
            let warnings = model.slots.chat.mcp.finish_after_lag();
            add_startup_warnings(model, warnings, fx);
        }
        ThreadEv::DeferredSubmit => model.last_submit = Some(fx.mint()),
        ThreadEv::ConnectionLost if model.has_writer() => {
            model.writer = Writer::Detached;
            model.outgoing = Pending::default();
            model.turn = TurnState::Idle;
            // A steer in flight will never be echoed: it goes back to the
            // composer with the rest that waited.
            let mut messages = std::mem::take(&mut model.rejected_steers);
            messages.extend(
                std::mem::take(&mut model.pending_steers)
                    .into_iter()
                    .map(|steer| steer.message),
            );
            give_back(model, messages, false);
            model.steers_after_interrupt = false;
            model.slots.chat.connection_lost();
        }
        ThreadEv::ConnectionLost => {}
        ThreadEv::Reverted => {
            model.transcript.reset();
            model.history_next = None;
            Rc::make_mut(&mut model.finished_turns).clear();
            Rc::make_mut(&mut model.authoritative_turns).clear();
            model.completed_items.clear();
            model.last_ended = None;
        }
        ThreadEv::Failure(error) => model.last_error = Some(error),
        ThreadEv::HistoryPage { turns, next } => {
            for snapshot in &turns {
                if snapshot.end.is_some() {
                    if matches!(&model.turn, TurnState::Running(running) if running.turn == snapshot.turn)
                    {
                        let end = snapshot.end.unwrap_or(TurnEnd::Completed);
                        end_turn(model, snapshot.turn.clone(), end, fx);
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
        // `submit_op`: a thread its parent controls takes no turn and no
        // compaction; the line stays where it was typed (no `last_submit`).
        ThreadEv::Submit { .. }
        | ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(screens::chat::ThreadIntent::Compact))
            if model.slots.chat.parent_owned =>
        {
            apply_ev(
                model,
                ThreadEv::Appended {
                    turn: None,
                    item: Item::Note(Note::from(NoteKey::ParentOwned)),
                },
                fx,
            )
        }
        ThreadEv::Submit { message } if model.has_writer() => {
            // The decision is here (R10). Enter during a running turn steers
            // it (`turn/steer`); while the turn is only starting (no turn id
            // yet) the message waits in the queue; with none running it starts
            // a turn, after any older queued message.
            let starting = model.outgoing.any(is_start_turn);
            if let (TurnState::Running(running), false) = (&model.turn, starting) {
                let turn = running.turn.clone();
                model.pending_steers.push(PendingSteer {
                    message: message.clone(),
                    admitted: None,
                });
                model.last_submit = Some(steer(fx, turn, false, message));
                return;
            }
            // MCP servers still starting hold the message (oracle
            // `maybe_send_next_queued_input`).
            if starting || model.slots.chat.mcp.is_starting() || !model.queue.is_empty() {
                enqueue(model, message, fx);
                return;
            }
            model.last_error = None;
            model.last_submit = Some(fx.start_turn(message));
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
                (
                    ThreadCtx::Screen(ScreenCtx::Chat(screens::chat::Ctx::Steer {
                        turn,
                        retried,
                        message,
                    })),
                    answer,
                ) => steer_reply(model, turn, retried, message, answer, fx),
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
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(screens::chat::ThreadIntent::Queue(
            message,
        ))) if model.has_writer() => enqueue(model, message, fx),
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
            screens::chat::ThreadIntent::InterruptForSteers,
        )) if model.has_writer() => {
            if model.can_interrupt() && !model.pending_steers.is_empty() {
                model.steers_after_interrupt = true;
                apply_ev(model, ThreadEv::Interrupt, fx);
            }
        }
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
            screens::chat::ThreadIntent::EditQueued,
        )) if model.has_writer() => {
            let message = model.queue.pop().or_else(|| model.rejected_steers.pop());
            if let Some(message) = message {
                give_back(model, vec![message], true);
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
        ThreadEv::TurnError { turn, fault } => {
            // Oracle `on_error`: the turn is finalized, then the error row, then
            // the next queued input; the failed turn's own completion is a no-op.
            end_turn(model, turn.clone(), TurnEnd::Failed, fx);
            model.transcript.report_error(&turn, &fault);
        }
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
            note_review(model, &item);
            if !model.finished_turns.contains(&key.turn) && model.transcript.item(&key).is_none() {
                steer_echoed(model, &item);
                model.transcript.upsert(key, item);
            }
        }
        ThreadEv::ItemUpsert { key, item } => {
            note_review(model, &item);
            if !model.authoritative_turns.contains(&key.turn) {
                if model.transcript.item(&key).is_none() {
                    steer_echoed(model, &item);
                }
                if !model.finished_turns.contains(&key.turn) && matches!(item, Item::Agent { .. }) {
                    model.completed_items.insert(key.clone());
                }
                model.transcript.upsert(key, item);
            }
        }
        ThreadEv::PlanUpdated { turn, update } => {
            // The status line follows the same report, for the running turn only.
            let status = ScreenThreadEv::Chat(screens::chat::plan_progress(&turn, &update));
            let key = model.transcript.mint(turn);
            model.transcript.upsert(key, Item::PlanUpdate(update));
            apply_ev(model, ThreadEv::Screen(status), fx);
        }
        ThreadEv::Appended { turn, item } => apply_appended(model, turn, item),
        ThreadEv::AgentDelta { key, delta } => {
            if !model.finished_turns.contains(&key.turn) && !model.completed_items.contains(&key) {
                model.transcript.append_agent(key, &delta);
            }
        }
        ThreadEv::CommandOutputDelta { key, delta } => {
            if !model.finished_turns.contains(&key.turn) {
                model.transcript.append_command_output(key, &delta);
            }
        }
        ThreadEv::TurnEnded { turn, end } => end_turn(model, turn, end, fx),
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
            if let Some(fault) = &error {
                model.transcript.report_error(&turn, fault);
            }
            end_turn(model, turn, end, fx);
        }
    }
}

fn end_turn(model: &mut ThreadModel, turn: TurnId, end: TurnEnd, fx: &mut ThreadFx<'_>) {
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
        if end == TurnEnd::Interrupted {
            interrupted(model, &turn, fx);
        } else if !model.outgoing.any(is_start_turn) {
            send_next(model, fx);
        }
    }
}

/// Esc or budget exhaustion ended the turn (`on_interrupted_server_turn`).
/// With `steers_after_interrupt` the steers go out as one message now; else
/// what waits (rejected and pending steers, then the queue) goes back to the
/// composer, never auto-sent. A steer the server admitted to another turn
/// is left alone.
fn interrupted(model: &mut ThreadModel, turn: &TurnId, fx: &mut ThreadFx<'_>) {
    let (later, mine): (Vec<_>, Vec<_>) = std::mem::take(&mut model.pending_steers)
        .into_iter()
        .partition(|steer| steer.admitted.as_ref().is_some_and(|id| id != turn));
    let submit = std::mem::take(&mut model.steers_after_interrupt);
    if submit && !mine.is_empty() {
        let merged = merge_messages(mine.into_iter().map(|steer| steer.message).collect());
        fx.start_turn(merged);
    } else {
        let mut messages = std::mem::take(&mut model.rejected_steers);
        messages.extend(mine.into_iter().map(|steer| steer.message));
        messages.append(&mut model.queue);
        give_back(model, messages, false);
    }
    model.pending_steers = later;
}

/// A message to the queue, behind the running turn; with none running and
/// nothing older waiting it goes out at once.
fn enqueue(model: &mut ThreadModel, message: UserMessage, fx: &mut ThreadFx<'_>) {
    let busy = model.is_running() || model.outgoing.any(is_start_turn);
    model.queue.push(message);
    model.last_submit = Some(fx.mint());
    if !busy {
        send_next(model, fx);
    }
}

/// `turn/steer` of `message` to `turn`.
fn steer(fx: &mut ThreadFx<'_>, turn: TurnId, retried: bool, message: UserMessage) -> Ticket {
    fx.request(
        screens::chat::Ctx::Steer {
            turn: turn.clone(),
            retried,
            message: message.clone(),
        },
        screens::chat::Command::Steer { turn, message },
    )
}

/// Messages back to the composer (`restore_composer_state`).
fn give_back(model: &mut ThreadModel, messages: Vec<UserMessage>, replace: bool) {
    if messages.is_empty() {
        return;
    }
    let seq = model.restore.as_ref().map_or(1, |restore| restore.seq + 1);
    model.restore = Some(Restore {
        seq,
        messages,
        replace,
    });
}

/// The user's item of a steer came back: its pending entry is done.
fn steer_echoed(model: &mut ThreadModel, item: &Item) {
    if let Item::User(prompt) = item
        && let Some(index) = model
            .pending_steers
            .iter()
            .position(|steer| steer.message.text == prompt.text)
    {
        model.pending_steers.remove(index);
    }
}

/// How the server refused a `turn/steer` (`active_turn_steer_race`,
/// `active_turn_not_steerable_turn_error`).
enum Refusal {
    /// A review or compact turn takes no steer.
    NotSteerable,
    /// The turn is over.
    NoActiveTurn,
    /// Another turn is the active one.
    Changed(TurnId),
    Other,
}

fn refusal(text: &str) -> Refusal {
    if text.starts_with("cannot steer a ") {
        return Refusal::NotSteerable;
    }
    if text == "no active turn to steer" {
        return Refusal::NoActiveTurn;
    }
    text.strip_prefix("expected active turn id `")
        .and_then(|rest| rest.split_once("` but found `"))
        .and_then(|(_, actual)| actual.strip_suffix('`'))
        .map_or(Refusal::Other, |actual| {
            Refusal::Changed(TurnId::new(actual))
        })
}

/// The answer of a `turn/steer`.
fn steer_reply(
    model: &mut ThreadModel,
    turn: TurnId,
    retried: bool,
    message: UserMessage,
    answer: Answer,
    fx: &mut ThreadFx<'_>,
) {
    let position = model
        .pending_steers
        .iter()
        .position(|steer| steer.message == message);
    let failure = match answer {
        Answer::Screen(screens::ScreenReply::Chat(screens::chat::Reply::Steered(admitted))) => {
            if let Some(index) = model
                .pending_steers
                .iter()
                .position(|steer| steer.admitted.is_none() && steer.message == message)
            {
                model.pending_steers[index].admitted = Some(admitted);
            }
            return;
        }
        Answer::Failed(RpcError::Remote(text)) => Failure::Message(text),
        Answer::Failed(error) => Failure::from(error),
        _ => return,
    };
    let text = match &failure {
        Failure::Message(text) => text.as_str(),
        _ => "",
    };
    match refusal(text) {
        Refusal::NotSteerable => match position {
            Some(index) => {
                let steer = model.pending_steers.remove(index);
                model.rejected_steers.push(steer.message);
            }
            None => model.last_error = Some(failure),
        },
        Refusal::NoActiveTurn => {
            if let Some(index) = position {
                model.pending_steers.remove(index);
            }
            fx.start_turn(message);
        }
        Refusal::Changed(actual) if !retried && actual != turn => {
            if let TurnState::Running(running) = &mut model.turn
                && running.turn == turn
                && !model.finished_turns.contains(&actual)
            {
                running.turn = actual.clone();
            }
            steer(fx, actual, true, message);
        }
        Refusal::Changed(_) | Refusal::Other => {
            if let Some(index) = position {
                model.pending_steers.remove(index);
            }
            model.last_error = Some(failure);
            give_back(model, vec![message], false);
        }
    }
}

/// The messages as one, joined by newlines; the local images' placeholders
/// are numbered afresh in order (`merge_user_messages`).
fn merge_messages(messages: Vec<UserMessage>) -> UserMessage {
    let mut merged = UserMessage::default();
    for (index, message) in messages.into_iter().enumerate() {
        if index > 0 {
            merged.text.push('\n');
        }
        let mut images = message.images;
        images.sort_by_key(|image| image.range.start);
        let mut cursor = 0;
        for image in images {
            let range = image.range;
            if range.start < cursor || range.end > message.text.len() {
                continue;
            }
            merged.text.push_str(&message.text[cursor..range.start]);
            let start = merged.text.len();
            let placeholder = &message.text[range.clone()];
            merged
                .text
                .push_str(&renumbered(placeholder, merged.images.len() + 1));
            merged.images.push(LocalImage {
                path: image.path,
                range: start..merged.text.len(),
            });
            cursor = range.end;
        }
        merged.text.push_str(&message.text[cursor..]);
    }
    merged
}

/// `placeholder` with the number inside it (its last run of digits) replaced
/// by `number`; a placeholder without one stays as it is. Whatever words the
/// placeholder is in (the UI's table words it) is kept.
fn renumbered(placeholder: &str, number: usize) -> String {
    let body = placeholder.trim_end_matches(|c: char| !c.is_ascii_digit());
    let head = body.trim_end_matches(|c: char| c.is_ascii_digit());
    if body.is_empty() {
        return placeholder.to_string();
    }
    format!("{head}{number}{}", &placeholder[body.len()..])
}

/// The next queued message becomes the next turn (one per turn end).
fn send_next(model: &mut ThreadModel, fx: &mut ThreadFx<'_>) {
    if model.slots.chat.mcp.is_starting() {
        return;
    }
    // Steers the server refused go first, as one message
    // (`pop_next_queued_user_message`).
    if !model.rejected_steers.is_empty() {
        let merged = merge_messages(std::mem::take(&mut model.rejected_steers));
        fx.start_turn(merged);
        return;
    }
    if model.queue.is_empty() {
        return;
    }
    let message = model.queue.remove(0);
    fx.start_turn(message);
}

fn is_start_turn(ctx: &ThreadCtx) -> bool {
    matches!(ctx, ThreadCtx::StartTurn { .. })
}

fn apply_appended(model: &mut ThreadModel, turn: Option<TurnId>, item: Item) {
    let turn = turn
        .or_else(|| match &model.turn {
            TurnState::Running(running) => Some(running.turn.clone()),
            TurnState::Idle => None,
        })
        .or_else(|| model.transcript.last_turn().cloned())
        .unwrap_or_default();
    let key = model.transcript.mint(turn);
    model.transcript.upsert(key, item);
}

/// Entering or leaving code review is the thread's state (oracle
/// `review.is_review_mode`), read by the slash-command gates.
fn note_review(model: &mut ThreadModel, item: &Item) {
    match item {
        Item::Notice(Notice::EnteredReview(_)) => model.slots.chat.set_review_mode(true),
        Item::Notice(Notice::ExitedReview(_)) => model.slots.chat.set_review_mode(false),
        _ => {}
    }
}

/// The warnings a startup update made, then what the settled round held back.
fn add_startup_warnings(
    model: &mut ThreadModel,
    warnings: Vec<screens::chat::McpWarning>,
    fx: &mut ThreadFx<'_>,
) {
    for warning in warnings {
        apply_appended(model, None, Item::StartupWarning(warning));
    }
    if !model.slots.chat.mcp.is_starting()
        && !model.is_running()
        && !model.outgoing.any(is_start_turn)
    {
        send_next(model, fx);
    }
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

    /// The edit of a past prompt (the transcript's backtrack) reads the
    /// persisted turns, then reverts before one of them: both are thread RPCs,
    /// made here and nowhere else (ADR-003).
    #[test]
    fn the_prompt_edit_reads_the_turns_then_reverts_before_one() {
        let ClientRequest::ThreadRead { params, .. } = (ThreadAsk::ReadTurns {
            thread: "t1".to_owned(),
        })
        .wire(RequestId::Integer(1)) else {
            panic!("a read of the turns is thread/read");
        };
        assert_eq!(params.thread_id, "t1");
        assert!(
            params.include_turns,
            "the edit resolves the prompt from the turns"
        );

        let ask = ThreadAsk::Revert {
            thread: "t1".to_owned(),
            before_turn: TurnId::new("turn-3"),
        };
        assert_eq!(ask.thread(), Some("t1"));
        let ClientRequest::ThreadRevert { params, .. } = ask.wire(RequestId::Integer(2)) else {
            panic!("a revert is thread/revert");
        };
        assert_eq!(params.thread_id, "t1");
        assert_eq!(params.before_turn_id, "turn-3");
    }
}
