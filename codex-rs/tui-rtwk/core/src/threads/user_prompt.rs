//! The prompt and its visible attachments, shared by history and live events.

use std::borrow::Cow;
use std::ops::Range;

use codex_app_server_protocol::UserInput;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserPrompt {
    pub text: String,
    /// Valid, disjoint byte ranges for inline elements already in the text.
    pub elements: Vec<Range<usize>>,
    /// Image numbers that need a separate visible label above the text.
    pub images: Vec<usize>,
}

/// What the transcript shows of a prompt: its text without control
/// sequences, and the inline elements that still fit it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown<'a> {
    pub text: Cow<'a, str>,
    pub elements: &'a [Range<usize>],
}

/// The marker the prompt of a request with context carries before the user's own words
/// (`PROMPT_REQUEST_BEGIN`).
const PROMPT_REQUEST_BEGIN: &str = "## My request for Codex:";

/// What the user asked in a message: its text inputs joined by a space, each without the context
/// before the request marker (`extract_prompt_request_with_offset`); what a title is made from.
pub fn request_text(content: &[UserInput]) -> String {
    content
        .iter()
        .filter_map(|input| match input {
            UserInput::Text { text, .. } => Some(
                text.rsplit_once(PROMPT_REQUEST_BEGIN)
                    .map_or(text.as_str(), |(_, request)| request.trim()),
            ),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl UserPrompt {
    /// The prompt as history draws it (`sanitize_user_text`, oracle
    /// history_cell/messages.rs): CSI sequences and control characters
    /// removed, tabs and newlines kept; a text that changed loses its inline
    /// elements, their byte ranges no longer fit it. The text itself, which
    /// the model has and steers are matched by, is not touched.
    pub fn shown(&self) -> Shown<'_> {
        let text = without_controls(&self.text);
        let elements = if matches!(text, Cow::Borrowed(_)) {
            self.elements.as_slice()
        } else {
            &[]
        };
        Shown { text, elements }
    }

    pub(super) fn from_input(content: &[UserInput]) -> Self {
        let mut prompt = Self::default();
        let mut text_parts = 0;
        let mut image_count = 0;
        for input in content {
            match input {
                UserInput::Text {
                    text,
                    text_elements,
                } => {
                    if text_parts > 0 {
                        prompt.text.push('\n');
                    }
                    text_parts += 1;
                    let offset = prompt.text.len();
                    prompt.text.push_str(text);
                    prompt
                        .elements
                        .extend(text_elements.iter().filter_map(|element| {
                            let range = element.byte_range.start..element.byte_range.end;
                            (!text.get(range.clone())?.is_empty())
                                .then_some(offset + range.start..offset + range.end)
                        }));
                }
                UserInput::Image { .. } | UserInput::LocalImage { .. } => image_count += 1,
                _ => {}
            }
        }
        prompt
            .elements
            .sort_unstable_by_key(|element| element.start);
        let mut end = 0;
        prompt.elements.retain(|element| {
            if element.start < end {
                return false;
            }
            end = element.end;
            true
        });
        let inline_images: std::collections::HashSet<_> = prompt
            .elements
            .iter()
            .filter_map(|range| {
                prompt.text[range.clone()]
                    .strip_prefix("[Image #")?
                    .strip_suffix(']')?
                    .parse::<usize>()
                    .ok()
            })
            .collect();
        prompt.images = (1..=image_count)
            .filter(|image| !inline_images.contains(image))
            .collect();
        prompt
    }
}

/// `text` without CSI sequences and control characters but tabs and
/// newlines. A lone ESC goes on its own; a CSI sequence that never ends
/// takes the rest of the text with it.
fn without_controls(text: &str) -> Cow<'_, str> {
    let hidden = |ch: char| ch == '\x1b' || ch.is_control() && !matches!(ch, '\n' | '\t');
    if !text.contains(hidden) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(hidden) {
        out.push_str(&rest[..at]);
        let after = &rest[at..];
        let sequence = after.strip_prefix("\x1b[");
        rest = match sequence {
            // a CSI sequence ends at its final byte, `@` to `~`
            Some(sequence) => match sequence.find(|ch: char| ('@'..='~').contains(&ch)) {
                Some(end) => &sequence[end + 1..],
                None => return Cow::Owned(out),
            },
            None => &after[after.chars().next().map_or(0, char::len_utf8)..],
        };
    }
    out.push_str(rest);
    Cow::Owned(out)
}
