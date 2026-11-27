//! The realtime item stream and the durable timeline use the same caption key.

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use codex_app_server_protocol as wire;

use super::{HistoryState, Item, ItemKey, Text, ThreadFx, Transcript};
use crate::ids::{ItemId, ThreadId, Ticket, TurnId};
use crate::screens::voice;

pub use wire::ThreadRealtimeTranscriptRole as Role;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Started(wire::ThreadRealtimeItem),
    Delta { item: ItemId, text: String },
    Completed(wire::ThreadRealtimeItem),
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

fn caption(item: wire::ThreadRealtimeItem) -> Option<(ItemKey, Role, String)> {
    let wire::ThreadRealtimeItemContent::TranscriptSegment { role, text } = item.content else {
        return None;
    };
    let key = ItemKey {
        // The server's turn IDs form a separate namespace from realtime sessions.
        turn: TurnId::new(format!("#voice/{}", item.realtime_session_id)),
        item: ItemId::new(item.id),
    };
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
                    self.live_voice_items.insert(
                        key.item.clone(),
                        LiveCaption {
                            key: key.clone(),
                            role,
                        },
                    );
                    if show_user_preview || role == Role::Assistant {
                        self.append_text(key, &text, Text::Voice(role));
                    }
                }
            }
            Event::Delta { item, text } => {
                if let Some(live) = self.live_voice_items.get(&item).cloned() {
                    if show_user_preview || live.role == Role::Assistant {
                        self.append_text(live.key, &text, Text::Voice(live.role));
                    } else {
                        self.drop_voice_preview(&live.key);
                    }
                }
            }
            Event::Completed(item) if item.realtime_session_id == session => {
                if let Some((key, role, text)) = caption(item) {
                    self.live_voice_items.remove(&key.item);
                    if text.trim().is_empty() {
                        self.drop_voice_preview(&key);
                    } else {
                        self.upsert(key, Item::VoiceTranscript { role, text });
                    }
                }
            }
            _ => {}
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
                .fold(false, |caption_seen, entry| match entry {
                    Entry::TurnStarted(turn) => {
                        Rc::make_mut(&mut self.voice_replay).seen_turns.insert(turn);
                        caption_seen
                    }
                    Entry::Ordinary(key) => {
                        Rc::make_mut(&mut self.voice_replay)
                            .order
                            .push_front(Anchor::Ordinary(key));
                        caption_seen
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
