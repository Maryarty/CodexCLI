//! What the title suggestion of the name prompt asks the model and reads of its answer (oracle
//! `app/thread_title.rs`): the schema, the prompt built from the recent messages of the
//! conversation, and the normalization of the generated title. Pure: the request itself is the
//! structured turn of `AppCx::structured_turn`.

use serde_json::{Value, json};

/// The longest title taken (`THREAD_MAX_CHARS`).
pub const MAX_CHARS: usize = 36;
/// The model the oracle asks for titles when the account and the catalog have it.
pub const MODEL: &str = "gpt-5.6-luna";
/// The longest prompt, in bytes (`THREAD_TITLE_PROMPT_MAX_BYTES`).
const PROMPT_MAX_BYTES: usize = 960;
/// How many of the latest messages the prompt holds (`THREAD_TITLE_RECENT_MESSAGES`).
const RECENT_MESSAGES: usize = 8;

/// One message of the conversation, as the title reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Said<'a> {
    /// What the user typed.
    User(&'a str),
    /// What the assistant said; commentary is not a substantive message.
    Assistant { text: &'a str, commentary: bool },
}

/// `thread_title_output_schema`: one nonempty title within the display limit.
pub fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_CHARS,
            },
        },
        "required": ["title"],
        "additionalProperties": false,
    })
}

fn instructions() -> String {
    format!(
        "Generate a concise, single-line task title of at most \
  {MAX_CHARS} characters and under five words where possible. \
  Start with an imperative verb. Capitalize only the first word unless the \
  user's language, proper nouns, acronyms, or code terms require otherwise. \
  Preserve ticket references exactly. Write in the user's language. \
  Do not use quotes, markdown, or trailing punctuation. \
  Do not answer the request."
    )
}

/// `recent_conversation_messages`: the latest substantive messages in order, escaped so that
/// their markup is not trusted, within the prompt's byte budget; none when the conversation has
/// nothing to name it by.
pub fn recent_conversation<'a>(items: impl DoubleEndedIterator<Item = Said<'a>>) -> Option<String> {
    let mut messages = items
        .rev()
        .filter_map(|said| match said {
            Said::User(text) => (!text.trim().is_empty()).then_some(("user", text)),
            Said::Assistant { text, commentary } => {
                (!commentary && !text.trim().is_empty()).then_some(("assistant", text))
            }
        })
        .take(RECENT_MESSAGES)
        .map(|(role, text)| {
            let escaped = text
                .trim()
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            (role, escaped)
        })
        .collect::<Vec<_>>();
    if messages.is_empty() {
        return None;
    }
    messages.reverse();

    let conversation_bytes = PROMPT_MAX_BYTES.saturating_sub(suggestion_prompt("").len());
    let markup_bytes = "<conversation>\n".len()
        + "\n</conversation>".len()
        + messages.len().saturating_sub(1)
        + messages
            .iter()
            .map(|(role, _)| "<message role=\"\"></message>".len() + role.len())
            .sum::<usize>();
    let message_bytes = conversation_bytes.saturating_sub(markup_bytes) / messages.len();
    let content_bytes = messages.iter().map(|(_, text)| text.len()).sum::<usize>();
    let should_truncate = content_bytes > conversation_bytes.saturating_sub(markup_bytes);
    let messages = messages
        .into_iter()
        .map(|(role, mut text)| {
            if should_truncate && text.len() > message_bytes {
                let mut end = message_bytes;
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                // an entity cut in two is dropped whole
                if let Some(entity_start) = text[..end].rfind('&')
                    && !text[entity_start..end].contains(';')
                {
                    end = entity_start;
                }
                text.truncate(end);
            }
            ["<message role=\"", role, "\">", &text, "</message>"].concat()
        })
        .collect::<Vec<_>>();
    Some(format!(
        "<conversation>\n{}\n</conversation>",
        messages.join("\n")
    ))
}

/// `recent_conversation_thread_title_prompt`: the whole prompt of a suggestion, bounded, with
/// complete Unicode characters.
pub fn suggestion_prompt(conversation: &str) -> String {
    let instructions = instructions();
    let prefix = format!(
        "{instructions}\n\
Prioritize the current task and latest substantive user request.\n\n\
Recent conversation messages:\n"
    );
    let conversation = conversation.trim();
    let remaining_bytes = PROMPT_MAX_BYTES.saturating_sub(prefix.len());
    let mut start = conversation.len().saturating_sub(remaining_bytes);
    while !conversation.is_char_boundary(start) {
        start += 1;
    }
    format!("{prefix}{}", &conversation[start..])
}

/// `parse_thread_title`: the generated title, trimmed of quotes, whitespace runs and a trailing
/// `.?!`, cut to the display limit; none when the answer is not the object the schema asks for or
/// nothing is left.
pub fn parse(response: &str) -> Option<String> {
    if !response.trim_start().starts_with('{') {
        return None;
    }
    let Value::Object(fields) = serde_json::from_str(response).ok()? else {
        return None;
    };
    if fields.len() != 1 {
        return None;
    }
    let title = fields.get("title")?.as_str()?;
    let normalized = title
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\'' | '`' | '“' | '”' | '‘' | '’'))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', '?', '!'])
        .trim_end()
        .to_owned();
    if normalized.is_empty() {
        return None;
    }
    Some(normalized.chars().take(MAX_CHARS).collect())
}
