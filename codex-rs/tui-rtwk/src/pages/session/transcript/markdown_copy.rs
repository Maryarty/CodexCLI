//! What a selection of a Markdown message copies besides its text (oracle `markdown_copy.rs`,
//! `transcript_view/selection.rs` `copy_selected_text_with`): the selected words with the
//! semantic formatting they had, as balanced Markdown, which the host renders to the HTML flavour
//! of the clipboard (`CopyFormat::MarkdownSelection`). The plain flavour is the text the area
//! selected, unchanged. A selection that is only code, or only a code span, offers no Markdown:
//! it stays literal. Rich selections keep the selected words and their emphasis, strong, strike,
//! code and links, the list markers and quote marks of the blocks, a heading's `#`s, and fence a
//! code block that is selected with prose; they do not reproduce the whole response's source
//! spelling (`/copy` does).
//!
//! The oracle annotates each rendered line with [`Line`]; here each [`Node`] (one `Text` of the
//! `MarkdownView`, `markdown_text`) carries it, and a selection is addressed as the nodes it spans
//! and a byte range of each, which [`locate`] finds from the text the area copied and the offsets
//! it tells (`SelectionAreaController::start`/`end`: the area gives the render object of a `Text`
//! and no more, so the node is found by its text). A selection that runs over more than one
//! message, or over a table, is left plain.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use rtwk::gestures::SelectionAreaController;

use super::markdown_text::Node;
use crate::widgets::finder::Finder;

/// Bounds a retained inline stack independently of how deeply the source nests.
const MAX_DEPTH: usize = 64;

/// What a run of a node's text is wrapped in, outermost first (a fixed, deduplicated order gives
/// each nested wrapper a distinct delimiter family).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Inline {
    /// One link around its formatted label, not a destination per style.
    Link(Arc<str>),
    Delimiter(&'static str),
    Code,
    /// Balances a link whose label has no copied wrapper (a destination that is not the web's).
    Ignored,
}

/// The semantic annotations of one node, in byte offsets of the node's text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    /// What the node's first line starts with where it is nested: the markers of the list items
    /// it opens and the `> ` of the quotes around it, the spaces of the items it continues.
    pub prefix: String,
    /// What each later line of the node starts with.
    pub continuation: String,
    /// The prefix with the marker of the item the node is in, for a selection that starts in the
    /// item's later paragraph.
    pub item_prefix: String,
    /// A code block: copied as it is, fenced when selected with prose.
    pub code: bool,
    /// A table cell: no Markdown is written for it yet.
    pub table: bool,
    /// How many bytes of the node's start are the `#`s of a heading, copied as they are.
    pub heading: usize,
    /// The offsets of the line breaks that are hard breaks.
    pub hard_breaks: Vec<usize>,
    runs: Vec<(Range<usize>, Vec<Inline>)>,
    literal: Vec<Range<usize>>,
}

impl Line {
    /// A code block.
    pub fn code() -> Line {
        Line {
            code: true,
            ..Line::default()
        }
    }

    /// A table cell.
    pub fn cell() -> Line {
        Line {
            table: true,
            ..Line::default()
        }
    }

    /// The next `length` bytes of the node's text are wrapped in `inline`.
    pub fn push(&mut self, length: usize, inline: &[Inline]) {
        if length == 0 {
            return;
        }
        let inline = &inline[..inline.len().min(MAX_DEPTH)];
        let literal = inline.iter().any(|mark| matches!(mark, Inline::Code));
        let mut inline: Vec<Inline> = inline
            .iter()
            .filter(|mark| !matches!(mark, Inline::Ignored))
            .cloned()
            .collect();
        inline.sort();
        inline.dedup();
        let start = self.runs.last().map_or(0, |(range, _)| range.end);
        // literal spans stay apart so they never split a prose run's escaping
        if literal {
            if let Some(range) = self.literal.last_mut()
                && range.end == start
            {
                range.end += length;
            } else {
                self.literal.push(start..start + length);
            }
        }
        if let Some((range, previous)) = self.runs.last_mut()
            && previous == &inline
        {
            range.end += length;
        } else {
            self.runs.push((start..start + length, inline));
        }
    }

