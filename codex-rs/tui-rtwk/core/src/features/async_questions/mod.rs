//! The questions the model asks while it keeps working (`request_user_input` of an agent message,
//! oracle `chatwidget/questions.rs`, `bottom_pane/async_questions/state.rs`) as a feature slice
//! (ADR-012, ADR-013): which questions of which thread wait, and what removes one.
//!
//! What the slice keeps is what IO and the stream need: the questions an agent message carried
//! (their ids are the desktop's, so a reply of another client names them), the messages already
//! seen (a replay cannot bring back a question that was handled) and the ids a reply answered
//! (history can arrive before the question does). What the user does with them is the editor's
//! (`src/sheets/approvals/questions`): the drafts, the question on show, the open or closed state.
//! The user's decision leaves as a verb (`Intent::Dismiss`: the answer went out, or the question
//! was skipped); an answer of another client is read off the stream (`project`).
//!
//! The reply envelope, its parse and the words it shows are `reply`.

pub mod reply;

use std::collections::{BTreeMap, BTreeSet};

use codex_app_server_protocol::{
    AsyncUserInputQuestion, ServerNotification, ThreadItemPayload, UserInput,
};

use super::AppCx;
use crate::ids::ThreadId;

/// The marker type of the feature.
pub struct AsyncQuestions;

/// The most choices of a question that are kept, and their longest (bytes): the model's
/// suggestions are bounded before anything wraps them.
pub const MAX_OPTIONS: usize = 32;
pub const MAX_OPTION_BYTES: usize = 512;

/// One question that waits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    /// The desktop's id of it: the reply says which question it answers.
    pub id: String,
    pub message: String,
    /// The question, Markdown.
    pub title: String,
    /// The suggested answers (none: answered freely).
    pub options: Vec<String>,
}

/// What the user decides of a question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The answer of the question went out, or the question was skipped.
    Dismiss { thread: ThreadId, id: String },
}

/// What the stream says of the questions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// An agent message completed with questions.
    Asked {
        thread: ThreadId,
        message: String,
        questions: Vec<AsyncUserInputQuestion>,
    },
    /// A user message of the thread answered the questions with these ids (another client's
    /// reply, or the echo of ours).
    Answered { thread: ThreadId, ids: Vec<String> },
}

/// The questions that wait, by thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pending: BTreeMap<ThreadId, Vec<Question>>,
    seen: BTreeSet<(ThreadId, String)>,
    answered: BTreeSet<String>,
}

impl State {
    /// The questions of `thread` that wait, in the order they came.
    pub fn of(&self, thread: &ThreadId) -> &[Question] {
        self.pending.get(thread).map_or(&[], Vec::as_slice)
    }

    fn ask(&mut self, thread: ThreadId, message: &str, questions: &[AsyncUserInputQuestion]) {
        if questions.is_empty() || !self.seen.insert((thread.clone(), message.to_owned())) {
            return;
        }
        for (index, question) in questions.iter().enumerate() {
            let id = reply::question_id(message, index);
            if self.answered.contains(&id) || self.answered.contains(message) {
                continue;
            }
            let options = question
                .options
                .iter()
                .flatten()
                .take(MAX_OPTIONS)
                .filter(|label| label.len() <= MAX_OPTION_BYTES)
                .cloned()
                .collect();
            self.pending.entry(thread.clone()).or_default().push(Question {
                id,
                message: message.to_owned(),
                title: question.title.clone(),
                options,
            });
        }
    }

    /// The questions named by `ids` (a message's id names all of its questions) are over.
    fn resolve(&mut self, thread: &ThreadId, ids: &[String]) {
        self.answered.extend(ids.iter().cloned());
        if let Some(list) = self.pending.get_mut(thread) {
            list.retain(|q| !ids.contains(&q.id) && !ids.contains(&q.message));
            if list.is_empty() {
                self.pending.remove(thread);
            }
        }
    }
}

/// The ids a reply of `content` answers.
fn answered_ids(content: &[UserInput]) -> Option<Vec<String>> {
    let mut texts = content
        .iter()
        .filter(|item| !matches!(item, UserInput::Skill { .. } | UserInput::Mention { .. }));
    let UserInput::Text { text, .. } = texts.next()? else {
        return None;
    };
    if texts.next().is_some() {
        return None;
    }
    Some(
        reply::parse(text)?
            .into_iter()
            .map(|reply| reply.question_item_id)
            .collect(),
    )
}

feature! {
    impl Feature for AsyncQuestions {
        const NAME: &'static str = "async_questions";
        type State = State;
        type Intent = Intent;
        type Event = Event;
        {
            fn intent(state: &mut State, intent: Intent, _cx: &mut AppCx<'_>) {
                let Intent::Dismiss { thread, id } = intent;
                state.resolve(&thread, &[id]);
            }

            /// An agent message with questions, and a user message that answers some.
            fn project(notification: &ServerNotification) -> Option<Event> {
                let ServerNotification::ItemCompleted(completed) = notification else {
                    return None;
                };
                let thread = ThreadId::new(completed.thread_id.as_str());
                match &completed.item.payload {
                    ThreadItemPayload::AgentMessage {
                        id,
                        questions: Some(questions),
                        ..
                    } if !questions.is_empty() => Some(Event::Asked {
                        thread,
                        message: id.clone(),
                        questions: questions.clone(),
                    }),
                    ThreadItemPayload::UserMessage { content, .. } => {
                        Some(Event::Answered {
                            thread,
                            ids: answered_ids(content)?,
                        })
                    }
                    _ => None,
                }
            }

            fn on_event(state: &mut State, event: Event, _cx: &mut AppCx<'_>) {
                match event {
                    Event::Asked {
                        thread,
                        message,
                        questions,
                    } => state.ask(thread, &message, &questions),
                    Event::Answered { thread, ids } => state.resolve(&thread, &ids),
                }
            }
        }
    }
}
