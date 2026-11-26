//! The agent's Markdown source as the oracle parses it (markdown.rs
//! `unwrap_markdown_fences`, named here without the R15 word): a ```` ```md ```` / ```` ```markdown ```` fence
//! whose body holds a table (a header row over a delimiter row) is unwrapped,
//! so `MarkdownView` lays the table out natively; any other fence, and one
//! still open at the end of a stream, stays as written. A rewrite of the
//! source before the parser (Flutter: the string handed to `MarkdownBody`),
//! not of laid-out text.
//!
//! With the `tui.rendering.tables` preference off the rewrite goes the other
//! way (oracle `markdown_render/source_tables.rs` `preserve`): a table, and a
//! Markdown fence that holds one (its fence lines too), is a plain code block
//! of its source text, the quote or list prefix of its lines kept.
//!
//! With `tui.rendering.lists` off a task item draws its box as the text it was written
//! (`[ ]`, `[x]`: oracle `ENABLE_TASKLISTS` off) in the parsed tree: `plain_lists`.

use std::borrow::Cow;

use std::sync::Arc;

use rtwk_rich::markdown::{Block, MdBlock, MdInline, MdList, MdListItem};

use codex_tui_core::app::facts::Rendering;

#[derive(Clone, Copy)]
struct Fence {
    marker: u8,
    len: usize,
    quoted: bool,
}

/// A source boundary that future input cannot change, and where it ends after rewriting.
#[derive(Default)]
struct Checkpoint {
    source: usize,
    output: usize,
}

impl Checkpoint {
    fn unchanged(source: &str) -> Self {
        let complete = source.rfind('\n').map_or(0, |at| at + 1);
        Self {
            source: complete,
            output: complete,
        }
    }
}

/// The normalized prefix of a live message. Only hold complete lines that
/// future chunks cannot change under the same table setting.
#[derive(Default)]
pub(super) struct LiveTableSource {
    stable_source: String,
    output: String,
    stable_output: usize,
    tables: bool,
}

impl LiveTableSource {
    pub(super) fn prepare(&mut self, source: &str, tables: bool) -> &str {
        let remaining = if self.tables == tables
            && let Some(remaining) = source.strip_prefix(self.stable_source.as_str())
        {
            remaining
        } else {
            self.tables = tables;
            self.stable_source.clear();
            self.stable_output = 0;
            source
        };
        self.output.truncate(self.stable_output);
        let (tail, stable) = if tables {
            lift_fences::<true>(remaining)
        } else {
            literal_tables::<true>(remaining)
        };
        self.output.push_str(&tail);
        self.stable_source.push_str(&remaining[..stable.source]);
        self.stable_output += stable.output;
        &self.output
    }
}

fn has_fence(source: &str) -> bool {
    source.contains("```") || source.contains("~~~")
}

/// `source` with its table-holding Markdown fences unwrapped.
pub fn lift_markdown_fences(source: &str) -> Cow<'_, str> {
    lift_fences::<false>(source).0
}

fn lift_fences<const LIVE: bool>(source: &str) -> (Cow<'_, str>, Checkpoint) {
    if !has_fence(source) {
        let checkpoint = if LIVE {
            Checkpoint::unchanged(source)
        } else {
            Checkpoint::default()
        };
        return (Cow::Borrowed(source), checkpoint);
    }
    let mut out = String::with_capacity(source.len());
    let mut read = 0;
    let mut stable = Checkpoint::default();
    // The open fence, whether it may unwrap, and its lines so far.
    let mut open: Option<(Fence, bool, Vec<&str>)> = None;
    for line in source.split_inclusive('\n') {
        if LIVE {
            read += line.len();
        }
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
        } else if let Some((fence, markdown)) = open_fence(line) {
            open = Some((fence, markdown, vec![line]));
        } else {
            out.push_str(line);
        }
        if LIVE && open.is_none() && line.ends_with('\n') {
            stable = Checkpoint {
                source: read,
                output: out.len(),
            };
        }
    }
    if let Some((_, _, body)) = open {
        out.extend(body);
    }
    (Cow::Owned(out), stable)
}

/// `source` as the `tui.rendering` preferences read it: the table-holding Markdown
/// fences unwrapped while tables are drawn, the table sources kept literal while they are not.
pub fn preferred(source: &str, rendering: Rendering) -> Cow<'_, str> {
    if rendering.tables {
        lift_markdown_fences(source)
    } else {
        keep_tables(source)
    }
}

/// `source` with every table (and every table-holding Markdown fence, its fence lines
/// too) as a code block of its own text: what the oracle's disabled tables draw.
pub fn keep_tables(source: &str) -> Cow<'_, str> {
    if !source.contains('|') {
        return Cow::Borrowed(source);
    }
    literal_tables::<false>(source).0
}

