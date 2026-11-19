//! Server notifications of threads: the oracle's
//! `track_agents_overview_notification` and `track_agents_overview_activity`.
//! Oracle: `agents_overview_threads.rs`.

use super::Model;
use super::Out;
use super::Reasoning;
use super::carried::PREVIEW_CHARS;
use super::carried::preview_markdown;
use super::carried::preview_text;
use codex_app_server_protocol::AuthMode;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadStatus;

use super::refresh::normalize;

/// The thread a notification is about (oracle: `server_notification_thread_target`
/// in `app_server_event_targets.rs`, adapted: only the notifications this
/// app reads).
pub fn notification_thread(notification: &ServerNotification) -> Option<&str> {
    use ServerNotification as N;
    Some(match notification {
        N::ThreadStarted(n) => &n.thread.id,
        N::ThreadStatusChanged(n) => &n.thread_id,
        N::ThreadNameUpdated(n) => &n.thread_id,
        N::ThreadArchived(n) => &n.thread_id,
        N::ThreadDeleted(n) => &n.thread_id,
        N::ThreadClosed(n) => &n.thread_id,
        N::ThreadReverted(n) => &n.thread_id,
        N::ThreadReadStateChanged(n) => &n.thread_id,
        N::ThreadTokenUsageUpdated(n) => &n.thread_id,
        N::TurnStarted(n) => &n.thread_id,
        N::TurnCompleted(n) => &n.thread_id,
        N::ItemStarted(n) => &n.thread_id,
        N::ItemCompleted(n) => &n.thread_id,
        N::AgentMessageDelta(n) => &n.thread_id,
        N::ReasoningSummaryTextDelta(n) => &n.thread_id,
        N::ReasoningSummaryPartAdded(n) => &n.thread_id,
        N::ServerRequestResolved(n) => &n.thread_id,
        N::Error(n) => &n.thread_id,
        _ => return None,
    })
}

/// Whether `observe` would change `model` for `notification`: decided on
/// `&Model`, so that a notification that changes nothing (a reasoning delta
/// after the header) copies no state.
pub fn wants(model: &Model, notification: &ServerNotification) -> bool {
    match notification {
        ServerNotification::ReasoningSummaryTextDelta(delta) => {
            if !model.threads.iter().any(|t| t.id == delta.thread_id) {
                return false;
            }
            match model
                .activity
                .get(&delta.thread_id)
                .and_then(|activity| activity.reasoning.as_ref())
            {
                Some(reasoning) => {
                    reasoning.header.is_none()
                        || reasoning.turn != delta.turn_id
                        || reasoning.item != delta.item_id
                        || reasoning.summary_index != delta.summary_index
                }
                None => true,
            }
        }
        // These only end a streamed summary.
        ServerNotification::TurnCompleted(done) => streaming(model, &done.thread_id),
        ServerNotification::ReasoningSummaryPartAdded(part) => streaming(model, &part.thread_id),
        ServerNotification::ItemStarted(started) => streaming(model, &started.thread_id),
        other => observes(other),
    }
}

fn streaming(model: &Model, thread: &str) -> bool {
    model
        .activity
        .get(thread)
        .is_some_and(|activity| activity.reasoning.is_some())
}

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
            | ServerNotification::ThreadReverted(_)
            | ServerNotification::ItemCompleted(_)
            | ServerNotification::ReasoningSummaryTextDelta(_)
            | ServerNotification::TurnStarted(_)
            | ServerNotification::TurnCompleted(_)
            | ServerNotification::ReasoningSummaryPartAdded(_)
            | ServerNotification::ItemStarted(_)
            | ServerNotification::AccountUpdated(_)
    )
}

/// A server notification: the oracle's `track_agents_overview_notification`
/// and `track_agents_overview_activity`. The tasks it changed are read again
/// (oracle: `refresh_thread_ids`), as the RPCs returned.
pub fn observe(model: &mut Model, notification: &ServerNotification) -> Vec<Out> {
    let id = changed_thread(notification);
    let held = |model: &Model, id: &str| model.threads.iter().any(|held| held.id == id);
    // The read of a held task is asked first: what the notification sets is
    // newer than it and is replayed over its answer (`overlay`).
    let mut outs = match id {
        Some(id) if held(model, id) => super::reads::want(model, [id.to_owned()]),
        _ => Vec::new(),
    };
    apply(model, notification);
    // A task the notification added is read too.
    if let Some(id) = id
        && outs.is_empty()
        && !model.reading.contains(id)
        && held(model, id)
    {
        outs = super::reads::want(model, [id.to_owned()]);
    }
    outs
}

