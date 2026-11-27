//! `ThreadModel` and `apply` (ADR-003 R3-R5, R9): one thread's data, changed
//! only here. A thread RPC is requested through `ThreadFx`, which stamps the
//! thread id.

mod previews;

pub use previews::{ThreadPreview, ThreadPreviews};

mod activity;
pub mod asks;
pub mod calls;
mod cells;
pub mod citations;
mod collab;
pub mod compaction;
pub mod composer;
pub mod daybreak;
pub mod decision;
pub mod directives;
pub mod exploration;
pub mod followups;
pub mod goal;
pub mod hooks;
pub mod inline_vis;
mod live_output;
pub mod local_links;
mod message;
mod note;
pub mod processes;
pub mod queue;
pub mod replay;
pub mod safety;
pub mod startup;
pub mod status;
pub mod stream_preview;
pub mod structured;
mod user_prompt;
pub mod voice_transcript;

pub use activity::{
    BlockBody, DynamicCall, DynamicOutput, ExplorationAction, McpBlock, McpCall, McpResult,
    McpResultKind, ToolActivity, ToolKind, ToolStatus,
};
pub use asks::{HISTORY_PAGE, ThreadAsk, ThreadReply, cloud_submit_params};
pub use calls::ShellClass;
pub use cells::{
    DelegatedMessage, GeneratedImage, HookEntry, HookEntryKind, HookRun, HookStatus, Notice,
    PatchActivity, PatchChange, PatchKind, PlanStep, PlanUpdate, StepStatus, WebAction,
    WebActivity,
};
pub use codex_protocol::models::MessagePhase;
pub use collab::{
    AgentOutcome, AgentRef, CollabEvent, SpawnRequest, SubAgentKind, requested_spawn,
};
pub use decision::ReviewLine;
pub use goal::{Goal, GoalStatus, ReviewTarget, SettingsPatch, goal_of_response};
pub use live_output::{OutputLine, RetainedOutput};
pub use message::{LocalImage, TaskRef, ToolMention, UserMessage};
pub use note::{CdRefusal, ModeNote, Note, NoteKey, RemoveNote, Stub, Toggle, Tone};
pub use processes::{Listed, Terminal, WaitStreak};
pub use queue::{DueCommand, Queued, Ran};
use queue::{enqueue, send_next};
pub(crate) use user_prompt::without_controls;
pub use user_prompt::{Shown, UserPrompt, request_text};

use std::collections::{BTreeMap, BTreeSet};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::TurnItemsView;
use codex_app_server_protocol::TurnStatus;

use crate::app::AppFx;
use crate::app::Command;
use crate::app::Now;
use crate::features::{self, FeatureCtx, FeatureThreadIntent, ThreadCx, ThreadView};
use crate::ids::ItemId;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TicketMint;
use crate::ids::TurnId;
use crate::rpc::Pending;
use crate::rpc::Reply;
use crate::rpc::RpcError;
use crate::rpc::{Address, Answer};
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
    VoiceTranscript {
        role: voice_transcript::Role,
        text: String,
    },
    /// An assistant message. `phase` is what the server said about it (oracle
    /// `AgentMarkdownCell::phase`): none for a streamed message or a legacy model, so
    /// callers treat none as "unknown".
    Agent {
        text: String,
        phase: Option<MessagePhase>,
        questions: Option<Vec<crate::question_reply::QuestionText>>,
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
    Voice(voice_transcript::Role),
}

impl Text {
    fn empty(self) -> Item {
        match self {
            Text::Voice(role) => Item::VoiceTranscript {
                role,
                text: String::new(),
            },
            Text::Agent => Item::Agent {
                text: String::new(),
                phase: None,
                questions: None,
            },
            Text::Plan => Item::Plan {
                text: String::new(),
            },
        }
    }

    fn text_of(self, item: &Item) -> Option<&String> {
        match (self, item) {
            (Text::Agent, Item::Agent { text, .. })
            | (Text::Plan, Item::Plan { text })
            | (Text::Voice(_), Item::VoiceTranscript { text, .. }) => Some(text),
            _ => None,
        }
    }

    fn text_mut(self, item: &mut Item) -> Option<&mut String> {
        match (self, item) {
            (Text::Agent, Item::Agent { text, .. })
            | (Text::Plan, Item::Plan { text })
            | (Text::Voice(_), Item::VoiceTranscript { text, .. }) => Some(text),
            _ => None,
        }
    }
}