fn literal_tables<const LIVE: bool>(source: &str) -> (Cow<'_, str>, Checkpoint) {
    if LIVE && !source.contains('|') && !has_fence(source) {
        return (Cow::Borrowed(source), Checkpoint::unchanged(source));
    }
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut out = String::with_capacity(source.len() + 16);
    let (mut at, mut read) = (0, 0);
    let mut stable = Checkpoint::default();
    while at < lines.len() {
        let line = lines[at];
        let complete = |index: usize| lines.get(index).is_some_and(|line| line.ends_with('\n'));
        let (end, settled) = if let Some((marker, len, markdown, quoted)) = fence_start(line) {
            // the block ends at its closing line; one still open takes the rest
            let close = (at + 1..lines.len()).find(|&index| fence_end(lines[index], marker, len));
            let end = close.map_or(lines.len(), |close| close + 1);
            if markdown && close.is_some() && holds_table(&lines[at..end - 1], quoted) {
                literal(&mut out, &lines[at..end]);
            } else {
                out.extend(lines[at..end].iter().copied());
            }
            (end, LIVE && close.is_some_and(complete))
        } else if let Some(end) = table_end(&lines, at) {
            literal(&mut out, &lines[at..end]);
            // Commit only after consuming its terminator. If that line is replaced,
            // the generated fence length may change.
            (end, false)
        } else {
            out.push_str(line);
            // A possible header can only be settled together with its next line.
            (at + 1, LIVE && complete(at) && !line.contains('|'))
        };
        if LIVE {
            read += lines[at..end].iter().map(|line| line.len()).sum::<usize>();
            if settled {
                stable = Checkpoint {
                    source: read,
                    output: out.len(),
                };
            }
        }
        at = end;
    }
    (Cow::Owned(out), stable)
}

/// A fence opened on `line`, in a quote or a list item too: its marker and length, whether
/// its info says Markdown, and whether the line is quoted.
fn fence_start(line: &str) -> Option<(u8, usize, bool, bool)> {
    let mut rest = line.trim_start();
    let quoted = rest.starts_with('>');
    while let Some(stripped) = rest.strip_prefix('>') {
        rest = stripped.trim_start();
    }
    // a list item's marker: a bullet or a number and its delimiter, then blanks
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let item = if digits > 0 && digits <= 9 {
        rest[digits..]
            .strip_prefix(['.', ')'])
            .filter(|after| after.starts_with(' '))
    } else {
        rest.strip_prefix(['-', '+', '*'])
            .filter(|after| after.starts_with(' '))
    };
    if let Some(after) = item {
        rest = after.trim_start();
    }
    let (marker, len) = marker(rest)?;
    let info = rest[len..].split_whitespace().next().unwrap_or_default();
    let markdown = info.eq_ignore_ascii_case("md") || info.eq_ignore_ascii_case("markdown");
    Some((marker, len, markdown, quoted))
}

/// Whether `line` closes the fence `marker`/`len` opened above (indented as the item it is in).
fn fence_end(line: &str, marker: u8, len: usize) -> bool {
    let mut rest = line.trim_start();
    while let Some(stripped) = rest.strip_prefix('>') {
        rest = stripped.trim_start();
    }
    let rest = rest.trim_end();
    rest.bytes().all(|byte| byte == marker) && rest.len() >= len
}

/// A row's text without its quote markers and blanks.
fn row(line: &str) -> &str {
    strip_quote(line.strip_suffix('\n').unwrap_or(line)).trim()
}

/// Where the table that starts at `lines[at]` ends: a header row over a delimiter row, then
/// the rows with a pipe up to a blank line or the end of the quote.
fn table_end(lines: &[&str], at: usize) -> Option<usize> {
    let header = row(lines[at]);
    if !header.contains('|')
        || !is_header(header)
        || is_delimiter(header)
        || !is_delimiter(row(lines.get(at + 1)?))
    {
        return None;
    }
    Some(
        (at + 2..lines.len())
            .find(|&index| {
                let text = row(lines[index]);
                text.is_empty() || !text.contains('|')
            })
            .unwrap_or(lines.len()),
    )
}

/// `lines` as a fenced code block without a language, each fence line after the prefix
/// (quote markers and blanks) of the first line, and long enough not to end inside.
fn literal(out: &mut String, lines: &[&str]) {
    let prefix_len = lines[0]
        .find(|c: char| !c.is_whitespace() && c != '>')
        .unwrap_or(0);
    let prefix = &lines[0][..prefix_len];
    let longest = lines
        .iter()
        .flat_map(|line| line.split(|c| c != '`').map(str::len))
        .max()
        .unwrap_or(0);
    let fence: String = std::iter::repeat_n('`', longest.max(2) + 1).collect();
    out.push_str(&format!("{prefix}{fence}\n"));
    out.extend(lines.iter().copied());
    if !lines[lines.len() - 1].ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("{prefix}{fence}\n"));
}

