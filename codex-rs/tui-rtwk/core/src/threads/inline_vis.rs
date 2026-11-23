//! Inline visualization directives of an assistant message (oracle
//! `inline_visualization.rs`, `rewrite_inline_visualizations` with no
//! context): a line that is a `::codex-inline-vis{…}` directive or a content
//! reference reads as an italic note (the caller's words: the visualization is
//! unavailable here); a line that starts one and never closes is dropped (a streaming
//! preview does not show a half-written directive). Code blocks keep their
//! text.
//!
//! The oracle's context (a thread directory under the codex home, the
//! sandbox's writable roots, the viewer HTML it writes and a `file://` link
//! to it) is host work: this is the rewrite for a screen that has none, and
//! it is what a directive reads as wherever the file cannot be linked.

use std::borrow::Cow;

const DIRECTIVE_PREFIX: &str = "::codex-inline-vis{";
const CONTENT_REFERENCE_PREFIX: &str = "\u{e200}visualize\u{e202}";
const CONTENT_REFERENCE_SUFFIX: char = '\u{e201}';

/// Whether `markdown` holds a directive or a content reference.
pub fn contains(markdown: &str) -> bool {
    markdown.contains(DIRECTIVE_PREFIX) || markdown.contains(CONTENT_REFERENCE_PREFIX)
}

/// `markdown` with its directive lines rewritten (see the module); `note` is the
/// words of the unavailable line, set in italics here.
pub fn rewrite<'a>(markdown: &'a str, note: &str) -> Cow<'a, str> {
    if !contains(markdown) {
        return Cow::Borrowed(markdown);
    }
    let mut out = String::with_capacity(markdown.len());
    let mut fence: Option<(u8, usize)> = None;
    let mut previous_blank = true;
    for source_line in markdown.split_inclusive('\n') {
        let (line, newline) = source_line
            .strip_suffix('\n')
            .map_or((source_line, ""), |line| (line, "\n"));
        let trimmed = line.trim();
        let is_code = in_code(line, &mut fence, previous_blank);
        previous_blank = trimmed.is_empty();
        let starts_directive =
            trimmed.starts_with(DIRECTIVE_PREFIX) || trimmed.starts_with(CONTENT_REFERENCE_PREFIX);
        if is_code || !starts_directive {
            out.push_str(line);
        } else if trimmed.ends_with(CONTENT_REFERENCE_SUFFIX)
            || (trimmed.starts_with(DIRECTIVE_PREFIX) && trimmed.ends_with('}'))
        {
            out.push('_');
            out.push_str(note);
            out.push('_');
        }
        out.push_str(newline);
    }
    Cow::Owned(out)
}

/// Whether `line` belongs to a code block: a fenced one (the fence lines
/// too, an unclosed one to the end) or an indented one after a blank line.
fn in_code(line: &str, fence: &mut Option<(u8, usize)>, previous_blank: bool) -> bool {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    let rest = &line[indent..];
    if let Some((marker, length)) = *fence {
        let run = rest.bytes().take_while(|byte| *byte == marker).count();
        if indent < 4 && run >= length && rest[run..].trim().is_empty() {
            *fence = None;
        }
        return true;
    }
    if indent < 4
        && let Some(&marker) = rest.as_bytes().first().filter(|b| matches!(b, b'`' | b'~'))
    {
        let run = rest.bytes().take_while(|byte| *byte == marker).count();
        // A backtick fence's info string holds no backtick.
        if run >= 3 && !(marker == b'`' && rest[run..].contains('`')) {
            *fence = Some((marker, run));
            return true;
        }
    }
    indent >= 4 && previous_blank
}
