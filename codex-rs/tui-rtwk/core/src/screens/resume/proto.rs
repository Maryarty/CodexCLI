//! The picker's RPCs on the wire and back: `thread/list` pages, the preview
//! and transcript reads, archive and unarchive. Core knows the protocol (the
//! shell sends what `wire` makes); no view file does.
//! Oracle: `resume_picker.rs` (`thread_list_params`, `row_from_app_server_thread`,
//! `load_app_server_page`) and `resume_picker_transcript_preview.rs`
//! (`append_transcript_preview_lines`).

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ThreadListCwdFilter;
use codex_app_server_protocol::ThreadSortKey;
use codex_app_server_protocol::ThreadSourceKind;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::UserInput;

use super::toolbar::Sort;
use super::{Command, CwdFilter, PreviewLine, Reply, Row, Speaker};
use crate::ids::{BackendId, ThreadId};
use crate::rpc::RpcError;
use crate::threads::asks::ListQuery;
use crate::threads::{ThreadAsk, ThreadReply, asks};

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
        } => ThreadAsk::list(ListQuery {
            current_account_only: true,
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
            cwd: cwd.as_ref().map(|filter| match filter {
                CwdFilter::One(cwd) => ThreadListCwdFilter::One(cwd.clone()),
                CwdFilter::Many(cwds) => ThreadListCwdFilter::Many(cwds.clone()),
            }),
            use_state_db_only: matches!(mode, super::listing::PageMode::StateDbOnly),
            ..ListQuery::default()
        })
        .wire(id),
        Command::Archive { thread, .. } => ThreadAsk::Archive {
            thread: thread.clone(),
        }
        .wire(id),
        Command::Unarchive { thread, .. } => ThreadAsk::Unarchive {
            thread: thread.clone(),
        }
        .wire(id),
    }
}

pub(super) fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    match command {
        Command::List { backend, .. } => {
            let page = asks::listed(result)?;
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
        Command::Archive { .. } => Ok(Reply::Archived),
        Command::Unarchive { thread, .. } => {
            match (ThreadAsk::Unarchive {
                thread: thread.clone(),
            })
            .decode(result)?
            {
                ThreadReply::Unarchived(thread) => Ok(Reply::Unarchived {
                    thread: ThreadId::new(&thread.id),
                    cwd: Some(thread.cwd.display().to_string()),
                }),
                _ => Err(RpcError::Mismatch),
            }
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

/// The newest transcript lines of a thread's turns (newest turn first, as
/// the thread's history reads them), newest first as the oracle collects
/// them, then in reading order (oracle: `append_transcript_preview_lines` /
/// `_text`): the user's and the assistant's messages, each message's
/// nonblank lines taken from its end.
pub fn preview_lines(turns: &[Turn]) -> Vec<PreviewLine> {
    let mut lines: Vec<PreviewLine> = Vec::new();
    let items = turns
        .iter()
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

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::{ThreadItem, TurnItemsView, TurnStatus};

    fn turn(items: Vec<ThreadItemPayload>) -> Turn {
        Turn {
            id: "t".into(),
            root_turn_id: None,
            items: items.into_iter().map(ThreadItem::untimed).collect(),
            items_view: TurnItemsView::Full,
            status: TurnStatus::Completed,
            error: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
        }
    }

    fn user(text: &str) -> ThreadItemPayload {
        ThreadItemPayload::UserMessage {
            id: "u".into(),
            client_id: None,
            content: vec![UserInput::Text {
                text: text.into(),
                text_elements: Vec::new(),
            }],
        }
    }

    fn assistant(text: &str) -> ThreadItemPayload {
        ThreadItemPayload::AgentMessage {
            moderation: None,
            id: "a".into(),
            text: text.into(),
            phase: None,
            memory_citation: None,
            delivery: None,
            questions: None,
        }
    }

    #[test]
    fn the_preview_is_the_newest_lines_in_reading_order() {
        // Newest turn first, as the thread's history reads them.
        let turns = vec![
            turn(vec![user("next"), assistant("one\n\n  two  \nthree")]),
            turn(vec![user("first")]),
        ];
        let lines = preview_lines(&turns);
        let spoken: Vec<_> = lines
            .iter()
            .map(|line| (line.speaker, line.text.as_str()))
            .collect();
        assert_eq!(
            spoken,
            vec![
                (Speaker::User, "first"),
                (Speaker::User, "next"),
                (Speaker::Assistant, "one"),
                (Speaker::Assistant, "two"),
                (Speaker::Assistant, "three"),
            ]
        );
    }

    #[test]
    fn the_preview_keeps_only_the_newest_six_lines() {
        let text = (1..=9)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let lines = preview_lines(&[turn(vec![assistant(&text)])]);
        let texts: Vec<_> = lines.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(texts, vec!["4", "5", "6", "7", "8", "9"]);
    }
}
