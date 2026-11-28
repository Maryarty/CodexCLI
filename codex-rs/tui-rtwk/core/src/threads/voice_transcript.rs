//! The realtime item stream and the durable timeline use the same caption key.

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use codex_app_server_protocol as wire;

use super::{HistoryState, Item, ItemKey, Text, ThreadFx, ThreadModel, Transcript};
use crate::ids::{ItemId, ThreadId, Ticket, TurnId};
use crate::screens::voice;

pub use wire::ThreadRealtimeTranscriptRole as Role;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Started(wire::ThreadRealtimeItem),
    Delta { item: ItemId, text: String },
    Completed(wire::ThreadRealtimeItem),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum History {
    #[default]
    Unknown,
    Canonical,
    Legacy,
}

impl History {
    pub(crate) fn of(thread: &wire::Thread) -> Self {
        if thread.history_mode == wire::ThreadHistoryMode::Paginated && !thread.ephemeral {
            Self::Canonical
        } else {
            Self::Legacy
        }
    }

    pub(crate) fn canonical(self) -> bool {
        self == Self::Canonical
    }
}

/// The pinned publisher emits one ordered stream per speaker within an owned call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Legacy {
    ordered: bool,
    user: Option<FlatSegment>,
    assistant: Option<FlatSegment>,
    current: Option<Role>,
    next: u64,
}

impl Legacy {
    pub(crate) fn new() -> Self {
        Self {
            ordered: true,
            user: None,
            assistant: None,
            current: None,
            next: 0,
        }
    }

    pub(crate) fn ordered(&self) -> bool {
        self.ordered
    }

    pub(crate) fn lose_order(&mut self) {
        self.ordered = false;
        self.user = None;
        self.assistant = None;
    }

    fn take(&mut self, role: Role) -> Option<FlatSegment> {
        match role {
            Role::User => self.user.take(),
            Role::Assistant => self.assistant.take(),
        }
    }

    fn completed(&mut self, role: Role) -> ItemId {
        self.take(role)
            .map_or_else(|| next_item(&mut self.next), |segment| segment.item)
    }

    fn append(&mut self, role: Role, text: &str) -> &FlatSegment {
        self.current = Some(role);
        let slot = match role {
            Role::User => &mut self.user,
            Role::Assistant => &mut self.assistant,
        };
        let segment = slot.get_or_insert_with(|| FlatSegment {
            item: next_item(&mut self.next),
            text: String::new(),
        });
        segment.text.push_str(text);
        segment
    }
}

