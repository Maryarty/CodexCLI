//! The wire form of the screen's RPCs and their replies, and a script's seed.
//! Oracle: `agents_overview_discovery.rs` (list), `agents_overview_actions.rs`.

use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::agents::Command;
use crate::screens::agents::Ev;
use crate::screens::agents::PAGE_SIZE;
use crate::screens::agents::Page;
use crate::screens::agents::Reply;
use crate::screens::agents::UsageOutcome;
use crate::screens::agents::carried::is_overview_thread;
use crate::screens::agents::carried::listed_threads;
use crate::screens::agents::carried::preview_markdown;
use crate::threads::asks::ListQuery;
use crate::threads::{ThreadAsk, ThreadReply};
use codex_app_server_protocol::Account;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GetAccountParams;
use codex_app_server_protocol::GetAccountResponse;
use codex_app_server_protocol::GetAccountTokenUsageParams;
use codex_app_server_protocol::GetAccountTokenUsageResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadSortKey;
use codex_app_server_protocol::ThreadSourceKind;
use codex_app_server_protocol::Turn;

use codex_app_server_protocol::UserInput;
use serde::Deserialize;

/// The wire request of `command` (oracle: `agents_overview_discovery.rs`,
/// `agents_overview_actions.rs`).
pub(crate) fn wire_request(command: &Command, request_id: RequestId) -> ClientRequest {
    match command {
        Command::List { cursor } => ThreadAsk::list(list_query(cursor.clone())).wire(request_id),
        Command::Rename { thread, name } => ThreadAsk::Rename {
            thread: thread.clone(),
            name: name.clone(),
        }
        .wire(request_id),
        Command::Archive { thread } => ThreadAsk::Archive {
            thread: ThreadId::new(thread),
        }
        .wire(request_id),
        Command::Delete { thread } => ThreadAsk::Delete {
            thread: ThreadId::new(thread),
        }
        .wire(request_id),
        Command::Pin { thread, pinned } => ThreadAsk::MoveSection {
            thread: ThreadId::new(thread),
            section: pinned.then(|| PINNED_SECTION.to_owned()),
        }
        .wire(request_id),
        Command::Search { cursor, .. } => ThreadAsk::list(ListQuery {
            limit: Some(SEARCH_PAGE_SIZE),
            ..list_query(cursor.clone())
        })
        .wire(request_id),
        Command::Loaded => ThreadAsk::Loaded.wire(request_id),
        Command::Usage { thread } => ClientRequest::GetAccountTokenUsage {
            request_id,
            params: Some(GetAccountTokenUsageParams {
                thread_id: Some(thread.clone()),
            }),
        },
        // A worktree is git work and a config write is none: `app::send_agents`
        // hands them to the shell, never here.
        Command::SaveGrouping { .. } | Command::NewWorktree { .. } => unreachable_wire(request_id),
        Command::Thread(ask) | Command::Descendants { list: ask, .. } => ask.wire(request_id),
    }
}

/// A config write is no RPC: `app::send_agents` never hands it to the wire.
/// Were it asked, the answer is a harmless read of nothing.
fn unreachable_wire(request_id: RequestId) -> ClientRequest {
    ThreadAsk::Loaded.wire(request_id)
}

/// A search scans pages of this many rows (oracle: `page_size = 100`).
pub(crate) const SEARCH_PAGE_SIZE: u32 = 100;

pub(crate) fn list_query(cursor: Option<String>) -> ListQuery {
    ListQuery {
        current_account_only: true,
        cursor,
        limit: Some(PAGE_SIZE),
        sort_key: Some(ThreadSortKey::RecencyAt),
        sort_direction: None,
        model_providers: Some(Vec::new()),
        source_kinds: Some(Vec::new()),
        archived: Some(false),
        cwd: None,
        use_state_db_only: true,
        ..ListQuery::default()
    }
}

/// `codex_state::PINNED_THREAD_SECTION_ID`, which core does not link.
pub(crate) const PINNED_SECTION: &str = "01984de2-8f74-7c91-a3b2-5c5e937cf318";