    fn is_literal(&self, range: &Range<usize>) -> bool {
        !range.is_empty()
            && self
                .literal
                .iter()
                .any(|literal| literal.start <= range.start && range.end <= literal.end)
    }

    /// The Markdown of `range` of `text`, its later lines starting with `continuation`.
    fn render(&self, text: &str, range: Range<usize>, depth: usize, continuation: &str) -> String {
        let mut out = String::new();
        let first = self.runs.partition_point(|(run, _)| run.end <= range.start);
        let mut runs = self.runs[first..]
            .iter()
            .take_while(|(run, _)| run.start < range.end)
            .peekable();
        while let Some((run, inline)) = runs.next() {
            let start = run.start.max(range.start);
            let mut end = run.end.min(range.end);
            if start >= end {
                continue;
            }
            let mark = inline.get(depth);
            while let Some((next, inline)) = runs.peek()
                && next.start == end
                && inline.get(depth) == mark
                && end < range.end
            {
                end = next.end.min(range.end);
                runs.next();
            }
            match mark {
                Some(Inline::Code) => {
                    let content = &text[start..end];
                    let fence = fence(content, 1);
                    let padded = (content.starts_with(['`', ' ']) || content.ends_with(['`', ' ']))
                        && !content.chars().all(|ch| ch == ' ');
                    let padding = if padded { " " } else { "" };
                    append(
                        &mut out,
                        &format!("{fence}{padding}{content}{padding}{fence}"),
                    );
                }
                Some(mark) => {
                    let body = self.render(text, start..end, depth + 1, continuation);
                    let trimmed = body.trim();
                    if trimmed.is_empty() {
                        out.push_str(&body);
                        continue;
                    }
                    out.push_str(&body[..body.len() - body.trim_start().len()]);
                    match mark {
                        Inline::Delimiter(delimiter) => {
                            append(&mut out, &format!("{delimiter}{trimmed}{delimiter}"));
                        }
                        Inline::Link(destination) => {
                            let destination = destination
                                .replace('\\', "\\\\")
                                .replace('&', "&amp;")
                                .replace('<', "%3C")
                                .replace('>', "%3E")
                                .replace('|', "%7C")
                                .replace('\n', "%0A")
                                .replace('\r', "%0D");
                            append(&mut out, &format!("[{trimmed}](<{destination}>)"));
                        }
                        Inline::Code | Inline::Ignored => {}
                    }
                    out.push_str(&body[body.trim_end().len()..]);
                }
                None => {
                    let marks = self.heading.min(end).max(start);
                    out.push_str(&text[start..marks]);
                    append(&mut out, &self.prose(text, marks..end, continuation));
                }
            }
        }
        out
    }

    /// Plain words as Markdown: escaped, a hard break kept as one, a later line indented.
    fn prose(&self, text: &str, range: Range<usize>, continuation: &str) -> String {
        let mut out = String::new();
        let mut at = range.start;
        for (offset, _) in text[range.clone()].match_indices('\n') {
            let line_break = range.start + offset;
            out.push_str(&escape(&text[at..line_break]));
            out.push_str(if self.hard_breaks.contains(&line_break) {
                "  \n"
            } else {
                "\n"
            });
            out.push_str(continuation);
            at = line_break + 1;
        }
        out.push_str(&escape(&text[at..range.end]));
        out
    }
}

/// One node's part in a selection.
#[derive(Clone, Copy)]
pub struct Selected<'a> {
    pub node: &'a Node,
    /// The selected bytes of the node's text.
    pub range: (usize, usize),
}

impl Selected<'_> {
    fn bytes(&self) -> Range<usize> {
        self.range.0..self.range.1
    }

    fn empty(&self) -> bool {
        self.range.0 >= self.range.1
    }
}

