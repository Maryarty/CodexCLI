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
    pub elements: Cow<'a, [Range<usize>]>,
}

/// The marker the prompt of a request with context carries before the user's own words
/// (`PROMPT_REQUEST_BEGIN`).
const PROMPT_REQUEST_BEGIN: &str = "## My request for Codex:";
const IDE_CONTEXT_BEGIN: &str = "# Context from my IDE setup:\n";
const IDE_CONTEXT_LENGTH: &str = "<!-- Codex IDE context bytes: ";
const IDE_REQUEST: &str = "\n## My request for Codex:\n";

/// Apply IDE context to the wire items after they have their attachment and
/// mention ranges. The caller retains the original user input.
pub(crate) fn prepend_ide_context(input: &mut Vec<UserInput>, context: &str) {
    let body = context.strip_prefix(IDE_CONTEXT_BEGIN).unwrap_or(context);
    // Delimit the context by length so an identical heading in selected code
    // or in the user's request remains part of its source.
    let prefix = format!(
        "{IDE_CONTEXT_BEGIN}{IDE_CONTEXT_LENGTH}{} -->\n{body}{IDE_REQUEST}",
        body.len()
    );
    if let Some(UserInput::Text {
        text,
        text_elements,
    }) = input
        .iter_mut()
        .find(|item| matches!(item, UserInput::Text { .. }))
    {
        for element in text_elements {
            element.byte_range.start += prefix.len();
            element.byte_range.end += prefix.len();
        }
        text.insert_str(0, &prefix);
    } else {
        input.insert(
            0,
            UserInput::Text {
                text: prefix,
                text_elements: Vec::new(),
            },
        );
    }
}

