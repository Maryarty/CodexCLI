//! Reading the tasks' details: the task as the server has it now, the
//! prompt and the last answer of its last turn. A chain per task, `Read` then
//! `Turns`, asked after a list page and after a notification changed the task.
//! Oracle: `agents_overview_threads.rs` (`start_agents_overview_refresh`,
//! `detail_thread_ids`), `agents_overview.rs` (the merge of a refresh).

use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadStatus;

use super::Command;
use super::Ctx;
use super::Model;
use super::Out;
use super::Reply;
use super::refresh::normalize;
use super::refresh::out;
use super::refresh::upsert_thread;
use crate::rpc::RpcError;

/// How many of the most recent tasks of a page are read whatever their state
/// (oracle: `RECENT_DETAIL_LIMIT`).
const RECENT_DETAIL_LIMIT: usize = 10;

/// The tasks of a page whose details are kept fresh: the most recent and
/// every one the server holds loaded (oracle: `detail_thread_ids`).
pub(super) fn recent_ids(threads: &[Thread]) -> Vec<String> {
    let mut threads: Vec<&Thread> = threads.iter().collect();
    threads.sort_by(|left, right| {
        right
            .recency_at
            .unwrap_or(right.updated_at)
            .cmp(&left.recency_at.unwrap_or(left.updated_at))
            .then_with(|| right.id.cmp(&left.id))
    });
    threads
        .into_iter()
        .enumerate()
        .filter(|(index, thread)| {
            *index < RECENT_DETAIL_LIMIT || thread.status != ThreadStatus::NotLoaded
        })
        .map(|(_, thread)| thread.id.clone())
        .collect()
}

/// The chains to start for `ids`: a task already being read is read again
/// when its read ends.
pub(super) fn want(model: &mut Model, ids: impl IntoIterator<Item = String>) -> Vec<Out> {
    let mut outs = Vec::new();
    for thread in ids {
        if model.removed.contains(&thread) {
            continue;
        }
        if model.reading.insert(thread.clone()) {
            outs.push(out(
                Command::Read {
                    thread: thread.clone(),
                },
                Ctx::Read { thread },
            ));
        } else {
            model.reread.insert(thread);
        }
    }
    outs
}

/// The server's list of loaded tasks (oracle: `thread/loaded/list` of the
/// first refresh): each is read.
pub(super) fn on_loaded(model: &mut Model, result: Result<Reply, RpcError>) -> Vec<Out> {
    match result {
        Ok(Reply::Loaded(ids)) => want(model, ids),
        _ => Vec::new(),
    }
}

/// The answer of `thread/read`: the row is the thread as read, preview
/// included (oracle: the merge of a refresh), then its last turn is asked.
pub(super) fn on_read(model: &mut Model, id: String, result: Result<Reply, RpcError>) -> Vec<Out> {
    let Ok(Reply::Read { thread, unread }) = result else {
        return finish(model, &id);
    };
    if model.removed.contains(&id) {
        return finish(model, &id);
    }
    let mut thread = *thread;
    if thread.ephemeral {
        model.threads.retain(|held| held.id != id);
        model.last_messages.remove(&id);
        model.activity.remove(&id);
        model.usage.remove(&id);
        return finish(model, &id);
    }
    thread.turns.clear();
    model.search_matches.retain(|held| held.id != id);
    // What notifications set while this read was in flight is newer.
    if let Some(overlay) = model.overlay.remove(&id) {
        if let Some(name) = overlay.name {
            thread.name = name;
        }
        if let Some(status) = overlay.status {
            thread.status = status;
        }
        if let Some(settings) = overlay.settings {
            thread.cwd = settings.cwd;
            thread.model = Some(settings.model);
            thread.model_provider = settings.model_provider;
        }
    }
    upsert_thread(&mut model.threads, normalize(thread));
    if unread {
        model.unread.insert(id.clone());
    } else {
        model.unread.remove(&id);
    }
    model.turning.insert(id.clone());
    vec![out(
        Command::Turns { thread: id.clone() },
        Ctx::Turns { thread: id },
    )]
}

/// The answer of `thread/turns/list`: the prompt and the last answer.
pub(super) fn on_turns(model: &mut Model, id: String, result: Result<Reply, RpcError>) -> Vec<Out> {
    model.turning.remove(&id);
    // Asked before a revert or a lag: the answer is of the old history; the
    // chain starts again.
    if model.stale.remove(&id) {
        model.reread.insert(id.clone());
        return finish(model, &id);
    }
    if let Ok(Reply::Turns {
        prompt,
        last_message,
    }) = result
        && !model.removed.contains(&id)
    {
        model.activity.entry(id.clone()).or_default().latest_prompt = prompt;
        if let Some(message) = last_message {
            model.last_messages.insert(id.clone(), message);
        }
    }
    finish(model, &id)
}

/// The chain of `id` ended; a change that came meanwhile starts the next.
fn finish(model: &mut Model, id: &str) -> Vec<Out> {
    model.reading.remove(id);
    model.turning.remove(id);
    model.overlay.remove(id);
    if model.reread.remove(id) {
        return want(model, [id.to_owned()]);
    }
    Vec::new()
}