/// The ordered transcript: entries keyed `(TurnId, ItemId)`, kept in the
/// order they first arrived, changed only by upsert (R4). There is no `push`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transcript {
    order: Rc<Vec<ItemKey>>,
    items: Rc<HashMap<ItemKey, ItemEntry>>,
    layout_revision: u64,
    reset_revision: u64,
    item_revision: u64,
    activity_revision: u64,
    timings: Rc<HashMap<TurnId, TimedTurn>>,
    /// The turns that ended as completed (oracle: a turn with its `FinalMessageSeparator`),
    /// whatever time the server gave them.
    completed: Rc<HashSet<TurnId>>,
    // Keep streaming rows separate so updating a preview never copies history
    // held by the provider. There is still one ordered index for the transcript.
    streaming: HashMap<ItemKey, StreamingItem>,
    live_voice_items: HashMap<ItemId, voice_transcript::LiveCaption>,
    voice_replay: Rc<voice_transcript::Replay>,
    /// Entries the transcript keyed itself (`mint`): the server sent no item.
    minted: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StreamingItem {
    entry: ItemEntry,
    committed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ItemEntry {
    item: Rc<Item>,
    revision: u64,
}

impl ItemEntry {
    fn updated(before: Option<&Self>, item: Item, revision: &mut u64) -> Self {
        if let Some(before) = before
            && *before.item == item
        {
            return before.clone();
        }
        *revision += 1;
        Self {
            item: Rc::new(item),
            revision: *revision,
        }
    }
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
    /// Changes when the retained transcript is replaced from scratch.
    pub fn reset_revision(&self) -> u64 {
        self.reset_revision
    }

    /// Changes when the ordered keys or their exploration classification change.
    /// A streamed text or command output delta leaves the layout index valid.
    pub fn layout_revision(&self) -> u64 {
        self.layout_revision
    }

    /// Changes when a tool, patch or search is upserted with different content.
    /// Activity controls can also depend on the row layout and streamed output.
    pub fn activity_revision(&self) -> u64 {
        self.activity_revision
    }

    /// The preview line count of an output receiving shell command deltas.
    pub fn streamed_shell_output_line_count(&self, key: &ItemKey) -> Option<usize> {
        let Item::Tool(tool) = self.streaming.get(key)?.entry.item.as_ref() else {
            return None;
        };
        tool.streamed_output_line_counts()
            .map(|(_, current)| current)
    }

    /// The initial and current output line counts of shell commands receiving deltas.
    /// The counts leave off trailing CR/LF and are updated from the new bytes alone.
    pub fn streamed_shell_output_lines(
        &self,
    ) -> impl Iterator<Item = (&ItemKey, &ToolActivity, usize, usize)> {
        self.streaming.iter().filter_map(|(key, live)| {
            let Item::Tool(tool) = live.entry.item.as_ref() else {
                return None;
            };
            let (initial, current) = tool.streamed_output_line_counts()?;
            Some((key, tool.as_ref(), initial, current))
        })
    }

    /// The revision of one row's content, independent of its position.
    pub fn item_revision(&self, key: &ItemKey) -> Option<u64> {
        self.entry(key).map(|entry| entry.revision)
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// An immutable view of the ordered keys, independent of item contents.
    pub fn item_keys(&self) -> Rc<Vec<ItemKey>> {
        self.order.clone()
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

    /// Commands run in `turn` and every one of them is the user's own `!` command
    /// (`only_user_shell_commands_running`): what the user types meanwhile waits for the turn
    /// instead of steering it.
    fn only_user_shell_commands_running(&self, turn: &TurnId) -> bool {
        let mut any = false;
        for key in self.order.iter().filter(|key| &key.turn == turn) {
            let Some(Item::Tool(tool)) = self.item(key) else {
                continue;
            };
            let ToolKind::Shell { user_shell, .. } = &tool.kind else {
                continue;
            };
            if tool.status != ToolStatus::Running {
                continue;
            }
            if !*user_shell {
                return false;
            }
            any = true;
        }
        any
    }

    fn entry(&self, key: &ItemKey) -> Option<&ItemEntry> {
        self.streaming
            .get(key)
            .map(|live| &live.entry)
            .or_else(|| self.items.get(key))
    }

    fn item(&self, key: &ItemKey) -> Option<&Item> {
        self.entry(key).map(|entry| entry.item.as_ref())
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
        {
            Arc::make_mut(tool).keep_previous_output(previous);
        }
        let live = self.streaming.remove(&key);
        let before = live
            .as_ref()
            .map(|live| &live.entry)
            .or_else(|| self.items.get(&key));
        let entry = ItemEntry::updated(before, item, &mut self.item_revision);
        let activity = |item: &Item| matches!(item, Item::Tool(_) | Item::Patch(_) | Item::Web(_));
        if before.is_none_or(|before| before.revision != entry.revision)
            && (activity(&entry.item) || before.is_some_and(|before| activity(&before.item)))
        {
            self.activity_revision += 1;
        }
        let before = Rc::make_mut(&mut self.items).insert(key.clone(), entry);
        if live.is_none() && before.is_none() {
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

    /// An entry the server has no item for carries a local key.
    fn is_local(key: &ItemKey) -> bool {
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
        if !matches!(fault, Fault::Misalignment { .. }) && !self.holds_error(turn, fault) {
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
                let (value, committed) = match self.items.get(entry.key()) {
                    Some(item) => match kind.text_of(&item.item) {
                        Some(text) => (item.clone(), text.len()),
                        None => return,
                    },
                    None => {
                        Rc::make_mut(&mut self.order).push(entry.key().clone());
                        self.layout_revision += 1;
                        (
                            ItemEntry::updated(None, kind.empty(), &mut self.item_revision),
                            0,
                        )
                    }
                };
                entry.insert(StreamingItem {
                    entry: value,
                    committed,
                })
            }
        };
        if let Some(text) = kind.text_mut(Rc::make_mut(&mut live.entry.item)) {
            if let Some(last) = delta.rfind('\n') {
                live.committed = text.len() + last + 1;
            }
            text.push_str(delta);
            self.item_revision += 1;
            live.entry.revision = self.item_revision;
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
                let Item::Tool(tool) = item.item.as_ref() else {
                    return;
                };
                if tool.status != ToolStatus::Running {
                    return;
                }
                entry.insert(StreamingItem {
                    entry: item.clone(),
                    committed: 0,
                })
            }
        };
        if let Item::Tool(tool) = Rc::make_mut(&mut live.entry.item)
            && tool.status == ToolStatus::Running
        {
            Arc::make_mut(tool).append_output(delta);
            self.item_revision += 1;
            live.entry.revision = self.item_revision;
        }
    }

    fn finish_streams(&mut self, turn: &TurnId) {
        for (key, live) in self.streaming.extract_if(|key, _| &key.turn == turn) {
            Rc::make_mut(&mut self.items).insert(key, live.entry);
        }
    }

    /// The server's history became shorter (`thread/revert`): every row and
    /// every timing is dropped; the retained turns are read again.
    fn reset(&mut self) {
        *self = Transcript {
            layout_revision: self.layout_revision + 1,
            reset_revision: self.reset_revision.wrapping_add(1),
            item_revision: self.item_revision,
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
        let mut replaced = HashMap::new();
        order.retain(|key| {
            if full.contains(&key.turn) {
                if let Some(item) = stored.remove(key) {
                    replaced.insert(key.clone(), item);
                }
                false
            } else {
                true
            }
        });
        replaced.extend(
            self.streaming
                .extract_if(|key, _| full.contains(&key.turn))
                .map(|(key, live)| (key, live.entry)),
        );
        let mut older = Vec::new();
        for HistoryTurn {
            turn, items, error, ..
        } in snapshots
        {
            let items = match items {
                TurnSnapshot::Full(items) | TurnSnapshot::Summary(items) => items,
            };
            for (item, value) in items {
                // a replayed unified-exec startup draws nothing, as when it started live
                // (oracle `on_command_execution_started`); the restore lists it if it still runs
                if processes::draws_nothing(&value, false) {
                    continue;
                }
                let key = ItemKey {
                    turn: turn.clone(),
                    item,
                };
                if !self.streaming.contains_key(&key)
                    && let std::collections::hash_map::Entry::Vacant(entry) =
                        stored.entry(key.clone())
                {
                    entry.insert(ItemEntry::updated(
                        replaced.get(&key),
                        value,
                        &mut self.item_revision,
                    ));
                    older.push(key);
                }
            }
            // The turn's own error follows its items (oracle `thread_transcript`).
            if let Some(fault) = error
                && !matches!(fault, Fault::Misalignment { .. })
                && !order.iter().chain(older.iter()).any(|key| {
                    key.turn == turn && stored.get(key).is_some_and(|held| fault.is(&held.item))
                })
            {
                self.minted += 1;
                let key = ItemKey {
                    turn: turn.clone(),
                    item: ItemId::new(format!("#{}", self.minted)),
                };
                stored.insert(
                    key.clone(),
                    ItemEntry::updated(None, fault.item(daybreak), &mut self.item_revision),
                );
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
        let live: HashMap<_, _> = self
            .streaming
            .extract_if(|key, _| &key.turn == turn)
            .map(|(key, live)| (key, live.entry))
            .collect();
        let mut replaced = HashMap::new();
        let history = Rc::make_mut(&mut self.items);
        let order = Rc::make_mut(&mut self.order);
        let first = order
            .iter()
            .position(|key| &key.turn == turn)
            .unwrap_or(order.len());
        // The server's items only: an entry it has no item for (an error, a hook run) is ours.
        order.retain(|key| {
            if &key.turn == turn && !Self::is_local(key) {
                if let Some(item) = history.remove(key) {
                    replaced.insert(key.clone(), item);
                }
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
            let before = live.get(&key).or_else(|| replaced.get(&key));
            let entry = ItemEntry::updated(before, entry, &mut self.item_revision);
            if history.insert(key.clone(), entry).is_none() {
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalWarning {
    /// Esc with steers waiting while a review runs: a review turn takes no
    /// steer (`REVIEW_STEER_UNAVAILABLE_MESSAGE`).
    ReviewSteerUnavailable,
    InvalidStatusLineItems(Vec<String>),
    InvalidTitleItems(Vec<String>),
}

/// How a turn ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnEnd {
    Completed,
    Interrupted,
    Failed,
}

/// The active turn ended on the live stream, so the page can retire its questions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveTurnEnd {
    pub turn: TurnId,
    /// A misalignment stop discards the question drafts.
    pub recover_questions: bool,
    /// Input the same live terminal event restored before the questions.
    pub restore: Option<Restore>,
}

/// A live fact the thread has applied to the owning model.
#[derive(Clone, Copy, Debug)]
pub enum LiveThreadEvent<'a> {
    ItemCompleted { key: &'a ItemKey, item: &'a Item },
    TurnEnded(&'a LiveTurnEnd),
}

/// A message sent to the running turn (`turn/steer`) that the server has not
/// yet echoed as the user's item (the oracle's `PendingSteer`): the pending-
/// input preview lists it until then.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingSteer {
    message: UserMessage,
    submission: Option<(Ticket, String)>,
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
        /// Cleared when an item confirms the submission or stream loss moves it to the queue.
        client_id: Option<String>,
        /// The instruction a misalignment failure's review was acknowledged for: its failure
        /// words nothing of the message (`safety::Verb::Continue`).
        acknowledged: bool,
    },
    Interrupt {
        turn: TurnId,
        retried: bool,
    },
    Screen(ScreenCtx),
    /// An RPC of a feature slice (`ThreadCx::request`).
    Feature(FeatureCtx),
}

/// What a command of the turn streams (`item/commandExecution/outputDelta`) or is written to
/// (`item/commandExecution/terminalInteraction`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandEv {
    /// Output of the command `key`.
    Output { key: ItemKey, delta: String },
    /// The model polled a background terminal (empty `stdin`) or wrote to it.
    Interaction { process: String, stdin: String },
    /// The thread was resumed and its history is in: ask the server for its live background
    /// terminals (oracle `begin_background_terminal_restore` + `list_background_terminals`).
    Restore,
}

/// What the thread is told of the asks made before it started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Deferred {
    /// A startup draft was accepted while the thread is pending: it waits on the blank thread's
    /// queue, drawn as a queued row, until the session starts.
    Submit(Queued),
    /// The session started and what waited went out: the blank thread's queue is empty again.
    Sent,
    /// The launch's prompt went out: the composer's recall keeps it (`record_local_submission`).
    Recorded(UserMessage),
}

/// The submission ID echoed by a live user item, or the content key of a legacy completion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserItemEcho {
    ClientId(String),
    Legacy { text: String, images: usize },
}

/// An event of one thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {
    /// What was asked before the thread started (`Deferred`).
    Deferred(Deferred),
    /// The thread tells (`false`) or does not tell (`true`) its transcript that a turn was
    /// interrupted: a side conversation keeps it out (`InterruptedTurnNoticeMode::Suppress`).
    QuietInterrupts(bool),
    ConnectionLost,
    /// The thread's event stream was lost while the connection stands (the server's `thread
    /// stream disconnected`): what the user sends waits in the queue until it is restored
    /// (oracle `thread_stream_disconnected`, `chatwidget/input_flow.rs`).
    StreamLost,
    /// The thread's event stream is back: the queue goes on if no turn runs. `read` is false when
    /// the host brought it back (what the gap held is unread, the history may be incomplete) and
    /// true when the thread was attached again and its history read (oracle: a resume of a thread
    /// whose stream was lost or came back).
    StreamRestored {
        read: bool,
        reconciled: bool,
    },
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
    VoiceCaption {
        event: voice_transcript::Event,
        show_user_preview: bool,
    },
    VoiceHistorySupported(bool),
    ReadVoiceHistory {
        oldest: Option<TurnId>,
        more_history: bool,
    },
    LoadOlderVoiceHistory,
    Reply {
        ticket: Ticket,
        reply: Reply,
    },
    /// A page of the thread's history (`thread/turns/list`, newest page
    /// first): the entries go before what the transcript has. A `replace`ing page first drops
    /// what is held (the thread is read again: a revert, a gap).
    HistoryPage {
        turns: Vec<HistoryTurn>,
        next: Option<String>,
        submissions: Vec<replay::Submission>,
        receipts: Vec<String>,
        replace: bool,
        /// The thread runs (the server's status is `active`, or the page is not the resume's
        /// own): a turn without an end is the running one. An idle thread's unfinished turn
        /// never recorded its end and starts nothing until a live `turn/started`.
        running: bool,
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
        receipt: Option<UserItemEcho>,
    },
    ItemUpsert {
        key: ItemKey,
        item: Item,
        receipt: Option<UserItemEcho>,
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
    /// What a command streams or is written to (`CommandEv`).
    Command(CommandEv),
    TurnEnded {
        turn: TurnId,
        end: TurnEnd,
    },
    TurnCompleted {
        turn: TurnId,
        end: TurnEnd,
        snapshot: Option<TurnSnapshot>,
        receipts: Vec<String>,
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
            // what the user says to the safety state is no news of the server
            Self::Screen(ScreenThreadEv::Chat(screens::chat::ThreadEv::Safety(
                safety::SafetyEv::Verb(_),
            ))) => false,
            Self::TurnStarted { .. }
            | Self::ItemStarted { .. }
            | Self::ItemUpsert { .. }
            | Self::PlanUpdated { .. }
            | Self::Appended { .. }
            | Self::HookStarted { .. }
            | Self::HookEnded { .. }
            | Self::AgentDelta { .. }
            | Self::PlanDelta { .. }
            | Self::Command(_)
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
    /// A message the composer did not type that went out for the user (the launch's prompt): its
    /// number and the message, for the composer's recall (`Edge::Recorded`).
    recorded: Option<(u64, UserMessage)>,
    /// Lines the queue key set aside while a turn runs: one message goes out per turn end, in
    /// order; a command or a shell line is run when its turn comes.
    queue: Vec<Queued>,
    /// A queued command the screen has to run (and report with `ThreadIntent::QueueEvent`): the
    /// queue waits for the answer.
    due: Option<DueCommand>,
    /// The modal operations that still hold queued input.
    queue_modals: BTreeMap<u64, queue::ModalPhase>,
    /// Steers sent to the running turn and not yet echoed, oldest first.
    pending_steers: Vec<PendingSteer>,
    /// Steers the server refused (a review or compact turn): they go out first
    /// when the turn ends, merged into one message.
    rejected_steers: Vec<UserMessage>,
    /// Esc with steers pending: the interrupt that follows sends them as one
    /// message (`submit_pending_steers_after_interrupt`).
    steers_after_interrupt: bool,
    /// An interrupted turn leaves no notice in the transcript (`QuietInterrupts`).
    quiet_interrupts: bool,
    /// Turns whose goal-budget stop was observed while this model was held.
    /// The protocol history cannot recover the reason after a transcript reset.
    budget_interruptions: Rc<HashSet<TurnId>>,
    /// The last messages given back to the composer.
    restore: Option<Restore>,
    /// A message went back to the composer while others wait (a refused image
    /// preparation): the queue sends nothing on its own until it is empty
    /// (oracle `recovered_queue`).
    recovered: bool,
    /// The unified-exec sessions that run in the background and the wait streak of their
    /// polls (oracle `unified_exec_processes`, `unified_exec_wait_streak`).
    processes: processes::Processes,
    /// The hook runs the server reported started and not yet completed.
    hooks: hooks::Hooks,
    last_error: Option<Failure>,
    /// The last turn that ended: a late start of it must not revive it.
    last_ended: Option<TurnId>,
    finished_turns: Rc<HashSet<TurnId>>,
    authoritative_turns: Rc<HashSet<TurnId>>,
    completed_items: HashSet<ItemKey>,
    /// Live legacy completion keys until the server confirms their turn ended. These survive
    /// transcript replacement while reattaching to the same running turn.
    legacy_user_completions: HashMap<TurnId, HashSet<ItemId>>,
    /// The cursor of the next older page of history; none once the oldest
    /// page was read.
    history_next: Option<String>,
    voice_history_supported: bool,
    /// The user messages the replayed turns seed the composer's history with.
    submissions: replay::Submissions,
    /// What became of the last ask for an older page.
    history_ask: HistoryAsk,
    /// The thread's goal (`thread/goal/updated|cleared`, the answers of `thread/goal/get|set`).
    goal: Option<Goal>,
    /// Live context compaction: the running one and how long the finished ones took.
    compaction: compaction::Compaction,
    /// Safety buffering, recourse and the misalignment stop.
    safety: safety::Safety,
    /// The thread's event stream was lost and has not come back (`ThreadEv::StreamLost`): what
    /// the user sends waits in the queue (oracle `thread_stream_disconnected`).
    stream_lost: bool,
    /// The stream is back but nothing read what it missed (`ThreadEv::StreamRestored`; oracle
    /// `history_may_be_incomplete`): opening the thread again reads its history from the server
    /// instead of showing what is held.
    history_may_be_incomplete: bool,
    /// Queue dispatch waits for the newest history page after reconnecting with pending input.
    reconciling_submissions: bool,
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
    Unavailable,
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
    /// A retained thread whose stream could not be attached again.
    Unavailable,
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
            Origin::Unavailable => {
                model.core.history_may_be_incomplete = true;
                Writer::Unavailable
            }
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

    /// The owner and original connection of a thread RPC.
    pub(crate) fn request(&self, ticket: Ticket) -> Option<(&ThreadId, Address)> {
        self.held
            .iter()
            .find_map(|(id, model)| model.outgoing.address(ticket).map(|address| (id, address)))
    }

    /// An IDE-gated steer has not left the app. Once the turn has restored or
    /// merged its input, discard the unused request without recovering it again.
    pub(crate) fn retain_preparing_steer(
        &mut self,
        thread: &ThreadId,
        ticket: Ticket,
        client_id: &str,
    ) -> bool {
        let Some(model) = self.held.get_mut(thread) else {
            return false;
        };
        if pending_steer(model, client_id).is_some() {
            return true;
        }
        Rc::make_mut(model).outgoing.drop_ticket(ticket);
        false
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
                recorded: None,
                queue: Vec::new(),
                due: None,
                queue_modals: BTreeMap::new(),
                pending_steers: Vec::new(),
                rejected_steers: Vec::new(),
                steers_after_interrupt: false,
                quiet_interrupts: false,
                budget_interruptions: Rc::default(),
                restore: None,
                recovered: false,
                processes: processes::Processes::default(),
                hooks: hooks::Hooks::default(),
                last_error: None,
                goal: None,
                compaction: compaction::Compaction::default(),
                safety: safety::Safety::default(),
                stream_lost: false,
                history_may_be_incomplete: false,
                reconciling_submissions: false,
                last_ended: None,
                finished_turns: Rc::default(),
                authoritative_turns: Rc::default(),
                completed_items: HashSet::new(),
                legacy_user_completions: HashMap::new(),
                history_next: None,
                voice_history_supported: false,
                history_ask: HistoryAsk::Idle,
                submissions: replay::Submissions::default(),
                writer: Writer::Detached,
            },
        }
    }

    /// The thread's own state and the screens' parts, read-only, for a feature (ADR-012).
    pub fn view(&self) -> ThreadView<'_> {
        ThreadView::new(&self.core, &self.slots).with_parts(&self.features)
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

    /// The nearest moment a feature asks this thread to be woken at (`Feature::thread_wake_at`).
    /// None where this process does not write the thread: the intent would not be delivered
    /// there (`apply_thread_intent`), so asking the shell for the moment would only spin it.
    pub(crate) fn next_wake(&self) -> Option<Now> {
        self.view()
            .has_writer()
            .then(|| features::next_thread_wake(&self.features))
            .flatten()
    }

    /// The thread intents the features asked to be woken with, reached by `now`.
    pub(crate) fn wakes_due(&self, now: Now) -> Vec<FeatureThreadIntent> {
        if self.view().has_writer() {
            features::thread_wakes_due(&self.features, now)
        } else {
            Vec::new()
        }
    }

    pub(crate) fn voice_history_supported(&self) -> bool {
        self.voice_history_supported
    }

    pub(crate) fn can_load_voice_history(&self) -> bool {
        self.history_next.is_none() && self.transcript.voice_replay.paused()
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

    /// The hook runs under way: the status row names the ones the view has revealed
    /// ([`hooks::summary`]).
    pub fn running_hooks(&self) -> &[hooks::RunningHook] {
        self.hooks.runs()
    }

    pub fn can_load_history(&self) -> bool {
        self.history_next.is_some() || self.transcript.voice_replay.paused()
    }

    /// How much of the thread's history the transcript holds (oracle `TranscriptHistoryState`;
    /// its `LoadingBeginning` is the sheet's own walk to the start, a controller's state).
    pub fn history_state(&self) -> HistoryState {
        match self.history_ask {
            HistoryAsk::Loading => HistoryState::Loading,
            HistoryAsk::Failed => HistoryState::Failed,
            HistoryAsk::Idle if self.history_next.is_some() => HistoryState::Partial,
            HistoryAsk::Idle => self.transcript.voice_replay.history_state(),
        }
    }

    pub fn transcript(&self) -> &Transcript {
        &self.transcript
    }

    /// Whether the live turn received the completion for this agent or reasoning item.
    pub fn received_item_completion(&self, key: &ItemKey) -> bool {
        self.completed_items.contains(key)
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

    /// The number of the last submit `apply` accepted: it grows with each, so a page that sees it
    /// change in the same thread knows the model took the line (the composer clears then). The
    /// ticket itself is core's.
    pub fn submit_seq(&self) -> Option<u64> {
        self.last_submit.map(Ticket::seq)
    }

    /// The newest message that went out for the user without the composer: its number (it grows
    /// with each) and the message.
    pub fn recorded(&self) -> Option<(u64, &UserMessage)> {
        self.recorded.as_ref().map(|(seq, message)| (*seq, message))
    }

    pub fn queue(&self) -> &[Queued] {
        &self.queue
    }

    /// The queued command the screen runs now: it dispatches the line (`dispatch_queued`) and
    /// reports `QueueEvent::Ran`. A change of `seq` is a new command.
    pub fn due_command(&self) -> Option<&DueCommand> {
        self.due.as_ref().filter(|due| due.issued())
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

    /// The process the model keeps polling with nothing to write, while no other entry
    /// came between: the status row reads `Waiting for background terminal` with its command
    /// (oracle `unified_exec_wait_streak`).
    pub fn terminal_wait(&self) -> Option<&WaitStreak> {
        self.processes.streak()
    }

    pub fn screen_slots(&self) -> &ThreadSlots {
        &self.slots
    }

    pub fn last_error(&self) -> Option<&Failure> {
        self.last_error.as_ref()
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

    /// A turn runs, its start is in flight, or a compaction was asked for and its turn has not
    /// begun (the oracle's `is_task_running` from the dispatch of `/compact`): a message waits
    /// in the queue then, a command that needs an idle thread is disabled.
    pub fn busy(&self) -> bool {
        self.core.busy() || self.slots.chat.is_compacting()
    }

    /// What the composer calls a task that runs: a turn or its start, a review, a server that
    /// still starts (`update_task_running_state`, the bottom pane's `is_task_running`). The queue
    /// key queues on it; a message still goes out unless `busy`.
    pub fn task_running(&self) -> bool {
        self.busy() || self.slots.chat.review_mode() || self.slots.chat.mcp.is_starting()
    }

    /// A user turn is running or its start is in flight, or a review runs (oracle
    /// `is_user_turn_pending_or_running`).
    pub fn user_turn_in_flight(&self) -> bool {
        self.busy() || self.slots.chat.review_mode()
    }
}

impl ThreadCore {
    pub fn last_ended(&self) -> Option<&TurnId> {
        self.last_ended.as_ref()
    }

    pub(crate) fn turn_finished(&self, turn: &TurnId) -> bool {
        self.finished_turns.contains(turn)
    }

    /// The thread's goal, none when it has none.
    pub fn goal(&self) -> Option<&Goal> {
        self.goal.as_ref()
    }

    pub fn id(&self) -> &ThreadId {
        &self.id
    }

    /// The background terminals (unified-exec sessions still running), oldest first.
    pub fn terminals(&self) -> &[Terminal] {
        self.processes.terminals()
    }

    /// The thread's context compaction facts.
    pub fn compaction(&self) -> &compaction::Compaction {
        &self.compaction
    }

    /// The thread's safety facts: buffering, recourse and the misalignment stop.
    pub fn safety(&self) -> &safety::Safety {
        &self.safety
    }

    /// The transcript, for a feature's view of the thread (`ThreadView`).
    pub(crate) fn transcript(&self) -> &Transcript {
        &self.transcript
    }

    pub fn is_running(&self) -> bool {
        matches!(self.turn, TurnState::Running(_))
    }

    /// A turn runs or its start is in flight.
    pub fn busy(&self) -> bool {
        self.is_running() || self.outgoing.any(is_start_turn) || self.safety.retrying()
    }

    /// The thread's event stream was lost and has not come back.
    pub fn stream_lost(&self) -> bool {
        self.stream_lost
    }

    /// The stream came back after a loss and the history it missed was not read.
    pub fn history_may_be_incomplete(&self) -> bool {
        self.history_may_be_incomplete
    }

    pub(crate) fn reconciling_submissions(&self) -> bool {
        self.reconciling_submissions
    }

    /// The turn that runs, if one does (`ThreadCx::interrupt`).
    pub(crate) fn running_turn(&self) -> Option<&TurnId> {
        match &self.turn {
            TurnState::Running(running) => Some(&running.turn),
            _ => None,
        }
    }

    /// This process is the thread's writer.
    pub(crate) fn attached(&self) -> bool {
        self.writer == Writer::Attached
    }
}

/// The effects of one thread's `apply`: thread RPCs, with the thread id
/// and the ticket stamped (R3). This is the only way a thread RPC is made.
pub struct ThreadFx<'a> {
    thread: &'a ThreadId,
    out: &'a mut AppFx,
    mint: &'a mut TicketMint,
    registered: Vec<(Ticket, ThreadCtx)>,
    published_name: Option<String>,
    voice_failure: Option<Failure>,
    address: Address,
    /// The app's arbitration for the completed turn carried by this event.
    prompt_priority: screens::chat::plan_mode::PromptPriority,
    /// How the Daybreak setting words a cyber refusal at this moment (the app's fact, stamped
    /// per event by `apply_thread`).
    daybreak: Daybreak,
    /// The server's collaboration-mode presets at this moment (the app's fact, stamped per
    /// event by `apply_thread`): what a switch of the mode chooses from.
    modes: &'a [screens::select::collaboration::Mask],
    plan_effort: Option<&'a codex_protocol::openai_models::ReasoningEffort>,
    /// The thread's backend uses native admission, with exact live receipts.
    cloud: bool,
    /// The app's facts at this moment (the clock the host told, the account, the `[tui]`
    /// settings): what a live completion falls back to when the server's turn carries no duration
    /// or completion time, and what a feature's thread step reads through `ThreadCx::facts`.
    /// None where the caller stamped none (a thread's own test): the defaults.
    facts: Option<features::AppFacts<'a>>,
    image_restrictions: Option<&'a BTreeSet<String>>,
}

impl<'a> ThreadFx<'a> {
    pub(crate) fn new(thread: &'a ThreadId, out: &'a mut AppFx, mint: &'a mut TicketMint) -> Self {
        Self {
            thread,
            out,
            mint,
            registered: Vec::new(),
            published_name: None,
            voice_failure: None,
            address: Address::default(),
            prompt_priority: screens::chat::plan_mode::PromptPriority::Ordinary,
            daybreak: Daybreak::Apply,
            modes: &[],
            plan_effort: None,
            cloud: false,
            facts: None,
            image_restrictions: None,
        }
    }

    pub(crate) fn with_cloud(mut self, cloud: bool) -> Self {
        self.cloud = cloud;
        self
    }

    pub(crate) fn addressed(mut self, address: Address) -> Self {
        self.address = address;
        self
    }

    pub(crate) fn with_prompt_priority(
        mut self,
        priority: screens::chat::plan_mode::PromptPriority,
    ) -> Self {
        self.prompt_priority = priority;
        self
    }

    pub(crate) fn with_image_restrictions(
        mut self,
        restricted: Option<&'a BTreeSet<String>>,
    ) -> Self {
        self.image_restrictions = restricted;
        self
    }

    /// The same, with the app's facts of the moment.
    pub(crate) fn with_facts(mut self, facts: features::AppFacts<'a>) -> Self {
        self.facts = Some(facts);
        self
    }

    /// The app's facts at this step (the defaults where none were stamped).
    pub(crate) fn facts(&self) -> features::AppFacts<'a> {
        match self.facts {
            Some(facts) => facts,
            None => features::AppFacts::unknown(),
        }
    }

    /// The app's clock, epoch seconds (0 until the host has said).
    fn now(&self) -> i64 {
        self.facts.map_or(0, |facts| facts.now().secs())
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
    pub(crate) fn with_modes(
        mut self,
        modes: &'a [screens::select::collaboration::Mask],
        plan_effort: Option<&'a codex_protocol::openai_models::ReasoningEffort>,
    ) -> Self {
        self.modes = modes;
        self.plan_effort = plan_effort;
        self
    }

    /// The server's collaboration-mode presets (empty until read).
    pub(crate) fn modes(&self) -> &[screens::select::collaboration::Mask] {
        self.modes
    }

    pub(crate) fn plan_effort(&self) -> Option<&codex_protocol::openai_models::ReasoningEffort> {
        self.plan_effort
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
        let client_id = self.mint.submission_id(ticket);
        self.registered.push((
            ticket,
            ThreadCtx::StartTurn {
                interrupt: false,
                message: message.clone(),
                client_id: Some(client_id.clone()),
                acknowledged: false,
            },
        ));
        self.out.commands.push(Command::StartTurn {
            ticket,
            thread: self.thread.clone(),
            message,
            client_id,
            misalignment_override: None,
            ide_prompt: None,
        });
        Ok(ticket)
    }

    /// The turn start of an acknowledged misalignment failure, `stamp` (epoch milliseconds) its
    /// acknowledgement. One start per step, as `start_turn`.
    fn start_acknowledged(
        &mut self,
        message: UserMessage,
        stamp: i64,
    ) -> Result<Ticket, UserMessage> {
        if self.start_said() {
            return Err(message);
        }
        let ticket = self.mint.next();
        let client_id = self.mint.submission_id(ticket);
        self.registered.push((
            ticket,
            ThreadCtx::StartTurn {
                interrupt: false,
                message: message.clone(),
                client_id: Some(client_id.clone()),
                acknowledged: true,
            },
        ));
        self.out.commands.push(Command::StartTurn {
            ticket,
            thread: self.thread.clone(),
            message,
            client_id,
            misalignment_override: Some(stamp),
            ide_prompt: None,
        });
        Ok(ticket)
    }

    /// `turn/interrupt` of the running turn `turn` (`ThreadCx::interrupt`).
    pub(crate) fn interrupt_turn(&mut self, turn: TurnId) {
        self.interrupt(turn, false);
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
        self.request_at(ticket, ctx.into(), command.into());
        ticket
    }

    fn request_at(&mut self, ticket: Ticket, ctx: ScreenCtx, command: ScreenCommand) {
        self.registered.push((ticket, ThreadCtx::Screen(ctx)));
        self.out.commands.push(Command::Screen {
            ticket,
            thread: self.thread.clone(),
            command,
        });
    }

    pub(crate) fn voice_owner(&mut self) -> (screens::voice::Owner, String) {
        let call = self.mint.next();
        let session = format!("{}:{}", self.thread.as_str(), self.mint.submission_id(call));
        (
            screens::voice::Owner {
                thread: self.thread.clone(),
                address: self.address,
                call,
            },
            session,
        )
    }

    pub(crate) fn voice_audio(&mut self, command: screens::voice::AudioCommand) {
        self.out.host(crate::app::effects::Ask::Audio(command));
    }

    pub(crate) fn voice_failure(&mut self, failure: Failure) {
        self.voice_failure = Some(failure);
    }

    pub(crate) fn publish_name(&mut self, name: String) {
        self.published_name = Some(name);
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
    // a compaction is the status header's, its cell comes with its completion
    if let Some(projected) =
        compaction::project(notification).or_else(|| safety::project(notification))
    {
        return Some(projected);
    }
    let transcript = match notification {
        ServerNotification::TurnStarted(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnStarted {
                turn: TurnId::new(&n.turn.id),
                started_at: n.turn.started_at,
            },
        )),
        ServerNotification::TurnCompleted(n) if n.turn.status != TurnStatus::InProgress => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::TurnCompleted {
                turn: TurnId::new(&n.turn.id),
                end: match n.turn.status {
                    TurnStatus::Interrupted => TurnEnd::Interrupted,
                    TurnStatus::Failed => TurnEnd::Failed,
                    TurnStatus::Completed | TurnStatus::InProgress => TurnEnd::Completed,
                },
                snapshot: turn_snapshot(&n.turn),
                receipts: n
                    .turn
                    .item_payloads()
                    .filter_map(receipt_id)
                    .cloned()
                    .collect(),
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
                ThreadEv::ItemUpsert { key, item, receipt } => {
                    (thread, ThreadEv::ItemStarted { key, item, receipt })
                }
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
            ThreadEv::Command(CommandEv::Output {
                key: ItemKey {
                    turn: TurnId::new(&n.turn_id),
                    item: ItemId::new(&n.item_id),
                },
                delta: n.delta.clone(),
            }),
        )),
        ServerNotification::TerminalInteraction(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::Command(CommandEv::Interaction {
                process: n.process_id.clone(),
                stdin: n.stdin.clone(),
            }),
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
    /// A stop as a precaution (`codexErrorInfo: MisalignmentPolicyViolation`): no row, the
    /// thread blocks input (`threads/safety.rs`).
    Misalignment {
        message: String,
        details: Option<codex_app_server_protocol::MisalignmentErrorDetails>,
    },
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
        if error.codex_error_info == Some(CodexErrorInfo::MisalignmentPolicyViolation) {
            Fault::Misalignment {
                message: error.message.clone(),
                details: error.misalignment.clone(),
            }
        } else if error.codex_error_info == Some(CodexErrorInfo::CyberPolicy) {
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
            // never a row (`safety::on_completed` takes it first)
            Fault::Misalignment { message, .. } => Item::Error {
                message: message.clone(),
            },
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
    let replay::HistoryProjection {
        turns,
        submissions,
        receipts,
    } = replay::project(turns);
    ThreadEv::HistoryPage {
        turns,
        next,
        submissions,
        receipts,
        replace: false,
        running: true,
    }
}

/// The first page of a thread the server answered a resume, fork or start with: its turns, and
/// its own status decides whether a turn without an end is running.
pub fn history_page_of(thread: &codex_app_server_protocol::Thread) -> ThreadEv {
    let running = matches!(
        thread.status,
        codex_app_server_protocol::ThreadStatus::Active { .. }
    );
    match history_page(&thread.turns, None) {
        ThreadEv::HistoryPage {
            turns,
            next,
            submissions,
            receipts,
            replace,
            ..
        } => ThreadEv::HistoryPage {
            turns,
            next,
            submissions,
            receipts,
            replace,
            running,
        },
        other => other,
    }
}

/// The page that drops the transcript and the history cursor, for a thread that is read again:
/// the pages of the server follow.
pub fn history_reset() -> ThreadEv {
    ThreadEv::HistoryPage {
        turns: Vec::new(),
        next: None,
        submissions: Vec::new(),
        receipts: Vec::new(),
        replace: true,
        running: true,
    }
}

fn item_entry(payload: &ThreadItemPayload, completed: bool) -> Option<(ItemId, Item)> {
    Some(match payload {
        ThreadItemPayload::UserMessage { id, content, .. } => {
            (ItemId::new(id), Item::User(UserPrompt::from_input(content)))
        }
        ThreadItemPayload::AgentMessage {
            id,
            text,
            phase,
            questions,
            ..
        } => (
            ItemId::new(id),
            Item::Agent {
                text: text.clone(),
                phase: phase.clone(),
                questions: questions
                    .as_ref()
                    .map(|questions| questions.iter().map(Into::into).collect()),
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

fn receipt_id(payload: &ThreadItemPayload) -> Option<&String> {
    match payload {
        ThreadItemPayload::UserMessage { client_id, .. } => client_id.as_ref(),
        _ => None,
    }
}

fn item_upsert(
    thread: &str,
    turn: &str,
    item: &ThreadItem,
    completed: bool,
) -> Option<(ThreadId, ThreadEv)> {
    let (id, entry) = item_entry(&item.payload, completed)?;
    let receipt = match &item.payload {
        ThreadItemPayload::UserMessage {
            client_id: Some(client_id),
            ..
        } => Some(UserItemEcho::ClientId(client_id.clone())),
        ThreadItemPayload::UserMessage { content, .. } if completed => {
            let (text, images) = legacy_steer_key(content);
            Some(UserItemEcho::Legacy { text, images })
        }
        _ => None,
    };
    Some((
        ThreadId::new(thread),
        ThreadEv::ItemUpsert {
            key: ItemKey {
                turn: TurnId::new(turn),
                item: id,
            },
            item: entry,
            receipt,
        },
    ))
}

/// The retained turns are not trusted (a revert, an unread gap): the transcript starts again and
/// the server's page is applied over it.
fn reset_history(model: &mut ThreadModel) {
    model.transcript.reset();
    features::clear_copy_status(&mut model.features);
    model.history_next = None;
    model.history_ask = HistoryAsk::Idle;
    Rc::make_mut(&mut model.finished_turns).clear();
    Rc::make_mut(&mut model.authoritative_turns).clear();
    model.completed_items.clear();
    model.last_ended = None;
}

/// Applies one event to one thread's model. Events are tolerant of order
/// (R9): a delta may come before its item, a started turn twice.
pub fn apply(model: &mut ThreadModel, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    apply_ev(model, ev, fx);
    if !model.slots.voice.has_call() {
        model.core.transcript.clear_live_voice();
    }
    if let Some(failure) = fx.voice_failure.take() {
        model.last_error = Some(failure);
    }
    for (ticket, ctx) in std::mem::take(&mut fx.registered) {
        if matches!(
            ctx,
            ThreadCtx::StartTurn { .. }
                | ThreadCtx::Screen(ScreenCtx::Chat(
                    screens::chat::Ctx::Steer { .. } | screens::chat::Ctx::Shell
                ))
        ) {
            features::clear_copy_status(&mut model.features);
        }
        model.outgoing.insert(ticket, fx.address, ctx);
    }
}

fn apply_ev(model: &mut ThreadModel, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    if ev.is_notification() {
        model.slots.chat.clear_retry();
    }
    safety::observe(model, &ev);
    if matches!(ev, ThreadEv::ConnectionLost | ThreadEv::StreamLost) {
        safety::apply(
            model,
            safety::SafetyEv::RetryLifecycle(safety::RetryLifecycle::RetireOffer),
            fx,
        );
        model.features.rename.connection_lost();
        model.slots.voice.retire(fx);
        model.core.transcript.voice_replay = Rc::default();
    }
    match ev {
        // the thread's own facts ride the chat's events (`threads/compaction.rs`, `safety.rs`)
        ThreadEv::Screen(ScreenThreadEv::Chat(screens::chat::ThreadEv::Compacting(ev))) => {
            compaction::apply(model, ev, fx)
        }
        ThreadEv::Screen(ScreenThreadEv::Chat(screens::chat::ThreadEv::Safety(ev))) => {
            safety::apply(model, ev, fx)
        }
        ThreadEv::Mcp { server, status } => {
            let warnings = model.slots.chat.mcp.update(server, status);
            add_startup_warnings(model, warnings, fx);
        }
        ThreadEv::McpLagged => {
            let warnings = model.slots.chat.mcp.finish_after_lag();
            add_startup_warnings(model, warnings, fx);
        }
        ThreadEv::Deferred(Deferred::Submit(queued)) => {
            model.queue.push(queued);
            model.last_submit = Some(fx.mint());
        }
        ThreadEv::Deferred(Deferred::Sent) => model.queue.clear(),
        ThreadEv::Deferred(Deferred::Recorded(message)) => {
            let seq = model.recorded.as_ref().map_or(1, |(seq, _)| seq + 1);
            model.recorded = Some((seq, message));
        }
        ThreadEv::ConnectionLost if model.has_writer() => {
            hold_unconfirmed(model);
            model.core.reconciling_submissions |= !model.queue.is_empty();
            model.core.history_may_be_incomplete |= model.reconciling_submissions;
            model.writer = Writer::Detached;
            // the stream went with the connection: an attach again starts a live one
            model.core.stream_lost = false;
            model.outgoing = Pending::default();
            model.turn = TurnState::Idle;
            // Known refusals can be edited in the composer.
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
        ThreadEv::ConnectionLost if fx.cloud => retire_submissions(model),
        ThreadEv::ConnectionLost => {}
        ThreadEv::StreamLost => {
            // Cloud can report the stream loss and then reject admission on the same connection.
            hold_unconfirmed(model);
            if fx.cloud {
                model.core.reconciling_submissions |=
                    matches!(model.queue.first(), Some(Queued::Unconfirmed { .. }));
            } else {
                retire_submissions(model);
            }
            model.core.stream_lost = true;
        }
        ThreadEv::StreamRestored { read, reconciled } => {
            // A successful Cloud stream attach can race the failure of an explicit resume.
            if model.core.stream_lost && model.writer == Writer::Unavailable {
                model.writer = Writer::Attached;
            }
            model.core.stream_lost = false;
            model.core.history_may_be_incomplete = !read;
            if reconciled
                && std::mem::take(&mut model.core.reconciling_submissions)
                && let Some(Queued::Unconfirmed { message, .. }) = model.queue.first()
            {
                apply_appended(
                    model,
                    None,
                    Item::Notice(Notice::SubmissionUnconfirmed(
                        crate::question_reply::display_text(&message.text)
                            .unwrap_or_else(|| message.text.clone()),
                    )),
                );
            }
            if !model.busy() {
                send_next(model, fx);
            }
        }
        ThreadEv::Reverted => {
            // the edit of a past prompt waits for this (`ThreadReverted`)
            let edited = model.slots.backtrack.editing();
            screens::apply_thread_ev(
                &mut model.slots,
                ScreenThreadEv::Backtrack(screens::backtrack::ThreadEv::Reverted),
                fx,
            );
            reset_history(model);
            // Our own edit says so after the history it kept (`FinishPromptRevert`: the retained
            // turns are read next and go before the line); a revert another client made says
            // nothing.
            if edited {
                apply_ev(
                    model,
                    ThreadEv::Appended {
                        turn: None,
                        item: Item::Notice(Notice::PromptReverted),
                    },
                    fx,
                );
            }
        }
        ThreadEv::Failure(error) => model.last_error = Some(error),
        ThreadEv::Goal(goal, budget_turn) => {
            model.goal = goal;
            if let Some(turn) = budget_turn {
                model.slots.chat.mark_budget_limited(turn);
            }
        }
        ThreadEv::HistoryPage {
            mut turns,
            next,
            submissions,
            receipts,
            replace,
            running,
        } => {
            if replace {
                reset_history(model);
            }
            // The latest turn of a thread read for the first time stopped as a precaution: the
            // thread blocks input, with nothing to continue from (`replay_thread_turns`).
            if model.transcript.is_empty()
                && !model.is_running()
                && turns
                    .last()
                    .is_some_and(|latest| matches!(latest.error, Some(Fault::Misalignment { .. })))
            {
                safety::replayed(model);
            }
            let released = reconcile_ids(model, &receipts);
            for snapshot in &mut turns {
                if snapshot.end.is_some() {
                    model.legacy_user_completions.remove(&snapshot.turn);
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
                    replay::settle_interruption(model, snapshot);
                } else if running {
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
            model.transcript.position_voice_history();
            model.submissions.prepend(submissions);
            model.transcript.remember_timings(timings);
            model.history_next = next.filter(|cursor| Some(cursor) != model.history_next.as_ref());
            model.history_ask = HistoryAsk::Idle;
            if released && !model.busy() {
                send_next(model, fx);
            }
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
        // a misalignment stop rejects what starts a turn: only a new chat goes on
        ThreadEv::Submit { .. } if model.safety.blocks_input() => {}
        // a thread whose model is known to be empty sends nothing: the error says so and the
        // line stays where it was typed (`submit_user_message`; the queue's drain gives its
        // message back instead)
        ThreadEv::Submit { .. }
            if model.has_writer()
                && startup::model_unavailable(model.slots.chat.model.as_deref()) =>
        {
            apply_ev(
                model,
                ThreadEv::Appended {
                    turn: None,
                    item: Item::Note(Note::from(crate::commands::Notice::ThreadModelUnavailable)),
                },
                fx,
            )
        }
        ThreadEv::Submit { message } if model.has_writer() => {
            if reject_image_input(model, fx, &message) {
                return;
            }
            // The decision is here (R10). Enter during a running turn steers
            // it (`turn/steer`); while the turn is only starting (no turn id
            // yet) the message waits in the queue; with none running it starts
            // a turn, after any older queued message.
            let starting = model.outgoing.any(is_start_turn);
            // During recovery or ambiguous admission, later input stays queued. After a Cloud
            // turn ends, its in-flight steers can still decide whether the next input may start.
            if model.safety.retrying()
                || model.stream_lost
                || model.reconciling_submissions
                || (fx.cloud && matches!(model.queue.first(), Some(Queued::Unconfirmed { .. })))
                || (!model.is_running() && cloud_admission_pending(model, fx))
            {
                enqueue(model, Queued::Message(message), fx);
                return;
            }
            // The plan's text streams: Enter queues the message behind the turn
            // (`is_plan_streaming_in_tui`).
            if model.slots.chat.plan.streaming() && model.is_running() {
                enqueue(model, Queued::Message(message), fx);
                return;
            }
            // The user's own `!` command runs: a message that is not one waits for it
            // (`only_user_shell_commands_running`), where Enter would steer the turn.
            if let (TurnState::Running(running), false) = (&model.turn, starting)
                && !message.text.starts_with('!')
                && model
                    .transcript
                    .only_user_shell_commands_running(&running.turn)
            {
                enqueue(model, Queued::Message(message), fx);
                return;
            }
            if let (TurnState::Running(running), false) = (&model.turn, starting) {
                let turn = running.turn.clone();
                model.slots.chat.plan.steered();
                let ticket = fx.mint();
                let client_id = fx.mint.submission_id(ticket);
                model.pending_steers.push(PendingSteer {
                    message: message.clone(),
                    submission: Some((ticket, client_id.clone())),
                    admitted: None,
                });
                steer(fx, ticket, turn, false, message, client_id);
                model.last_submit = Some(ticket);
                return;
            }
            // MCP servers still starting hold the message (oracle
            // `maybe_send_next_queued_input`).
            let behind = !model.queue.is_empty() && !model.recovered;
            // a compaction asked for holds the thread as a running turn does, before its turn
            // begins (the oracle queues the message: `is_task_running`)
            let compacting = model.slots.chat.is_compacting();
            if starting || compacting || model.slots.chat.mcp.is_starting() || behind {
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
        ThreadEv::Command(CommandEv::Restore) => {
            let restore = model.processes.begin_restore();
            fx.request(
                screens::chat::Ctx::ListTerminals {
                    restore,
                    pages: 1,
                    listed: Vec::new(),
                },
                screens::chat::Command::ListTerminals { cursor: None },
            );
        }
        ThreadEv::Reply { ticket, reply } => {
            // A reply whose ticket is not ours any more is stale.
            let Some((ctx, result)) = model.outgoing.resolve_reply(ticket, reply) else {
                return;
            };
            let answer = match result {
                crate::rpc::ReplyResult::Answer(answer) => answer,
                crate::rpc::ReplyResult::SubmissionUnconfirmed(_) => {
                    hold_submission_reply(model, ticket, ctx, true);
                    return;
                }
                crate::rpc::ReplyResult::CloudSubmission(result) => {
                    cloud_submission_reply(model, ticket, ctx, result, fx);
                    return;
                }
            };
            match (ctx, answer) {
                (
                    ThreadCtx::Screen(ScreenCtx::Chat(screens::chat::Ctx::Steer {
                        turn,
                        retried,
                        message,
                        client_id,
                    })),
                    answer,
                ) => steer_reply(model, turn, retried, message, client_id, answer, fx),
                // the live terminals of a resumed thread: a page, then the next or the end; a list
                // that fails ends the restore with nothing (oracle `unwrap_or_default`)
                (
                    ThreadCtx::Screen(ScreenCtx::Chat(screens::chat::Ctx::ListTerminals {
                        restore,
                        pages,
                        listed,
                    })),
                    answer,
                ) => restore_reply(model, restore, pages, listed, answer, fx),
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
                (
                    ThreadCtx::StartTurn {
                        interrupt, message, ..
                    },
                    Answer::Thread(ThreadReply::TurnStarted { turn, .. }),
                ) => {
                    model.core.safety.submitted(turn.clone(), message);
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
                // an RPC diagnostic can hold the instruction: the failure is worded without it
                (
                    ThreadCtx::StartTurn {
                        acknowledged: true, ..
                    },
                    Answer::Failed(_),
                ) => {
                    model.last_error = Some(Failure::Message(
                        "Couldn’t continue this chat. Review its latest status before trying again."
                            .to_owned(),
                    ));
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
                    Answer::Thread(ThreadReply::InterruptTurnChanged(actual)),
                ) => {
                    if let TurnState::Running(running) = &mut model.core.turn
                        && running.turn == turn
                        && !model.core.finished_turns.contains(&actual)
                    {
                        running.turn = actual.clone();
                        fx.interrupt(actual, true);
                    }
                }
                (
                    ThreadCtx::Interrupt { turn, .. },
                    Answer::Thread(ThreadReply::InterruptTurnChanged(_)),
                ) => {
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
                (
                    ThreadCtx::Screen(ScreenCtx::Voice(screens::voice::Ctx::ReadTimeline)),
                    answer,
                ) => {
                    let page = match answer {
                        Answer::Screen(crate::screens::ScreenReply::Voice(
                            screens::voice::Reply::Timeline(page),
                        )) => Some(page),
                        _ => None,
                    };
                    model
                        .core
                        .transcript
                        .receive_voice_history(ticket, page, fx);
                }
                (ThreadCtx::Screen(ctx), answer @ (Answer::Screen(_) | Answer::Failed(_))) => {
                    let operation = match &ctx {
                        ScreenCtx::Chat(screens::chat::Ctx::PlanEffort { operation, .. }) => {
                            operation.clone()
                        }
                        _ => None,
                    };
                    let result = match answer {
                        Answer::Screen(reply) => Ok(reply),
                        Answer::Failed(error) => Err(error),
                        _ => unreachable!(),
                    };
                    screens::apply_reply(&mut model.slots, ctx, result, fx);
                    if let Some(operation) = operation {
                        finish_plan_effort_pick(model, operation, fx);
                    }
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
        // `/plan`, with the preset the app chose, or a settled Plan reasoning change.
        ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(screens::chat::ThreadIntent::SetMask(
            change,
        ))) if model.has_writer() => match change {
            screens::select::collaboration::Change::Preset(mask) => switch_mode(model, &mask, fx),
            screens::select::collaboration::Change::PlanEffort {
                effort,
                scope,
                operation,
            } => {
                model
                    .slots
                    .chat
                    .set_plan_effort(effort, scope, operation, fx);
            }
        },
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
            model.processes.clear();
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
        ThreadEv::QuietInterrupts(quiet) => model.quiet_interrupts = quiet,
        ThreadEv::FeatureIntent(intent) => {
            let rename_held_queue = model.features.rename.holds_queue();
            with_features(model, fx, |parts, cx| {
                features::apply_thread_intent(parts, intent, cx);
            });
            resume_after_rename(model, fx, rename_held_queue);
        }
        // The model picker can settle a command already issued while its writer is detached.
        // The queue gate waits for the writer before it sends the next line.
        ThreadEv::ScreenIntent(intent)
            if model.has_writer() || queue::is_queue_settlement(&intent) =>
        {
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
                    submission: None,
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
        ThreadEv::VoiceHistorySupported(supported) => model.voice_history_supported = supported,
        ThreadEv::LoadOlderVoiceHistory => model.core.transcript.load_older_voice_history(fx),
        ThreadEv::ReadVoiceHistory {
            oldest,
            more_history,
        } => {
            model
                .core
                .transcript
                .read_voice_history(oldest, more_history, fx);
        }
        ThreadEv::VoiceCaption {
            event,
            show_user_preview,
        } => {
            if let Some(session) = model.slots.voice.caption_session(fx.address) {
                model
                    .core
                    .transcript
                    .apply_voice_caption(session, event, show_user_preview);
            }
        }
        ThreadEv::Screen(ev) => {
            if let ScreenThreadEv::Chat(ref ev) = ev
                && let Some(turn) = ev.turn()
                && !matches!(&model.turn, TurnState::Running(running) if &running.turn == turn)
            {
                return;
            }
            if let ScreenThreadEv::Chat(screens::chat::ThreadEv::Reasoning(reasoning)) = &ev
                && model.completed_items.contains(&reasoning.key())
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
            let named = if let ScreenThreadEv::Chat(screens::chat::ThreadEv::Name(name)) = &ev {
                model.features.rename.name_updated(name.as_deref())
            } else {
                false
            };
            screens::apply_thread_ev(&mut model.slots, ev, fx);
            append_history(model);
            if named && !model.busy() {
                send_next(model, fx);
            }
        }
        ThreadEv::TurnError { turn, fault } => {
            // A refusal that alternatives are prepared for waits for the turn's own end.
            if safety::defer(model, &turn, &fault) {
                return;
            }
            // The precaution of a misalignment stop drops the queue before the turn is finalized.
            safety::misalignment(model, &turn, &fault);
            // Oracle `on_error`: the turn is finalized, then the error row, then
            // the next queued input; the failed turn's own completion is a no-op.
            end_live_turn(model, turn.clone(), TurnEnd::Failed, fx);
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
        ThreadEv::HookStarted { id, message } => hooks::started(model, id, message),
        ThreadEv::HookEnded { id, turn, item } => hooks::ended(model, &id, turn, item),
        ThreadEv::ItemStarted { key, item, receipt } => {
            processes::track(model, &key, &item);
            note_review(model, &item);
            let (echo, released) = match receipt {
                Some(UserItemEcho::ClientId(id)) => reconcile_id(model, &id),
                _ => (false, false),
            };
            if !model.finished_turns.contains(&key.turn)
                && model.transcript.item(&key).is_none()
                && !review_prompt(model, &item, echo)
                && !processes::draws_nothing(&item, false)
            {
                if matches!(item, Item::User(_)) {
                    observe_status_copy_item(model, &key, &item);
                }
                model.transcript.upsert(key, item);
            }
            if released && !model.busy() {
                send_next(model, fx);
            }
        }
        ThreadEv::ItemUpsert { key, item, receipt } => {
            processes::track(model, &key, &item);
            note_review(model, &item);
            let (echo, released) = match receipt {
                Some(UserItemEcho::ClientId(id)) => reconcile_id(model, &id),
                Some(UserItemEcho::Legacy { text, images }) if !fx.cloud => {
                    (reconcile_legacy_steer(model, &key, (text, images)), false)
                }
                _ => (false, false),
            };
            let turn_finished = model.finished_turns.contains(&key.turn);
            if !turn_finished {
                features::on_live_thread_event(
                    &mut model.features,
                    LiveThreadEvent::ItemCompleted {
                        key: &key,
                        item: &item,
                    },
                );
            }
            if !model.authoritative_turns.contains(&key.turn)
                && !processes::draws_nothing(&item, turn_finished)
            {
                if review_prompt(model, &item, echo) {
                    return;
                }
                if !turn_finished {
                    observe_status_copy_item(model, &key, &item);
                    match &item {
                        Item::Agent { .. } => {
                            model.completed_items.insert(key.clone());
                            processes::flush(model, Some(key.turn.clone()));
                        }
                        Item::Reasoning { .. } => {
                            model.completed_items.insert(key.clone());
                        }
                        Item::Plan { text } => model.slots.chat.plan.plan_completed(text),
                        _ => {}
                    }
                }
                model.transcript.upsert(key, item);
            }
            if released && !model.busy() {
                send_next(model, fx);
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
                processes::flush(model, Some(key.turn.clone()));
                model.transcript.append_agent(key, &delta);
            }
        }
        ThreadEv::PlanDelta { key, delta } => {
            if !model.finished_turns.contains(&key.turn) && !model.completed_items.contains(&key) {
                model.slots.chat.plan.plan_delta();
                processes::flush(model, Some(key.turn.clone()));
                model.transcript.append_plan(key, &delta);
            }
        }
        ThreadEv::Command(CommandEv::Output { key, delta }) => {
            processes::output(model, &key, &delta);
            if !model.finished_turns.contains(&key.turn) {
                model.transcript.append_command_output(key, &delta);
            }
        }
        ThreadEv::Command(CommandEv::Interaction { process, stdin }) => {
            processes::interaction(model, &process, stdin);
        }
        ThreadEv::TurnEnded { turn, end } => {
            model.legacy_user_completions.remove(&turn);
            processes::turn_ended(model, &turn, end);
            end_live_turn(model, turn, end, fx);
        }
        ThreadEv::TurnCompleted {
            turn,
            end,
            snapshot,
            receipts,
            timing,
            error,
        } => {
            model.legacy_user_completions.remove(&turn);
            processes::turn_ended(model, &turn, end);
            let released = reconcile_ids(model, &receipts);
            let live = !model.turn_finished(&turn);
            match snapshot {
                // A list with no item says nothing of a turn whose items came as they ran: the
                // oracle's live chat keeps what the notifications drew (an interrupted turn's
                // running command stays, finished as failed); only a list that names items is
                // the turn's own.
                Some(TurnSnapshot::Full(items)) if !items.is_empty() => {
                    Rc::make_mut(&mut model.authoritative_turns).insert(turn.clone());
                    for (id, item) in &items {
                        let key = ItemKey {
                            turn: turn.clone(),
                            item: id.clone(),
                        };
                        if live {
                            observe_status_copy_item(model, &key, item);
                        }
                        processes::track(model, &key, item);
                    }
                    model.transcript.replace_turn(&turn, items);
                }
                Some(TurnSnapshot::Summary(items))
                    if !model.authoritative_turns.contains(&turn) =>
                {
                    for (id, item) in items {
                        let key = ItemKey {
                            turn: turn.clone(),
                            item: id,
                        };
                        if live {
                            observe_status_copy_item(model, &key, &item);
                        }
                        model.transcript.upsert(key, item);
                    }
                }
                _ => {}
            }
            let timing = live_timing(model, &turn, end, timing, fx.now());
            model.transcript.remember_timings([(turn.clone(), timing)]);
            let error = safety::on_completed(model, &turn, end == TurnEnd::Failed, error, fx);
            if let Some(fault) = &error {
                model.transcript.report_error(&turn, fault, fx.daybreak());
            }
            end_live_turn(model, turn, end, fx);
            if released && !model.busy() {
                send_next(model, fx);
            }
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
    let rename_held_queue = model.features.rename.holds_queue();
    with_features(model, fx, |parts, cx| {
        features::apply_thread_reply(parts, ctx, result, cx);
    });
    resume_after_rename(model, fx, rename_held_queue);
}

fn resume_after_rename(model: &mut ThreadModel, fx: &mut ThreadFx<'_>, held: bool) {
    if held && !model.features.rename.holds_queue() && !model.busy() {
        send_next(model, fx);
    }
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
    if let Some(name) = fx.published_name.take() {
        screens::apply_thread_ev(
            &mut model.slots,
            ScreenThreadEv::Chat(screens::chat::ThreadEv::Name(Some(name))),
            fx,
        );
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

fn end_live_turn(model: &mut ThreadModel, turn: TurnId, end: TurnEnd, fx: &mut ThreadFx<'_>) {
    let previous_restore = model.restore().map(Restore::seq);
    let recover_questions = !model
        .safety
        .violation()
        .is_some_and(|violation| violation.turn.as_ref().is_none_or(|failed| failed == &turn));
    end_turn(model, turn.clone(), end, fx);
    let restore = model
        .restore()
        .filter(|r| previous_restore != Some(r.seq()))
        .cloned();
    features::on_live_thread_event(
        &mut model.features,
        LiveThreadEvent::TurnEnded(&LiveTurnEnd {
            turn,
            recover_questions,
            restore,
        }),
    );
}

fn end_turn(model: &mut ThreadModel, turn: TurnId, end: TurnEnd, fx: &mut ThreadFx<'_>) {
    // The reviews of the turn are over with it (oracle `on_task_complete`).
    let over = ScreenThreadEv::Approvals(screens::approvals::ThreadEv::TurnEnded);
    screens::apply_thread_ev(&mut model.slots, over, fx);
    let first_end = Rc::make_mut(&mut model.finished_turns).insert(turn.clone());
    if end == TurnEnd::Completed {
        model.transcript.mark_completed(&turn);
    }
    model.transcript.finish_streams(&turn);
    if end != TurnEnd::Completed {
        model.transcript.fail_running(&turn);
    }
    model.completed_items.retain(|key| key.turn != turn);
    let ends_running = matches!(&model.turn, TurnState::Running(running) if running.turn == turn);
    if ends_running || (first_end && !model.is_running()) {
        model.last_ended = Some(turn.clone());
        // a later turn's runs are not this turn's to drop
        hooks::clear(model);
    }
    model
        .outgoing
        .retain(|ctx| !matches!(ctx, ThreadCtx::Interrupt { turn: target, .. } if *target == turn));
    if ends_running {
        model.turn = TurnState::Idle;
        model.core.safety.turn_ended();
        model.core.compaction.clear();
        // `maybe_prompt_plan_implementation`: nothing waits to be sent behind the turn.
        let waiting = !model.queue.is_empty()
            || !model.pending_steers.is_empty()
            || !model.rejected_steers.is_empty();
        let mode = model.slots.chat.mode();
        model.slots.chat.plan.turn_ended(
            mode,
            end == TurnEnd::Completed,
            waiting,
            fx.prompt_priority,
        );
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
/// composer, never auto-sent. Cloud steers still wait for admission or an exact item echo.
/// A steer admitted to another turn is also left alone.
fn interrupted(model: &mut ThreadModel, turn: &TurnId, fx: &mut ThreadFx<'_>) {
    let awaiting_admission = cloud_admission_pending(model, fx);
    let (later, mine): (Vec<_>, Vec<_>) = std::mem::take(&mut model.pending_steers)
        .into_iter()
        .partition(|steer| fx.cloud || steer.admitted.as_ref().is_some_and(|id| id != turn));
    let submit = std::mem::take(&mut model.steers_after_interrupt);
    // `on_interrupted_turn`: the steer notice when Esc sent the steers, else
    // the budget's message when the goal stopped the turn, else the plain one.
    let budget = replay::budget_interruption(model, turn);
    let notice = if submit {
        Notice::InterruptedForSteer
    } else if budget {
        Notice::GoalBudgetReached
    } else {
        Notice::Interrupted
    };
    if !model.quiet_interrupts {
        model
            .transcript
            .upsert(replay::interruption_key(turn), Item::Notice(notice));
    }
    if submit && !mine.is_empty() {
        let merged = merge_messages(mine.into_iter().map(|steer| steer.message).collect());
        if reject_image_input(model, fx, &merged) {
            recover(model, merged);
        } else if let Err(merged) = fx.start_turn(merged) {
            // refused (a start was said in this step): the next `send_next` sends it first
            model.rejected_steers.push(merged);
        }
    } else {
        let mut messages = std::mem::take(&mut model.rejected_steers);
        messages.extend(mine.into_iter().map(|steer| steer.message));
        if !awaiting_admission
            && !model.reconciling_submissions
            && !matches!(model.queue.first(), Some(Queued::Unconfirmed { .. }))
        {
            messages.extend(model.queue.drain(..).map(Queued::into_message));
        }
        give_back(model, messages, false);
    }
    model.pending_steers = later;
}

/// `turn/steer` of `message` to `turn`.
fn steer(
    fx: &mut ThreadFx<'_>,
    ticket: Ticket,
    turn: TurnId,
    retried: bool,
    message: UserMessage,
    client_id: String,
) {
    fx.request_at(
        ticket,
        screens::chat::Ctx::Steer {
            turn: turn.clone(),
            retried,
            message: message.clone(),
            client_id: client_id.clone(),
        }
        .into(),
        screens::chat::Command::Steer {
            turn,
            message,
            client_id,
            ide_prompt: None,
        }
        .into(),
    );
}

/// Reject an image message only when this connection listed the selected model as text-only.
fn reject_image_input(model: &mut ThreadModel, fx: &ThreadFx<'_>, message: &UserMessage) -> bool {
    if message.images.is_empty() && message.remote_images.is_empty() {
        return false;
    }
    let Some(rejected_model) = model.slots.chat.model.as_deref().filter(|name| {
        fx.image_restrictions
            .is_some_and(|models| models.contains(*name))
    }) else {
        return false;
    };
    let note = Note::from(crate::commands::Notice::ImageInputsUnsupported(
        rejected_model.to_owned(),
    ));
    apply_appended(model, None, Item::Note(note));
    true
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

fn finish_plan_effort_pick(
    model: &mut ThreadModel,
    operation: queue::QueueOperation,
    fx: &mut ThreadFx<'_>,
) {
    let intent = ScreenThreadIntent::Chat(screens::chat::ThreadIntent::QueueEvent(
        queue::QueueEvent::ModelApplied(operation),
    ));
    apply_ev(model, ThreadEv::ScreenIntent(intent), fx);
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

/// A reply can put just its own submission on hold, if the exact echo was not seen yet.
fn hold_submission_reply(
    model: &mut ThreadModel,
    ticket: Ticket,
    ctx: ThreadCtx,
    await_restore: bool,
) -> bool {
    let Some((submitted, client_id, message)) = take_pending_submission(model, ticket, ctx) else {
        return false;
    };
    hold_submission(model, submitted, client_id, message, await_restore);
    true
}

fn take_pending_submission(
    model: &mut ThreadModel,
    ticket: Ticket,
    ctx: ThreadCtx,
) -> Option<(Ticket, String, UserMessage)> {
    match ctx {
        ThreadCtx::StartTurn {
            message,
            client_id: Some(client_id),
            ..
        } => Some((ticket, client_id, message)),
        ThreadCtx::Screen(ScreenCtx::Chat(screens::chat::Ctx::Steer { client_id, .. })) => {
            let index = pending_steer(model, &client_id)?;
            let steer = model.pending_steers.remove(index);
            let (submitted, id) = steer.submission?;
            Some((submitted, id, steer.message))
        }
        _ => None,
    }
}

fn cloud_submission_reply(
    model: &mut ThreadModel,
    ticket: Ticket,
    ctx: ThreadCtx,
    result: crate::rpc::CloudSubmission,
    fx: &mut ThreadFx<'_>,
) {
    use crate::rpc::CloudSubmission;

    let held = model.queue.iter().position(
        |entry| matches!(entry, Queued::Unconfirmed { submitted, .. } if *submitted == ticket),
    );
    let (turn, started) = match result {
        CloudSubmission::Started(turn) => (turn, true),
        CloudSubmission::Steered(turn) => (turn, false),
        CloudSubmission::Duplicate | CloudSubmission::Unconfirmed(_) => {
            let held = held.is_some() || hold_submission_reply(model, ticket, ctx, false);
            if held && let CloudSubmission::Unconfirmed(message) = result {
                model.last_error = Some(Failure::Message(message));
            } else if !held && !model.busy() {
                send_next(model, fx);
            }
            return;
        }
        CloudSubmission::Preflight(error) => {
            let rejected = match held {
                Some(index) => Some(model.queue.remove(index).into_message()),
                None => take_pending_submission(model, ticket, ctx).map(|(_, _, message)| message),
            };
            if let Some(message) = rejected {
                model.last_error = Some(Failure::Message(error));
                recover(model, message);
            }
            return;
        }
    };
    // Once the attachment is lost, only the exact live item or recovered history clears a hold.
    if held.is_some() || model.stream_lost || model.reconciling_submissions {
        return;
    }
    let (expected, message, interrupt) = match ctx {
        ThreadCtx::StartTurn {
            interrupt,
            message,
            client_id,
            ..
        } => {
            if !started && let Some(client_id) = client_id {
                model.pending_steers.push(PendingSteer {
                    message: message.clone(),
                    submission: Some((ticket, client_id)),
                    admitted: Some(turn.clone()),
                });
            }
            (None, message, interrupt)
        }
        ThreadCtx::Screen(ScreenCtx::Chat(screens::chat::Ctx::Steer {
            turn: expected,
            message,
            client_id,
            ..
        })) => {
            if let Some(index) = pending_steer(model, &client_id) {
                if started {
                    model.pending_steers.remove(index);
                } else {
                    model.pending_steers[index].admitted = Some(turn.clone());
                }
            }
            (Some(expected), message, false)
        }
        _ => return,
    };
    // A native receipt can correct the turn used to make this request.
    // A different turn observed since dispatch has its own lifecycle.
    if !model.finished_turns.contains(&turn)
        && model
            .running_turn()
            .is_none_or(|running| Some(running) == expected.as_ref() && running != &turn)
    {
        model.turn = TurnState::Idle;
        start_turn(model, turn.clone(), fx.now());
    }
    let active = model.running_turn() == Some(&turn);
    if active && started {
        model.core.safety.submitted(turn, message);
    }
    if active && interrupt {
        apply_ev(model, ThreadEv::Interrupt, fx);
    } else if !model.busy() {
        send_next(model, fx);
    }
}

fn cloud_admission_pending(model: &ThreadModel, fx: &ThreadFx<'_>) -> bool {
    fx.cloud
        && (model.outgoing.any(is_start_turn)
            || model
                .pending_steers
                .iter()
                .any(|steer| steer.admitted.is_none()))
}

fn hold_submission(
    model: &mut ThreadModel,
    submitted: Ticket,
    client_id: String,
    message: UserMessage,
    await_restore: bool,
) {
    model.core.reconciling_submissions |= await_restore;
    model.core.history_may_be_incomplete = true;
    // Requests can fail out of order. Keep uncertain messages in submission order before
    // everything that was still waiting to send.
    let index = model.queue.partition_point(|entry| {
        matches!(
            entry, Queued::Unconfirmed { submitted: prior, .. } if *prior < submitted
        )
    });
    model.queue.insert(
        index,
        Queued::Unconfirmed {
            message: Box::new(message),
            client_id,
            submitted,
        },
    );
}

/// Preserve submitted input ahead of unsent input when the server outcome becomes unknown.
fn hold_unconfirmed(model: &mut ThreadModel) {
    let mut held = Vec::new();
    for (ticket, context) in model.outgoing.iter_mut() {
        if let ThreadCtx::StartTurn {
            message, client_id, ..
        } = context
            && let Some(client_id) = client_id.take()
        {
            held.push((ticket, client_id, message.clone()));
        }
    }
    model.pending_steers.retain_mut(|steer| {
        let Some((ticket, client_id)) = steer.submission.take() else {
            return true;
        };
        held.push((ticket, client_id, std::mem::take(&mut steer.message)));
        false
    });
    for (ticket, client_id, message) in held {
        hold_submission(model, ticket, client_id, message, true);
    }
}

fn retire_submissions(model: &mut ThreadModel) {
    model.outgoing.retain(|ctx| {
        !matches!(
            ctx,
            ThreadCtx::StartTurn { .. }
                | ThreadCtx::Screen(ScreenCtx::Chat(screens::chat::Ctx::Steer { .. }))
        )
    });
}

/// The oracle compares the concatenated wire text and attachment count on legacy servers.
fn legacy_steer_key(input: &[codex_app_server_protocol::UserInput]) -> (String, usize) {
    use codex_app_server_protocol::UserInput;
    let mut text = String::new();
    let mut images = 0;
    for item in input {
        match item {
            UserInput::Text { text: part, .. } => text.push_str(part),
            UserInput::Image { .. } | UserInput::LocalImage { .. } => images += 1,
            _ => {}
        }
    }
    (text, images)
}

/// Legacy completions can retire the first live steer. Recovering submissions require IDs.
fn reconcile_legacy_steer(model: &mut ThreadModel, item: &ItemKey, key: (String, usize)) -> bool {
    if model.finished_turns.contains(&item.turn) {
        return false;
    }
    let first_completion = model
        .legacy_user_completions
        .entry(item.turn.clone())
        .or_default()
        .insert(item.item.clone());
    if !first_completion
        || !model.has_writer()
        || model.stream_lost
        || model.reconciling_submissions
    {
        return false;
    }
    let matches = model.pending_steers.first().is_some_and(|steer| {
        key == legacy_steer_key(&crate::protocol::turn_input_in(
            &steer.message,
            Some(model.id().as_str()),
        ))
    });
    if matches {
        model.pending_steers.remove(0);
    }
    matches
}

/// A server item confirms only the submission that supplied this client ID.
fn reconcile_id(model: &mut ThreadModel, client_id: &str) -> (bool, bool) {
    if let Some(ThreadCtx::StartTurn { client_id, .. }) = model.outgoing.find_mut(
        |ctx| matches!(ctx, ThreadCtx::StartTurn { client_id: Some(id), .. } if id == client_id),
    ) {
        *client_id = None;
    }
    let pending = pending_steer(model, client_id);
    if let Some(index) = pending {
        model.pending_steers.remove(index);
    }
    let queued = model.queue.len();
    model.queue.retain(
        |entry| !matches!(entry, Queued::Unconfirmed { client_id: id, .. } if id == client_id),
    );
    let released = queued != model.queue.len();
    (pending.is_some() || released, released)
}

fn reconcile_ids(model: &mut ThreadModel, receipts: &[String]) -> bool {
    let mut released = false;
    for receipt in receipts {
        released |= reconcile_id(model, receipt).1;
    }
    released
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

/// The most pages of the terminal list one restore reads (the oracle pages until the server
/// stops; this keeps a server that never does from looping).
const RESTORE_PAGES: u8 = 20;

/// The answer of a `thread/backgroundTerminals/list` page of a restore: the next page is asked
/// for while the server names one, the end (or a failure) settles the restore with what was
/// listed (`last_error` and the chat's error stay untouched, as the oracle swallows it).
fn restore_reply(
    model: &mut ThreadModel,
    restore: u32,
    pages: u8,
    mut listed: Vec<Listed>,
    answer: Answer,
    fx: &mut ThreadFx<'_>,
) {
    match answer {
        Answer::Screen(screens::ScreenReply::Chat(screens::chat::Reply::Terminals {
            data,
            next,
        })) => {
            listed.extend(data);
            match next {
                Some(cursor) if pages < RESTORE_PAGES => {
                    fx.request(
                        screens::chat::Ctx::ListTerminals {
                            restore,
                            pages: pages + 1,
                            listed,
                        },
                        screens::chat::Command::ListTerminals {
                            cursor: Some(cursor),
                        },
                    );
                }
                _ => model.processes.restore(restore, listed),
            }
        }
        Answer::Failed(_) => model.processes.restore(restore, Vec::new()),
        _ => {}
    }
}

fn pending_steer(model: &ThreadModel, client_id: &str) -> Option<usize> {
    model.pending_steers.iter().position(|steer| {
        steer
            .submission
            .as_ref()
            .is_some_and(|(_, id)| id == client_id)
    })
}

/// The answer of a `turn/steer`.
fn steer_reply(
    model: &mut ThreadModel,
    turn: TurnId,
    retried: bool,
    message: UserMessage,
    client_id: String,
    answer: Answer,
    fx: &mut ThreadFx<'_>,
) {
    let position = pending_steer(model, &client_id);
    let failure = match answer {
        Answer::Screen(screens::ScreenReply::Chat(screens::chat::Reply::Steered(admitted))) => {
            if let Some(index) = position {
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
            if reject_image_input(model, fx, &message) {
                if position.is_some() {
                    recover(model, message);
                }
            } else if let Err(message) = fx.start_turn(message) {
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
            if reject_image_input(model, fx, &message) {
                if let Some(index) = position {
                    model.pending_steers.remove(index);
                    recover(model, message);
                }
            } else {
                let ticket = fx.mint();
                steer(fx, ticket, actual, true, message, client_id);
            }
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

/// The messages as one, joined by newlines; the remote images of all of
/// them in order, and the local images' placeholders numbered afresh after
/// them (`merge_user_messages`).
fn merge_messages(messages: Vec<UserMessage>) -> UserMessage {
    let mut merged = UserMessage::default();
    let remote: usize = messages.iter().map(|m| m.remote_images.len()).sum();
    for (index, message) in messages.into_iter().enumerate() {
        if index > 0 {
            merged.text.push('\n');
        }
        merged.remote_images.extend(message.remote_images);
        for mention in message.mentions {
            if !merged.mentions.contains(&mention) {
                merged.mentions.push(mention);
            }
        }
        let base = merged.text.len();
        let mut images = message.images;
        images.sort_by_key(|image| image.range.start);
        let mut cursor = 0;
        // (end in the message's text, bytes the renumbering added) of each image taken
        let mut grown: Vec<(usize, isize)> = Vec::new();
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
                .push_str(&renumbered(placeholder, remote + merged.images.len() + 1));
            merged.images.push(LocalImage {
                path: image.path,
                range: start..merged.text.len(),
            });
            grown.push((
                range.end,
                (merged.text.len() - start) as isize - range.len() as isize,
            ));
            cursor = range.end;
        }
        merged.text.push_str(&message.text[cursor..]);
        // the tasks' ranges move with the text before them
        let mut tasks = std::mem::take(&mut merged.tasks).into_vec();
        tasks.extend(message.tasks.into_vec().into_iter().map(|task| {
            let delta: isize = grown
                .iter()
                .filter(|(end, _)| *end <= task.range.start)
                .map(|(_, delta)| *delta)
                .sum();
            let start = (base + task.range.start).saturating_add_signed(delta);
            let end = start + task.range.len();
            TaskRef {
                range: start..end,
                ..task
            }
        }));
        merged.tasks = tasks.into();
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

/// A newly displayed user input or copyable response supersedes an existing Status source.
fn observe_status_copy_item(model: &mut ThreadModel, key: &ItemKey, item: &Item) {
    if !features::has_copy_status(&model.features) {
        return;
    }
    let supersedes = match item {
        Item::User(_) => model.transcript.item(key).is_none(),
        Item::Agent { text, .. } | Item::Plan { text } => {
            (model.transcript.streaming.contains_key(key)
                || model.transcript.item(key) != Some(item))
                && features::has_visible_copy_response(
                    text,
                    model.slots.chat.cwd.as_deref().unwrap_or("."),
                )
        }
        _ => false,
    };
    if supersedes {
        features::clear_copy_status(&mut model.features);
    }
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
    if matches!(item, Item::Status { .. }) {
        features::remember_copy_status(&mut model.features, key.clone());
    }
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
    if !model.slots.chat.mcp.is_starting() && !model.busy() {
        send_next(model, fx);
    }
}

fn start_turn(model: &mut ThreadModel, turn: TurnId, now: i64) {
    // A turn that already ended is never revived by a late start.
    if model.finished_turns.contains(&turn) {
        return;
    }
    model.core.safety.turn_started(&turn);
    match &model.turn {
        TurnState::Idle => {
            features::clear_copy_status(&mut model.features);
            hooks::clear(model);
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

    /// Messages merged into one keep every remote image in order, and the
    /// local images' placeholders count on after them (`merge_user_messages`).
    #[test]
    fn merged_messages_keep_their_remote_images_and_number_local_ones_after_them() {
        let with_image = |text: &str, remote: &[&str]| {
            let placeholder = "[Image #1]";
            UserMessage {
                text: format!("{placeholder} {text}"),
                images: vec![LocalImage {
                    path: "/tmp/a.png".into(),
                    range: 0..placeholder.len(),
                }],
                remote_images: remote.iter().map(|url| (*url).to_owned()).collect(),
                ..UserMessage::default()
            }
        };
        let merged = merge_messages(vec![
            with_image("one", &["https://example.com/a.png"]),
            with_image("two", &["https://example.com/b.png"]),
        ]);
        assert_eq!(
            merged.remote_images,
            ["https://example.com/a.png", "https://example.com/b.png"]
        );
        assert_eq!(merged.text, "[Image #3] one\n[Image #4] two");
        assert_eq!(merged.images.len(), 2);
    }
}