/// Keeps the read receipt with its row as the oracle does. Each accepted
/// row moves out of the wire page into the model's page.
pub(crate) fn listed_page(mut page: ThreadListResponse) -> Page {
    let next_cursor = page.next_cursor.take();
    let mut threads = Vec::with_capacity(page.data.len());
    let mut unread = Vec::new();
    for (thread, is_unread) in listed_threads(page) {
        if is_overview_thread(&thread) {
            if is_unread {
                unread.push(thread.id.clone());
            }
            threads.push(thread);
        }
    }
    Page {
        threads,
        unread,
        next_cursor,
        append: false,
        pins: None,
    }
}

pub(crate) fn decode_reply(
    command: &Command,
    result: serde_json::Value,
) -> Result<Reply, RpcError> {
    let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
    match command {
        Command::List { .. } | Command::Search { .. } => {
            crate::threads::asks::listed(result).map(|page| Reply::Listed(listed_page(page)))
        }
        Command::Descendants { list, .. } => match list.decode(result)? {
            ThreadReply::Listed(page) => Ok(Reply::Descendants(page)),
            _ => Err(RpcError::Mismatch),
        },
        Command::Thread(ask) => from_thread(decode_thread(ask, result)?),
        Command::Usage { .. } => {
            let response: GetAccountTokenUsageResponse =
                serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Usage(match response.thread_usage {
                Some(estimate) => UsageOutcome::Available(estimate),
                None => UsageOutcome::Disabled,
            }))
        }
        Command::Loaded => from_thread(ThreadAsk::Loaded.decode(result)?),
        _ => Ok(Reply::Done),
    }
}

/// The center only needs acknowledgment of its interrupt. The lifecycle
/// caller decodes the interrupt response and handles a typed turn-ID race.
pub(crate) fn decode_thread(
    ask: &ThreadAsk,
    result: serde_json::Value,
) -> Result<ThreadReply, RpcError> {
    if matches!(ask, ThreadAsk::Interrupt { .. }) {
        return Ok(ThreadReply::Interrupted);
    }
    ask.decode(result)
}

/// The screen projects a thread result onto its own view state.
pub(crate) fn from_thread(reply: ThreadReply) -> Result<Reply, RpcError> {
    Ok(match reply {
        ThreadReply::Resumed(thread) => Reply::Resumed(thread),
        ThreadReply::Forked(thread) => Reply::Forked(thread),
        ThreadReply::Started(thread) => Reply::Started(thread),
        ThreadReply::Read { thread, unread } => Reply::Read { thread, unread },
        ThreadReply::LatestTurn(turn) => turn_previews(turn.as_deref()),
        ThreadReply::History { turns, next } => Reply::History { turns, next },
        ThreadReply::McpServers(servers) => Reply::McpServers(servers),
        ThreadReply::Temporary { thread, as_asked } => Reply::Temporary {
            thread: thread.to_string(),
            as_asked,
        },
        ThreadReply::StructuredTurn { turn } => Reply::StructuredTurn {
            turn: turn.to_string(),
        },
        ThreadReply::Loaded(threads) => Reply::Loaded(threads),
        ThreadReply::ActiveTurn(turn) => Reply::ActiveTurn(turn),
        ThreadReply::Done | ThreadReply::Interrupted => Reply::Done,
        _ => return Err(RpcError::Mismatch),
    })
}