fn next_item(next: &mut u64) -> ItemId {
    *next += 1;
    ItemId::new(format!("#{}", *next))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FlatSegment {
    item: ItemId,
    text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatCaption {
    role: Role,
    text: String,
    completed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEvent {
    Caption {
        event: Event,
        show_user_preview: bool,
    },
    LegacyCaption {
        caption: FlatCaption,
        show_user_preview: bool,
    },
    LegacyClosed(voice::CloseReason),
    LostLegacyOrder,
    Supported(History),
    Read {
        oldest: Option<TurnId>,
        more_history: bool,
    },
    LoadOlder,
}

pub(super) fn apply(model: &mut ThreadModel, event: ThreadEvent, fx: &mut ThreadFx<'_>) {
    match event {
        ThreadEvent::Supported(history) => model.slots.voice.set_history(history),
        ThreadEvent::Read {
            oldest,
            more_history,
        } => {
            model
                .core
                .transcript
                .read_voice_history(oldest, more_history, fx);
        }
        ThreadEvent::LoadOlder => model.core.transcript.load_older_voice_history(fx),
        ThreadEvent::LegacyClosed(reason) => {
            if model.slots.voice.accepts_close(reason)
                && let Some((session, legacy)) = model.slots.voice.legacy_captions(fx.address)
            {
                model.core.transcript.seal_legacy_captions(session, legacy);
            }
            voice::apply_thread(&mut model.slots.voice, voice::ThreadEv::Closed(reason), fx);
        }
        ThreadEvent::LostLegacyOrder => {
            if model.slots.voice.lose_legacy_order() {
                model.core.transcript.clear_live_voice();
            }
        }
        ThreadEvent::LegacyCaption {
            caption,
            show_user_preview,
        } => {
            if let Some((session, legacy)) = model.slots.voice.legacy_captions(fx.address) {
                model.core.transcript.apply_legacy_caption(
                    session,
                    legacy,
                    caption,
                    show_user_preview,
                );
            }
        }
        ThreadEvent::Caption {
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
    }
}

pub(crate) fn project(notification: &wire::ServerNotification) -> Option<(ThreadId, Event)> {
    use wire::ServerNotification as Note;
    let (thread, event) = match notification {
        Note::ThreadRealtimeItemStarted(note) => {
            (&note.thread_id, Event::Started(note.item.clone()))
        }
        Note::ThreadRealtimeItemTranscriptDelta(note) => (
            &note.thread_id,
            Event::Delta {
                item: ItemId::new(&note.item_id),
                text: note.delta.clone(),
            },
        ),
        Note::ThreadRealtimeItemCompleted(note) => {
            (&note.thread_id, Event::Completed(note.item.clone()))
        }
        _ => return None,
    };
    Some((ThreadId::new(thread), event))
}

pub(crate) fn project_legacy(
    notification: &wire::ServerNotification,
    accepts: impl FnOnce(&ThreadId) -> bool,
) -> Option<(ThreadId, FlatCaption)> {
    use wire::ServerNotification as Note;
    let (thread, role, text, completed) = match notification {
        Note::ThreadRealtimeTranscriptDelta(note) => {
            (&note.thread_id, &note.role, &note.delta, false)
        }
        Note::ThreadRealtimeTranscriptDone(note) => (&note.thread_id, &note.role, &note.text, true),
        _ => return None,
    };
    let role = match role.as_str() {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        _ => return None,
    };
    let thread = ThreadId::new(thread);
    if !accepts(&thread) {
        return None;
    }
    Some((
        thread,
        FlatCaption {
            role,
            text: text.clone(),
            completed,
        },
    ))
}

fn key(session: &str, item: ItemId) -> ItemKey {
    // The server's turn IDs form a separate namespace from realtime sessions.
    ItemKey {
        turn: TurnId::new(format!("#/{session}")),
        item,
    }
}

fn caption(item: wire::ThreadRealtimeItem) -> Option<(ItemKey, Role, String)> {
    let wire::ThreadRealtimeItemContent::TranscriptSegment { role, text } = item.content else {
        return None;
    };
    let key = key(&item.realtime_session_id, ItemId::new(item.id));
    Some((key, role, text))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LiveCaption {
    key: ItemKey,
    role: Role,
}

impl Transcript {
    pub(super) fn apply_voice_caption(
        &mut self,
        session: &str,
        event: Event,
        show_user_preview: bool,
    ) {
        match event {
            Event::Started(item) if item.realtime_session_id == session => {
                if let Some((key, role, text)) = caption(item)
                    && !self.items.contains_key(&key)
                    && !self.live_voice_items.contains_key(&key.item)
                {
                    self.preview_voice_caption(key, role, &text, show_user_preview);
                }
            }
            Event::Delta { item, text } => {
                if let Some(live) = self.live_voice_items.get(&item).cloned() {
                    self.append_voice_preview(live.key, live.role, &text, show_user_preview);
                }
            }
            Event::Completed(item) if item.realtime_session_id == session => {
                if let Some((key, role, text)) = caption(item) {
                    self.finish_voice_caption(key, role, text);
                }
            }
            _ => {}
        }
    }

    fn apply_legacy_caption(
        &mut self,
        session: &str,
        legacy: &mut Legacy,
        caption: FlatCaption,
        show_user_preview: bool,
    ) {
        let FlatCaption {
            role,
            text,
            completed,
        } = caption;
        if completed {
            self.finish_voice_caption(key(session, legacy.completed(role)), role, text);
        } else if !text.is_empty() {
            let segment = legacy.append(role, &text);
            let key = key(session, segment.item.clone());
            let delta = if self.streaming.contains_key(&key) {
                &text
            } else {
                &segment.text
            };
            self.preview_voice_caption(key, role, delta, show_user_preview);
        }
    }

    fn seal_legacy_captions(&mut self, session: &str, legacy: &mut Legacy) {
        // The oracle flushes the interleaved speaker before the current speaker.
        let order = if legacy.current == Some(Role::User) {
            [Role::Assistant, Role::User]
        } else {
            [Role::User, Role::Assistant]
        };
        for role in order {
            if let Some(segment) = legacy.take(role) {
                let key = key(session, segment.item);
                self.drop_voice_preview(&key);
                self.finish_voice_caption(key, role, segment.text);
            }
        }
    }

    fn preview_voice_caption(
        &mut self,
        key: ItemKey,
        role: Role,
        text: &str,
        show_user_preview: bool,
    ) {
        self.live_voice_items
            .entry(key.item.clone())
            .or_insert_with(|| LiveCaption {
                key: key.clone(),
                role,
            });
        self.append_voice_preview(key, role, text, show_user_preview);
    }

    fn append_voice_preview(
        &mut self,
        key: ItemKey,
        role: Role,
        text: &str,
        show_user_preview: bool,
    ) {
        if show_user_preview || role == Role::Assistant {
            self.append_text(key, text, Text::Voice(role));
        } else {
            self.drop_voice_preview(&key);
        }
    }

    fn finish_voice_caption(&mut self, key: ItemKey, role: Role, text: String) {
        self.live_voice_items.remove(&key.item);
        if text.trim().is_empty() {
            self.drop_voice_preview(&key);
        } else {
            self.upsert(key, Item::VoiceTranscript { role, text });
        }
    }

    fn drop_voice_preview(&mut self, key: &ItemKey) {
        if self.streaming.remove(key).is_some() && !self.items.contains_key(key) {
            Rc::make_mut(&mut self.order).retain(|held| held != key);
            self.layout_revision += 1;
        }
    }

    pub(super) fn clear_live_voice(&mut self) {
        if self.live_voice_items.is_empty() {
            return;
        }
        let live = std::mem::take(&mut self.live_voice_items);
        let mut removed = false;
        for item in live.values() {
            removed |= self.streaming.remove(&item.key).is_some();
        }
        if removed {
            Rc::make_mut(&mut self.order).retain(|key| {
                live.get(&key.item).is_none_or(|item| item.key != *key)
                    || self.items.contains_key(key)
            });
            self.layout_revision += 1;
        }
    }
}

pub(super) const PAGE_LIMIT: u32 = 100;
const PAGES_PER_READ: u8 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    TurnStarted(TurnId),
    Ordinary(ItemKey),
    Caption(ItemKey, Role, String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    entries: Vec<Entry>,
    next: Option<String>,
}

impl From<wire::ThreadTimelineListResponse> for Page {
    fn from(response: wire::ThreadTimelineListResponse) -> Self {
        use wire::ThreadTimelineEntry as Timeline;
        let next = response.next_cursor.filter(|_| !response.data.is_empty());
        let entries = response
            .data
            .into_iter()
            .filter_map(|entry| match entry {
                Timeline::TurnStarted { turn_id, .. } => {
                    Some(Entry::TurnStarted(TurnId::new(turn_id)))
                }
                Timeline::Item { turn_id, item, .. } => Some(Entry::Ordinary(ItemKey {
                    turn: TurnId::new(turn_id),
                    item: ItemId::new(item.id()),
                })),
                Timeline::Realtime { item, .. } => {
                    let (key, role, text) = caption(item)?;
                    (!text.trim().is_empty()).then_some(Entry::Caption(key, role, text))
                }
                Timeline::TurnCompleted { .. } => None,
            })
            .collect();
        Self { entries, next }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Boundary {
    Turn(TurnId),
    Beginning,
}

/// The timeline cursor belongs to this transcript generation. Normal history controls how
/// far to read it. Each page is bounded by the protocol and can expose additional anchors.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Replay {
    cursor: Option<String>,
    seen_cursors: HashSet<String>,
    seen_turns: HashSet<TurnId>,
    target: Option<Boundary>,
    pending: Option<Ticket>,
    exhausted: bool,
    remaining: u8,
    foreground: bool,
    order: VecDeque<Anchor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Anchor {
    Ordinary(ItemKey),
    Caption(ItemKey),
}

impl Replay {
    fn needs_page(&self) -> bool {
        !self.exhausted
            && match &self.target {
                Some(Boundary::Beginning) => true,
                Some(Boundary::Turn(turn)) => !self.seen_turns.contains(turn),
                None => false,
            }
    }

    pub(super) fn paused(&self) -> bool {
        self.pending.is_none() && self.remaining == 0 && self.needs_page()
    }

    pub(super) fn history_state(&self) -> HistoryState {
        if self.foreground && self.pending.is_some() {
            HistoryState::Loading
        } else if self.paused() {
            HistoryState::Partial
        } else {
            HistoryState::Complete
        }
    }

    fn request(&mut self, fx: &mut ThreadFx<'_>) {
        if self.remaining > 0 && self.needs_page() && self.pending.is_none() {
            self.remaining -= 1;
            self.pending = Some(fx.request(
                voice::Ctx::ReadTimeline,
                voice::Command::ReadTimeline {
                    cursor: self.cursor.clone(),
                },
            ));
        }
    }
}

impl Transcript {
    pub(super) fn read_voice_history(
        &mut self,
        oldest: Option<TurnId>,
        more_history: bool,
        fx: &mut ThreadFx<'_>,
    ) {
        let replay = Rc::make_mut(&mut self.voice_replay);
        replay.remaining = PAGES_PER_READ;
        if !more_history {
            replay.target = Some(Boundary::Beginning);
        } else if let Some(oldest) = oldest
            && replay.target != Some(Boundary::Beginning)
        {
            replay.target = Some(Boundary::Turn(oldest));
        }
        replay.request(fx);
    }

    pub(super) fn load_older_voice_history(&mut self, fx: &mut ThreadFx<'_>) {
        if self.voice_replay.paused() {
            let replay = Rc::make_mut(&mut self.voice_replay);
            replay.foreground = true;
            replay.remaining = PAGES_PER_READ;
            replay.request(fx);
        }
    }

    pub(super) fn receive_voice_history(
        &mut self,
        ticket: Ticket,
        page: Option<Page>,
        fx: &mut ThreadFx<'_>,
    ) {
        if self.voice_replay.pending != Some(ticket) {
            return;
        }
        let replay = Rc::make_mut(&mut self.voice_replay);
        replay.pending = None;
        let Some(page) = page else {
            // An older server can decline the experimental read. Its ordinary history still works.
            replay.exhausted = true;
            return;
        };

        let changed =
            page.entries
                .into_iter()
                .rev()
                .fold(false, |layout_changed, entry| match entry {
                    Entry::TurnStarted(turn) => {
                        Rc::make_mut(&mut self.voice_replay).seen_turns.insert(turn);
                        layout_changed
                    }
                    Entry::Ordinary(key) => {
                        Rc::make_mut(&mut self.voice_replay)
                            .order
                            .push_front(Anchor::Ordinary(key));
                        true
                    }
                    Entry::Caption(key, role, text) => {
                        self.live_voice_items.remove(&key.item);
                        self.upsert(key.clone(), Item::VoiceTranscript { role, text });
                        Rc::make_mut(&mut self.voice_replay)
                            .order
                            .push_front(Anchor::Caption(key));
                        true
                    }
                });
        if changed {
            self.position_voice_history();
        }
        let replay = Rc::make_mut(&mut self.voice_replay);
        replay.cursor = page
            .next
            .filter(|cursor| replay.seen_cursors.insert(cursor.clone()));
        replay.exhausted = replay.cursor.is_none();
        replay.request(fx);
    }

    pub(super) fn position_voice_history(&mut self) {
        let timeline = &self.voice_replay.order;
        let captions: HashSet<_> = timeline
            .iter()
            .filter_map(|entry| match entry {
                Anchor::Caption(key) => Some(key),
                Anchor::Ordinary(_) => None,
            })
            .collect();
        if captions.is_empty() {
            return;
        }

        let base: Vec<_> = self
            .order
            .iter()
            .filter(|key| !captions.contains(key))
            .collect();
        let indices: HashMap<_, _> = base
            .iter()
            .enumerate()
            .map(|(index, &key)| (key, index))
            .collect();
        let mut index = timeline
            .iter()
            .rev()
            .find_map(|entry| match entry {
                Anchor::Ordinary(key) => indices.get(key).map(|index| index + 1),
                Anchor::Caption(_) => None,
            })
            .unwrap_or_default();

        let mut placements: HashMap<usize, Vec<&ItemKey>> = HashMap::new();
        let mut placed = HashSet::new();
        for entry in timeline.iter().rev() {
            match entry {
                Anchor::Ordinary(key) => {
                    if let Some(&anchor) = indices.get(key) {
                        index = anchor;
                    }
                }
                Anchor::Caption(key) if placed.insert(key) => {
                    placements.entry(index).or_default().push(key);
                }
                Anchor::Caption(_) => {}
            }
        }

        let mut order = Vec::with_capacity(self.order.len());
        for (index, key) in base
            .into_iter()
            .map(Some)
            .chain(std::iter::once(None))
            .enumerate()
        {
            if let Some(captions) = placements.remove(&index) {
                order.extend(captions.into_iter().rev().cloned());
            }
            order.extend(key.cloned());
        }
        if order != *self.order {
            self.order = Rc::new(order);
            self.layout_revision += 1;
        }
    }
}
