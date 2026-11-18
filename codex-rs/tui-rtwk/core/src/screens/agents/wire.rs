//! The wire form of the screen's RPCs and their replies, and a script's seed.
//! Oracle: `agents_overview_discovery.rs` (list), `agents_overview_actions.rs`.

use super::Command;
use super::Ev;
use super::PAGE_SIZE;
use super::Page;
use super::Reply;
use super::UsageOutcome;
use super::carried::is_overview_thread;
use crate::rpc::RpcError;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GetAccountTokenUsageParams;
use codex_app_server_protocol::GetAccountTokenUsageResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadArchiveParams;
use codex_app_server_protocol::ThreadDeleteParams;
use codex_app_server_protocol::ThreadForkParams;
use codex_app_server_protocol::ThreadForkResponse;
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
use serde::Deserialize;

/// The wire request of `command` (oracle: `agents_overview_discovery.rs`,
/// `agents_overview_actions.rs`).
pub(super) fn wire_request(command: &Command, request_id: RequestId) -> ClientRequest {
    match command {
        Command::List { cursor } => ClientRequest::ThreadList {
            request_id,
            params: list_params(cursor.clone()),
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
        Command::Fork { thread } => ClientRequest::ThreadFork {
            request_id,
            params: ThreadForkParams {
                thread_id: thread.clone(),
                ..ThreadForkParams::default()
            },
        },
        Command::Search { cursor, .. } => ClientRequest::ThreadList {
            request_id,
            params: ThreadListParams {
                limit: Some(SEARCH_PAGE_SIZE),
                ..list_params(cursor.clone())
            },
        },
        Command::Usage { thread } => ClientRequest::GetAccountTokenUsage {
            request_id,
            params: Some(GetAccountTokenUsageParams {
                thread_id: Some(thread.clone()),
            }),
        },
        // A worktree is git work, not an RPC: `app::send_agents` hands it to
        // the shell, and never here. Were it asked, the task would start in
        // the directory itself.
        Command::Start { cwd } | Command::NewWorktree { cwd } => ClientRequest::ThreadStart {
            request_id,
            params: ThreadStartParams {
                cwd: cwd.clone(),
                ..ThreadStartParams::default()
            },
        },
    }
}

/// A search scans pages of this many rows (oracle: `page_size = 100`).
const SEARCH_PAGE_SIZE: u32 = 100;

fn list_params(cursor: Option<String>) -> ThreadListParams {
    ThreadListParams {
        include_attached_threads: false,
        current_account_only: true,
        cursor,
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
        Command::List { .. } | Command::Search { .. } => {
            let page: ThreadListResponse = serde_json::from_value(result).map_err(malformed)?;
            let unread = page
                .read_states
                .iter()
                .flatten()
                .filter(|(_, state)| state.first_unread.is_some())
                .map(|(id, _)| id.clone())
                .collect();
            Ok(Reply::Listed(Page {
                threads: page.data.into_iter().filter(is_overview_thread).collect(),
                unread,
                next_cursor: page.next_cursor,
                append: false,
            }))
        }
        Command::Resume { .. } => {
            let response: ThreadResumeResponse =
                serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Resumed(Box::new(response.thread)))
        }
        Command::Fork { .. } => {
            let response: ThreadForkResponse = serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Forked(Box::new(response.thread)))
        }
        Command::Usage { .. } => {
            let response: GetAccountTokenUsageResponse =
                serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Usage(match response.thread_usage {
                Some(estimate) => UsageOutcome::Available(estimate),
                None => UsageOutcome::Disabled,
            }))
        }
        Command::Start { .. } => {
            // Only the task is read: the rest of the response is the
            // server's settings, which the center does not use.
            let thread = result
                .get("thread")
                .cloned()
                .ok_or_else(|| RpcError::Malformed("thread/start: no thread".to_owned()))?;
            let thread: Thread = serde_json::from_value(thread).map_err(malformed)?;
            Ok(Reply::Started(Box::new(thread)))
        }
        Command::ActiveTurn { .. } => {
            let response: ThreadReadResponse = serde_json::from_value(result).map_err(malformed)?;
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
        "backend" => {
            #[derive(Deserialize)]
            struct Seed {
                #[serde(default)]
                cloud: bool,
                #[serde(default)]
                worktrees: bool,
            }
            let seed: Seed = serde_json::from_value(body).map_err(text)?;
            Ok(Ev::Backend {
                cloud: seed.cloud,
                worktrees: seed.worktrees,
            })
        }
        "clock" => serde_json::from_value(body).map(Ev::Clock).map_err(text),
        "current" => serde_json::from_value(body).map(Ev::Current).map_err(text),
        "pinned" => serde_json::from_value(body).map(Ev::Pinned).map_err(text),
        "voice" => serde_json::from_value(body).map(Ev::Voice).map_err(text),
        "notice" => seed_notice(body).map(Ev::Notice),
        "request" => serde_json::from_value::<(String, String)>(body)
            .map(|(thread, preview)| Ev::Request(thread, Some(preview)))
            .map_err(text),
        other => Err(other.to_owned()),
    }
}

/// A seeded notice: `null` (none), `"reconnecting"`, `"reconnect_failed"`,
/// `"creating_worktree"`, `"load_failed"`, `{"server_version": text}`,
/// `{"error": text}` or `{"removing": "archive" | "delete"}`.
fn seed_notice(body: serde_json::Value) -> Result<Option<super::Notice>, String> {
    use super::Action;
    use super::Notice;
    match &body {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(name) => match name.as_str() {
            "reconnecting" => Ok(Some(Notice::Reconnecting)),
            "reconnect_failed" => Ok(Some(Notice::ReconnectFailed)),
            "creating_worktree" => Ok(Some(Notice::CreatingWorktree)),
            "load_failed" => Ok(Some(Notice::LoadFailed)),
            other => Err(format!("notice {other}")),
        },
        serde_json::Value::Object(object) => {
            let (key, value) = object
                .iter()
                .next()
                .ok_or_else(|| "an empty notice".to_owned())?;
            let text = value.as_str().unwrap_or_default().to_owned();
            match key.as_str() {
                "server_version" => Ok(Some(Notice::ServerVersion(text))),
                "error" => Ok(Some(Notice::Error(text))),
                "removing" => match text.as_str() {
                    "archive" => Ok(Some(Notice::Removing(Action::Archive))),
                    "delete" => Ok(Some(Notice::Removing(Action::Delete))),
                    other => Err(format!("removing {other}")),
                },
                other => Err(format!("notice {other}")),
            }
        }
        _ => Err("a notice is null, a name or an object".to_owned()),
    }
}
