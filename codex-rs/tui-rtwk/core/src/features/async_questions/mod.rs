//! The questions the model asks while it keeps working (`request_user_input` of an agent message,
//! oracle `chatwidget/questions.rs`, `bottom_pane/async_questions/state.rs`) as a feature slice
//! (ADR-012, ADR-013): which questions of which thread wait, and what removes one.
//!
//! What the slice keeps is what IO and the stream need: the questions an agent message carried
//! (their ids are the desktop's, so a reply of another client names them), the messages already
//! seen (a replay cannot bring back a question that was handled) and the ids a reply answered
//! (history can arrive before the question does). What the user does with them is the editor's
//! (`src/sheets/approvals/questions`): the drafts, the question on show, the open or closed state.
//! The user's decision leaves as a verb (`Intent::Resolve`: the answer went out, or the question
//! was skipped); an answer of another client is read off the stream (`project`).
//!
//! The reply envelope, its parse and the words it shows are `reply`.

use std::collections::{BTreeMap, BTreeSet};

use codex_app_server_protocol::{
    AsyncUserInputQuestion, ServerNotification, ThreadItemPayload, UserInput,
};

use super::AppCx;
use crate::app::Now;
use crate::ids::{ThreadId, TurnId};
use crate::question_reply as reply;
use crate::threads::{Item, LiveThreadEvent, LiveTurnEnd};

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
    pub turn: TurnId,
    /// The question, Markdown.
    pub title: String,
    /// The suggested answers (none: answered freely).
    pub options: Vec<String>,
}

/// What the user decides of a question (the enum is `question_reply::Intent`: the session page
/// says it and may not name this feature, so the shape lives where both reach it).
pub use crate::question_reply::Intent;

/// What the stream says of the questions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// An agent message completed with questions.
    Asked {
        thread: ThreadId,
        turn: TurnId,
        message: String,
        questions: Vec<AsyncUserInputQuestion>,
    },
    /// A user message of the thread answered the questions with these ids (another client's
    /// reply, or the echo of ours).
    Answered { thread: ThreadId, ids: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadFact {
    /// The turns that still own questions.
    Pending(BTreeSet<TurnId>),
    /// The page applied this end, including any returned input it carried.
    Acknowledged(TurnId),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    pending: BTreeSet<TurnId>,
    synced: bool,
    ends: Vec<LiveTurnEnd>,
}

impl ThreadPart {
    pub fn ends(&self) -> &[LiveTurnEnd] {
        &self.ends
    }
}

/// How long a question that arrives while the editor is closed waits before it is worth a
/// countdown, and how much of that the summary shows (`expires_at`, `countdown`).
const WAIT_MILLIS: i64 = 30_000;
const COUNTDOWN_MILLIS: i64 = 20_000;

/// `millis` in whole seconds, rounded up.
fn ceil_secs(millis: i64) -> i64 {
    (millis + 999).div_euclid(1000)
}

/// The questions that wait, by thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pending: BTreeMap<ThreadId, Vec<Question>>,
    seen: BTreeSet<(ThreadId, String)>,
    answered: BTreeSet<String>,
    /// The moment (epoch milliseconds) each question's countdown ends; none: it does not count
    /// (the user was at it, or it came while the editor was open).
    expires: BTreeMap<String, i64>,
    /// The last moment the feature heard of: the countdown is measured on it.
    now: i64,
}

impl State {
    /// The questions of `thread` that wait, in the order they came.
    pub fn of(&self, thread: &ThreadId) -> &[Question] {
        self.pending.get(thread).map_or(&[], Vec::as_slice)
    }

    /// The seconds left of the countdown of `thread`'s questions, when they are the last
    /// `COUNTDOWN_MILLIS` (`countdown`: the nearest end, rounded up).
    pub fn countdown(&self, thread: &ThreadId) -> Option<u64> {
        self.of(thread)
            .iter()
            .filter_map(|q| self.expires.get(&q.id))
            .map(|end| end - self.now)
            .filter(|left| (1..=COUNTDOWN_MILLIS).contains(left))
            .min()
            .map(|left| u64::try_from(ceil_secs(left)).unwrap_or(0))
    }

    /// The moment the shown countdown changes next: the last twenty seconds begin, then each
    /// second, then the end.
    fn next_change(&self) -> Option<i64> {
        let end = self
            .expires
            .values()
            .copied()
            .filter(|end| *end > self.now)
            .min()?;
        let left = end - self.now;
        let at = if left > COUNTDOWN_MILLIS {
            end - COUNTDOWN_MILLIS
        } else {
            end - (ceil_secs(left) - 1) * 1000
        };
        Some(at.max(self.now + 1))
    }

