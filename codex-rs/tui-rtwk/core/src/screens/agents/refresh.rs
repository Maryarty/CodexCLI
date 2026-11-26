//! Refreshing the list and the answers of our RPCs: pages merge into the
//! model, actions complete; `Out` names an RPC to send.
//! Oracle: `agents_overview.rs` (refresh apply, open),
//! `agents_overview_actions.rs` (action results), `agents_overview_loading.rs`.

use super::DetailRead;
use crate::ids::ThreadId;
use crate::threads::{ThreadAsk, ThreadPreviews};
use std::collections::HashMap;
use std::collections::HashSet;

use super::Action;
use super::Command;
use super::Ctx;
use super::Ev;
use super::Model;
use super::Notice;
use super::Page;
use super::Reply;
use crate::ids::BackendId;
use crate::rpc::RpcError;
use codex_app_server_protocol::SessionSource;
use codex_app_server_protocol::Thread;
use codex_protocol::protocol::SubAgentSource;

/// An RPC to send and what it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Out {
    pub command: Command,
    pub ctx: Ctx,
}

pub(super) fn out(command: Command, ctx: Ctx) -> Out {
    Out { command, ctx }
}

/// Oracle: `agents_overview_view`'s parent link: a spawned subagent hangs
/// under the thread that spawned it.
pub(super) fn normalize(mut thread: Thread) -> Thread {
    if thread.parent_thread_id.is_none()
        && let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) = &thread.source
    {
        thread.parent_thread_id = Some(parent_thread_id.to_string());
    }
    thread
}

/// The position of each task by id.
fn positions(threads: &[Thread]) -> HashMap<String, usize> {
    threads
        .iter()
        .enumerate()
        .map(|(at, thread)| (thread.id.clone(), at))
        .collect()
}

/// Replaces the held task of that id or adds it at the end.
fn upsert(threads: &mut Vec<Thread>, at: &mut HashMap<String, usize>, thread: Thread) {
    match at.get(&thread.id) {
        Some(&held) => threads[held] = thread,
        None => {
            at.insert(thread.id.clone(), threads.len());
            threads.push(thread);
        }
    }
}

/// Replaces the held task of that id or adds it at the end.
pub(super) fn upsert_thread(threads: &mut Vec<Thread>, thread: Thread) {
    match threads.iter().position(|held| held.id == thread.id) {
        Some(held) => threads[held] = thread,
        None => threads.push(thread),
    }
}

fn merge_page(model: &mut Model, owner: BackendId, page: Page) {
    // What the list reaches: the rows this backend owns (a row of an id
    // another backend owns is not this list's, and the held one stays).
    let mut known = HashSet::new();
    let mut fresh = Vec::new();
    for thread in page.threads {
        let ours = model.observe(owner, &thread.id);
        if ours {
            known.insert(thread.id.clone());
        }
        if ours && !thread.ephemeral && !model.removed.contains(&thread.id) {
            fresh.push(normalize(thread));
        }
    }
    if page.append {
        let mut at = positions(&model.threads);
        for thread in fresh {
            upsert(&mut model.threads, &mut at, thread);
        }
    } else {
        // A refresh keeps what the list does not reach (children, rows past
        // the first page) and replaces what it does.
        let mut merged = fresh;
        merged.extend(
            model
                .threads
                .drain(..)
                .filter(|held| !known.contains(&held.id)),
        );
        model.threads = merged;
    }
    for id in page.unread {
        model.unread.insert(id);
    }
    if let Some(pins) = page.pins {
        // A pinned task need not be among the recent ones: it is a row too.
        let mut at = positions(&model.threads);
        for thread in pins.threads {
            if thread.ephemeral
                || model.removed.contains(&thread.id)
                || !model.observe(owner, &thread.id)
            {
                continue;
            }
            upsert(&mut model.threads, &mut at, normalize(thread));
        }
        // Only the ids this backend owns, once each (oracle: the pins are
        // filtered through `BackendRegistry::observe`).
        let ids = pins.ids.map(|ids| {
            let mut seen = HashSet::new();
            ids.into_iter()
                .filter(|id| model.observe(owner, id) && seen.insert(id.clone()))
                .collect()
        });
        model.set_pins(owner, ids);
    }
    let arrived: HashSet<&str> = model.threads.iter().map(|held| held.id.as_str()).collect();
    model
        .search_matches
        .retain(|held| !arrived.contains(held.id.as_str()));
    // Another backend's page sets where its own list continues (oracle:
    // `discovery` and `initialized` are per backend); the displayed list's
    // window stays as it is.
    if owner != model.displayed {
        let parked = model.parked.entry(owner).or_default();
        if !parked.initialized || page.append {
            parked.cursor = page.next_cursor;
            parked.buffer.clear();
            parked.initialized = true;
        }
        parked.has_more = parked.more();
        return;
    }
    // The first page sets where the list continues; a refresh after the
    // window grew leaves it (oracle: a refresh without discovery).
    if !model.paging.initialized || page.append {
        model.paging.cursor = page.next_cursor;
        model.paging.buffer.clear();
        model.paging.initialized = true;
    }
    super::discovery::sync(model);
    model.paging.loading = false;
    model.paging.failed = false;
    if matches!(model.notice, Some(Notice::LoadFailed)) {
        model.notice = None;
    }
}

