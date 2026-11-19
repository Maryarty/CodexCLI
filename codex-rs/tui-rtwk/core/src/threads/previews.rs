//! Recent activity of backend threads, including catalog entries whose
//! transcript is not attached to this process.

use std::collections::BTreeMap;
use std::rc::Rc;

use codex_app_server_protocol::{ServerNotification, ThreadItemPayload, ThreadStatus};

use crate::ids::{ItemId, ThreadId, TurnId};

use crate::screens::agents::{PREVIEW_CHARS, preview_markdown, preview_text};

/// What the latest reasoning summary of a task said (oracle:
/// `AgentsOverviewActivity`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPreview {
    pub latest_prompt: Option<String>,
    /// The reasoning summary being streamed (oracle: `ReasoningPreview`).
    pub reasoning: Option<Reasoning>,
    pub last_message: Option<String>,
    pub history_message: Option<String>,
    pub seeded_request: Option<String>,
}

impl ThreadPreview {
    /// The first `**bold**` run of the streamed summary, once it closed.
    pub fn reasoning_header(&self) -> Option<&String> {
        self.reasoning.as_ref()?.header.as_ref()
    }
}

/// A reasoning summary being streamed (oracle: `ReasoningPreview`): deltas of
/// one `(turn, item, summary index)` accumulate in `buffer` until the first
/// `**bold**` run closes into `header`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reasoning {
    pub turn: TurnId,
    pub item: ItemId,
    pub summary_index: i64,
    pub buffer: String,
    pub header: Option<String>,
}

/// Catalog entries include threads that have no local transcript writer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPreviews {
    entries: Rc<BTreeMap<ThreadId, ThreadPreview>>,
}

impl ThreadPreviews {
    pub fn get(&self, id: &str) -> Option<&ThreadPreview> {
        self.entries.get(id)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries = Rc::default();
    }

    fn entry(&mut self, id: &str) -> &mut ThreadPreview {
        Rc::make_mut(&mut self.entries)
            .entry(ThreadId::new(id))
            .or_default()
    }

    pub(crate) fn forget(&mut self, id: &str) {
        if self.entries.contains_key(id) {
            Rc::make_mut(&mut self.entries).remove(id);
        }
    }

    pub(crate) fn seed_request(&mut self, id: &ThreadId, preview: Option<String>) {
        self.entry(id.as_str()).seeded_request = preview;
    }

    pub(crate) fn record_history(
        &mut self,
        id: &str,
        prompt: Option<String>,
        last_message: Option<String>,
    ) {
        let preview = self.entry(id);
        preview.latest_prompt = prompt;
        if last_message.is_some() {
            preview.history_message = last_message;
        }
    }

    pub fn observe(&mut self, notification: &ServerNotification, listed: bool) {
        use ServerNotification as N;
        match notification {
            N::ThreadArchived(event) => self.forget(&event.thread_id),
            N::ThreadDeleted(event) => self.forget(&event.thread_id),
            N::ThreadClosed(event) => {
                if let Some(preview) = self.get(event.thread_id.as_str())
                    && (preview.latest_prompt.is_some()
                        || preview.reasoning.is_some()
                        || preview.last_message.is_some())
                {
                    let preview = self.entry(&event.thread_id);
                    preview.latest_prompt = None;
                    preview.reasoning = None;
                    preview.last_message = None;
                }
            }
            N::ThreadReverted(event) => self.forget(&event.thread_id),
            N::ThreadStatusChanged(event)
                if !matches!(event.status, ThreadStatus::Active { .. }) =>
            {
                self.end_summary(&event.thread_id);
            }
            N::ItemCompleted(event) => {
                if let ThreadItemPayload::AgentMessage { text, .. } = &event.item.payload {
                    let preview = self.entry(&event.thread_id);
                    preview.last_message = Some(preview_markdown(text));
                    preview.reasoning = None;
                }
            }
            N::ReasoningSummaryTextDelta(delta) if listed => {
                let turn = TurnId::new(&delta.turn_id);
                let item = ItemId::new(&delta.item_id);
                let previous = self
                    .get(delta.thread_id.as_str())
                    .and_then(|preview| preview.reasoning.as_ref());
                let same = previous.is_some_and(|reasoning| {
                    reasoning.turn == turn
                        && reasoning.item == item
                        && reasoning.summary_index == delta.summary_index
                });
                if same && previous.is_some_and(|reasoning| reasoning.header.is_some()) {
                    return;
                }
                let preview = self.entry(&delta.thread_id);
                if !same {
                    preview.reasoning = Some(Reasoning {
                        turn,
                        item,
                        summary_index: delta.summary_index,
                        ..Reasoning::default()
                    });
                }
                if let Some(reasoning) = &mut preview.reasoning {
                    let room = PREVIEW_CHARS.saturating_sub(reasoning.buffer.chars().count());
                    reasoning.buffer.extend(delta.delta.chars().take(room));
                    reasoning.header = first_bold(&reasoning.buffer).map(preview_text);
                }
            }
            N::TurnStarted(event) => self.end_summary(&event.thread_id),
            N::TurnCompleted(event) => self.end_summary(&event.thread_id),
            N::ReasoningSummaryPartAdded(event) => self.end_summary(&event.thread_id),
            N::ItemStarted(event) => self.end_summary(&event.thread_id),
            _ => {}
        }
    }

    fn end_summary(&mut self, id: &str) {
        if self
            .get(id)
            .is_some_and(|preview| preview.reasoning.is_some())
        {
            self.entry(id).reasoning = None;
        }
    }
}

/// The first bold run of the streamed summary (oracle: `extract_first_bold`).
fn first_bold(text: &str) -> Option<&str> {
    let (_, rest) = text.split_once("**")?;
    let (header, _) = rest.split_once("**")?;
    Some(header)
}
