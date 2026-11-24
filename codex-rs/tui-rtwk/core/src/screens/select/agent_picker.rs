//! `/subagents` (oracle `app/session_lifecycle.rs` `open_agent_picker` and
//! `agent_picker_selection_view_params`, `app/agent_navigation.rs`): the sub-agent threads of the
//! conversation in use as rows, derived from facts the app already holds (the thread list of the
//! agents screen, the threads it runs, the requests waiting). The rows are never stored: the
//! sheet reads the facts and calls `rows`. A pick is the app's one door to switch a thread,
//! `Intent::OpenThread(id)` (ADR-003); there is no switch verb of its own.
//!
//! The first thread of the tree (the primary, `Main [default]`) is the root of the thread in
//! use; its spawned descendants follow in the order the backend listed them; a thread that
//! waits for an approval is listed first. A thread archived or removed (`Model::hidden`,
//! `removed`) is not listed.

use std::collections::BTreeSet;

use codex_app_server_protocol::{Thread, ThreadStatus};

use crate::ids::ThreadId;

/// How a row names its agent (`format_agent_picker_item_name`); the words are the view's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Name {
    /// The primary thread: `Main [default]`.
    Main,
    /// `nickname [role]`, `nickname`, `[role]`, or `Agent` when neither is known.
    Agent {
        nickname: Option<String>,
        role: Option<String>,
    },
}

impl Name {
    /// The label of a row, as `format_agent_picker_item_name` writes it.
    pub fn label(&self) -> String {
        match self {
            Name::Main => "Main [default]".to_owned(),
            Name::Agent { nickname, role } => match (nickname, role) {
                (Some(nickname), Some(role)) => format!("{nickname} [{role}]"),
                (Some(nickname), None) => nickname.clone(),
                (None, Some(role)) => format!("[{role}]"),
                (None, None) => "Agent".to_owned(),
            },
        }
    }
}

/// One row of the picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub thread: ThreadId,
    pub name: Name,
    /// The thread is the one in use.
    pub current: bool,
    /// An approval of this thread waits (`name (needs approval)`; such rows come first).
    pub needs_approval: bool,
    /// The thread is not running and not loaded (the status dot is the closed one).
    pub closed: bool,
}

/// What `/subagents` finds (the cases of `open_agent_picker` after the refresh).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
    /// The picker opens on these rows.
    Rows(Vec<Row>),
    /// `No agents available yet.` (an info line).
    NoAgents,
}

fn clean(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// The root of `active` in the thread list: the primary thread of the tree.
fn root_of<'a>(threads: &'a [Thread], active: &'a str) -> &'a str {
    let mut at = active;
    // A parent chain is short; the bound only guards a list that loops.
    for _ in 0..threads.len() + 1 {
        match threads
            .iter()
            .find(|thread| thread.id == at)
            .and_then(|thread| thread.parent_thread_id.as_deref())
        {
            Some(parent) => at = parent,
            None => break,
        }
    }
    at
}

/// Whether `thread` descends from `root`.
fn descends(threads: &[Thread], thread: &Thread, root: &str) -> bool {
    let mut at = thread;
    for _ in 0..threads.len() + 1 {
        let Some(parent) = at.parent_thread_id.as_deref() else {
            return false;
        };
        if parent == root {
            return true;
        }
        match threads.iter().find(|candidate| candidate.id == parent) {
            Some(next) => at = next,
            None => return false,
        }
    }
    false
}

/// The rows of the picker. `threads` is the agents screen's list; `hidden` and `removed` its sets
/// of threads not listed; `active` the thread in use; `waiting` the threads with an approval
/// waiting; `live` whether the app runs the thread (a thread it does not run and the server has not
/// loaded is closed).
pub fn rows(
    threads: &[Thread],
    hidden: &BTreeSet<String>,
    removed: &BTreeSet<String>,
    active: Option<&ThreadId>,
    waiting: &[ThreadId],
    live: impl Fn(&ThreadId) -> bool,
) -> Vec<Row> {
    let Some(active) = active else {
        return Vec::new();
    };
    let root = root_of(threads, active.as_str()).to_owned();
    let listed = |id: &str| !hidden.contains(id) && !removed.contains(id);
    let mut rows = Vec::new();
    let mut push = |id: &str, name: Name, status: Option<&ThreadStatus>| {
        let thread = ThreadId::new(id.to_owned());
        let closed = !live(&thread) && matches!(status, Some(ThreadStatus::NotLoaded));
        rows.push(Row {
            current: &thread == active,
            needs_approval: waiting.contains(&thread),
            closed,
            name,
            thread,
        });
    };
    // The primary is listed whether or not the backend's list has it.
    let primary = threads.iter().find(|thread| thread.id == root);
    push(&root, Name::Main, primary.map(|thread| &thread.status));
    for thread in threads
        .iter()
        .filter(|thread| thread.id != root && listed(&thread.id) && descends(threads, thread, &root))
    {
        let name = Name::Agent {
            nickname: clean(thread.agent_nickname.as_deref()),
            role: clean(thread.agent_role.as_deref()),
        };
        push(&thread.id, name, Some(&thread.status));
    }
    // A stable sort: the rows that wait for an approval come first, the rest keep their order.
    rows.sort_by_key(|row| !row.needs_approval);
    rows
}

/// `open_agent_picker` once its refreshes are in: no thread known says `No agents available yet.`,
/// else the rows (a conversation with only its primary still lists it).
pub fn opened(rows: Vec<Row>) -> Opened {
    if rows.is_empty() {
        Opened::NoAgents
    } else {
        Opened::Rows(rows)
    }
}

/// A thread other than the primary is known (`has_non_primary_thread`): where the subagents
/// feature is off, this decides between the picker and the prompt that offers to enable it.
pub fn has_subagents(rows: &[Row]) -> bool {
    rows.iter().any(|row| row.name != Name::Main)
}