    fn ask(
        &mut self,
        thread: ThreadId,
        turn: &TurnId,
        message: &str,
        questions: &[AsyncUserInputQuestion],
    ) {
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
            self.expires.insert(id.clone(), self.now + WAIT_MILLIS);
            self.pending
                .entry(thread.clone())
                .or_default()
                .push(Question {
                    id,
                    message: message.to_owned(),
                    turn: turn.clone(),
                    title: question.title.clone(),
                    options,
                });
        }
    }

    /// The questions named by `ids` (a message's id names all of its questions) are over.
    fn resolve(&mut self, thread: &ThreadId, ids: &[String]) {
        self.answered.extend(ids.iter().cloned());
        if let Some(list) = self.pending.get_mut(thread) {
            for q in list.iter() {
                if ids.contains(&q.id) || ids.contains(&q.message) {
                    self.expires.remove(&q.id);
                }
            }
            list.retain(|q| !ids.contains(&q.id) && !ids.contains(&q.message));
            if list.is_empty() {
                self.pending.remove(thread);
            }
        }
    }

    fn sync_turns(&self, thread: &ThreadId, cx: &mut AppCx<'_>) {
        let turns = self.of(thread).iter().map(|q| q.turn.clone()).collect();
        cx.thread(thread, ThreadFact::Pending(turns));
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
            .map(|reply| reply.id)
            .collect(),
    )
}

feature! {
    impl Feature for AsyncQuestions {
        const NAME: &'static str = "async_questions";
        type State = State;
        type ThreadPart = ThreadPart;
        type Intent = Intent;
        type ThreadIntent = ThreadFact;
        type Event = Event;
        {
            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                state.now = cx.view().facts().now().millis();
                match intent {
                    Intent::Resolve { thread, id } => {
                        state.resolve(&thread, &[id]);
                        state.sync_turns(&thread, cx);
                    }
                    Intent::TurnEnded { thread, turn } => {
                        let ids = state.of(&thread)
                            .iter()
                            .filter(|q| q.turn == turn)
                            .map(|q| q.id.clone())
                            .collect::<Vec<_>>();
                        state.resolve(&thread, &ids);
                        state.sync_turns(&thread, cx);
                        cx.thread(&thread, ThreadFact::Acknowledged(turn));
                    }
                    Intent::Superseded { thread } => {
                        let ids: Vec<String> =
                            state.of(&thread).iter().map(|q| q.id.clone()).collect();
                        state.resolve(&thread, &ids);
                        state.sync_turns(&thread, cx);
                    }
                    Intent::Snooze { thread } => {
                        let ids: Vec<String> =
                            state.of(&thread).iter().map(|q| q.id.clone()).collect();
                        state.expires.retain(|id, _| !ids.contains(id));
                    }
                    Intent::Tick => {
                        let now = state.now;
                        state.expires.retain(|_, end| *end > now);
                    }
                }
            }

            fn thread_fact(part: &mut ThreadPart, fact: ThreadFact) -> Option<ThreadFact> {
                match fact {
                    ThreadFact::Pending(turns) => {
                        part.pending = turns;
                        part.synced = true;
                    }
                    ThreadFact::Acknowledged(turn) => part.ends.retain(|end| end.turn != turn),
                }
                None
            }

            fn on_live_thread_event(part: &mut ThreadPart, event: LiveThreadEvent<'_>) {
                match event {
                    // The app may have observed this while the owner was still attaching.
                    // In that case its pending fact could not reach the thread yet.
                    LiveThreadEvent::ItemCompleted { key, item: Item::Agent { questions: Some(questions), .. } }
                        if !part.synced && !questions.is_empty() =>
                    {
                        part.pending.insert(key.turn.clone());
                    }
                    LiveThreadEvent::TurnEnded(end)
                        if part.pending.contains(&end.turn)
                            && !part.ends.iter().any(|held| held.turn == end.turn) =>
                    {
                        part.ends.push(end.clone());
                    }
                    _ => {}
                }
            }

            /// The countdown of the summary changes at the start of its last twenty seconds and
            /// then each second.
            fn wake_at(state: &State) -> Option<(Now, Intent)> {
                state
                    .next_change()
                    .map(|at| (Now::at_millis(at), Intent::Tick))
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
                        turn: TurnId::new(&completed.turn_id),
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

            fn on_event(state: &mut State, event: Event, cx: &mut AppCx<'_>) {
                state.now = cx.view().facts().now().millis();
                match event {
                    Event::Asked {
                        thread,
                        turn,
                        message,
                        questions,
                    } => {
                        if cx.view().thread(&thread).is_some_and(|t| t.core().turn_finished(&turn)) {
                            return;
                        }
                        state.ask(thread.clone(), &turn, &message, &questions);
                        state.sync_turns(&thread, cx);
                    }
                    Event::Answered { thread, ids } => {
                        state.resolve(&thread, &ids);
                        state.sync_turns(&thread, cx);
                    }
                }
            }
        }
    }
}
