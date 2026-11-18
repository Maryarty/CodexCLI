//! Discovery over the oracle's two sources: the interactive threads and the
//! Exec / AppServer ones, merged by recency (oracle:
//! `agents_overview_discovery.rs`, `AgentsOverviewDiscovery::next_batch`).
//! The oracle retains each source's cursor and unshown rows in the app; here
//! the retained state is the page cursor the model already keeps, made of
//! both sources' cursors (`Cursors`), so core stays a pure function of the
//! replies: the shell sends `Fanout::requests` and hands the results back to
//! `Fanout::reply`. No IO.
//!
//! A source's rows that a page did not show stay with the source: its part
//! of the cursor keeps the page it was read from and how many of its rows
//! were shown (`skip`), and the next page reads that page again and skips
//! them. Only when all of a source's rows were shown does it move on to the
//! cursor the server gave.

use std::collections::VecDeque;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadListParams;
use codex_app_server_protocol::ThreadListResponse;
use codex_app_server_protocol::ThreadSortKey;
use codex_app_server_protocol::ThreadSourceKind;
use serde::Deserialize;
use serde::Serialize;

use super::Command;
use super::PAGE_SIZE;
use super::Page;
use super::Reply;
use super::carried::is_overview_thread;
use crate::rpc::RpcError;

/// A search scans pages of this many rows of each source (oracle:
/// `page_size = 100`).
const SEARCH_PAGE_SIZE: u32 = 100;

/// The prefix of a cursor made by `reply`; any other cursor is the
/// interactive source's own (a seeded page, an older reply).
const TAG: &str = "sources:";

/// Where one source continues.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Part {
    /// The server's cursor of the page to read.
    cursor: Option<String>,
    /// Rows of that page already shown.
    skip: usize,
    /// The source has no more pages.
    done: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Cursors {
    interactive: Part,
    other: Part,
}

impl Cursors {
    fn read(cursor: Option<&str>) -> Self {
        let Some(text) = cursor else {
            return Self::default();
        };
        text.strip_prefix(TAG)
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_else(|| Self {
                interactive: Part {
                    cursor: Some(text.to_owned()),
                    ..Part::default()
                },
                other: Part::default(),
            })
    }

    /// `None` once both sources ended.
    fn write(&self) -> Option<String> {
        (!(self.interactive.done && self.other.done))
            .then(|| format!("{TAG}{}", serde_json::to_string(self).unwrap_or_default()))
    }
}

/// The requests one `List` or `Search` command stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fanout {
    cursors: Cursors,
    /// A history search scan, not a page of the list.
    search: bool,
}

/// The fan-out of `command`, when it is one: a page of the list or of a
/// history search. `cloud`: the backend's task list cannot be filtered to
/// Exec and AppServer threads, so only the interactive source is asked.
pub fn fanout(command: &Command, cloud: bool) -> Option<Fanout> {
    let (cursor, search) = match command {
        Command::List { cursor } => (cursor, false),
        Command::Search { cursor, .. } => (cursor, true),
        _ => return None,
    };
    let mut cursors = Cursors::read(cursor.as_deref());
    if cloud {
        cursors.other.done = true;
    }
    Some(Fanout { cursors, search })
}

impl Fanout {
    /// The sources still asked, in the order the results come back.
    fn active(&self) -> Vec<(bool, &Part)> {
        [
            (false, &self.cursors.interactive),
            (true, &self.cursors.other),
        ]
        .into_iter()
        .filter(|(_, part)| !part.done)
        .collect()
    }

