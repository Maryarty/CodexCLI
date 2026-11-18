//! Server notifications of threads: the oracle's
//! `track_agents_overview_notification` and `track_agents_overview_activity`.
//! Oracle: `agents_overview_threads.rs`.

use super::Model;
use super::carried::preview_markdown;
use super::carried::preview_text;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadStatus;

use super::refresh::normalize;

/// Whether `observe` reads `notification`.
pub fn observes(notification: &ServerNotification) -> bool {
    matches!(
        notification,
        ServerNotification::ThreadStarted(_)
            | ServerNotification::ThreadStatusChanged(_)
            | ServerNotification::ThreadNameUpdated(_)
            | ServerNotification::ThreadArchived(_)
            | ServerNotification::ThreadDeleted(_)
            | ServerNotification::ThreadClosed(_)
            | ServerNotification::ThreadReadStateChanged(_)
            | ServerNotification::ThreadTokenUsageUpdated(_)
            | ServerNotification::ItemCompleted(_)
            | ServerNotification::ReasoningSummaryTextDelta(_)
            | ServerNotification::TurnStarted(_)
    )
}

/// A server notification: the oracle's `track_agents_overview_notification`
/// and `track_agents_overview_activity`.
pub fn observe(model: &mut Model, notification: &ServerNotification) {
    match notification {
        ServerNotification::ThreadStarted(started) => {
            if !started.thread.ephemeral {
                let mut thread = started.thread.clone();
                thread.turns.clear();
                let thread = normalize(thread);
                match model.threads.iter_mut().find(|held| held.id == thread.id) {
                    Some(held) => *held = thread,
                    None => model.threads.insert(0, thread),
                }
            }
        }
        ServerNotification::ThreadStatusChanged(change) => {
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == change.thread_id) {
                thread.status = change.status.clone();
            }
            if !matches!(change.status, ThreadStatus::Active { .. })
                && let Some(activity) = model.activity.get_mut(&change.thread_id)
            {
                activity.reasoning_header = None;
            }
        }
        ServerNotification::ThreadNameUpdated(update) => {
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == update.thread_id) {
                thread.name.clone_from(&update.thread_name);
            }
        }
        ServerNotification::ThreadArchived(gone) => remove(model, &gone.thread_id),
        ServerNotification::ThreadDeleted(gone) => remove(model, &gone.thread_id),
        ServerNotification::ThreadClosed(closed) => {
            model.activity.remove(&closed.thread_id);
            if let Some(usage) = model.usage.get_mut(&closed.thread_id) {
                usage.tokens = None;
            }
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == closed.thread_id) {
                thread.status = ThreadStatus::NotLoaded;
            }
        }
        ServerNotification::ThreadReadStateChanged(change) => {
            if change.read_state.first_unread.is_some() {
                model.unread.insert(change.thread_id.clone());
            } else {
                model.unread.remove(&change.thread_id);
            }
        }
        ServerNotification::ThreadTokenUsageUpdated(update) => {
            if model.threads.iter().any(|t| t.id == update.thread_id) {
                model
                    .usage
                    .entry(update.thread_id.clone())
                    .or_default()
                    .tokens = Some((
                    update.token_usage.total.input_tokens,
                    update.token_usage.total.output_tokens,
                ));
            }
        }
        ServerNotification::ItemCompleted(ItemCompletedNotification {
            thread_id, item, ..
        }) => {
            if let ThreadItemPayload::AgentMessage { text, .. } = &item.payload {
                let activity = model.activity.entry(thread_id.clone()).or_default();
                activity.last_message = Some(preview_markdown(text));
                activity.reasoning_header = None;
            }
        }
        ServerNotification::ReasoningSummaryTextDelta(delta) => {
            let activity = model.activity.entry(delta.thread_id.clone()).or_default();
            if activity.reasoning_header.is_none() {
                let buffer = activity.latest_prompt.clone().unwrap_or_default();
                let _ = buffer;
                activity.reasoning_header = first_bold(&delta.delta).map(|h| preview_text(&h));
            }
        }
        ServerNotification::TurnStarted(started) => {
            if let Some(activity) = model.activity.get_mut(&started.thread_id) {
                activity.reasoning_header = None;
            }
        }
        _ => return,
    }
    model.reconcile();
}

fn remove(model: &mut Model, id: &str) {
    model.remove_subtree(id);
    model.activity.remove(id);
    model.last_messages.remove(id);
    model.usage.remove(id);
}

/// The first `**bold**` run of `text` (oracle: `extract_first_bold`).
fn first_bold(text: &str) -> Option<String> {
    let start = text.find("**")? + 2;
    let end = text[start..].find("**")? + start;
    Some(text[start..end].to_owned())
}