/// What the user asked in a message: its text inputs joined by a space, each without the context
/// before the request marker (`extract_prompt_request_with_offset`); what a title is made from.
pub fn request_text(content: &[UserInput]) -> String {
    content
        .iter()
        .filter_map(|input| match input {
            UserInput::Text { text, .. } => {
                let (request, prefix) = UserPrompt::request_part(text);
                if prefix > 0 && !request.starts_with("## Referenced chats with Codex:\n") {
                    return Some(request.trim());
                }
                Some(
                    request
                        .rsplit_once(PROMPT_REQUEST_BEGIN)
                        .map_or(request, |(_, request)| request.trim()),
                )
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl UserPrompt {
    /// The user text and its byte offset in a wire text item. A recognized IDE
    /// prefix belongs to the model input and stays out of history and drafts.
    pub(crate) fn request_part(text: &str) -> (&str, usize) {
        let Some(body) = text.strip_prefix(IDE_CONTEXT_BEGIN) else {
            return (text, 0);
        };
        let request = if let Some(framed) = body.strip_prefix(IDE_CONTEXT_LENGTH) {
            framed
                .split_once(" -->\n")
                .and_then(|(length, body)| body.get(length.parse::<usize>().ok()?..))
                .and_then(|suffix| suffix.strip_prefix(IDE_REQUEST))
        } else {
            // Older sessions can come from the oracle's unframed IDE input.
            body.rsplit_once(IDE_REQUEST).map(|(_, request)| request)
        };
        request.map_or((text, 0), |request| (request, text.len() - request.len()))
    }

    /// The prompt as history draws it (`sanitize_user_text`, oracle
    /// history_cell/messages.rs): CSI sequences and control characters
    /// removed, tabs and newlines kept; a text that changed loses its inline
    /// elements, their byte ranges no longer fit it. The text itself, which
    /// the model has and steers are matched by, is not touched.
    pub fn shown(&self) -> Shown<'_> {
        // an answer to an async question shows as the question quoted over the answer
        // (`display_replies`), whoever sent it
        if let Some(text) = crate::question_reply::display_text(&self.text) {
            return Shown {
                text: Cow::Owned(without_controls(&text).into_owned()),
                elements: Cow::Borrowed(&[]),
            };
        }
        let text = without_controls(&self.text);
        let elements = if matches!(text, Cow::Borrowed(_)) {
            self.elements.as_slice()
        } else {
            &[]
        };
        // a task mention was sent as its link: the history shows the `@title` the user typed
        // (`decode_task_links`)
        if !elements.is_empty() && text.contains("thread://") {
            let (decoded, moved) = crate::kit::mentions::decode_task_links(&text, elements);
            return Shown {
                text: Cow::Owned(decoded),
                elements: Cow::Owned(moved),
            };
        }
        Shown {
            text,
            elements: Cow::Borrowed(elements),
        }
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
                    let (text, prefix) = Self::request_part(text);
                    if prefix > 0 && text.is_empty() {
                        continue;
                    }
                    if text_parts > 0 {
                        prompt.text.push('\n');
                    }
                    text_parts += 1;
                    let offset = prompt.text.len();
                    prompt.text.push_str(text);
                    prompt
                        .elements
                        .extend(text_elements.iter().filter_map(|element| {
                            let range = element.byte_range.start.checked_sub(prefix)?
                                ..element.byte_range.end.checked_sub(prefix)?;
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

#[cfg(test)]
mod tests {
    use codex_app_server_protocol::{ByteRange, TextElement};

    use super::*;

    fn local_image() -> UserInput {
        UserInput::LocalImage {
            path: "/workspace/image.png".into(),
            detail: None,
        }
    }

    #[test]
    fn ide_prefix_keeps_original_unicode_delimiters_and_image_ranges() {
        let request = "Prüfe [Image #1]\n## My request for Codex:\nzeig den Marker";
        let image = request.find("[Image #1]").unwrap_or_default();
        let element = TextElement::new(
            ByteRange {
                start: image,
                end: image + "[Image #1]".len(),
            },
            Some("[Image #1]".to_owned()),
        );
        let mut input = vec![
            local_image(),
            UserInput::Text {
                text: request.to_owned(),
                text_elements: vec![element.clone()],
            },
        ];
        prepend_ide_context(
            &mut input,
            "# Context from my IDE setup:\nfile: 雪.rs\n## My request for Codex:\nselected",
        );
        assert_eq!(input[0], local_image());
        let UserInput::Text {
            text,
            text_elements,
        } = &input[1]
        else {
            panic!("the text item stays after its image");
        };
        assert!(text.contains("selected"));
        assert!(text.ends_with(request));
        let shifted = &text_elements[0];
        assert_eq!(shifted.placeholder(), element.placeholder());
        assert_eq!(
            &text[shifted.byte_range.start..shifted.byte_range.end],
            "[Image #1]"
        );
        let shown = UserPrompt::from_input(&input);
        assert_eq!(shown.text, request);
        assert_eq!(shown.elements, [image..image + "[Image #1]".len()]);
        assert!(shown.images.is_empty());
        assert_eq!(request_text(&input), request);
    }

    #[test]
    fn image_only_replay_keeps_the_image_without_the_ide_header() {
        let mut input = vec![local_image()];
        prepend_ide_context(&mut input, "# Context from my IDE setup:\nselection");
        assert_eq!(input[1], local_image());
        let shown = UserPrompt::from_input(&input);
        assert!(shown.text.is_empty());
        assert!(shown.elements.is_empty());
        assert_eq!(shown.images, [1]);
    }

    #[test]
    fn legacy_ide_prefix_is_read_without_hiding_malformed_framed_text() {
        let old = "# Context from my IDE setup:\nold selection\n## My request for Codex:\nhello";
        assert_eq!(UserPrompt::request_part(old).0, "hello");

        let malformed = "# Context from my IDE setup:\n<!-- Codex IDE context bytes: 10 -->\n短\n## My request for Codex:\nhello";
        assert_eq!(UserPrompt::request_part(malformed), (malformed, 0));
    }
}

/// `text` without CSI sequences and control characters but tabs and
/// newlines. A lone ESC goes on its own; a CSI sequence that never ends
/// takes the rest of the text with it.
pub(crate) fn without_controls(text: &str) -> Cow<'_, str> {
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
