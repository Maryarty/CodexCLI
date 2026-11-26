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
use codex_app_server_protocol::ThreadSourceKind;

use crate::threads::asks::ListQuery;
use crate::threads::{ThreadAsk, asks};
use serde::Deserialize;
use serde::Serialize;

use super::Command;
use super::PAGE_SIZE;
use super::Page;
use super::Pins;
use super::Reply;
use super::carried::is_overview_thread;
use super::carried::supports_shared_pinning;
use crate::protocol::agents::listed_page;
use crate::rpc::RpcError;
use crate::rpc::ServerError;

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
    /// The first page also reads the pinned tasks of both sources (oracle:
    /// `refresh` calls `list_pinned_threads` with every refresh): the shell
    /// does it apart, with a `PinReader`.
    pins: bool,
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
    let pins = !search && cursor.is_none();
    let mut cursors = Cursors::read(cursor.as_deref());
    if cloud {
        cursors.other.done = true;
    }
    Some(Fanout {
        cursors,
        search,
        pins,
    })
}

struct SourcePage {
    other: bool,
    next_cursor: Option<String>,
    rows: VecDeque<Thread>,
    shown: usize,
}

impl Fanout {
    /// Whether this page also reads the pinned tasks.
    pub fn reads_pins(&self) -> bool {
        self.pins
    }

    /// The sources still asked, in the order the results come back.
    fn active(&self) -> impl Iterator<Item = (bool, &Part)> {
        [
            (false, &self.cursors.interactive),
            (true, &self.cursors.other),
        ]
        .into_iter()
        .filter(|(_, part)| !part.done)
    }

    /// One `thread/list` per source that is not exhausted, each under an id
    /// from `ids`.
    pub fn requests(&self, mut ids: impl FnMut() -> RequestId) -> Vec<ClientRequest> {
        self.active()
            .map(|(other, part)| {
                ThreadAsk::list(ListQuery {
                    limit: Some(if self.search {
                        crate::protocol::agents::SEARCH_PAGE_SIZE
                    } else {
                        PAGE_SIZE
                    }),
                    source_kinds: Some(if other {
                        vec![ThreadSourceKind::Exec, ThreadSourceKind::AppServer]
                    } else {
                        Vec::new()
                    }),
                    ..crate::protocol::agents::list_query(part.cursor.clone())
                })
                .wire(ids())
            })
            .collect()
    }

    /// The merged page out of the results of `requests`, in the same order
    /// (a JSON result or the client's failure), with the pins the shell read
    /// for a first page. A source that failed fails the page: the retry asks
    /// both again from the same cursor.
    pub fn reply(
        &self,
        results: Vec<Result<serde_json::Value, ServerError>>,
        pins: Option<Pins>,
    ) -> Result<Reply, RpcError> {
        let mut fetched = Vec::new();
        let mut unread = Vec::new();
        for ((other, part), result) in self.active().zip(results) {
            let json = result.map_err(|error| RpcError::Remote(error.message))?;
            let response = asks::listed(json)?;
            if response.next_cursor.is_some() && response.next_cursor == part.cursor {
                return Err(RpcError::Remote("the cursor did not advance".to_owned()));
            }

            let page = listed_page(response);
            unread.extend(page.unread);
            let mut rows = VecDeque::from(page.threads);
            rows.drain(..part.skip.min(rows.len()));
            fetched.push(SourcePage {
                other,
                next_cursor: page.next_cursor,
                rows,
                shown: 0,
            });
        }
        let limit = if self.search {
            usize::MAX
        } else {
            PAGE_SIZE as usize
        };
        let mut threads = Vec::new();
        while threads.len() < limit {
            // The newer head wins; on a tie the greater id (oracle: `next_batch`).
            let next = fetched
                .iter()
                .enumerate()
                .filter_map(|(index, page)| page.rows.front().map(|thread| (index, thread)))
                .max_by(|(_, left), (_, right)| super::compare_recency(left, right))
                .map(|(index, _)| index);
            let Some(index) = next else { break };
            let page = &mut fetched[index];
            if let Some(thread) = page.rows.pop_front() {
                threads.push(thread);
                page.shown += 1;
            }
        }
        let mut cursors = self.cursors.clone();
        for page in fetched {
            let target = if page.other {
                &mut cursors.other
            } else {
                &mut cursors.interactive
            };
            if page.rows.is_empty() {
                target.skip = 0;
                target.done = page.next_cursor.is_none();
                target.cursor = page.next_cursor;
            } else {
                target.skip += page.shown;
            }
        }
        Ok(Reply::Listed(Page {
            threads,
            unread,
            next_cursor: cursors.write(),
            append: false,
            pins: if self.pins { pins } else { None },
        }))
    }
}

/// Reads the pinned tasks of both sources, page after page (oracle:
/// `list_pinned_threads`: per source, a loop on the server's cursor): the
/// shell asks `next_request`, sends it, `feed`s the answer back, and asks
/// again until none is left; `finish` gives the pins. No IO here.
///
/// A server that refuses the request as malformed (the client's error code
/// in -32602..=-32600) has no pins (`Some` with no ids, oracle: the
/// `Ok(None)` arm); any other failure leaves the pins as they were (`None`,
/// oracle: the warning).
#[derive(Clone, Debug, Default)]
pub struct PinReader {
    /// The source being read: 0 the interactive one, 1 the Exec / AppServer.
    source: usize,
    cursor: Option<String>,
    threads: Vec<Thread>,
    end: Option<End>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    Done,
    Refused,
    Failed,
}

impl PinReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// The next page to ask for, under `id`; `None` once the read ended.
    pub fn next_request(&mut self, id: RequestId) -> Option<ClientRequest> {
        self.end.is_none().then(|| {
            crate::protocol::agents::pinned_request(id, self.source == 1, self.cursor.clone())
        })
    }

    /// The answer to the request `next_request` gave last.
    pub fn feed(&mut self, answer: Result<serde_json::Value, ServerError>) {
        if self.end.is_some() {
            return;
        }
        let page = match answer {
            Ok(json) => asks::listed(json).ok(),
            Err(error) if error.refused() => {
                self.end = Some(End::Refused);
                return;
            }
            Err(_) => None,
        };
        let Some(page) = page else {
            self.end = Some(End::Failed);
            return;
        };
        self.threads.extend(page.data.into_iter().filter(|thread| {
            is_overview_thread(thread) && supports_shared_pinning(&thread.source)
        }));
        // A server repeating the cursor it was given cannot advance.
        match page.next_cursor {
            Some(next) if self.cursor.as_ref() != Some(&next) => self.cursor = Some(next),
            _ => {
                self.cursor = None;
                self.source += 1;
                if self.source == 2 {
                    self.end = Some(End::Done);
                }
            }
        }
    }

    /// The pins, once the read ended; `None` when it failed or has not.
    pub fn finish(self) -> Option<Pins> {
        match self.end? {
            End::Failed => None,
            End::Refused => Some(Pins {
                ids: None,
                threads: Vec::new(),
            }),
            End::Done => Some(Pins {
                ids: Some(
                    self.threads
                        .iter()
                        .map(|thread| thread.id.clone())
                        .collect(),
                ),
                threads: self.threads,
            }),
        }
    }
}
