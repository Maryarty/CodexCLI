//! A task (another thread of the user's) that an `@` mention can name, and
//! the link syntax its binding is written to the history file in
//! (`task_mentions.rs`: `TaskMention`, `valid_thread_path`, `parse_task_link`,
//! `format_task_link`). The search that finds tasks (`thread/search`) and the
//! prompt context a submit adds (`apply_task_references`) are the shell's.

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

/// The task link `[@title](thread://id)` at `start`: the title (escapes
/// undone), the path and where the link ends.
pub fn parse_task_link(text: &str, start: usize) -> Option<(String, String, usize)> {
    let remaining = text.get(start..)?.strip_prefix("[@")?;
    let mut title = String::new();
    let mut chars = remaining.char_indices();
    for _ in 0..=MAX_TASK_TITLE_CHARS {
        let (index, ch) = chars.next()?;
        match ch {
            '\\' => title.push(chars.next()?.1),
            ']' if !title.is_empty() => {
                let suffix = remaining.get(index + 1..)?.strip_prefix('(')?;
                let (path, _) = suffix.split_once(')')?;
                valid_thread_path(path)?;
                return Some((title, path.to_string(), start + index + path.len() + 5));
            }
            ']' => return None,
            _ => title.push(ch),
        }
    }
    None
}

/// The link of a task mention, the title escaped.
pub fn format_task_link(title: &str, path: &str) -> String {
    format!(
        "[@{}]({path})",
        title
            .replace('\\', "\\\\")
            .replace("](", "]\\(")
            .replace(']', "\\]")
    )
}
