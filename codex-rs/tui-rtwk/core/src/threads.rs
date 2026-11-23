//! `ThreadModel` and `apply` (ADR-003 R3-R5, R9): one thread's data, changed
//! only here. A thread RPC is requested through `ThreadFx`, which stamps the
//! thread id.

mod previews;

pub use previews::{ThreadPreview, ThreadPreviews};

mod activity;
pub mod calls;
mod cells;
pub mod citations;
mod collab;
pub mod daybreak;
pub mod decision;
pub mod directives;
pub mod followups;
pub mod goal;
pub mod inline_vis;
mod message;
mod note;
pub mod queue;
pub mod replay;
pub mod status;
pub mod stream_preview;
mod user_prompt;

pub use activity::{
    BlockBody, DynamicCall, DynamicOutput, ExplorationAction, McpBlock, McpCall, McpResult,
    McpResultKind, ToolActivity, ToolKind, ToolStatus,
};
pub use calls::ShellClass;
pub use cells::{
    DelegatedMessage, GeneratedImage, HookEntry, HookEntryKind, HookRun, HookStatus, Notice,
    PatchActivity, PatchChange, PatchKind, PlanStep, PlanUpdate, StepStatus, WebAction,
    WebActivity,
};
pub use codex_protocol::models::MessagePhase;
pub use collab::{AgentOutcome, AgentRef, CollabEvent, SpawnRequest, SubAgentKind};
pub use decision::ReviewLine;
pub use goal::{Goal, GoalStatus, ReviewTarget, SettingsPatch, goal_of_response};
pub use message::{LocalImage, ToolMention, UserMessage};
pub use note::{CdRefusal, ModeNote, Note, NoteKey, RemoveNote, Stub, Tone};
pub use queue::{DueCommand, Queued, Ran};
use queue::{enqueue, send_next};
pub use user_prompt::{Shown, UserPrompt};

use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ReviewDelivery;
use codex_app_server_protocol::ReviewStartParams;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::SortDirection;
use codex_app_server_protocol::ThreadBackgroundTerminalsCleanParams;
use codex_app_server_protocol::ThreadCompactStartParams;
use codex_app_server_protocol::ThreadForkParams;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadPrepareApprovedPlanParams;
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
use crate::features::{self, FeatureCtx, FeatureThreadIntent, ThreadCx, ThreadView};
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
use crate::screens::cells::Daybreak;

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
    /// An assistant message. `phase` is what the server said about it (oracle
    /// `AgentMarkdownCell::phase`): none for a streamed message or a legacy model, so
    /// callers treat none as "unknown".
    Agent {
        text: String,
        phase: Option<MessagePhase>,
    },
    Tool(Arc<ToolActivity>),
    /// The model's reasoning summary; shown only in the detailed transcript
    /// (oracle `ReasoningSummaryCell::transcript_only`). `with_raw` is the same
    /// body with the raw reasoning content after the summary parts, kept only when
    /// the item has content: the oracle joins it under `show_raw_agent_reasoning`
    /// (`replace_reasoning_parts`), which the view reads (`Item::reasoning_text`).
    Reasoning {
        summary: String,
        with_raw: Option<String>,
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
    /// A warning the client itself raises, shown only in the detailed
    /// transcript like every `new_warning_event`; its words are the view's.
    LocalWarning(LocalWarning),
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
    /// What an answer to an approval request or the auto-reviewer's verdict left (oracle
    /// `new_approval_decision_cell`, the plain permissions line).
    Decision(screens::approvals::decided::Decided),
    /// A line `/auto-review` or the reviewer's paths added.
    Review(ReviewLine),
    /// The card of a `/status` command (oracle `StatusHistoryCell`). `request`
    /// is the refresh of the rate limits the card still waits for.
    Status {
        card: Arc<screens::status::StatusCard>,
        request: Option<u64>,
    },
}

impl Item {
    /// The Markdown body of a reasoning item (empty for another kind): the summary, and with
    /// `show_raw` (`show_raw_agent_reasoning`) the raw reasoning content after it.
    pub fn reasoning_text(&self, show_raw: bool) -> &str {
        match self {
            Self::Reasoning { summary, with_raw } => match with_raw {
                Some(text) if show_raw => text,
                _ => summary,
            },
            _ => "",
        }
    }

    /// Entries only the detailed transcript (Ctrl+T) shows: the reasoning
    /// summary, the server's warnings and its deprecation notices.
    pub fn is_detailed_only(&self) -> bool {
        matches!(
            self,
            Self::Reasoning { .. }
                | Self::Warning { .. }
                | Self::LocalWarning(_)
                | Self::Deprecation { .. }
        )
    }

    /// The run of neighbors the entry can share one visible section with
    /// (oracle `thread_transcript` `PendingActivity`): explorations with
    /// explorations, computer calls with computer calls.
    pub fn group(&self) -> Option<ItemGroup> {
        let Self::Tool(tool) = self else {
            return None;
        };
        match &tool.kind {
            ToolKind::Shell { actions, .. } if !actions.is_empty() => Some(ItemGroup::Exploration),
            _ if tool.is_computer_activity() => Some(ItemGroup::Computer),
            _ => None,
        }
    }
}

/// What entries of one run have in common (`Item::group`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemGroup {
    Exploration,
    Computer,
}

/// The kinds of item whose text streams in deltas.
#[derive(Clone, Copy)]
enum Text {
    Agent,
    Plan,
}

impl Text {
    fn empty(self) -> Item {
        match self {
            Text::Agent => Item::Agent {
                text: String::new(),
                phase: None,
            },
            Text::Plan => Item::Plan {
                text: String::new(),
            },
        }
    }

    fn text_of(self, item: &Item) -> Option<&String> {
        match (self, item) {
            (Text::Agent, Item::Agent { text, .. }) | (Text::Plan, Item::Plan { text }) => {
                Some(text)
            }
            _ => None,
        }
    }

