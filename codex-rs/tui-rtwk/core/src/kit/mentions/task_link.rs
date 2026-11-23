//! The link syntax a task mention's binding is written to the history file in,
//! `[@title](thread://id)` (`task_mentions.rs`: `parse_task_link`, `format_task_link`). The task
//! and the path it may carry are `core::data::task`.

use crate::data::task::{MAX_TASK_TITLE_CHARS, valid_thread_path};

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