/// Where `text`, as a `SelectionArea` copied it, lies in `nodes`: the nodes it spans, whole but
/// for the first and the last, as `(node, bytes)`. `from` and `to` are the offsets the area tells
/// for the first and the last node, when it tells them; the first place that fits is taken (the
/// same words twice in a message with the same offsets look the same to the reader).
pub fn locate(
    nodes: &[Node],
    text: &str,
    from: Option<usize>,
    to: Option<usize>,
) -> Option<Vec<(usize, Range<usize>)>> {
    if text.is_empty() {
        return None;
    }
    for (first, node) in nodes.iter().enumerate() {
        let starts: Vec<usize> = match from {
            Some(at) if at <= node.text.len() && node.text.is_char_boundary(at) => vec![at],
            Some(_) => continue,
            None => node
                .text
                .char_indices()
                .map(|(at, _)| at)
                .chain([node.text.len()])
                .collect(),
        };
        for start in starts {
            let suffix = &node.text[start..];
            // all of it in this node
            if suffix.starts_with(text) {
                let end = start + text.len();
                if to.is_none_or(|to| to == end) {
                    return Some(vec![(first, start..end)]);
                }
            }
            if suffix.is_empty() || !text.starts_with(suffix) {
                continue;
            }
            if let Some(found) = follow(nodes, first, start, &text[suffix.len()..], to) {
                return Some(found);
            }
        }
    }
    None
}

/// The nodes after `first` that `rest` is the text of.
fn follow(
    nodes: &[Node],
    first: usize,
    start: usize,
    mut rest: &str,
    to: Option<usize>,
) -> Option<Vec<(usize, Range<usize>)>> {
    let mut found = vec![(first, start..nodes[first].text.len())];
    for (at, node) in nodes.iter().enumerate().skip(first + 1) {
        rest = rest.strip_prefix(node.before)?;
        if let Some(after) = rest.strip_prefix(node.text.as_str()) {
            found.push((at, 0..node.text.len()));
            if after.is_empty() {
                return to.is_none_or(|to| to == node.text.len()).then_some(found);
            }
            rest = after;
        } else if node.text.starts_with(rest) {
            found.push((at, 0..rest.len()));
            return to.is_none_or(|to| to == rest.len()).then_some(found);
        } else {
            return None;
        }
    }
    None
}

/// The Markdown of a selection of `nodes` that reads `text`: none where it is not a rich one (only
/// code, only a code span, a table) or `text` is not in the nodes.
pub fn serialize(
    nodes: &[Node],
    text: &str,
    from: Option<usize>,
    to: Option<usize>,
) -> Option<String> {
    let found = locate(nodes, text, from, to)?;
    let lines: Vec<Selected<'_>> = found
        .iter()
        .map(|(at, range)| Selected {
            node: &nodes[*at],
            range: (range.start, range.end),
        })
        .collect();
    selection(&lines)
}

