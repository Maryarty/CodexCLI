//! The prompt and its visible attachments, shared by history and live events.

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

impl UserPrompt {
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
