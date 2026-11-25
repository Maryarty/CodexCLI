//! The reply envelope of an answered async question (oracle `async_question_reply.rs`, context
//! fragment `AnsweredQuestion`): what the editor sends as the user's message, what a desktop client
//! sends for the same question, and the words the transcript and the queue show for it.

use serde::Deserialize;

use crate::ids::ThreadId;

/// What the user decides of a question: the verb of the async-questions feature, kept here so the
/// session page that says it names no feature (`features::verb` takes it as `Into<FeatureIntent>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The answer of the question went out, or the question was skipped.
    Resolve { thread: ThreadId, id: String },
    /// The user is at the questions of `thread` (the editor opened, a key or a paste landed in
    /// it): their countdown stops (`snooze_auto_resolution`).
    Snooze { thread: ThreadId },
    /// The clock reached the moment the countdown changes (`Feature::wake_at`): never said by a
    /// page.
    Tick,
}

const START: &str = "<send_user_message_question_reply>";
const END: &str = "</send_user_message_question_reply>";
/// The bytes of a question the envelope keeps (`AnsweredQuestion::new`).
const QUESTION_BYTES: usize = 512;

/// One answered question of an envelope.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    #[serde(rename = "questionItemId")]
    pub id: String,
    pub question: String,
    pub answer: String,
}

/// The id of question `index` of the message `message` (the desktop's
/// `JSON.stringify([tool name, item id, question index])`).
pub fn question_id(message: &str, index: usize) -> String {
    serde_json::json!(["request_user_input_async", message, index]).to_string()
}

/// `AnsweredQuestion::new(id, title, answer).render()`: the reply as the model takes it. A
/// question id over the budget is not named and the reply is the plain quote.
pub fn render(id: &str, title: &str, answer: &str) -> String {
    let mut end = title.len().min(QUESTION_BYTES);
    while !title.is_char_boundary(end) {
        end -= 1;
    }
    let question = title[..end].replace(['\n', '\r'], " ");
    if id.len() > QUESTION_BYTES {
        return format!("> {question}\n\n{answer}");
    }
    let replies = serde_json::json!([{
        "answer": answer,
        "question": question,
        "questionItemId": id,
    }]);
    format!("{START}\n{replies}\n{END}")
}

/// The replies of `text`, when it is a whole envelope (optionally after the IDE context).
pub fn parse(text: &str) -> Option<Vec<Reply>> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Replies {
        Many(Vec<Reply>),
        One(Reply),
    }
    let text = text.trim();
    let text = if text.starts_with("# Context from my IDE setup:\n") {
        text.rsplit_once("\n## My request for Codex:\n")?.1.trim()
    } else {
        text
    };
    let json = text.strip_prefix(START)?.strip_suffix(END)?;
    let replies = match serde_json::from_str::<Replies>(json).ok()? {
        Replies::Many(replies) => replies,
        Replies::One(reply) => vec![reply],
    };
    (!replies.is_empty()).then_some(replies)
}

/// `display_replies`: each reply as the question quoted, then the answer.
pub fn display_replies(replies: &[Reply]) -> String {
    replies
        .iter()
        .map(|reply| format!("> {}\n\n{}", reply.question, reply.answer))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// What the transcript and the queue show of `text` when it is a reply.
pub fn display_text(text: &str) -> Option<String> {
    Some(display_replies(&parse(text)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_reads_back_and_shows_its_question_on_one_line() {
        let id = question_id("m", 0);
        let text = render(&id, "First line\nSecond", "yes");
        let replies = parse(&text).unwrap();
        assert_eq!(replies[0].id, id);
        assert_eq!(display_text(&text).unwrap(), "> First line Second\n\nyes");
    }

    #[test]
    fn plain_text_is_no_reply() {
        assert_eq!(parse("> q\n\na"), None);
        assert_eq!(display_text("hello"), None);
    }
}
