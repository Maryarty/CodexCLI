//! Refreshing the list and the answers of our RPCs: pages merge into the
//! model, actions complete; `Out` names an RPC to send.
//! Oracle: `agents_overview.rs` (refresh apply, open),
//! `agents_overview_actions.rs` (action results), `agents_overview_loading.rs`.

use super::Action;
use super::Command;
use super::Ctx;
use super::Ev;
use super::Model;
use super::Notice;
use super::Page;
use super::Reply;
use crate::rpc::RpcError;
use codex_app_server_protocol::Account;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GetAccountParams;
use codex_app_server_protocol::GetAccountResponse;
use codex_app_server_protocol::RequestId;
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

fn merge_page(model: &mut Model, page: Page) {
    let known: Vec<String> = page
        .threads
        .iter()
        .map(|thread| thread.id.clone())
        .collect();
    let fresh = page
        .threads
        .into_iter()
        .filter(|thread| !thread.ephemeral && !model.removed.contains(&thread.id))
        .map(normalize)
        .collect::<Vec<_>>();
    if page.append {
        for thread in fresh {
            match model.threads.iter_mut().find(|held| held.id == thread.id) {
                Some(held) => *held = thread,
                None => model.threads.push(thread),
            }
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
        for thread in pins.threads {
            if thread.ephemeral || model.removed.contains(&thread.id) {
                continue;
            }
            let thread = normalize(thread);
            match model.threads.iter_mut().find(|held| held.id == thread.id) {
                Some(held) => *held = thread,
                None => model.threads.push(thread),
            }
        }
        model.pinned = pins.ids;
    }
    let arrived: Vec<String> = model.threads.iter().map(|held| held.id.clone()).collect();
    model
        .search_matches
        .retain(|held| !arrived.contains(&held.id));
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
        Ev::Listed(page) => merge_page(model, page),
        Ev::Clock(now) => model.now = now,
        Ev::Home(home) => model.home = home,
        Ev::Backend { cloud, worktrees } => {
            model.cloud = cloud;
            model.worktrees = worktrees;
        }
        Ev::Directories(keys) => model.dirs.extend(keys),
        Ev::Account { plan, backend_auth } => {
            model.plan = plan;
            model.backend_auth = backend_auth;
            super::usage::reset(model);
        }
        Ev::ThreadColors(on) => model.thread_colors_off = !on,
        Ev::Current(current) => model.current = current,
        Ev::Pinned(pinned) => {
            model.pinned = pinned;
        }
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
        Ev::Started(thread) => opened(model, &thread),
        Ev::Voice(owner) => model.voice = owner,
        Ev::Request(thread, preview) => match preview {
            Some(preview) => {
                model.requests.insert(thread, preview);
            }
            None => {
                model.requests.remove(&thread);
            }
        },
    }
}

/// A task the app started opens in the chat; the busy notice ends.
fn opened(model: &mut Model, thread: &Thread) {
    model.hidden.remove(&thread.id);
    model.current = Some(thread.id.clone());
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
pub fn apply_answer(model: &mut Model, ctx: Ctx, result: Result<Reply, RpcError>) -> Vec<Out> {
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
            merge_page(model, page);
            return super::discovery::refill(model);
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
        (Ctx::Usage { thread, id }, result) => {
            super::usage::finish(model, &thread, id, result.ok());
        }
        (Ctx::NewWorktree, _) => {
            if matches!(model.notice, Some(Notice::CreatingWorktree)) {
                model.notice = None;
            }
        }
        (Ctx::Fork, Ok(Reply::Forked(thread))) => {
            // The copy opens in the chat.
            model.current = Some(thread.id);
            model.resumes += 1;
        }
        (Ctx::Fork, Ok(_)) => {}
        (Ctx::Fork, Err(error)) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Rename { thread }, Ok(_)) => {
            let _ = thread;
        }
        (Ctx::Action { thread, action }, Ok(_)) => {
            if let Some(action) = action {
                // A cloud archive removes only the task (oracle:
                // `removes_subtree`).
                let whole = !(model.cloud && action == Action::Archive);
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
            return vec![out(Command::Interrupt { thread, turn }, Ctx::Interrupt)];
        }
        (Ctx::Stop { .. } | Ctx::Interrupt, Ok(_)) => {}
        (Ctx::Pin { .. }, Err(error)) => {
            model.pin_pending = false;
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (
            Ctx::Rename { .. } | Ctx::Resume { .. } | Ctx::Stop { .. } | Ctx::Interrupt,
            Err(error),
        ) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Pin { thread, pinned }, Ok(_)) => {
            model.pin_pending = false;
            // Sets the requested state (oracle `complete_agents_overview_pin`):
            // idempotent, and a new pin goes last.
            if let Some(pins) = &mut model.pinned {
                pins.retain(|pin| *pin != thread);
                if pinned {
                    pins.push(thread);
                }
            }
        }
        (Ctx::Resume { thread }, Ok(_)) => {
            model.hidden.remove(&thread);
            model.current = Some(thread);
            model.resumes += 1;
        }
        (Ctx::Start, Ok(Reply::Started(thread))) => opened(model, &thread),
        (Ctx::Start, Ok(_)) => {}
        (Ctx::Start, Err(error)) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
    }
    Vec::new()
}

fn error_text(error: RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) => message,
        RpcError::Mismatch => "unexpected reply".to_owned(),
    }
}

/// The bootstrap's `account/read` (oracle: `read_account`, which never
/// refreshes the token).
pub fn account_request(request_id: RequestId) -> ClientRequest {
    ClientRequest::GetAccount {
        request_id,
        params: GetAccountParams {
            refresh_token: false,
        },
    }
}

/// The account's plan and Codex backend auth out of the reply of
/// `account_request` (oracle: `SessionAccount::from`: a ChatGPT account has
/// a plan and the Codex backend auth; an API key or none has neither).
pub fn account_ev(result: serde_json::Value) -> Result<Ev, RpcError> {
    let response: GetAccountResponse = serde_json::from_value(result)
        .map_err(|error| RpcError::Malformed(error.to_string()))?;
    Ok(match response.account {
        Some(Account::Chatgpt { plan_type, .. }) => Ev::Account {
            plan: Some(plan_type),
            backend_auth: true,
        },
        _ => Ev::Account {
            plan: None,
            backend_auth: false,
        },
    })
}
