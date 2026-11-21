//! The agent's Markdown source as the oracle parses it (markdown.rs
//! `unwrap_markdown_fences`): a ```` ```md ```` / ```` ```markdown ```` fence
//! whose body holds a table (a header row over a delimiter row) is unwrapped,
//! so `MarkdownView` lays the table out natively; any other fence, and one
//! still open at the end of a stream, stays as written. A rewrite of the
//! source before the parser (Flutter: the string handed to `MarkdownBody`),
//! not of laid-out text.

use std::borrow::Cow;

#[derive(Clone, Copy)]
struct Fence {
    marker: u8,
    len: usize,
    quoted: bool,
}

/// `source` with its table-holding Markdown fences unwrapped.
pub fn unwrap_markdown_fences(source: &str) -> Cow<'_, str> {
    if !source.contains("```") && !source.contains("~~~") {
        return Cow::Borrowed(source);
    }
    let mut out = String::with_capacity(source.len());
    // The open fence, whether it may unwrap, and its lines so far.
    let mut open: Option<(Fence, bool, Vec<&str>)> = None;
    for line in source.split_inclusive('\n') {
        if let Some((fence, markdown, mut body)) = open.take() {
            if is_close(line, fence) {
                if markdown && holds_table(&body, fence.quoted) {
                    body.remove(0);
                    out.extend(body);
                } else {
                    out.extend(body);
                    out.push_str(line);
                }
            } else {
                body.push(line);
                open = Some((fence, markdown, body));
            }
            continue;
        }
        if let Some((fence, markdown)) = open_fence(line) {
            open = Some((fence, markdown, vec![line]));
        } else {
            out.push_str(line);
        }
    }
    if let Some((_, _, body)) = open {
        out.extend(body);
    }
    Cow::Owned(out)
}

/// The line without its break and up to three blanks of indent; `None` for
/// an indented code line (four columns).
fn strip_indent(line: &str) -> Option<&str> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let mut column = 0;
    for (index, byte) in line.bytes().enumerate() {
        column += match byte {
            b' ' => 1,
            b'\t' => 4,
            _ => return Some(&line[index..]),
        };
        if column >= 4 {
            return None;
        }
    }
    Some("")
}

/// oracle table_detect.rs `strip_blockquote_prefix`.
fn strip_quote(line: &str) -> &str {
    let mut rest = line.trim_start();
    while let Some(stripped) = rest.strip_prefix('>') {
        rest = stripped.strip_prefix(' ').unwrap_or(stripped).trim_start();
    }
    rest
}

fn marker(line: &str) -> Option<(u8, usize)> {
    let first = *line.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let len = line.bytes().take_while(|&byte| byte == first).count();
    (len >= 3).then_some((first, len))
}

fn open_fence(line: &str) -> Option<(Fence, bool)> {
    let trimmed = strip_indent(line)?;
    let quoted = trimmed.trim_start().starts_with('>');
    let text = strip_quote(trimmed);
    let (marker, len) = marker(text)?;
    let info = text[len..].split_whitespace().next().unwrap_or_default();
    let markdown = info.eq_ignore_ascii_case("md") || info.eq_ignore_ascii_case("markdown");
    Some((
        Fence {
            marker,
            len,
            quoted,
        },
        markdown,
    ))
}

fn is_close(line: &str, fence: Fence) -> bool {
    let Some(trimmed) = strip_indent(line) else {
        return false;
    };
    let text = if fence.quoted {
        if !trimmed.trim_start().starts_with('>') {
            return false;
        }
        strip_quote(trimmed)
    } else {
        trimmed
    };
    marker(text).is_some_and(|(marker, len)| {
        marker == fence.marker && len >= fence.len && text[len..].trim().is_empty()
    })
}

/// A header row directly over a delimiter row (oracle
/// `markdown_fence_contains_table`); `body[0]` is the opening line.
fn holds_table(body: &[&str], quoted: bool) -> bool {
    let mut previous: Option<&str> = None;
    for line in body.iter().skip(1) {
        let text = if quoted { strip_quote(line) } else { line };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            previous = None;
            continue;
        }
        if let Some(previous) = previous
            && is_header(previous)
            && !is_delimiter(previous)
            && is_delimiter(trimmed)
        {
            return true;
        }
        previous = Some(trimmed);
    }
    false
}

/// oracle table_detect.rs `parse_table_segments`: the cells of a row with a
/// pipe, outer pipes optional.
fn segments(line: &str) -> Option<Vec<&str>> {
    let trimmed = line.trim();
    let outer = trimmed.starts_with('|') || trimmed.ends_with('|');
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    // A `\|` is a pipe inside a cell.
    let mut cells = Vec::new();
    let (mut start, mut escaped) = (0, false);
    for (index, byte) in inner.bytes().enumerate() {
        match byte {
            b'\\' if !escaped => escaped = true,
            b'|' if !escaped => {
                cells.push(inner[start..index].trim());
                start = index + 1;
            }
            _ => escaped = false,
        }
    }
    cells.push(inner[start..].trim());
    (!trimmed.is_empty() && (outer || cells.len() > 1)).then_some(cells)
}

fn is_header(line: &str) -> bool {
    segments(line).is_some_and(|cells| cells.iter().any(|cell| !cell.is_empty()))
}

fn is_delimiter(line: &str) -> bool {
    segments(line).is_some_and(|cells| {
        cells.into_iter().all(|cell| {
            let body = cell.strip_prefix(':').unwrap_or(cell);
            let body = body.strip_suffix(':').unwrap_or(body);
            body.len() >= 3 && body.bytes().all(|byte| byte == b'-')
        })
    })
}
