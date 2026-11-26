//! Server notifications of threads: the oracle's
//! `track_agents_overview_notification` and `track_agents_overview_activity`.
//! Oracle: `agents_overview_threads.rs`.

use super::{DetailRead, Model, Out};
use codex_app_server_protocol::{AuthMode, ServerNotification, ThreadStatus};

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
            | ServerNotification::ThreadUnarchived(_)
            | ServerNotification::ThreadSettingsUpdated(_)
            | ServerNotification::ThreadClosed(_)
            | ServerNotification::ThreadReadStateChanged(_)
            | ServerNotification::ThreadTokenUsageUpdated(_)
            | ServerNotification::ThreadReverted(_)
            | ServerNotification::AccountUpdated(_)
    )
}

/// A server notification: the oracle's `track_agents_overview_notification`
/// and `track_agents_overview_activity`. The tasks it changed are read again
/// (oracle: `refresh_thread_ids`), as the RPCs returned.
pub fn observe(model: &mut Model, notification: &ServerNotification) -> Vec<Out> {
    let descendant_read = super::refresh::descendant_changed(model, notification);
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
        && !model.reading.contains_key(id)
        && held(model, id)
    {
        outs = super::reads::want(model, [id.to_owned()]);
    }
    outs.extend(descendant_read);
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
        ServerNotification::ThreadSettingsUpdated(update) => Some(&update.thread_id),
        _ => None,
    }
}

fn apply(model: &mut Model, notification: &ServerNotification) {
    match notification {
        ServerNotification::ThreadStarted(started) => {
            if !started.thread.ephemeral {
                model.removed.remove(&started.thread.id);
                model.descendant_archives.remove(started.thread.id.as_str());
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
        }
        ServerNotification::ThreadNameUpdated(update) => {
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == update.thread_id) {
                thread.name.clone_from(&update.thread_name);
            }
            if let Some(overlay) = overlay(model, &update.thread_id) {
                overlay.name = Some(update.thread_name.clone());
            }
        }
        ServerNotification::ThreadUnarchived(back) => {
            model.removed.remove(&back.thread_id);
            model.descendant_archives.remove(back.thread_id.as_str());
        }
        ServerNotification::ThreadSettingsUpdated(update) => {
            let settings = &update.thread_settings;
            if let Some(thread) = model.threads.iter_mut().find(|t| t.id == update.thread_id) {
                thread.cwd.clone_from(&settings.cwd);
                thread.model = Some(settings.model.clone());
                thread.model_provider.clone_from(&settings.model_provider);
            }
            if let Some(overlay) = overlay(model, &update.thread_id) {
                overlay.settings = Some(settings.clone());
            }
        }
        ServerNotification::ThreadArchived(gone) => remove(model, &gone.thread_id),
        ServerNotification::ThreadDeleted(gone) => remove(model, &gone.thread_id),
        ServerNotification::ThreadClosed(closed) => {
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
            if let Some(phase) = model.reading.get_mut(reverted.thread_id.as_str())
                && *phase == DetailRead::Turns
            {
                *phase = DetailRead::StaleTurns;
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
        .contains_key(thread)
        .then(|| model.overlay.entry(thread.to_owned()).or_default())
}

fn remove(model: &mut Model, id: &str) {
    model.descendant_archives.remove(id);
    model.remove_subtree(id);
    model.usage.remove(id);
}
