//! The wire form of the screen's RPCs and their replies, and a script's seed.
//! Oracle: `agents_overview_discovery.rs` (list), `agents_overview_actions.rs`.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadArchiveParams;
use codex_app_server_protocol::ThreadDeleteParams;
use codex_app_server_protocol::ThreadListParams;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadReadResponse;
use codex_app_server_protocol::ThreadResumeParams;
use codex_app_server_protocol::ThreadResumeResponse;
use codex_app_server_protocol::ThreadSectionMoveParams;
use codex_app_server_protocol::ThreadSetNameParams;
use codex_app_server_protocol::ThreadSortKey;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::TurnInterruptParams;
use codex_app_server_protocol::TurnStatus;
use crate::rpc::RpcError;
use serde::Deserialize;
use super::Command;
use super::Ev;
use super::PAGE_SIZE;
use super::Page;
use super::Reply;
use super::carried::is_overview_thread;


/// The wire request of `command` (oracle: `agents_overview_discovery.rs`,
/// `agents_overview_actions.rs`).
pub(super) fn wire_request(command: &Command, request_id: RequestId) -> ClientRequest {
    match command {
        Command::List { cursor } => ClientRequest::ThreadList {
            request_id,
            params: ThreadListParams {
                include_attached_threads: false,
                current_account_only: true,
                cursor: cursor.clone(),
                limit: Some(PAGE_SIZE),
                excluded_thread_ids: None,
                sort_key: Some(ThreadSortKey::RecencyAt),
                sort_direction: None,
                model_providers: Some(Vec::new()),
                source_kinds: Some(Vec::new()),
                originators: None,
                archived: Some(false),
                section_id: None,
                project_id: None,
                cwd: None,
                use_state_db_only: true,
                search_term: None,
                parent_thread_id: None,
                ancestor_thread_id: None,
                attached_to_thread_id: None,
            },
        },
        Command::Rename { thread, name } => ClientRequest::ThreadSetName {
            request_id,
            params: ThreadSetNameParams {
                thread_id: thread.clone(),
                name: name.clone(),
            },
        },
        Command::Archive { thread } => ClientRequest::ThreadArchive {
            request_id,
            params: ThreadArchiveParams {
                thread_id: thread.clone(),
            },
        },
        Command::Delete { thread } => ClientRequest::ThreadDelete {
            request_id,
            params: ThreadDeleteParams {
                thread_id: thread.clone(),
            },
        },
        Command::ActiveTurn { thread } => ClientRequest::ThreadRead {
            request_id,
            params: ThreadReadParams {
                thread_id: thread.clone(),
                include_turns: true,
            },
        },
        Command::Interrupt { thread, turn } => ClientRequest::TurnInterrupt {
            request_id,
            params: TurnInterruptParams {
                thread_id: thread.clone(),
                turn_id: turn.clone(),
            },
        },
        Command::Pin { thread, pinned } => ClientRequest::ThreadSectionMove {
            request_id,
            params: ThreadSectionMoveParams {
                thread_id: thread.clone(),
                section_id: pinned.then(|| PINNED_SECTION.to_owned()),
                before_thread_id: None,
            },
        },
        Command::Resume { thread } => ClientRequest::ThreadResume {
            request_id,
            params: ThreadResumeParams {
                thread_id: thread.clone(),
                ..ThreadResumeParams::default()
            },
        },
        Command::Start { cwd } => ClientRequest::ThreadStart {
            request_id,
            params: ThreadStartParams {
                cwd: cwd.clone(),
                ..ThreadStartParams::default()
            },
        },
    }
}

/// `codex_state::PINNED_THREAD_SECTION_ID`, which core does not link.
const PINNED_SECTION: &str = "01984de2-8f74-7c91-a3b2-5c5e937cf318";

pub(super) fn decode_reply(
    command: &Command,
    result: serde_json::Value,
) -> Result<Reply, RpcError> {
    let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
    match command {
        Command::List { .. } => {
            let page: ThreadListResponse = serde_json::from_value(result).map_err(malformed)?;
            let unread = page
                .read_states
                .iter()
                .flatten()
                .filter(|(_, state)| state.first_unread.is_some())
                .map(|(id, _)| id.clone())
                .collect();
            Ok(Reply::Listed(Page {
                threads: page
                    .data
                    .into_iter()
                    .filter(is_overview_thread)
                    .collect(),
                unread,
                next_cursor: page.next_cursor,
                append: false,
            }))
        }
        Command::Resume { .. } => {
            let response: ThreadResumeResponse = serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Resumed(Box::new(response.thread)))
        }
        Command::ActiveTurn { .. } => {
            let response: ThreadReadResponse =
                serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::ActiveTurn(
                response
                    .thread
                    .turns
                    .into_iter()
                    .find(|turn| turn.status == TurnStatus::InProgress)
                    .map(|turn| turn.id),
            ))
        }
        _ => Ok(Reply::Done),
    }
}

/// A script's seed, see `seed` in `mod.rs`.
pub(super) fn seed_ev(value: serde_json::Value) -> Result<Ev, String> {
    let (key, body) = value
        .as_object()
        .and_then(|object| object.iter().next())
        .map(|(key, body)| (key.clone(), body.clone()))
        .ok_or_else(|| "an agents event is an object with one key".to_owned())?;
    let text = |error: serde_json::Error| error.to_string();
    match key.as_str() {
        "listed" => {
            #[derive(Deserialize)]
            struct Seed {
                threads: Vec<Thread>,
                #[serde(default)]
                unread: Vec<String>,
                #[serde(default)]
                next_cursor: Option<String>,
            }
            let seed: Seed = serde_json::from_value(body).map_err(text)?;
            Ok(Ev::Listed(Page {
                threads: seed.threads,
                unread: seed.unread,
                next_cursor: seed.next_cursor,
                append: false,
            }))
        }
        "home" => serde_json::from_value(body).map(Ev::Home).map_err(text),
        "clock" => serde_json::from_value(body).map(Ev::Clock).map_err(text),
        "current" => serde_json::from_value(body).map(Ev::Current).map_err(text),
        "pinned" => serde_json::from_value(body).map(Ev::Pinned).map_err(text),
        "request" => serde_json::from_value::<(String, String)>(body)
            .map(|(thread, preview)| Ev::Request(thread, Some(preview)))
            .map_err(text),
        other => Err(other.to_owned()),
    }
}