/// The task a notification changed the row of: the oracle re-reads every
/// tracked thread's notification but the reverted, token and read-state ones.
fn changed_thread(notification: &ServerNotification) -> Option<&str> {
    match notification {
        ServerNotification::ThreadStarted(started) => Some(&started.thread.id),
        ServerNotification::ThreadClosed(closed) => Some(&closed.thread_id),
        ServerNotification::ThreadStatusChanged(change) => Some(&change.thread_id),
        ServerNotification::ThreadNameUpdated(update) => Some(&update.thread_id),
        _ => None,
    }
}

fn apply(model: &mut Model, notification: &ServerNotification) {
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
            if let Some(overlay) = overlay(model, &change.thread_id) {
                overlay.status = Some(change.status.clone());
            }
            if !matches!(change.status, ThreadStatus::Active { .. })
                && let Some(activity) = model.activity.get_mut(&change.thread_id)
            {
                activity.reasoning = None;
            }
        }
        ServerNotification::ThreadNameUpdated(update) => {
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == update.thread_id) {
                thread.name.clone_from(&update.thread_name);
            }
            if let Some(overlay) = overlay(model, &update.thread_id) {
                overlay.name = Some(update.thread_name.clone());
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
            if let Some(overlay) = overlay(model, &closed.thread_id) {
                overlay.status = Some(ThreadStatus::NotLoaded);
            }
        }
        // The history was rewound: what was derived from it is stale (oracle:
        // `ThreadReverted`).
        ServerNotification::ThreadReverted(reverted) => {
            model.activity.remove(&reverted.thread_id);
            model.last_messages.remove(&reverted.thread_id);
            if model.turning.contains(&reverted.thread_id) {
                model.stale.insert(reverted.thread_id.clone());
            }
            if let Some(usage) = model.usage.get_mut(&reverted.thread_id) {
                usage.tokens = None;
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
                activity.reasoning = None;
            }
        }
        ServerNotification::ReasoningSummaryTextDelta(delta) => {
            if !model.threads.iter().any(|t| t.id == delta.thread_id) {
                return;
            }
            let activity = model.activity.entry(delta.thread_id.clone()).or_default();
            if activity.reasoning.as_ref().is_none_or(|reasoning| {
                reasoning.turn != delta.turn_id
                    || reasoning.item != delta.item_id
                    || reasoning.summary_index != delta.summary_index
            }) {
                activity.reasoning = Some(Reasoning {
                    turn: delta.turn_id.clone(),
                    item: delta.item_id.clone(),
                    summary_index: delta.summary_index,
                    ..Reasoning::default()
                });
            }
            if let Some(reasoning) = activity.reasoning.as_mut()
                && reasoning.header.is_none()
            {
                let room = PREVIEW_CHARS.saturating_sub(reasoning.buffer.chars().count());
                reasoning.buffer.extend(delta.delta.chars().take(room));
                reasoning.header = first_bold(&reasoning.buffer).map(|h| preview_text(&h));
            }
        }
        ServerNotification::TurnStarted(started) => end_stream(model, &started.thread_id),
        ServerNotification::TurnCompleted(done) => end_stream(model, &done.thread_id),
        ServerNotification::ReasoningSummaryPartAdded(part) => end_stream(model, &part.thread_id),
        ServerNotification::ItemStarted(started) => end_stream(model, &started.thread_id),
        // A new account: what the old one had of usage is dropped, and the
        // plan and the Codex backend auth are the new ones (oracle:
        // `AccountUpdated`).
        ServerNotification::AccountUpdated(update) => {
            super::usage::reset(model);
            model.plan = update.plan_type;
            model.backend_auth = matches!(
                update.auth_mode,
                Some(
                    AuthMode::Chatgpt
                        | AuthMode::ChatgptAuthTokens
                        | AuthMode::AgentIdentity
                        | AuthMode::PersonalAccessToken
                )
            );
        }
        _ => {}
    }
}

/// The overlay of a task being read: what notifications set meanwhile.
fn overlay<'a>(model: &'a mut Model, thread: &str) -> Option<&'a mut super::Overlay> {
    model
        .reading
        .contains(thread)
        .then(|| model.overlay.entry(thread.to_owned()).or_default())
}

fn end_stream(model: &mut Model, thread: &str) {
    if let Some(activity) = model.activity.get_mut(thread) {
        activity.reasoning = None;
    }
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