/// `blocks` with every task item's box as plain text at the start of its first paragraph
/// (`[x]` for a checked one, whatever case the source used), in lists and quotes at any depth.
pub fn plain_lists(blocks: &[Block]) -> Vec<Block> {
    blocks.iter().map(unboxed).collect()
}

fn unboxed(block: &Block) -> Block {
    match &**block {
        MdBlock::BlockQuote(children) => Arc::new(MdBlock::BlockQuote(plain_lists(children))),
        MdBlock::List(list) => Arc::new(MdBlock::List(MdList {
            items: list.items.iter().map(plain_item).collect(),
            ..list.clone()
        })),
        _ => Arc::clone(block),
    }
}

fn plain_item(item: &Arc<MdListItem>) -> Arc<MdListItem> {
    let mut children = plain_lists(&item.children);
    if let Some(checked) = item.checked {
        let boxed = if checked { "[x]" } else { "[ ]" };
        match children.first().map(|first| &**first) {
            Some(MdBlock::Paragraph(inlines)) => {
                let mut text = vec![MdInline::text(&format!("{boxed} "))];
                text.extend(inlines.iter().cloned());
                children[0] = Arc::new(MdBlock::Paragraph(text));
            }
            _ => children.insert(0, Arc::new(MdBlock::Paragraph(vec![MdInline::text(boxed)]))),
        }
    }
    Arc::new(MdListItem {
        children,
        checked: None,
    })
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

#[cfg(test)]
mod tests {
    use super::{LiveTableSource, Rendering, preferred};

    #[test]
    fn live_table_source_matches_whole_at_each_character_boundary() {
        let source = concat!(
            "Summary.\r\n\r\n",
            "~~~rust\nlet α = 3;\n~~~\n\n",
            "> ~~~markdown\n> | Héader | Value |\n> | ---- | ----- |\n> | one | two |\n> ~~~\n\n",
            "Name | Value\n---- | -----\nthis \\| thing | that\n\n",
            "- ~~~md\n  | Name | Value |\n  | ---- | ----- |\n  | \x60name\x60 | \x60value\x60 |\n  ~~~\n\n",
            "~~~mermaid\ngraph TD\nA --> B\n~~~\n\n",
            "Final paragraph.",
        );
        for tables in [true, false] {
            let rendering = Rendering {
                tables,
                ..Rendering::default()
            };
            let mut prepared = LiveTableSource::default();
            for end in source
                .char_indices()
                .map(|(end, _)| end)
                .chain([source.len()])
            {
                let prefix = &source[..end];
                assert_eq!(
                    prepared.prepare(prefix, tables),
                    preferred(prefix, rendering)
                );
            }
        }
    }

    #[test]
    fn live_table_source_keeps_lookahead_in_the_rewritable_suffix() {
        let mut prepared = LiveTableSource::default();
        let introduction = "Start.\n\n";
        let incomplete_fence = concat!(
            "Start.\n\n",
            "~~~md\n| Name | Value |\n| ---- | ----- |\n| A | B |\n~~~",
        );
        assert_eq!(
            prepared.prepare(incomplete_fence, true),
            preferred(incomplete_fence, Rendering::default())
        );
        assert_eq!(prepared.stable_source, introduction);
        let changed_close = [incomplete_fence, " and more\n"].concat();
        assert_eq!(
            prepared.prepare(&changed_close, true),
            preferred(&changed_close, Rendering::default())
        );
        assert_eq!(prepared.stable_source, introduction);

        let table = "Start.\n\nA | B\n--- | ---\none | two\n";
        let prefs = Rendering {
            tables: false,
            ..Rendering::default()
        };
        assert_eq!(prepared.prepare(table, false), preferred(table, prefs));
        assert_eq!(prepared.stable_source, introduction);
        let finished = [table, "Done.\n"].concat();
        assert_eq!(
            prepared.prepare(&finished, false),
            preferred(&finished, prefs)
        );
        assert_eq!(prepared.stable_source, finished);
        let longer_table = [table, "new | \x60\x60\x60\x60\n"].concat();
        assert_eq!(
            prepared.prepare(&longer_table, false),
            preferred(&longer_table, prefs)
        );
        assert_eq!(prepared.stable_source, introduction);
        assert_eq!(
            prepared.prepare(table, true),
            preferred(table, Rendering::default())
        );
        // With tables disabled, a prefix without any pipe can already contain an
        // open code fence. A later pipe row still belongs to it.
        let code = [introduction, "~~~rust\nplain code\n"].concat();
        assert_eq!(prepared.prepare(&code, false), preferred(&code, prefs));
        assert_eq!(prepared.stable_source, introduction);
        let code_with_pipes = [code.as_str(), "A | B\n--- | ---\n~~~\n"].concat();
        assert_eq!(
            prepared.prepare(&code_with_pipes, false),
            preferred(&code_with_pipes, prefs)
        );
        assert_eq!(prepared.prepare("", false), "");
    }
}
