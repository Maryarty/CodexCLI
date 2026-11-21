//! The picker's RPCs on the wire and back: `thread/list` pages, the preview
//! and transcript reads, archive and unarchive. Core knows the protocol (the
//! shell sends what `wire` makes); no view file does.
//! Oracle: `resume_picker.rs` (`thread_list_params`, `row_from_app_server_thread`,
//! `load_app_server_page`) and `resume_picker_transcript_preview.rs`
//! (`append_transcript_preview_lines`).

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadArchiveParams;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadListCwdFilter;
use codex_app_server_protocol::ThreadListParams;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadReadParams;
use codex_app_server_protocol::ThreadReadResponse;
use codex_app_server_protocol::ThreadSortKey;
use codex_app_server_protocol::ThreadSourceKind;
use codex_app_server_protocol::ThreadUnarchiveParams;
use codex_app_server_protocol::ThreadUnarchiveResponse;
use codex_app_server_protocol::UserInput;

use super::toolbar::Sort;
use super::{Command, CwdFilter, PreviewLine, Reply, Row, Speaker};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::RpcError;

/// A page is this many rows (oracle: `PAGE_SIZE`).
pub const PAGE_SIZE: u32 = 25;
/// The preview keeps this many of the newest lines (oracle:
/// `MAX_TRANSCRIPT_PREVIEW_LINES`).
pub const PREVIEW_LINES: usize = 6;

pub(super) fn wire(command: &Command, id: RequestId) -> ClientRequest {
    match command {
        Command::List {
            cursor,
            mode,
            sort,
            archived,
            cwd,
            ..
        } => ClientRequest::ThreadList {
            request_id: id,
            params: ThreadListParams {
                include_attached_threads: false,
                current_account_only: true,
                excluded_thread_ids: None,
                attached_to_thread_id: None,
                originators: None,
                cursor: cursor.clone(),
                limit: Some(PAGE_SIZE),
                sort_key: Some(match sort {
                    Sort::Updated => ThreadSortKey::UpdatedAt,
                    Sort::Created => ThreadSortKey::CreatedAt,
                }),
                sort_direction: None,
                model_providers: None,
                source_kinds: Some(vec![ThreadSourceKind::Cli, ThreadSourceKind::VsCode]),
                archived: Some(*archived),
                section_id: None,
                project_id: None,
                parent_thread_id: None,
                ancestor_thread_id: None,
                cwd: cwd.as_ref().map(|filter| match filter {
                    CwdFilter::One(cwd) => ThreadListCwdFilter::One(cwd.clone()),
                    CwdFilter::Many(cwds) => ThreadListCwdFilter::Many(cwds.clone()),
                }),
                use_state_db_only: matches!(mode, super::listing::PageMode::StateDbOnly),
                search_term: None,
            },
        },
        Command::Preview { thread, .. } | Command::Transcript { thread, .. } => {
            ClientRequest::ThreadRead {
                request_id: id,
                params: ThreadReadParams {
                    thread_id: thread.to_string(),
                    include_turns: true,
                },
            }
        }
        Command::Archive { thread, .. } => ClientRequest::ThreadArchive {
            request_id: id,
            params: ThreadArchiveParams {
                thread_id: thread.to_string(),
            },
        },
        Command::Unarchive { thread, .. } => ClientRequest::ThreadUnarchive {
            request_id: id,
            params: ThreadUnarchiveParams {
                thread_id: thread.to_string(),
            },
        },
    }
}

pub(super) fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
    match command {
        Command::List { backend, .. } => {
            let page: ThreadListResponse = serde_json::from_value(result).map_err(malformed)?;
            let unread = page
                .read_states
                .as_ref()
                .map(|states| {
                    states
                        .iter()
                        .filter(|(_, state)| state.first_unread.is_some())
                        .map(|(id, _)| id.clone())
                        .collect()
                })
                .unwrap_or_default();
            Ok(Reply::Page {
                rows: page
                    .data
                    .iter()
                    .map(|thread| row_of(thread, *backend))
                    .collect(),
                next: page.next_cursor,
                unread,
            })
        }
        Command::Preview { .. } => {
            let response: ThreadReadResponse = serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Preview(preview_lines(&response.thread)))
        }
        Command::Transcript { .. } => {
            let response: ThreadReadResponse = serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Transcript {
                turns: response.thread.turns.len(),
            })
        }
        Command::Archive { .. } => Ok(Reply::Archived),
        Command::Unarchive { .. } => {
            let response: ThreadUnarchiveResponse =
                serde_json::from_value(result).map_err(malformed)?;
            Ok(Reply::Unarchived {
                thread: ThreadId::new(&response.thread.id),
                cwd: Some(response.thread.cwd.display().to_string()),
            })
        }
    }
}

/// One saved thread of a `thread/list` page (oracle: `row_from_app_server_thread`).
pub fn row_of(thread: &Thread, backend: BackendId) -> Row {
    Row {
        id: ThreadId::new(&thread.id),
        preview: thread.preview.clone(),
        model_provider: thread.model_provider.clone(),
        started: String::new(),
        thread_name: thread.name.clone(),
        created: Some(thread.created_at),
        updated: Some(thread.updated_at),
        cwd: Some(thread.cwd.display().to_string()),
        branch: thread.git_info.as_ref().and_then(|git| git.branch.clone()),
        cloud: false,
        unread: false,
        backend,
        path: thread.path.as_ref().map(|path| path.display().to_string()),
    }
}

/// The newest transcript lines of a thread read with its turns, newest
/// first as the oracle collects them, then in reading order (oracle:
/// `append_transcript_preview_lines` / `_text`): the user's and the
/// assistant's messages, each message's nonblank lines taken from its end.
pub fn preview_lines(thread: &Thread) -> Vec<PreviewLine> {
    let mut lines: Vec<PreviewLine> = Vec::new();
    let items = thread
        .turns
        .iter()
        .rev()
        .flat_map(|turn| turn.items.iter().rev())
        .map(|item| &item.payload);
    for item in items {
        let (speaker, text) = match item {
            ThreadItemPayload::UserMessage { content, .. } => (
                Speaker::User,
                content
                    .iter()
                    .filter_map(|input| match input {
                        UserInput::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            ThreadItemPayload::AgentMessage { text, .. } => (Speaker::Assistant, text.clone()),
            _ => continue,
        };
        let remaining = PREVIEW_LINES - lines.len();
        lines.extend(
            text.lines()
                .rev()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .take(remaining)
                .map(|line| PreviewLine {
                    speaker,
                    text: line.to_owned(),
                }),
        );
        if lines.len() == PREVIEW_LINES {
            break;
        }
    }
    lines.reverse();
    lines
}