/// The prompt and the last answer of a turn (oracle: the refresh's `thread/
/// turns/list` branch): the prompt is the text of the turn's user message.
fn turn_previews(turn: Option<&Turn>) -> Reply {
    let Some(turn) = turn else {
        return Reply::Turns {
            prompt: None,
            last_message: None,
        };
    };
    let prompt = turn.item_payloads().find_map(|item| match item {
        ThreadItemPayload::UserMessage { content, .. } => Some(preview_markdown(
            &content
                .iter()
                .filter_map(|input| match input {
                    UserInput::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>(),
        )),
        _ => None,
    });
    let last_message = turn.item_payloads().rev().find_map(|item| match item {
        ThreadItemPayload::AgentMessage { text, .. } => Some(preview_markdown(text)),
        _ => None,
    });
    Reply::Turns {
        prompt,
        last_message,
    }
}

/// A script's seed, see `seed` in `mod.rs`.
pub(crate) fn seed_ev(value: serde_json::Value) -> Result<Ev, String> {
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
                pins: None,
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
        "current" => serde_json::from_value::<Option<String>>(body)
            .map(|id| Ev::Current(id.map(crate::ids::ThreadId::new)))
            .map_err(text),
        "pinned" => serde_json::from_value(body).map(Ev::Pinned).map_err(text),
        "voice" => serde_json::from_value(body).map(Ev::Voice).map_err(text),
        "thread_colors" => serde_json::from_value(body)
            .map(Ev::ThreadColors)
            .map_err(text),
        "grouping" => serde_json::from_value::<String>(body)
            .map_err(text)
            .and_then(|name| crate::screens::agents::Grouping::from_config(&name).ok_or(name))
            .map(Ev::Grouping),
        "notice" => seed_notice(body).map(Ev::Notice),
        "server_version" => Ok(Ev::ServerVersion(body.as_str().map(str::to_owned))),
        "reconnecting" => Ok(Ev::Reconnecting),
        "reconnected" => Ok(Ev::Reconnected),
        "request" => serde_json::from_value::<(String, String)>(body)
            .map(|(thread, preview)| Ev::Request(crate::ids::ThreadId::new(thread), Some(preview)))
            .map_err(text),
        other => Err(other.to_owned()),
    }
}

/// A seeded notice: `null` (none), `"reconnecting"`, `"reconnect_failed"`,
/// `"creating_worktree"`, `"load_failed"`, `{"error": text}` or `{"removing": "archive" | "delete"}`.
fn seed_notice(body: serde_json::Value) -> Result<Option<crate::screens::agents::Notice>, String> {
    use crate::screens::agents::Action;
    use crate::screens::agents::Notice;
    match &body {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(name) => match name.as_str() {
            "reconnecting" => Ok(Some(Notice::Reconnecting)),
            "reconnect_failed" => Ok(Some(Notice::ReconnectFailed)),
            "creating_worktree" => Ok(Some(Notice::CreatingWorktree)),
            "load_failed" => Ok(Some(Notice::LoadFailed)),
            other => Err(["notice ", other].concat()),
        },
        serde_json::Value::Object(object) => {
            let (key, value) = object
                .iter()
                .next()
                .ok_or_else(|| "an empty notice".to_owned())?;
            let text = value.as_str().unwrap_or_default().to_owned();
            match key.as_str() {
                "error" => Ok(Some(Notice::Error(text))),
                "removing" => match text.as_str() {
                    "archive" => Ok(Some(Notice::Removing(Action::Archive))),
                    "delete" => Ok(Some(Notice::Removing(Action::Delete))),
                    other => Err(["removing ", other].concat()),
                },
                other => Err(["notice ", other].concat()),
            }
        }
        _ => Err("a notice is null, a name or an object".to_owned()),
    }
}

/// One page of the pinned tasks of one source, in rank order: the pinned
/// section, sorted by its position (oracle: `list_pinned_threads`).
pub(crate) fn pinned_request(
    request_id: RequestId,
    other: bool,
    cursor: Option<String>,
) -> ClientRequest {
    ThreadAsk::list(ListQuery {
        current_account_only: true,
        cursor,
        limit: Some(SEARCH_PAGE_SIZE),
        sort_key: Some(ThreadSortKey::SectionPosition),
        sort_direction: None,
        model_providers: Some(Vec::new()),
        source_kinds: Some(if other {
            vec![ThreadSourceKind::Exec, ThreadSourceKind::AppServer]
        } else {
            Vec::new()
        }),
        archived: Some(false),
        section_id: Some(PINNED_SECTION.to_owned()),
        cwd: None,
        use_state_db_only: true,
        ..ListQuery::default()
    })
    .wire(request_id)
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
    let response: GetAccountResponse =
        serde_json::from_value(result).map_err(|error| RpcError::Malformed(error.to_string()))?;
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
