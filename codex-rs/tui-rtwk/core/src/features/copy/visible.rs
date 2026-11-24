//! Carried from codex-rs/tui/src/git_action_directives.rs (`parse_assistant_markdown`,
//! `strip_line_directives`, `rewrite_code_comment_line`; framework-free logic, the git actions
//! themselves are not carried: a copy keeps only what is visible) and from
//! `markdown_render/inline_directives.rs` (`followup_labels`, here `threads::followups::labels`).
//!
//! What a copy of an assistant message holds: the message without the directives the Codex App
//! reads (`::git-stage{..}`), a review comment worded as a line, the follow-ups as their labels.
//! Whitespace of unchanged lines is preserved: Markdown hard breaks and code depend on it.

use std::path::Path;

use crate::threads::directives::{AssistantDirective, QuoteEscaping, parse_assistant_directive};
use crate::threads::followups;

/// `ChatWidget::record_agent_markdown`'s `last_agent_markdown`: the visible Markdown of `source`
/// with the follow-ups as their labels; none when nothing visible is left.
pub fn copy_markdown(source: &str, cwd: &str) -> Option<String> {
    let visible = visible_markdown(source, Path::new(cwd));
    (!visible.is_empty()).then(|| followups::labels(&visible).into_owned())
}

/// `parse_assistant_markdown(..).visible_markdown`.
pub fn visible_markdown(markdown: &str, cwd: &Path) -> String {
    let mut visible_lines = Vec::new();
    for line in markdown.lines() {
        let visible_line =
            rewrite_code_comment_line(line, cwd).unwrap_or_else(|| strip_line_directives(line));
        visible_lines.push(if visible_line == line {
            visible_line
        } else {
            visible_line.trim_end().to_string()
        });
    }
    while visible_lines
        .last()
        .is_some_and(|line| line.trim().is_empty())
    {
        visible_lines.pop();
    }
    visible_lines.join("\n")
}

fn rewrite_code_comment_line(line: &str, cwd: &Path) -> Option<String> {
    let content = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - content.len()];
    let directive = parse_assistant_directive(content, QuoteEscaping::Backslash)?;
    if directive.name != "code-comment" {
        return None;
    }
    let suffix = &content[directive.raw.len()..];
    let title = directive.attributes.get("title")?;
    let body = directive.attributes.get("body")?;
    let file = directive.attributes.get("file")?;
    let title = title.trim();
    let body = body.trim();
    let file = file.trim();
    (!title.is_empty() && !body.is_empty() && !file.is_empty()).then_some(())?;

    let start = directive_integer(&directive, "start").unwrap_or(1).max(1);
    let end = directive_integer(&directive, "end")
        .unwrap_or(start)
        .max(start);
    let title = if title_has_priority(title) {
        title.to_string()
    } else if let Some(priority @ 0..=3) = directive_integer(&directive, "priority") {
        format!("[P{priority}] {title}")
    } else {
        title.to_string()
    };
    let file_path = Path::new(file);
    let file = file_path
        .strip_prefix(cwd)
        .unwrap_or(file_path)
        .to_string_lossy()
        .replace('\\', "/");
    let location = if start == end {
        format!("{file}:{start}")
    } else {
        format!("{file}:{start}-{end}")
    };

    let suffix = strip_line_directives(suffix);
    Some(format!(
        "{indent}- {title} — {location}\n{indent}  {body}{suffix}"
    ))
}

/// `line` without its `::git-…{…}` directives (the Codex App's, never drawn).
pub fn strip_line_directives(line: &str) -> String {
    let mut visible = String::new();
    let mut remaining = line;

    while let Some(start) = remaining.find("::git-") {
        visible.push_str(&remaining[..start]);
        let source = &remaining[start..];
        if let Some(directive) = parse_assistant_directive(source, QuoteEscaping::Literal) {
            remaining = &source[directive.raw.len()..];
        } else if let Some((_, attributes)) = source.split_once('{')
            && let Some((_, suffix)) = attributes.split_once('}')
        {
            remaining = suffix;
        } else {
            visible.push_str(source);
            return visible;
        }
    }
    visible.push_str(remaining);
    visible
}

fn directive_integer(directive: &AssistantDirective<'_>, name: &str) -> Option<i64> {
    directive
        .attributes
        .get(name)?
        .trim()
        .trim_start_matches(['P', 'p'])
        .parse()
        .ok()
}

fn title_has_priority(title: &str) -> bool {
    let bytes = title.trim_start().as_bytes();
    bytes.len() >= 4
        && bytes[0] == b'['
        && matches!(bytes[1], b'P' | b'p')
        && bytes[2].is_ascii_digit()
        && bytes[3] == b']'
}
