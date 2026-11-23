//! Replay of a persisted thread (oracle `chatwidget/replay.rs` `replay_thread_turns`): the turns a
//! resume or a history page loads go into the transcript without the live side effects, and
//! three rules decide which user messages count.
//!
//! - A user message inside a review (between `EnteredReviewMode` and `ExitedReviewMode`, across
//!   turns) is the review prompt the server made, not something the user typed: it adds no row
//!   and seeds nothing (`on_committed_user_message`: `from_replay` and `is_review_mode`).
//! - The user messages of a hidden nested review turn are skipped (`is_hidden_nested_review_turn`:
//!   an interrupted turn right after a finished review whose two user messages are equal), and
//!   the turn counts as completed.
//! - Every other user message is a submission the composer's history is seeded with
//!   (`record_replayed_user_message_history`). Core keeps each as the typed prompt the composer
//!   restores from (`backtrack::Prompt`), oldest first; the composer derives its entries from it
//!   (mention bindings, placeholders, attachments).

use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnItemsView;
use codex_app_server_protocol::TurnStatus;

use crate::screens::backtrack::Prompt;

use super::Fault;
use super::HistoryTurn;
use super::ItemId;
use super::ItemKey;
use super::TurnEnd;
use super::TurnId;
use super::TurnSnapshot;
use super::TurnTiming;
use super::item_entry;
use super::turn_end;

/// A user message of the replayed turns that seeds the composer's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    /// The transcript row of the message.
    pub key: ItemKey,
    /// What the server stored of it: text and its elements, images, skills, mentions.
    pub prompt: Prompt,
}

/// The replayed submissions of one thread, oldest first. `generation` grows whenever a page of
/// older turns went in front of the entries a composer may already have seeded from: it then
/// seeds again from the whole list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Submissions {
    generation: u64,
    entries: Vec<Submission>,
}

impl Submissions {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn entries(&self) -> &[Submission] {
        &self.entries
    }

    /// A page of turns older than the held ones; a message held already is not added twice.
    pub(super) fn prepend(&mut self, mut older: Vec<Submission>) {
        older.retain(|new| self.entries.iter().all(|held| held.key != new.key));
        if older.is_empty() {
            return;
        }
        older.append(&mut self.entries);
        self.entries = older;
        self.generation += 1;
    }
}

/// Oracle `is_hidden_nested_review_turn(previous, turn)`.
fn hidden_nested_review_turn(previous: &Turn, turn: &Turn) -> bool {
    let has =
        |turn: &Turn, wanted: fn(&ThreadItemPayload) -> bool| turn.item_payloads().any(wanted);
    if previous.status != TurnStatus::Completed
        || turn.status != TurnStatus::Interrupted
        || turn.completed_at.is_some()
        || !has(previous, |item| {
            matches!(item, ThreadItemPayload::EnteredReviewMode { .. })
        })
        || !has(previous, |item| {
            matches!(item, ThreadItemPayload::ExitedReviewMode { .. })
        })
    {
        return false;
    }
    let mut messages = turn.item_payloads().filter_map(|item| match item {
        ThreadItemPayload::UserMessage { content, .. } => Some(content),
        _ => None,
    });
    matches!(
        (messages.next(), messages.next(), messages.next()),
        (Some(first), Some(second), None) if first == second
    )
}

/// The turns of a page, newest first as the server lists them, projected in transcript order,
/// with the submissions they seed (also in transcript order).
pub(super) fn project(turns: &[Turn]) -> (Vec<HistoryTurn>, Vec<Submission>) {
    let chronological: Vec<&Turn> = turns.iter().rev().collect();
    let mut in_review = false;
    let mut submissions = Vec::new();
    let mut projected = Vec::new();
    for (index, turn) in chronological.iter().enumerate() {
        let hidden = index
            .checked_sub(1)
            .is_some_and(|before| hidden_nested_review_turn(chronological[before], turn));
        let turn_id = TurnId::new(&turn.id);
        let mut items = Vec::new();
        for payload in turn.item_payloads() {
            match payload {
                ThreadItemPayload::EnteredReviewMode { .. } => in_review = true,
                ThreadItemPayload::ExitedReviewMode { .. } => in_review = false,
                ThreadItemPayload::UserMessage { id, content, .. } => {
                    if hidden || in_review {
                        continue;
                    }
                    submissions.push(Submission {
                        key: ItemKey {
                            turn: turn_id.clone(),
                            item: ItemId::new(id),
                        },
                        prompt: Prompt::from_inputs(content),
                    });
                }
                _ => {}
            }
            if let Some(entry) = item_entry(payload, true) {
                items.push(super::cells::history(payload, entry));
            }
        }
        let items = match turn.items_view {
            TurnItemsView::Full => TurnSnapshot::Full(items),
            TurnItemsView::Summary => TurnSnapshot::Summary(items),
            // Oracle `replay_thread_turns`: a finished turn's rows wait for their page, but an
            // active or failed turn restores its lifecycle before any item is available.
            TurnItemsView::NotLoaded
                if matches!(turn.status, TurnStatus::Completed | TurnStatus::Interrupted) =>
            {
                continue;
            }
            TurnItemsView::NotLoaded => TurnSnapshot::Summary(items),
        };
        projected.push(HistoryTurn {
            turn: turn_id,
            end: if hidden {
                Some(TurnEnd::Completed)
            } else {
                turn_end(&turn.status)
            },
            started_at: turn.started_at,
            timing: TurnTiming::from_turn(turn),
            error: turn.error.as_ref().map(Fault::of),
            items,
        });
    }
    (projected, submissions)
}