    fn text_mut(self, item: &mut Item) -> Option<&mut String> {
        match (self, item) {
            (Text::Agent, Item::Agent { text, .. }) | (Text::Plan, Item::Plan { text }) => {
                Some(text)
            }
            _ => None,
        }
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
    /// The turns that ended as completed (oracle: a turn with its `FinalMessageSeparator`),
    /// whatever time the server gave them.
    completed: Rc<HashSet<TurnId>>,
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

/// How much of a thread's history the transcript holds (oracle `TranscriptHistoryState`
/// `Complete`, `Partial`, `LoadingOlder`, `Failed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryState {
    /// The oldest page was read.
    Complete,
    /// Older pages exist and none is being read.
    Partial,
    /// An older page was asked for and has not come.
    Loading,
    /// The last ask for an older page failed; asking again retries it.
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoryAsk {
    Idle,
    Loading,
    Failed,
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

    /// The turn ended as completed, not interrupted or failed: its tool calls and messages
    /// before the final answer can fold behind its prompt (oracle `turn_details`).
    pub fn is_completed(&self, turn: &TurnId) -> bool {
        self.completed.contains(turn)
    }

    fn mark_completed(&mut self, turn: &TurnId) {
        if !self.completed.contains(turn) {
            Rc::make_mut(&mut self.completed).insert(turn.clone());
            // the fold of its details changes what the rows are
            self.layout_revision += 1;
        }
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
        if self.item(&key).map(Item::group) != Some(item.group()) {
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
    fn report_error(&mut self, turn: &TurnId, fault: &Fault, daybreak: Daybreak) {
        if !self.holds_error(turn, fault) {
            let key = self.mint(turn.clone());
            self.upsert(key, fault.item(daybreak));
        }
    }

    /// A partial answer still receiving text deltas.
    pub fn is_agent_streaming(&self, key: &ItemKey) -> bool {
        self.streaming.contains_key(key)
    }

    /// The source eligible for a stable commit. The raw item retains its
    /// partial line for rtwk's live preview.
    pub fn committed_agent_text(&self, key: &ItemKey) -> Option<&str> {
        let Item::Agent { text, .. } = self.item(key)? else {
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
        self.append_text(key, delta, Text::Agent);
    }

    /// Appends a delta of the proposed plan (`item/plan/delta`), as an answer's text streams.
    pub fn append_plan(&mut self, key: ItemKey, delta: &str) {
        self.append_text(key, delta, Text::Plan);
    }

    fn append_text(&mut self, key: ItemKey, delta: &str, kind: Text) {
        if delta.is_empty() {
            return;
        }
        use std::collections::hash_map::Entry;
        let live = match self.streaming.entry(key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let (item, committed) = match self.items.get(entry.key()) {
                    Some(item) => match kind.text_of(item) {
                        Some(text) => (item.clone(), text.len()),
                        None => return,
                    },
                    None => {
                        Rc::make_mut(&mut self.order).push(entry.key().clone());
                        self.layout_revision += 1;
                        (Rc::new(kind.empty()), 0)
                    }
                };
                entry.insert(StreamingItem { item, committed })
            }
        };
        if let Some(text) = kind.text_mut(Rc::make_mut(&mut live.item)) {
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
    fn prepend_snapshots(&mut self, snapshots: Vec<HistoryTurn>, daybreak: Daybreak) {
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
                stored.insert(key.clone(), Rc::new(fault.item(daybreak)));
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
    /// The app's clock when the client saw the turn start (epoch seconds, 0 unknown): the
    /// oracle's status timer, the elapsed time of a turn the server timed no duration for.
    began: i64,
    interrupt_sent: bool,
}

/// What the client itself warns of (the words are the view's, ADR-010).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalWarning {
    /// Esc with steers waiting while a review runs: a review turn takes no
    /// steer (`REVIEW_STEER_UNAVAILABLE_MESSAGE`).
    ReviewSteerUnavailable,
}

/// How a turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnEnd {
    Completed,
    Interrupted,
    Failed,
}

/// One unified-exec session that runs in the background (oracle
/// `UnifiedExecProcessSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminal {
    /// The server's process id, else the item's id.
    pub key: String,
    pub command: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RunningHook {
    id: String,
    message: Option<String>,
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
    /// A turn start in flight with the message it carries (a provider refusal
    /// gives it back to the front of the queue).
    StartTurn {
        interrupt: bool,
        message: UserMessage,
    },
    Interrupt {
        turn: TurnId,
        retried: bool,
    },
    Screen(ScreenCtx),
    /// An RPC of a feature slice (`ThreadCx::request`).
    Feature(FeatureCtx),
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
    /// What the user sent during a turn, set as a fixture does: the pending
    /// steers, the refused ones and the queue (the real ones come from
    /// `Submit`, the steer replies and `Queue`).
    Preview(screens::chat::PendingTexts),
    /// `thread/reverted`: the server's history is now the prefix before a
    /// turn (the edit of a past prompt). Nothing held of the old history is
    /// trusted: the transcript is cleared and the app reads the retained
    /// turns again (oracle `AppEvent::PromptRevert`, `set_session`).
    Reverted,
    ScreenIntent(ScreenThreadIntent),
    /// An intent for a feature slice's part of the thread (ADR-012), the one door of a
    /// change of a thread that a feature asks for.
    FeatureIntent(FeatureThreadIntent),
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
        submissions: Vec<replay::Submission>,
    },
    /// The app asked for the next older page (`HistoryState::Loading` until it answers).
    HistoryRequested,
    /// The ask for an older page failed or its connection went (`HistoryState::Failed`).
    HistoryFailed,
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
    /// A hook run started (`hook/started`): the status row names it while it
    /// runs (oracle `HookCell::running_status_summary`).
    HookStarted {
        id: String,
        message: Option<String>,
    },
    /// A hook run ended (`hook/completed`): it leaves the status row, and a
    /// run that has something to say leaves its row in the transcript.
    HookEnded {
        id: String,
        turn: Option<TurnId>,
        item: Option<Item>,
    },
    AgentDelta {
        key: ItemKey,
        delta: String,
    },
    /// The proposed plan's text streams (`item/plan/delta`).
    PlanDelta {
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
    /// The goal changed (`thread/goal/updated`, an answer of `thread/goal/get|set`) or was
    /// cleared (`None`: `thread/goal/cleared`).
    /// The goal changed or was cleared; the turn a budget-limited goal stopped
    /// (oracle `budget_limited_turn_ids`), marked for the interruption notice.
    Goal(Option<Goal>, Option<TurnId>),
    /// An MCP server's start or outcome (`mcpServer/startupStatus/updated`).
    Mcp {
        server: String,
        status: screens::chat::McpStatus,
    },
    /// The app server's event stream lagged (`BackendEv::Lagged`).
    McpLagged,
}

impl ThreadEv {
    /// Whether the event can put a turn's error in the transcript (so its wording needs the
    /// Daybreak facts): a failed turn, a restored page. Not a delta.
    pub(crate) fn may_fail_a_turn(&self) -> bool {
        matches!(
            self,
            Self::TurnError { .. } | Self::TurnCompleted { .. } | Self::HistoryPage { .. }
        )
    }

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
            | Self::HookStarted { .. }
            | Self::HookEnded { .. }
            | Self::AgentDelta { .. }
            | Self::PlanDelta { .. }
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
            RpcError::Remote(message)
            | RpcError::Malformed(message)
            | RpcError::Prepare(message) => Failure::Message(message),
            RpcError::Mismatch => Failure::ReplyMismatch,
            RpcError::Provider(error) => Failure::Message(error.to_string()),
        }
    }
}

/// One thread's model: the thread's own state (`core`, read by the features as a
/// `ThreadView` beside their own part), the screens' per-thread parts (`slots`) and the
/// features' (`features`, ADR-012). The three are fields of their own so a feature is
/// handed `&mut` of its part and `&` of the rest at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadModel {
    core: ThreadCore,
    /// The per-thread parts of the screen slots.
    slots: ThreadSlots,
    /// The per-thread parts of the features (`feature_list!`).
    features: crate::features::FeatureThreadParts,
    /// Keeps the struct literal inside this module: a `ThreadModel` is made
    /// only by `ThreadSet::open` (ADR-003).
    seal: Seal,
}

/// What a thread is, apart from the parts its screens and features keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadCore {
    id: ThreadId,
    transcript: Transcript,
    turn: TurnState,
    /// Our RPCs of this thread, keyed `(ThreadId, Ticket)` by being in the
    /// thread's model (R3).
    outgoing: Pending<ThreadCtx>,
    /// The ticket of the last submit `apply` accepted: the composer clears
    /// when it changes, never before.
    last_submit: Option<Ticket>,
    /// Lines the queue key set aside while a turn runs: one message goes out per turn end, in
    /// order; a command or a shell line is run when its turn comes.
    queue: Vec<Queued>,
    /// A queued command the screen has to run (and report with `ThreadIntent::QueueRan`): the
    /// queue waits for the answer.
    due: Option<DueCommand>,
    /// Counts the queued commands handed to the screen.
    due_seq: u64,
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
    /// A message went back to the composer while others wait (a refused image
    /// preparation): the queue sends nothing on its own until it is empty
    /// (oracle `recovered_queue`).
    recovered: bool,
    /// The unified-exec sessions that run in the background (oracle
    /// `unified_exec_processes`), oldest first.
    terminals: Vec<Terminal>,
    /// The hook runs the server reported started and not yet completed.
    hooks: Vec<RunningHook>,
    last_error: Option<Failure>,
    /// The last turn that ended: a late start of it must not revive it.
    last_ended: Option<TurnId>,
    finished_turns: Rc<HashSet<TurnId>>,
    authoritative_turns: Rc<HashSet<TurnId>>,
    completed_items: HashSet<ItemKey>,
    /// The cursor of the next older page of history; none once the oldest
    /// page was read.
    history_next: Option<String>,
    /// The user messages the replayed turns seed the composer's history with.
    submissions: replay::Submissions,
    /// What became of the last ask for an older page.
    history_ask: HistoryAsk,
    /// The thread's goal (`thread/goal/updated|cleared`, the answers of `thread/goal/get|set`).
    goal: Option<Goal>,
    writer: Writer,
}

impl std::ops::Deref for ThreadModel {
    type Target = ThreadCore;
    fn deref(&self) -> &ThreadCore {
        &self.core
    }
}

impl std::ops::DerefMut for ThreadModel {
    fn deref_mut(&mut self) -> &mut ThreadCore {
        &mut self.core
    }
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
            // A thread this process just started has run nothing (the composer sparkle's
            // "fresh task"); the server's `thread/started` confirms or ends that.
            let mut fresh = ThreadModel::new(id.clone());
            fresh.slots.chat.mark_fresh(true);
            self.held.insert(id.clone(), Rc::new(fresh));
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