/// The Markdown of the nodes' selected parts: the oracle's `render_selection`, which is a line
/// to it where a node is a `Text` here.
pub fn selection(lines: &[Selected<'_>]) -> Option<String> {
    // a selection inside one code span is the words it selects
    if let [line] = lines
        && line.node.copy.is_literal(&line.bytes())
    {
        return None;
    }
    let content: Vec<&Selected<'_>> = lines.iter().filter(|line| !line.empty()).collect();
    if content.iter().any(|line| line.node.copy.table) {
        return None;
    }
    // quote-only selections omit the quote marks; a task list still has its checkboxes to keep
    let quote_only = !content.is_empty()
        && content
            .iter()
            .all(|line| line.node.copy.prefix.contains("> "));
    if quote_only
        && !lines.iter().any(|line| {
            let item = &line.node.copy.item_prefix;
            item.contains("[x] ") || item.contains("[ ] ")
        })
    {
        return None;
    }
    if content.iter().all(|line| line.node.copy.code) {
        return None;
    }
    let unquote = |prefix: &str| {
        if quote_only {
            prefix.replace("> ", "")
        } else {
            prefix.to_owned()
        }
    };
    let mut out = String::new();
    let mut first = true;
    let mut indentation: Option<usize> = None;
    // the continuation of the node before, as it is written
    let mut before = String::new();
    for (at, line) in lines.iter().enumerate() {
        let copy = &line.node.copy;
        let more = at + 1 < lines.len();
        let continuation = unquote(&copy.continuation);
        let held = dedent(&continuation, indentation.unwrap_or(0));
        if !first {
            // a blank line keeps the quotes both sides are in, and no more
            let common: String = held
                .chars()
                .zip(before.chars())
                .take_while(|(left, right)| left == right)
                .map(|(ch, _)| ch)
                .collect();
            out.push_str(&separator(line.node.before, common.trim_end()));
        }
        before = held;
        let prefix = unquote(if indentation.is_none() && !line.empty() && more {
            &copy.item_prefix
        } else {
            &copy.prefix
        });
        if !line.empty() {
            let spaces: usize = prefix
                .split("> ")
                .map(|part| part.len() - part.trim_start_matches(' ').len())
                .sum();
            indentation = Some(indentation.map_or(spaces, |previous| previous.min(spaces)));
        }
        let prefix = dedent(&prefix, indentation.unwrap_or(0));
        let continuation = dedent(&continuation, indentation.unwrap_or(0));
        if line.empty() {
            if line.node.text.is_empty() {
                out.push_str(prefix.trim_end());
            }
            first = false;
            continue;
        }
        let text = line.node.text.as_str();
        if copy.code {
            let code = &text[line.bytes()];
            let fence = fence(code, 3);
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            // a checkbox label comes before the fence on its own line
            if prefix.ends_with("[ ] ") || prefix.ends_with("[x] ") {
                out.push_str(prefix.trim_end());
                out.push('\n');
                out.push_str(&continuation);
            } else {
                out.push_str(&prefix);
            }
            out.push_str(&format!("{fence}\n"));
            for row in code.split_inclusive('\n') {
                out.push_str(&continuation);
                out.push_str(row);
            }
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&continuation);
            out.push_str(&fence);
        } else {
            if line.range.0 == 0 || first && more {
                out.push_str(&prefix);
            }
            out.push_str(&copy.render(text, line.bytes(), 0, &continuation));
        }
        first = false;
    }
    out.retain(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'));
    Some(out)
}

/// What lies between two selected nodes, `blank` being what a blank line between them holds (the
/// `>` of a quote, so that it stays one).
fn separator(before: &str, blank: &str) -> String {
    if before == "\n\n" {
        format!("\n{blank}\n")
    } else {
        before.to_owned()
    }
}

/// Removes the padding of the list items that are not selected, without the quote marks.
fn dedent(prefix: &str, mut spaces: usize) -> String {
    prefix
        .split_inclusive("> ")
        .map(|part| {
            let remove = spaces.min(part.len() - part.trim_start_matches(' ').len());
            spaces -= remove;
            &part[remove..]
        })
        .collect()
}

/// The backticks that fence `text` (at least `minimum`, one more than its longest run).
fn fence(text: &str, minimum: usize) -> String {
    let longest = text.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
    std::iter::repeat_n('`', longest.saturating_add(1).max(minimum)).collect()
}

/// Entity decoding and omitted link wrappers can turn a delimiter's punctuation neighbour into a
/// word character: protect that boundary without changing the selected text or adding space.
fn append(out: &mut String, text: &str) {
    let word = |ch: char| !ch.is_whitespace() && !ch.is_ascii_punctuation();
    let left = text.starts_with('_')
        || text.starts_with(['*', '~'])
            && text
                .chars()
                .next()
                .and_then(|delimiter| text.trim_start_matches(delimiter).chars().next())
                .is_some_and(|ch| !ch.is_alphanumeric());
    if left && let Some(ch) = out.chars().last().filter(|ch| word(*ch)) {
        out.pop();
        out.push_str(&format!("&#{};", u32::from(ch)));
    }
    let right = out.ends_with('_')
        || out.ends_with(['*', '~'])
            && out
                .chars()
                .next_back()
                .and_then(|delimiter| out.trim_end_matches(delimiter).chars().next_back())
                .is_some_and(|ch| !ch.is_alphanumeric());
    if right && let Some(ch) = text.chars().next().filter(|ch| word(*ch)) {
        out.push_str(&format!("&#{};", u32::from(ch)));
        out.push_str(&text[ch.len_utf8()..]);
    } else {
        out.push_str(text);
    }
}

/// Plain words with the characters Markdown would read as markup escaped.
fn escape(text: &str) -> String {
    let mut out = String::new();
    let mut whitespace_prefix = true;
    let mut numeric_prefix = false;
    let mut number_has_trailing_space = false;
    for ch in text.chars() {
        let block_marker =
            "=-+".contains(ch) && whitespace_prefix || matches!(ch, '.' | ')') && numeric_prefix;
        if "\\`*_[]<>~#|&!".contains(ch) || block_marker {
            out.push('\\');
        }
        out.push(ch);
        if ch.is_whitespace() {
            number_has_trailing_space |= numeric_prefix;
        } else {
            numeric_prefix = ch.is_ascii_digit()
                && (whitespace_prefix || numeric_prefix && !number_has_trailing_space);
            whitespace_prefix = false;
        }
    }
    out
}

/// Whether `destination` is one the web serves (`http` or `https` with a host): the links whose
/// label is copied with its destination.
pub fn web(destination: &str) -> bool {
    destination
        .strip_prefix("https://")
        .or_else(|| destination.strip_prefix("http://"))
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .is_some_and(|host| !host.is_empty() && !host.chars().any(char::is_control))
}

// ------------------------------------------------------------------ the thread's marks --

/// The Markdown bodies of the transcript's rows and the selection area that reads them: what the
/// thread's copies ask of a selection that no mode made. Held with the thread's finder
/// ([`of`]); the pane fills it while something is selected.
#[derive(Clone, Default)]
pub struct Marks(Rc<Shared>);

#[derive(Default)]
struct Shared {
    /// The nodes of the Markdown body of each row, none for a row that has none.
    rows: RefCell<Vec<Rc<[Node]>>>,
    area: RefCell<Option<SelectionAreaController>>,
}

impl PartialEq for Marks {
    fn eq(&self, other: &Marks) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The marks of the thread whose finder this is (made on the first ask).
pub fn of(finder: &Finder) -> Marks {
    finder.held(Marks::default)
}

impl Marks {
    /// The area the rows are selected in.
    pub fn attach(&self, area: SelectionAreaController) {
        *self.0.area.borrow_mut() = Some(area);
    }

    /// The Markdown bodies of the rows.
    pub fn fill(&self, rows: Vec<Rc<[Node]>>) {
        *self.0.rows.borrow_mut() = rows;
    }

    /// The Markdown of what the area has selected, if it is a rich selection of one message and
    /// says more than the text does: the oracle also offers words with no formatting as Markdown
    /// (an HTML paragraph of them); here they stay plain text, which pastes the same.
    pub fn selected(&self, text: &str) -> Option<String> {
        let area = self.0.area.borrow().clone()?;
        if area.text() != text {
            return None;
        }
        let from = area.start().map(|point| point.offset);
        let to = area.end().map(|point| point.offset);
        self.0
            .rows
            .borrow()
            .iter()
            .find_map(|nodes| serialize(nodes, text, from, to))
            .filter(|source| source != text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn escape_protects_markup_and_block_markers() {
        assert_eq!(escape("a_b *c* [d]"), "a\\_b \\*c\\* \\[d\\]");
        assert_eq!(escape("1. x"), "1\\. x");
        assert_eq!(escape("- x"), "\\- x");
    }

    #[test]
    fn a_fence_is_longer_than_the_backticks_inside() {
        assert_eq!(fence("a", 3), "```");
        assert_eq!(fence("a ``` b", 3), "````");
        assert_eq!(fence("a ` b", 1), "``");
    }

    #[test]
    fn the_web_serves_http_with_a_host() {
        assert!(web("https://example.com/a"));
        assert!(web("http://example.com"));
        assert!(!web("file:///tmp/a"));
        assert!(!web("https:///a"));
        assert!(!web("mailto:a@b.c"));
    }

    #[test]
    fn a_prefix_loses_the_padding_of_the_unselected_items_not_the_quote() {
        assert_eq!(dedent("  > - ", 2), "> - ");
        assert_eq!(dedent(">   ", 2), "> ");
        assert_eq!(dedent("> ", 2), "> ");
    }
}