pub(super) fn apply_ev(model: &mut Model, ev: Ev) {
    match ev {
        Ev::Listed(page) => merge_page(model, model.displayed, page),
        Ev::ListedBy(owner, page) => merge_page(model, owner, page),
        Ev::Home(home) => model.home = home,
        Ev::Displayed(backend) => model.display(backend),
        Ev::CloudBackend(backend) => {
            model.clouds.insert(backend);
            model.cloud = model.clouds.contains(&model.displayed);
        }
        Ev::Backend { cloud, worktrees } => {
            model.cloud = cloud;
            if cloud {
                model.clouds.insert(model.displayed);
            } else {
                model.clouds.remove(&model.displayed);
            }
            model.worktrees = worktrees;
        }
        Ev::Directories(keys) => model.dirs.extend(keys),
        Ev::Account { plan, backend_auth } => {
            model.plan = plan;
            model.backend_auth = backend_auth;
            super::usage::reset(model);
        }
        Ev::ThreadColors(on) => model.thread_colors_off = !on,
        Ev::Grouping(grouping) => model.grouping = grouping,
        Ev::Current(_) | Ev::Request(..) => {}
        Ev::Pinned(pinned) => model.set_pins(model.displayed, pinned),
        Ev::ListFailed => {
            model.paging.loading = false;
            model.paging.failed = true;
        }
        Ev::Notice(notice) => {
            if matches!(notice, Some(Notice::Reconnecting)) {
                super::usage::reset(model);
            }
            model.notice = notice;
        }
        Ev::ServerVersion(text) => model.server_version = text,
        Ev::Reconnecting => reconnecting(model),
        Ev::Reconnected => model.notice = None,
        Ev::Lagged => lagged(model),
        Ev::Voice(owner) => model.voice = owner,
    }
}

/// The connection was lost (oracle: `begin_reconnect`): what was read from
/// the old one is dropped, the reads in flight will never answer, and the
/// footer says so until a new connection is up.
fn reconnecting(model: &mut Model) {
    super::usage::reset(model);
    model.reading.clear();
    model.reread.clear();
    model.pin_pending = false;
    model.paging.loading = false;
    model.paging.initialized = false;
    // The older-server notice is judged again by the new connection.
    model.server_version = None;
    model.notice = Some(Notice::Reconnecting);
}

/// Events were dropped (oracle: `AppServerEvent::Lagged`): what was derived
/// from the stream is dropped, and what is being read is read again.
fn lagged(model: &mut Model) {
    super::usage::reset(model);
    for phase in model.reading.values_mut() {
        if *phase == DetailRead::Turns {
            *phase = DetailRead::StaleTurns;
        }
    }
    model.paging.initialized = false;
}

/// A task the app started opens in the chat; the busy notice ends.
pub(crate) fn finish_start(model: &mut Model, thread: &str) {
    model.hidden.remove(thread);
    model.resumes += 1;
    if matches!(model.notice, Some(Notice::CreatingWorktree)) {
        model.notice = None;
    }
}