    /// The thread is gone (archived or deleted): the app holds no model of it
    /// any more (`discard_thread_local_state`).
    pub(crate) fn forget(&mut self, id: &ThreadId) {
        self.held.remove(id);
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
            slots: ThreadSlots::default(),
            features: crate::features::FeatureThreadParts::default(),
            core: ThreadCore {
                id,
                transcript: Transcript::default(),
                turn: TurnState::Idle,
                outgoing: Pending::default(),
                last_submit: None,
                queue: Vec::new(),
                due: None,
                due_seq: 0,
                pending_steers: Vec::new(),
                rejected_steers: Vec::new(),
                steers_after_interrupt: false,
                restore: None,
                recovered: false,
                terminals: Vec::new(),
                hooks: Vec::new(),
                last_error: None,
                goal: None,
                last_ended: None,
                finished_turns: Rc::default(),
                authoritative_turns: Rc::default(),
                completed_items: HashSet::new(),
                history_next: None,
                history_ask: HistoryAsk::Idle,
                submissions: replay::Submissions::default(),
                writer: Writer::Detached,
            },
        }
    }

    /// The thread's own state and the screens' parts, read-only, for a feature (ADR-012).
    pub fn view(&self) -> ThreadView<'_> {
        ThreadView::new(&self.core, &self.slots)
    }

    /// A feature's `&mut` part and everything else of the thread read-only, at once.
    fn split_features(&mut self) -> (&mut features::FeatureThreadParts, ThreadView<'_>) {
        let Self {
            core,
            slots,
            features,
            ..
        } = self;
        (features, ThreadView::new(core, slots))
    }

    pub(crate) fn history_cursor(&self) -> Option<&str> {
        self.history_next.as_deref()
    }

    /// The user messages of the loaded history that are submissions of this conversation, oldest
    /// first (oracle `record_replayed_user_message_history`): the composer seeds its history
    /// from them; review prompts and hidden nested review turns are not among them.
    pub fn replayed_submissions(&self) -> &replay::Submissions {
        &self.submissions
    }

    pub fn can_load_history(&self) -> bool {
        self.history_next.is_some()
    }

    /// How much of the thread's history the transcript holds (oracle `TranscriptHistoryState`;
    /// its `LoadingBeginning` is the sheet's own walk to the start, a controller's state).
    pub fn history_state(&self) -> HistoryState {
        match self.history_ask {
            HistoryAsk::Loading => HistoryState::Loading,
            HistoryAsk::Failed => HistoryState::Failed,
            HistoryAsk::Idle if self.history_next.is_some() => HistoryState::Partial,
            HistoryAsk::Idle => HistoryState::Complete,
        }
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

    pub fn queue(&self) -> &[Queued] {
        &self.queue
    }

    /// The queued command the screen runs now: it dispatches the line (`dispatch_queued`) and
    /// answers `ThreadIntent::QueueRan`. A change of `seq` is a new command.
    pub fn due_command(&self) -> Option<&DueCommand> {
        self.due.as_ref()
    }

    /// What waits in the queue is held back: it goes out only when the user sends again
    /// (oracle `recovered_queue`).
    pub fn queue_held(&self) -> bool {
        self.recovered
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

    /// The background terminals (unified-exec sessions still running), oldest first.
    pub fn terminals(&self) -> &[Terminal] {
        &self.terminals
    }

    /// What the status row says of the running hooks (oracle
    /// `HookCell::running_status_summary`): their shared message, else
    /// `Running hooks` for several and `Running hook` for one.
    pub fn hook_status(&self) -> Option<String> {
        let mut messages = self
            .hooks
            .iter()
            .map(|hook| {
                hook.message
                    .as_deref()
                    .map(str::trim)
                    .filter(|m| !m.is_empty())
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

    pub fn screen_slots(&self) -> &ThreadSlots {
        &self.slots
    }

    pub fn last_error(&self) -> Option<&Failure> {
        self.last_error.as_ref()
    }

    /// The thread's goal, none when it has none.
    pub fn goal(&self) -> Option<&Goal> {
        self.goal.as_ref()
    }

    pub fn last_ended(&self) -> Option<&TurnId> {
        self.last_ended.as_ref()
    }

    pub fn read_only(&self) -> bool {
        self.writer == Writer::Foreign
    }

    /// This process writes the thread and the server has not refused it for its
    /// model provider (oracle `blocks_direct_input` after `show_provider_error`).
    pub(crate) fn has_writer(&self) -> bool {
        self.writer == Writer::Attached && self.slots.chat.provider_error.is_none()
    }

    /// Why the server refused the thread for its model provider; the thread
    /// takes no input then.
    pub fn provider_error(&self) -> Option<&codex_app_server_protocol::ModelProviderError> {
        self.slots.chat.provider_error.as_ref()
    }

    pub fn can_interrupt(&self) -> bool {
        self.has_writer() && (self.is_running() || self.outgoing.any(is_start_turn))
    }
}

impl ThreadCore {
    pub fn id(&self) -> &ThreadId {
        &self.id
    }

    pub fn is_running(&self) -> bool {
        matches!(self.turn, TurnState::Running(_))
    }

    /// A turn runs or its start is in flight.
    pub fn busy(&self) -> bool {
        self.is_running() || self.outgoing.any(is_start_turn)
    }

    /// This process is the thread's writer.
    pub(crate) fn attached(&self) -> bool {
        self.writer == Writer::Attached
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
    /// `review/start`, inline on the thread (oracle `review_start`).
    Review {
        thread: String,
        target: ReviewTarget,
    },
    /// `thread/settings/update`: the settings the picks of the model, effort, permissions and
    /// collaboration-mode popups change for the turns after them.
    SettingsUpdate {
        thread: String,
        patch: SettingsPatch,
    },
    /// `thread/prepareApprovedPlan`: the new thread takes the approved plan and answers with the
    /// prompt that starts its work (the prompt's "clear context" row).
    PrepareApprovedPlan {
        thread: String,
        markdown: String,
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
            | ThreadAsk::Review { thread, .. }
            | ThreadAsk::SettingsUpdate { thread, .. }
            | ThreadAsk::PrepareApprovedPlan { thread, .. }
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
            ThreadAsk::Review { thread, target } => ClientRequest::ReviewStart {
                request_id,
                params: ReviewStartParams {
                    thread_id: thread.clone(),
                    target: target.wire(),
                    delivery: Some(ReviewDelivery::Inline),
                },
            },
            ThreadAsk::SettingsUpdate { thread, patch } => ClientRequest::ThreadSettingsUpdate {
                request_id,
                params: patch.wire(thread),
            },
            ThreadAsk::PrepareApprovedPlan { thread, markdown } => {
                ClientRequest::ThreadPrepareApprovedPlan {
                    request_id,
                    params: ThreadPrepareApprovedPlanParams {
                        thread_id: thread.clone(),
                        markdown: markdown.clone(),
                        attachment_path: None,
                    },
                }
            }
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
    /// How the Daybreak setting words a cyber refusal at this moment (the app's fact, stamped
    /// per event by `apply_thread`).
    daybreak: Daybreak,
    /// The server's collaboration-mode presets at this moment (the app's fact, stamped per
    /// event by `apply_thread`): what a switch of the mode chooses from.
    modes: &'a [screens::select::collaboration::Mask],
    /// The app's clock, epoch seconds (0 until the host has said): what a live completion falls
    /// back to when the server's turn carries no duration or completion time.
    now: i64,
}

impl<'a> ThreadFx<'a> {
    pub(crate) fn new(thread: &'a ThreadId, out: &'a mut AppFx, mint: &'a mut TicketMint) -> Self {
        Self {
            thread,
            out,
            mint,
            registered: Vec::new(),
            daybreak: Daybreak::Apply,
            modes: &[],
            now: 0,
        }
    }

    /// The same, at the app's clock `now` (epoch seconds).
    pub(crate) fn at(mut self, now: i64) -> Self {
        self.now = now;
        self
    }

    fn now(&self) -> i64 {
        self.now
    }

    /// The same, with the wording of a cyber refusal at this moment.
    pub(crate) fn stamped(mut self, daybreak: Daybreak) -> Self {
        self.daybreak = daybreak;
        self
    }

    fn daybreak(&self) -> Daybreak {
        self.daybreak
    }

    /// The same, with the collaboration-mode presets of the moment.
    pub(crate) fn with_modes(mut self, modes: &'a [screens::select::collaboration::Mask]) -> Self {
        self.modes = modes;
        self
    }

    /// The server's collaboration-mode presets (empty until read).
    pub(crate) fn modes(&self) -> &[screens::select::collaboration::Mask] {
        self.modes
    }

    /// A turn start was said in this step and its ticket is not in the model yet.
    pub(crate) fn start_said(&self) -> bool {
        self.registered.iter().any(|(_, ctx)| is_start_turn(ctx))
    }

    /// A fresh ticket, for an acceptance that sends no RPC.
    pub fn mint(&mut self) -> Ticket {
        self.mint.next()
    }

    /// Asks the backend to start a turn with `message`: one start per step. A second start in the
    /// same step (two features' hooks after the thread's own work) is refused and the message is
    /// given back, so the caller keeps it (ADR-012: one turn start at a time). Whether the model
    /// is busy is the caller's to ask, it holds the model (`ThreadView::busy` for a feature).
    pub fn start_turn(&mut self, message: UserMessage) -> Result<Ticket, UserMessage> {
        if self.start_said() {
            return Err(message);
        }
        let ticket = self.mint.next();
        self.registered.push((
            ticket,
            ThreadCtx::StartTurn {
                interrupt: false,
                message: message.clone(),
            },
        ));
        self.out.commands.push(Command::StartTurn {
            ticket,
            thread: self.thread.clone(),
            message,
        });
        Ok(ticket)
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

    /// An RPC of a feature slice: `ctx` is kept with the ticket and comes back with the
    /// reply, to `Feature::thread_answered`.
    #[allow(unreachable_code, unused_variables)]
    pub fn request_feature(
        &mut self,
        ctx: impl Into<FeatureCtx>,
        command: impl Into<features::FeatureCommand>,
    ) -> Ticket {
        let ticket = self.mint.next();
        self.registered
            .push((ticket, ThreadCtx::Feature(ctx.into())));
        self.out.commands.push(Command::Feature {
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
        ServerNotification::ThreadGoalUpdated(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::Goal(
                Some(Goal::of(&n.goal)),
                (n.goal.status == codex_app_server_protocol::ThreadGoalStatus::BudgetLimited)
                    .then(|| n.turn_id.as_deref().map(TurnId::new))
                    .flatten(),
            ),
        )),
        ServerNotification::ThreadGoalCleared(n) => {
            Some((ThreadId::new(&n.thread_id), ThreadEv::Goal(None, None)))
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
        // A diagnostic naming a thread is a row of that thread (the oracle's `new_warning_event`).
        notification
            if screens::dialogs::incoming(notification).is_some_and(|w| w.thread.is_some()) =>
        {
            screens::dialogs::incoming(notification).and_then(|warning| {
                warning.thread.map(|thread| {
                    (
                        ThreadId::new(&thread),
                        ThreadEv::Appended {
                            turn: None,
                            item: Item::Warning {
                                message: warning.message,
                            },
                        },
                    )
                })
            })
        }
        ServerNotification::AuthRecoveryStarted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::Appended {
                turn: None,
                item: Item::Note(Note::from(NoteKey::Server(n.message.clone()))),
            },
        )),
        ServerNotification::AuthRecoveryCompleted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::Appended {
                turn: None,
                item: Item::Note(Note::from(NoteKey::ServerDone(n.message.clone()))),
            },
        )),
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
        ServerNotification::HookStarted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::HookStarted {
                id: n.run.id.clone(),
                message: n.run.status_message.clone(),
            },
        )),
        ServerNotification::HookCompleted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::HookEnded {
                id: n.run.id.clone(),
                turn: n.turn_id.as_deref().map(TurnId::new),
                item: HookRun::from_summary(&n.run).map(|run| Item::Hook(Arc::new(run))),
            },
        )),
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
        ServerNotification::PlanDelta(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::PlanDelta {
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
    transcript
        .or_else(|| {
            screens::project_thread(notification).map(|(thread, ev)| (thread, ThreadEv::Screen(ev)))
        })
        .or_else(|| {
            // The auto-reviewer's notifications (`screens::project_thread` is the popups'
            // file: the approvals slot is routed here until it adds the arm).
            screens::approvals::project(notification)
                .map(|(thread, ev)| (thread, ThreadEv::Screen(ScreenThreadEv::Approvals(ev))))
        })
}

/// What the server sends without a thread and the chat shows in the thread
/// the app displays (oracle: the chat widget's `on_warning` and
/// `on_deprecation_notice`). The app picks the thread and applies the event.
pub fn project_unrouted(notification: &ServerNotification) -> Option<ThreadEv> {
    let item = match notification {
        // One without a thread is the displayed one's.
        notification
            if screens::dialogs::incoming(notification).is_some_and(|w| w.thread.is_none()) =>
        {
            Item::Warning {
                message: screens::dialogs::incoming(notification)?.message,
            }
        }
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
    /// A refused cybersecurity request (`codexErrorInfo: CyberPolicy`, oracle
    /// `on_cyber_policy_error`): the notice is worded by the Daybreak setting at the moment it
    /// was refused.
    CyberPolicy,
    /// A request refused for biology safety (oracle `is_bio_policy_error`).
    SafetyAccessBlock,
}

const LEGACY_SAFETY_ACCESS_BLOCK_PREFIX: &str =
    "Invalid prompt: we've limited access to this content for safety reasons.";
const BIO_POLICY_SAFETY_ACCESS_BLOCK_PREFIX: &str =
    "This content was flagged for possible biological risk.";

/// Oracle `chatwidget/turn_runtime.rs` `is_bio_policy_error`: the message, or the JSON error
/// object the server wrapped it in.
fn is_bio_policy_error(message: &str) -> bool {
    let blocked = |text: &str| {
        text.starts_with(LEGACY_SAFETY_ACCESS_BLOCK_PREFIX)
            || text.starts_with(BIO_POLICY_SAFETY_ACCESS_BLOCK_PREFIX)
            || text.starts_with("This request has been flagged for possible biological risk.")
    };
    blocked(message)
        || serde_json::from_str::<serde_json::Value>(message).is_ok_and(|response| {
            response["error"]["code"].as_str() == Some("bio_policy")
                || response["error"]["message"].as_str().is_some_and(blocked)
        })
}

impl Fault {
    fn of(error: &codex_app_server_protocol::TurnError) -> Fault {
        use codex_app_server_protocol::CodexErrorInfo;
        // The oracle's order (`handle_non_retry_error`): cyber policy, biology policy, then the
        // overload that said nothing.
        if error.codex_error_info == Some(CodexErrorInfo::CyberPolicy) {
            Fault::CyberPolicy
        } else if is_bio_policy_error(&error.message) {
            Fault::SafetyAccessBlock
        } else if matches!(
            error.codex_error_info,
            Some(CodexErrorInfo::ServerOverloaded | CodexErrorInfo::PolicyCapacityUnavailable)
        ) && error.message.trim().is_empty()
        {
            Fault::HighLoad
        } else {
            Fault::Message(error.message.clone())
        }
    }

    /// The row of the turn; a cyber refusal is worded by `daybreak`, the setting of the moment.
    fn item(&self, daybreak: Daybreak) -> Item {
        match self {
            Fault::Message(message) => Item::Error {
                message: message.clone(),
            },
            Fault::HighLoad => Item::HighLoad,
            Fault::CyberPolicy => Item::Notice(Notice::CyberPolicy(daybreak)),
            Fault::SafetyAccessBlock => Item::Notice(Notice::SafetyAccessBlock),
        }
    }

    /// Whether `item` is this fault's row.
    fn is(&self, item: &Item) -> bool {
        match (self, item) {
            (Fault::Message(message), Item::Error { message: held }) => message == held,
            (Fault::HighLoad, Item::HighLoad) => true,
            (Fault::CyberPolicy, Item::Notice(Notice::CyberPolicy(_))) => true,
            (Fault::SafetyAccessBlock, Item::Notice(Notice::SafetyAccessBlock)) => true,
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

/// The server lists newest turns first. Project them in transcript order (`replay`: the review
/// prompts and hidden nested review turns are left out, the submissions are kept).
pub fn history_page(turns: &[codex_app_server_protocol::Turn], next: Option<String>) -> ThreadEv {
    let (turns, submissions) = replay::project(turns);
    ThreadEv::HistoryPage {
        turns,
        next,
        submissions,
    }
}

fn item_entry(payload: &ThreadItemPayload, completed: bool) -> Option<(ItemId, Item)> {
    Some(match payload {
        ThreadItemPayload::UserMessage { id, content, .. } => {
            (ItemId::new(id), Item::User(UserPrompt::from_input(content)))
        }
        ThreadItemPayload::AgentMessage {
            id, text, phase, ..
        } => (
            ItemId::new(id),
            Item::Agent {
                text: text.clone(),
                phase: phase.clone(),
            },
        ),
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
            // the edit of a past prompt waits for this (`ThreadReverted`)
            screens::apply_thread_ev(
                &mut model.slots,
                ScreenThreadEv::Backtrack(screens::backtrack::ThreadEv::Reverted),
                fx,
            );
            model.transcript.reset();
            model.history_next = None;
            model.history_ask = HistoryAsk::Idle;
            Rc::make_mut(&mut model.finished_turns).clear();
            Rc::make_mut(&mut model.authoritative_turns).clear();
            model.completed_items.clear();
            model.last_ended = None;
        }
        ThreadEv::Failure(error) => model.last_error = Some(error),
        ThreadEv::Goal(goal, budget_turn) => {
            model.goal = goal;
            if let Some(turn) = budget_turn {
                model.slots.chat.mark_budget_limited(turn);
            }
        }
        ThreadEv::HistoryPage {
            turns,
            next,
            submissions,
        } => {
            for snapshot in &turns {
                if snapshot.end.is_some() {
                    if matches!(&model.turn, TurnState::Running(running) if running.turn == snapshot.turn)
                    {
                        let end = snapshot.end.unwrap_or(TurnEnd::Completed);
                        end_turn(model, snapshot.turn.clone(), end, fx);
                    } else {
                        Rc::make_mut(&mut model.finished_turns).insert(snapshot.turn.clone());
                    }
                    if snapshot.end == Some(TurnEnd::Completed) {
                        model.transcript.mark_completed(&snapshot.turn);
                    }
                    if matches!(snapshot.items, TurnSnapshot::Full(_)) {
                        Rc::make_mut(&mut model.authoritative_turns).insert(snapshot.turn.clone());
                    }
                } else {
                    start_turn(model, snapshot.turn.clone(), fx.now());
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
            model.transcript.prepend_snapshots(turns, fx.daybreak());
            model.submissions.prepend(submissions);
            model.transcript.remember_timings(timings);
            model.history_next = next.filter(|cursor| Some(cursor) != model.history_next.as_ref());
            model.history_ask = HistoryAsk::Idle;
        }
        ThreadEv::HistoryRequested => model.history_ask = HistoryAsk::Loading,
        // Without a cursor there is no page to retry: the first read's failure is the
        // session's, not the history's.
        ThreadEv::HistoryFailed if model.history_next.is_some() => {
            model.history_ask = HistoryAsk::Failed;
        }
        ThreadEv::HistoryFailed => model.history_ask = HistoryAsk::Idle,
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
            // The plan's text streams: Enter queues the message behind the turn
            // (`is_plan_streaming_in_tui`).
            if model.slots.chat.plan.streaming() && model.is_running() {
                enqueue(model, Queued::Message(message), fx);
                return;
            }
            if let (TurnState::Running(running), false) = (&model.turn, starting) {
                let turn = running.turn.clone();
                model.slots.chat.plan.steered();
                model.pending_steers.push(PendingSteer {
                    message: message.clone(),
                    admitted: None,
                });
                model.last_submit = Some(steer(fx, turn, false, message));
                return;
            }
            // MCP servers still starting hold the message (oracle
            // `maybe_send_next_queued_input`).
            let behind = !model.queue.is_empty() && !model.recovered;
            if starting || model.slots.chat.mcp.is_starting() || behind {
                enqueue(model, Queued::Message(message), fx);
                return;
            }
            model.last_error = None;
            match fx.start_turn(message) {
                Ok(ticket) => model.last_submit = Some(ticket),
                // a start was said in this step already: the message waits behind it
                Err(message) => enqueue(model, Queued::Message(message), fx),
            }
        }
        ThreadEv::Submit { .. } => {}
        ThreadEv::Interrupt if model.can_interrupt() => {
            if let TurnState::Running(running) = &mut model.turn {
                if !running.interrupt_sent {
                    running.interrupt_sent = true;
                    fx.interrupt(running.turn.clone(), false);
                }
            } else if let Some(ThreadCtx::StartTurn { interrupt, .. }) =
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
                // The fresh thread has the approved plan: its prompt starts the work.
                (
                    ThreadCtx::Screen(ScreenCtx::Chat(screens::chat::Ctx::ApprovePlan)),
                    Answer::Screen(screens::ScreenReply::Chat(screens::chat::Reply::Prepared(
                        prompt,
                    ))),
                ) => apply_ev(
                    model,
                    ThreadEv::Submit {
                        message: UserMessage::text(prompt),
                    },
                    fx,
                ),
                (ThreadCtx::StartTurn { interrupt, .. }, Answer::TurnStarted { turn, .. }) => {
                    // After a `Lagged` gap the end of this turn can have been
                    // seen before this reply: nothing will end it again, so
                    // the next queued message goes out now.
                    let over = model.finished_turns.contains(&turn);
                    start_turn(model, turn, fx.now());
                    if over {
                        send_next(model, fx);
                    } else if interrupt {
                        apply_ev(model, ThreadEv::Interrupt, fx);
                    }
                }
                // The server refused the thread for its model provider: the message
                // waits first in the queue, nothing goes out (`show_provider_error`).
                (
                    ThreadCtx::StartTurn { message, .. },
                    Answer::Failed(RpcError::Provider(error)),
                ) => {
                    model.queue.insert(0, Queued::Message(message));
                    model.slots.chat.provider_error = Some(error);
                }
                // Its images could not be prepared: the message never left, it goes back
                // whole to the composer and the queue waits (`on_images_prepared`).
                (ThreadCtx::StartTurn { message, .. }, Answer::Failed(RpcError::Prepare(text))) => {
                    model.last_error = Some(Failure::Message(text));
                    recover(model, message);
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
                    if let TurnState::Running(running) = &mut model.core.turn
                        && running.turn == turn
                        && !model.core.finished_turns.contains(&actual)
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
                (ThreadCtx::Feature(ctx), Answer::Feature(reply)) => {
                    feature_reply(model, ctx, Ok(reply), fx);
                }
                (ThreadCtx::Feature(ctx), Answer::Failed(error)) => {
                    feature_reply(model, ctx, Err(error), fx);
                }
                _ => {}
            }
        }
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
            screens::chat::ThreadIntent::InterruptForSteers,
        )) if model.has_writer() => {
            if !model.can_interrupt() {
                return;
            }
            let waiting = !model.pending_steers.is_empty() || !model.rejected_steers.is_empty();
            if model.slots.chat.review_mode() && waiting {
                // a review turn takes no steer: say so, and keep going
                apply_appended(
                    model,
                    None,
                    Item::LocalWarning(LocalWarning::ReviewSteerUnavailable),
                );
            } else if !model.pending_steers.is_empty() {
                model.steers_after_interrupt = true;
                apply_ev(model, ThreadEv::Interrupt, fx);
            }
        }
        // a line of the chat's own: a read-only thread says it too
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
            screens::chat::ThreadIntent::NoPreviousMessage,
        )) => apply_appended(model, None, Item::Notice(Notice::NoPreviousMessage)),
        // The prompt's Yes: Default mode, then the message that starts the work.
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
            screens::chat::ThreadIntent::ImplementPlan,
        )) if model.has_writer() => {
            if model.is_running() || model.outgoing.any(is_start_turn) {
                let note = Note::from(NoteKey::ModeBusy);
                apply_appended(model, None, Item::Note(note));
                return;
            }
            // the row is disabled (`Default mode unavailable`) when the catalog has no Default
            let Some(mask) = screens::select::collaboration::default_mode_mask(fx.modes()) else {
                return;
            };
            switch_mode(model, &mask, fx);
            let message = UserMessage::text(screens::chat::plan_mode::IMPLEMENT_MESSAGE);
            apply_ev(model, ThreadEv::Submit { message }, fx);
        }
        // `/plan`, with the preset the app chose.
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(screens::chat::ThreadIntent::SetMask(
            mask,
        ))) if model.has_writer() => switch_mode(model, &mask, fx),
        // Shift+Tab (`cycle_collaboration_mode`): the next preset of the server's list, while no
        // turn runs and none is about to start.
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
            screens::chat::ThreadIntent::CycleMode,
        )) if model.has_writer() => {
            let busy = model.is_running() || model.outgoing.any(is_start_turn);
            let current = Some(model.slots.chat.mode());
            if !busy
                && let Some(mask) = screens::select::collaboration::next_mask(fx.modes(), current)
            {
                switch_mode(model, &mask, fx);
            }
        }
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(
            screens::chat::ThreadIntent::CleanTerminals,
        )) if model.has_writer() => {
            model.terminals.clear();
            let busy = model.is_running() || model.outgoing.any(is_start_turn);
            screens::apply_thread_intent(
                &mut model.slots,
                ScreenThreadIntent::Chat(screens::chat::ThreadIntent::CleanTerminals),
                busy,
                fx,
            );
        }
        // the continue key of a refused thread: the text waits in the fresh thread's composer
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(screens::chat::ThreadIntent::Draft(
            text,
        ))) => give_back(model, vec![UserMessage::text(text)], true),
        ThreadEv::FeatureIntent(intent) => with_features(model, fx, |parts, cx| {
            features::apply_thread_intent(parts, intent, cx);
        }),
        // The queue's five intents (Tab, the screen's report of a command it ran, what stopped
        // the queue is over, the hold and the edit key) are `threads/queue.rs`'s; it gives back
        // the others, which are the screens'.
        ThreadEv::ScreenIntent(intent) if model.has_writer() => {
            if let Err(intent) = queue::apply(model, intent, fx) {
                let busy = model.is_running() || model.outgoing.any(is_start_turn);
                screens::apply_thread_intent(&mut model.slots, intent, busy, fx);
            }
        }
        ThreadEv::ScreenIntent(_) => {}
        ThreadEv::Preview(texts) => {
            model.pending_steers = texts
                .steers
                .into_iter()
                .map(|text| PendingSteer {
                    message: UserMessage::text(text),
                    admitted: None,
                })
                .collect();
            model.rejected_steers = texts.rejected.into_iter().map(UserMessage::text).collect();
            model.queue = texts
                .queued
                .into_iter()
                .map(|text| Queued::Message(UserMessage::text(text)))
                .collect();
        }
        ThreadEv::Screen(ev) => {
            if let ScreenThreadEv::Chat(ref ev) = ev
                && let Some(turn) = ev.turn()
                && !matches!(&model.turn, TurnState::Running(running) if &running.turn == turn)
            {
                return;
            }
            // The card of a `/status` is a cell of the transcript: its refresh ends there.
            if let ScreenThreadEv::Status(screens::status::ThreadEv::Refreshed {
                request,
                limits,
            }) = &ev
            {
                model.transcript.finish_status(*request, limits.clone());
            }
            screens::apply_thread_ev(&mut model.slots, ev, fx);
            append_history(model);
        }
        ThreadEv::TurnError { turn, fault } => {
            // Oracle `on_error`: the turn is finalized, then the error row, then
            // the next queued input; the failed turn's own completion is a no-op.
            end_turn(model, turn.clone(), TurnEnd::Failed, fx);
            model.transcript.report_error(&turn, &fault, fx.daybreak());
        }
        ThreadEv::TurnStarted { turn, started_at } => {
            start_turn(model, turn.clone(), fx.now());
            if let TurnState::Running(running) = &mut model.turn
                && running.turn == turn
                && started_at.is_some()
            {
                running.started_at = started_at;
            }
            if model.outgoing.any(|ctx| {
                matches!(
                    ctx,
                    ThreadCtx::StartTurn {
                        interrupt: true,
                        ..
                    }
                )
            }) {
                apply_ev(model, ThreadEv::Interrupt, fx);
            }
        }
        ThreadEv::HookStarted { id, message } => {
            model.hooks.retain(|hook| hook.id != id);
            model.hooks.push(RunningHook { id, message });
        }
        ThreadEv::HookEnded { id, turn, item } => {
            model.hooks.retain(|hook| hook.id != id);
            if let Some(item) = item {
                apply_appended(model, turn, item);
            }
        }
        ThreadEv::ItemStarted { key, item } => {
            track_terminal(model, &key, &item);
            note_review(model, &item);
            if !model.finished_turns.contains(&key.turn) && model.transcript.item(&key).is_none() {
                let echo = steer_echoed(model, &item);
                if !review_prompt(model, &item, echo) {
                    model.transcript.upsert(key, item);
                }
            }
        }
        ThreadEv::ItemUpsert { key, item } => {
            track_terminal(model, &key, &item);
            note_review(model, &item);
            if !model.authoritative_turns.contains(&key.turn) {
                let echo = model.transcript.item(&key).is_none() && steer_echoed(model, &item);
                if review_prompt(model, &item, echo) {
                    return;
                }
                if !model.finished_turns.contains(&key.turn) && matches!(item, Item::Agent { .. }) {
                    model.completed_items.insert(key.clone());
                }
                if let Item::Plan { text } = &item
                    && !model.finished_turns.contains(&key.turn)
                {
                    model.slots.chat.plan.plan_completed(text);
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
        ThreadEv::PlanDelta { key, delta } => {
            if !model.finished_turns.contains(&key.turn) && !model.completed_items.contains(&key) {
                model.slots.chat.plan.plan_delta();
                model.transcript.append_plan(key, &delta);
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
                    for (id, item) in &items {
                        let key = ItemKey {
                            turn: turn.clone(),
                            item: id.clone(),
                        };
                        track_terminal(model, &key, item);
                    }
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
            let timing = live_timing(model, &turn, end, timing, fx.now());
            model.transcript.remember_timings([(turn.clone(), timing)]);
            if let Some(fault) = &error {
                model.transcript.report_error(&turn, fault, fx.daybreak());
            }
            end_turn(model, turn, end, fx);
        }
    }
}

/// The answer of a feature's RPC on this thread.
fn feature_reply(
    model: &mut ThreadModel,
    ctx: FeatureCtx,
    result: Result<features::FeatureReply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    // An RPC of a feature that failed is the thread's last failure, as a turn start's is: the
    // feature words what else it wants said (a note), and need not set the error line itself.
    if let Err(error) = &result {
        model.last_error = Some(Failure::from(error.clone()));
    }
    with_features(model, fx, |parts, cx| {
        features::apply_thread_reply(parts, ctx, result, cx);
    });
}

/// The lifecycle hooks of the features (ADR-012), in list order, after the thread's own
/// turn-end work.
fn feature_turn_ended(model: &mut ThreadModel, end: TurnEnd, fx: &mut ThreadFx<'_>) {
    with_features(model, fx, |parts, cx| features::on_turn_end(parts, end, cx));
}

fn feature_interrupted(model: &mut ThreadModel, fx: &mut ThreadFx<'_>) {
    with_features(model, fx, features::on_interrupted);
}

/// Runs one step of the features over this thread: their parts `&mut`, the rest read-only.
/// A reply or a lifecycle fact reaches the features whether or not this process still writes
/// the thread (a part that waits for it must not stay stuck when a provider refusal took the
/// writer); the effects a step has are the writer's (ADR-003: `ThreadCx::request` is shut
/// without it, and an ask, `Feature::thread_intent`, is not delivered).
fn with_features(
    model: &mut ThreadModel,
    fx: &mut ThreadFx<'_>,
    step: impl FnOnce(&mut features::FeatureThreadParts, &mut ThreadCx<'_, '_>),
) {
    let (parts, view) = model.split_features();
    step(parts, &mut ThreadCx::new(view, fx));
}

/// A unified-exec command that runs is a background terminal until its item
/// completes (oracle `track_unified_exec_process_begin` / `_end`).
fn track_terminal(model: &mut ThreadModel, key: &ItemKey, item: &Item) {
    let Item::Tool(tool) = item else {
        return;
    };
    let ToolKind::Shell {
        command,
        process_id,
        unified_exec: true,
        ..
    } = &tool.kind
    else {
        return;
    };
    let key = process_id
        .clone()
        .unwrap_or_else(|| key.item.as_str().to_string());
    if tool.status == ToolStatus::Running {
        match model
            .terminals
            .iter_mut()
            .find(|terminal| terminal.key == key)
        {
            Some(terminal) => terminal.command.clone_from(command),
            None => model.terminals.push(Terminal {
                key,
                command: command.clone(),
            }),
        }
    } else {
        model.terminals.retain(|terminal| terminal.key != key);
    }
}

/// A live completion the server timed only in part (oracle `FinalMessageSeparator::from_turn_once`):
/// the elapsed time falls back to the status timer (the clock since the turn was seen to start),
/// the completion time to the local clock.
fn live_timing(
    model: &ThreadModel,
    turn: &TurnId,
    end: TurnEnd,
    timing: Option<TurnTiming>,
    now: i64,
) -> Option<TurnTiming> {
    if end != TurnEnd::Completed {
        return timing;
    }
    // A completion of a turn the client never saw start is a restored one (no status timer ran);
    // with no clock yet (the host has not said) no time is invented either.
    let elapsed = match &model.turn {
        TurnState::Running(running) if running.turn == *turn && now > 0 => {
            let since = running
                .started_at
                .filter(|at| *at > 0)
                .unwrap_or(running.began);
            Some(now.saturating_sub(since).max(0) * 1_000)
        }
        _ => return timing,
    };
    let known = timing.unwrap_or(TurnTiming {
        duration_ms: None,
        completed_at: None,
    });
    let merged = TurnTiming {
        duration_ms: known.duration_ms.or(elapsed),
        completed_at: known.completed_at.or((now > 0).then_some(now)),
    };
    (merged.duration_ms.is_some() || merged.completed_at.is_some()).then_some(merged)
}

fn end_turn(model: &mut ThreadModel, turn: TurnId, end: TurnEnd, fx: &mut ThreadFx<'_>) {
    // The reviews of the turn are over with it (oracle `on_task_complete`).
    let over = ScreenThreadEv::Approvals(screens::approvals::ThreadEv::TurnEnded);
    screens::apply_thread_ev(&mut model.slots, over, fx);
    Rc::make_mut(&mut model.finished_turns).insert(turn.clone());
    if end == TurnEnd::Completed {
        model.transcript.mark_completed(&turn);
    }
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
        // `maybe_prompt_plan_implementation`: nothing waits to be sent behind the turn.
        let waiting = !model.queue.is_empty()
            || !model.pending_steers.is_empty()
            || !model.rejected_steers.is_empty();
        let mode = model.slots.chat.mode();
        model
            .slots
            .chat
            .plan
            .turn_ended(mode, end == TurnEnd::Completed, waiting);
        model.slots.chat.end_turn();
        if end == TurnEnd::Interrupted {
            interrupted(model, &turn, fx);
        } else if !model.outgoing.any(is_start_turn) {
            send_next(model, fx);
        }
        // The features' hooks run after the thread's own work, in list order (ADR-012).
        feature_turn_ended(model, end, fx);
        if end == TurnEnd::Interrupted {
            feature_interrupted(model, fx);
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
    // `on_interrupted_turn`: the steer notice when Esc sent the steers, else
    // the budget's message when the goal stopped the turn, else the plain one.
    let budget = model.slots.chat.take_budget_limited(turn);
    let notice = if submit {
        Notice::InterruptedForSteer
    } else if budget {
        Notice::GoalBudgetReached
    } else {
        Notice::Interrupted
    };
    apply_appended(model, Some(turn.clone()), Item::Notice(notice));
    if submit && !mine.is_empty() {
        let merged = merge_messages(mine.into_iter().map(|steer| steer.message).collect());
        if let Err(merged) = fx.start_turn(merged) {
            // refused (a start was said in this step): the next `send_next` sends it first
            model.rejected_steers.push(merged);
        }
    } else {
        let mut messages = std::mem::take(&mut model.rejected_steers);
        messages.extend(mine.into_iter().map(|steer| steer.message));
        messages.extend(model.queue.drain(..).map(Queued::into_message));
        give_back(model, messages, false);
    }
    model.pending_steers = later;
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

/// A message that never left goes back to the composer whole; what waits in
/// the queue stays there, unsent (oracle `recovered_queue`).
fn recover(model: &mut ThreadModel, message: UserMessage) {
    model.recovered |= !model.queue.is_empty() || !model.rejected_steers.is_empty();
    give_back(model, vec![message], false);
}

/// The user's switch to the mode `mask` makes: the thread goes on in it, the server is told, and
/// a change of the model or the effort that came with it is said
/// (`set_collaboration_mask_from_user_action`).
fn switch_mode(
    model: &mut ThreadModel,
    mask: &screens::select::collaboration::Mask,
    fx: &mut ThreadFx<'_>,
) {
    if let Some(change) = model.slots.chat.set_mask(mask, fx) {
        let note = Note::from(NoteKey::Mode(ModeNote::ModelChanged(change)));
        apply_appended(model, None, Item::Note(note));
    }
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
fn steer_echoed(model: &mut ThreadModel, item: &Item) -> bool {
    if let Item::User(prompt) = item
        && let Some(index) = model
            .pending_steers
            .iter()
            .position(|steer| steer.message.text == prompt.text)
    {
        model.pending_steers.remove(index);
        return true;
    }
    false
}

/// In review mode the server's user item is the review prompt, not something
/// the user typed: it adds no row unless it is a steer's echo
/// (`on_committed_user_message`: `!self.review.is_review_mode`).
fn review_prompt(model: &ThreadModel, item: &Item, echo: bool) -> bool {
    matches!(item, Item::User(_)) && !echo && model.slots.chat.review_mode()
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
        // the steer never left: it is no pending steer, its message is the composer's again
        Answer::Failed(RpcError::Prepare(text)) => {
            if let Some(index) = position {
                model.pending_steers.remove(index);
            }
            model.last_error = Some(Failure::Message(text));
            recover(model, message);
            return;
        }
        // only the refused steer goes back: it is first to go out again
        Answer::Failed(RpcError::Provider(error)) => {
            if let Some(index) = position {
                let steer = model.pending_steers.remove(index);
                model.rejected_steers.push(steer.message);
            }
            model.slots.chat.provider_error = Some(error);
            return;
        }
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
            if let Err(message) = fx.start_turn(message) {
                // refused (a start was said in this step): the next `send_next` sends it first
                model.rejected_steers.push(message);
            }
        }
        Refusal::Changed(actual) if !retried && actual != turn => {
            if let TurnState::Running(running) = &mut model.core.turn
                && running.turn == turn
                && !model.core.finished_turns.contains(&actual)
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
        for mention in message.mentions {
            if !merged.mentions.contains(&mention) {
                merged.mentions.push(mention);
            }
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

/// What the approvals slot owes the history (`History`: decisions, the reviewer's warnings and
/// the `/auto-review` lines), appended in the order it came.
fn append_history(model: &mut ThreadModel) {
    for history in model.slots.approvals.take_history() {
        apply_appended(model, None, decision::item_of(history));
    }
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

fn start_turn(model: &mut ThreadModel, turn: TurnId, now: i64) {
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
                began: now,
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
