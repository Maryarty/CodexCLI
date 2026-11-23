//! Follow-up directives of an assistant message (oracle
//! markdown_render/inline_directives.rs): `:codex-followup[label]{prompt="…"}`
//! reads as its label, the prompt never drawn. Pure text logic; where the
//! scan puts a directive is the source's offsets, the view decides what the
//! Markdown around it (code, HTML, links) leaves literal.

use std::borrow::Cow;
use std::ops::Range;

use super::citations::{Directive, directives};

/// One `:codex-followup[label]{…}` directive of a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Followup {
    /// The directive's bytes in the source, colons to closing brace.
    pub range: Range<usize>,
    /// The text between the brackets: Markdown, never blank.
    pub label: String,
}

/// The follow-ups of `input` in source order (see `citations::citations` for
/// what is and is not a directive).
pub fn followups(input: &str) -> Vec<Followup> {
    directives(input)
        .into_iter()
        .filter_map(|found| match found {
            Directive::Followup(followup) => Some(followup),
            Directive::Citation(_) => None,
        })
        .collect()
}

/// `input` with every follow-up replaced by its label and the Markdown
/// otherwise kept as written (oracle `followup_labels`: what a copy holds).
pub fn labels(input: &str) -> Cow<'_, str> {
    let found = followups(input);
    if found.is_empty() {
        return Cow::Borrowed(input);
    }
    let mut copied = String::with_capacity(input.len());
    let mut at = 0;
    for followup in &found {
        copied.push_str(&input[at..followup.range.start]);
        copied.push_str(&followup.label);
        at = followup.range.end;
    }
    copied.push_str(&input[at..]);
    Cow::Owned(copied)
}