pub(super) fn list(model: &mut Model, cursor: Option<String>, append: bool) -> Out {
    model.paging.loading = true;
    out(Command::List { cursor }, Ctx::List { append })
}

/// The answer of an RPC of ours.
pub fn apply_answer(
    model: &mut Model,
    previews: &mut ThreadPreviews,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
) -> Vec<Out> {
    match (ctx, result) {
        (Ctx::List { append: true }, Ok(Reply::Listed(page))) => {
            super::discovery::on_more_page(model, Some(page));
            return super::discovery::refill(model);
        }
        (Ctx::List { append: true }, Err(_) | Ok(_)) => {
            super::discovery::on_more_page(model, None);
        }
        (Ctx::List { append: false }, Ok(Reply::Listed(mut page))) => {
            page.append = false;
            let first = !model.paging.initialized;
            let ids = super::reads::recent_ids(&page.threads);
            merge_page(model, model.displayed, page);
            let mut outs = super::discovery::refill(model);
            outs.extend(super::reads::want(model, ids));
            if first {
                outs.push(out(Command::Loaded, Ctx::Loaded));
            }
            return outs;
        }
        (Ctx::List { .. }, Err(_) | Ok(_)) => {
            model.paging.loading = false;
            model.paging.failed = true;
            model.notice = Some(Notice::LoadFailed);
        }
        (Ctx::Search { id, limit }, Ok(Reply::Listed(page))) => {
            return super::discovery::on_search_page(model, id, limit, Some(page));
        }
        (Ctx::Search { id, limit }, Err(_) | Ok(_)) => {
            return super::discovery::on_search_page(model, id, limit, None);
        }
        (Ctx::Read { thread }, result) => {
            return super::reads::on_read(model, previews, thread, result);
        }
        (Ctx::Turns { thread }, result) => {
            return super::reads::on_turns(model, previews, thread, result);
        }
        (Ctx::Loaded, result) => return super::reads::on_loaded(model, result),
        (Ctx::Usage { thread, id }, result) => {
            super::usage::finish(model, &thread, id, result.ok());
        }
        (Ctx::SaveGrouping, _) => {}
        (Ctx::NewWorktree, _) => {
            if matches!(model.notice, Some(Notice::CreatingWorktree)) {
                model.notice = None;
            }
        }
        (Ctx::Fork { .. }, Ok(Reply::Forked(_))) => {
            // The copy opens in the chat.
            model.resumes += 1;
        }
        (Ctx::Fork { .. }, Ok(_)) => {}
        (Ctx::Fork { .. }, Err(error)) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Rename { thread }, Ok(_)) => {
            let _ = thread;
        }
        (Ctx::Action { thread, action }, Ok(_)) => {
            if let Some(action) = action {
                // A cloud archive removes only the task (oracle:
                // `removes_subtree`).
                let whole = !(model.is_cloud(model.owner_of(&thread)) && action == Action::Archive);
                model.remove_tree(&thread, whole);
                model.hidden.remove(&thread);
                model.notice = None;
                return super::discovery::refill(model);
            }
        }
        (Ctx::Action { thread, action }, Err(error)) => {
            // The task was hidden while the request ran: it is back.
            model.hidden.remove(&thread);
            match action {
                Some(action) => {
                    model.notice = None;
                    model.failure = Some(super::Failure {
                        action,
                        error: error_text(error),
                        attempted: true,
                    });
                }
                None => model.notice = Some(Notice::Error(error_text(error))),
            }
        }
        (Ctx::Stop { thread }, Ok(Reply::ActiveTurn(Some(turn)))) => {
            return vec![out(
                Command::Thread(ThreadAsk::Interrupt { thread, turn }),
                Ctx::Interrupt,
            )];
        }
        (Ctx::Stop { .. } | Ctx::Interrupt, Ok(_)) => {}
        (Ctx::Pin { pinned, .. }, Err(error)) => {
            model.pin_pending = false;
            // Oracle: `complete_agents_overview_pin`, then the list is read
            // again whatever the answer was.
            model.notice = Some(Notice::Error(format!(
                "Failed to {} task: {}",
                if pinned { "pin" } else { "unpin" },
                error_text(error)
            )));
            return vec![list(model, None, false)];
        }
        // The thread has another writer (oracle: `is_active_writer_error`):
        // it opens read-only, its history shown, not as a failure.
        (Ctx::Resume { thread }, Err(error)) if is_active_writer(&error) => {
            let owner = model.owner_of(thread.as_str());
            model.display(owner);
            model.hidden.remove(thread.as_str());
            model.opening = None;
            model.resumes += 1;
            return vec![out(
                Command::Thread(ThreadAsk::History {
                    thread: thread.to_string(),
                    cursor: None,
                }),
                Ctx::History {
                    thread: thread.to_string(),
                    latest: true,
                },
            )];
        }
        (Ctx::Resume { .. }, Err(error)) => {
            // The page opened over the center goes back to it with the error.
            model.opening = None;
            // Oracle: `agents_overview.rs`, `Failed to attach to task: {error}`.
            model.notice = Some(Notice::Error(format!(
                "Failed to attach to task: {}",
                error_text(error)
            )));
        }
        (Ctx::Rename { .. } | Ctx::Stop { .. } | Ctx::Interrupt, Err(error)) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Pin { thread, pinned }, Ok(_)) => {
            model.pin_pending = false;
            // Sets the requested state (oracle `complete_agents_overview_pin`):
            // idempotent, and a new pin goes last.
            let owner = model.owner_of(thread.as_str());
            if let Some(Some(pins)) = model.pins_by_backend.get_mut(&owner) {
                pins.retain(|pin| pin.as_str() != thread);
                if pinned {
                    pins.push(ThreadId::new(thread));
                }
            }
            return vec![list(model, None, false)];
        }
        (Ctx::Resume { thread }, result @ Ok(_)) => {
            // Opening older history is a local addition: the row stays, even
            // when no list reaches it.
            if let Ok(Reply::Resumed(resumed)) = result {
                let owner = model.owner_of(thread.as_str());
                if !resumed.ephemeral
                    && !model.removed.contains(thread.as_str())
                    && model.observe(owner, thread.as_str())
                {
                    let mut resumed = *resumed;
                    resumed.turns.clear();
                    upsert_thread(&mut model.threads, normalize(resumed));
                }
            }
            // A task of another backend resumes on its owner, which is the
            // displayed backend from then on (oracle: `ResumeBackendSession`).
            let owner = model.owner_of(thread.as_str());
            model.display(owner);
            model.hidden.remove(thread.as_str());
            model.opening = None;
            model.resumes += 1;
            // The session's history, the newest page first.
            return vec![out(
                Command::Thread(ThreadAsk::History {
                    thread: thread.to_string(),
                    cursor: None,
                }),
                Ctx::History {
                    thread: thread.to_string(),
                    latest: true,
                },
            )];
        }
        // The page of history goes to the thread's transcript (the app
        // routes it); the center has nothing to change.
        (Ctx::History { .. }, _) => {}
        (Ctx::Start, Ok(Reply::Started(thread))) => finish_start(model, &thread.id),
        (Ctx::Start, Ok(_)) => {}
        (Ctx::Start, Err(error)) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
    }
    Vec::new()
}

/// A task this process already holds is shown again without a new resume
/// (ADR-003: one `ThreadModel` per thread, kept while another is shown, and
/// the server has one writer per thread): the opening is over, the center
/// pops as it does on a resume's answer.
pub fn attach_loaded(model: &mut Model, thread: &str) {
    let owner = model.owner_of(thread);
    model.display(owner);
    model.hidden.remove(thread);
    model.opening = None;
    model.resumes += 1;
}

/// The server refused the resume because another process writes the thread
/// (oracle: `app_server_session::is_active_writer_error`, by the message), or
/// because the thread's model provider is refused (`model_provider_error`): the
/// thread opens read-only, its history shown.
pub fn is_active_writer(error: &RpcError) -> bool {
    matches!(error, RpcError::Provider(_))
        || matches!(error, RpcError::Remote(message) if message.contains("already has an active writer"))
}

fn error_text(error: RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) => message,
        RpcError::Mismatch => "unexpected reply".to_owned(),
        RpcError::Prepare(text) => text,
        RpcError::Provider(error) => error.to_string(),
    }
}
