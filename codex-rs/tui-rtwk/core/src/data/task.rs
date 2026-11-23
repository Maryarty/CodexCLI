//! A task (another thread of the user's) that an `@` mention can name, and the path a
//! reference to it may carry (`task_mentions.rs`: `TaskMention`, `valid_thread_path`). The link
//! syntax its binding is written to the history file in (`parse_task_link`, `format_task_link`)
//! is the codec's, `kit::mentions::task_link`. The search that finds tasks (`thread/search`) and
//! the prompt context a submit adds (`apply_task_references`) are the shell's.

use crate::ids::ThreadId;

/// The most tasks one prompt may reference.
pub const MAX_REFERENCED_TASKS: usize = 16;
/// The longest task title a mention keeps, in characters.
pub const MAX_TASK_TITLE_CHARS: usize = 160;

/// One task found by the search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskMention {
    pub thread_id: ThreadId,
    pub title: String,
    pub cwd: String,
    pub snippet: String,
}

/// The thread id of a `thread://` path the shell may reference.
pub fn valid_thread_path(path: &str) -> Option<&str> {
    let thread_id = path.strip_prefix("thread://")?;
    (!thread_id.is_empty()
        && thread_id.len() <= 64
        && thread_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
    .then_some(thread_id)
}
