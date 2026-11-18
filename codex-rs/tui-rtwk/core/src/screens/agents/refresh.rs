//! Refreshing the list and the answers of our RPCs: pages merge into the
//! model, actions complete; `Out` names an RPC to send.
//! Oracle: `agents_overview.rs` (refresh apply, open),
//! `agents_overview_actions.rs` (action results), `agents_overview_loading.rs`.

use codex_app_server_protocol::SessionSource;
use codex_app_server_protocol::Thread;
use codex_protocol::protocol::SubAgentSource;
use crate::rpc::RpcError;
use super::Command;
use super::Ctx;
use super::Ev;
use super::Model;
use super::Notice;
use super::Page;
use super::Reply;


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
        && let SessionSource::SubAgent(
            SubAgentSource::ThreadSpawn {
                parent_thread_id, ..
            },
        ) = &thread.source
    {
        thread.parent_thread_id = Some(parent_thread_id.to_string());
    }
    thread
}

fn merge_page(model: &mut Model, page: Page) {
    let known: Vec<String> = page.threads.iter().map(|thread| thread.id.clone()).collect();
    let fresh = page
        .threads
        .into_iter()
        .filter(|thread| !thread.ephemeral)
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
    model.paging.cursor = page.next_cursor.clone();
    model.paging.has_more = page.next_cursor.is_some();
    model.paging.loading = false;
    model.paging.failed = false;
    if matches!(model.notice, Some(Notice::LoadFailed)) {
        model.notice = None;
    }
    model.reconcile();
}

pub(super) fn apply_ev(model: &mut Model, ev: Ev) {
    match ev {
        Ev::Listed(page) => merge_page(model, page),
        Ev::Clock(now) => model.now = now,
        Ev::Home(home) => model.home = home,
        Ev::Current(current) => model.current = current,
        Ev::Pinned(pinned) => {
            model.pinned = pinned;
            model.reconcile();
        }
        Ev::ListFailed => {
            model.paging.loading = false;
            model.paging.failed = true;
        }
        Ev::Notice(notice) => model.notice = notice,
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

pub(super) fn list(model: &mut Model, cursor: Option<String>, append: bool) -> Out {
    model.paging.loading = true;
    out(Command::List { cursor }, Ctx::List { append })
}

/// The answer of an RPC of ours.
pub fn apply_answer(model: &mut Model, ctx: Ctx, result: Result<Reply, RpcError>) -> Vec<Out> {
    match (ctx, result) {
        (Ctx::List { append }, Ok(Reply::Listed(mut page))) => {
            page.append = append;
            merge_page(model, page);
        }
        (Ctx::List { .. }, Err(_) | Ok(_)) => {
            model.paging.loading = false;
            model.paging.failed = true;
            model.notice = Some(Notice::LoadFailed);
        }
        (Ctx::Rename { thread }, Ok(_)) => {
            let _ = thread;
        }
        (Ctx::Action { thread, action }, Ok(_)) => {
            if action.is_some() {
                model.remove_subtree(&thread);
                model.notice = None;
                model.reconcile();
            }
        }
        (Ctx::Action { action, .. }, Err(error)) => {
            let _ = action;
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Stop { thread }, Ok(Reply::ActiveTurn(Some(turn)))) => {
            return vec![out(Command::Interrupt { thread, turn }, Ctx::Interrupt)];
        }
        (Ctx::Stop { .. } | Ctx::Interrupt, Ok(_)) => {}
        (
            Ctx::Rename { .. }
            | Ctx::Pin { .. }
            | Ctx::Resume { .. }
            | Ctx::Stop { .. }
            | Ctx::Interrupt,
            Err(error),
        ) => {
            model.notice = Some(Notice::Error(error_text(error)));
        }
        (Ctx::Pin { thread }, Ok(_)) => {
            if let Some(pins) = &mut model.pinned {
                if pins.contains(&thread) {
                    pins.retain(|pin| *pin != thread);
                } else {
                    pins.insert(0, thread);
                }
            }
        }
        (Ctx::Resume { thread }, Ok(_)) => {
            model.hidden.remove(&thread);
            model.current = Some(thread);
            model.resumes += 1;
        }
        (Ctx::Start, _) => {}
    }
    Vec::new()
}

fn error_text(error: RpcError) -> String {
    match error {
        RpcError::Remote(message) | RpcError::Malformed(message) => message,
        RpcError::Mismatch => "unexpected reply".to_owned(),
    }
}