    /// One `thread/list` per source that is not exhausted, each under an id
    /// from `ids`.
    pub fn requests(&self, mut ids: impl FnMut() -> RequestId) -> Vec<ClientRequest> {
        self.active()
            .into_iter()
            .map(|(other, part)| ClientRequest::ThreadList {
                request_id: ids(),
                params: ThreadListParams {
                    include_attached_threads: false,
                    current_account_only: true,
                    excluded_thread_ids: None,
                    attached_to_thread_id: None,
                    originators: None,
                    cursor: part.cursor.clone(),
                    limit: Some(if self.search {
                        SEARCH_PAGE_SIZE
                    } else {
                        PAGE_SIZE
                    }),
                    sort_key: Some(ThreadSortKey::RecencyAt),
                    sort_direction: None,
                    model_providers: Some(Vec::new()),
                    source_kinds: Some(if other {
                        vec![ThreadSourceKind::Exec, ThreadSourceKind::AppServer]
                    } else {
                        Vec::new()
                    }),
                    archived: Some(false),
                    section_id: None,
                    project_id: None,
                    parent_thread_id: None,
                    ancestor_thread_id: None,
                    cwd: None,
                    use_state_db_only: true,
                    search_term: None,
                },
            })
            .collect()
    }

    /// The merged page out of the results of `requests`, in the same order
    /// (a JSON result or the client's failure text). A source that failed
    /// fails the page: the retry asks both again from the same cursor.
    pub fn reply(
        &self,
        results: Vec<Result<serde_json::Value, String>>,
    ) -> Result<Reply, RpcError> {
        let active = self.active();
        let mut fetched: Vec<(bool, ThreadListResponse, VecDeque<Thread>)> = Vec::new();
        let mut unread = Vec::new();
        for ((other, part), result) in active.iter().zip(results) {
            let json = result.map_err(RpcError::Remote)?;
            let page: ThreadListResponse = serde_json::from_value(json)
                .map_err(|error| RpcError::Malformed(error.to_string()))?;
            // The server repeating the cursor it was given cannot advance.
            if page.next_cursor.is_some() && page.next_cursor == part.cursor {
                return Err(RpcError::Remote("the cursor did not advance".to_owned()));
            }
            unread.extend(
                page.read_states
                    .iter()
                    .flatten()
                    .filter(|(_, state)| state.first_unread.is_some())
                    .map(|(id, _)| id.clone()),
            );
            let rows = page
                .data
                .iter()
                .filter(|thread| is_overview_thread(thread))
                .skip(part.skip)
                .cloned()
                .collect();
            fetched.push((*other, page, rows));
        }
        let limit = if self.search {
            usize::MAX
        } else {
            PAGE_SIZE as usize
        };
        let mut queues: Vec<VecDeque<Thread>> =
            fetched.iter().map(|(_, _, rows)| rows.clone()).collect();
        let mut threads = Vec::new();
        let mut shown = vec![0_usize; queues.len()];
        while threads.len() < limit {
            // The newer head wins; on a tie the greater id (oracle:
            // `next_batch`).
            let next = queues
                .iter()
                .enumerate()
                .filter_map(|(index, rows)| rows.front().map(|thread| (index, thread)))
                .max_by(|(_, left), (_, right)| {
                    left.recency_at
                        .unwrap_or(left.updated_at)
                        .cmp(&right.recency_at.unwrap_or(right.updated_at))
                        .then_with(|| left.id.cmp(&right.id))
                })
                .map(|(index, _)| index);
            let Some(index) = next else { break };
            if let Some(thread) = queues[index].pop_front() {
                threads.push(thread);
                shown[index] += 1;
            }
        }
        let mut cursors = self.cursors.clone();
        for (((other, page, rows), shown), (_, part)) in
            fetched.iter().zip(shown).zip(active.iter())
        {
            let target = if *other {
                &mut cursors.other
            } else {
                &mut cursors.interactive
            };
            if shown < rows.len() {
                // Rows of this page were not shown: stay on it.
                target.skip = part.skip + shown;
            } else {
                target.skip = 0;
                target.cursor.clone_from(&page.next_cursor);
                target.done = page.next_cursor.is_none();
            }
        }
        Ok(Reply::Listed(Page {
            threads,
            unread,
            next_cursor: cursors.write(),
            append: false,
        }))
    }
}
