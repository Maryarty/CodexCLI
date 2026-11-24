//! `/recap` as a feature slice (ADR-012; oracle `app/recap.rs`, `chatwidget/recap.rs`): a short
//! catch-up of the conversation in use, made by a temporary structured turn (`AppCx::structured_turn`:
//! an ephemeral thread that sees only the excerpt of the visible exchanges), then said in the
//! conversation as a note.
//!
//! Only the manual `/recap` is here. What the oracle schedules after the user has been away
//! (`RecapState`: focus lost, completed turns, the 30 minute delay, the retry) needs the window's
//! focus and a timer, which are the surface's: UI LEFT. So is the cloud refusal
//! (`Conversation recaps are not available for cloud conversations.`), which needs the thread's
//! settings owner.
//!
//! What is kept here is what IO needs: the request in flight (a record by thread, so a second
//! `/recap` says it is already generating, and the "Recapping..." row is drawn while
//! `State::generating`). The request is stale when the conversation changed under it (the
//! oracle's `turn_revision` and `completed_turns` checks), a turn runs, or the thread is no
//! longer the one in use: its answer is dropped without a word. The words of the notes:
//! `src/features/recap/`.

mod history;
mod prompt;

use codex_app_server_protocol::{ClientRequest, RequestId};
use serde_json::{Value, json};

use super::{
    AppCx, Feature, Never, Slash, StructuredFailure, StructuredKind, StructuredTurn, ThreadView,
};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::{Item, Tone};

pub use history::{Message, recap_history};

const RECAP_MAX_CHARS: usize = 700;
const RECAP_NEXT_MAX_CHARS: usize = 200;

/// The marker type of the feature.
pub struct Recap;

/// What the user asks of `/recap`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/recap`, typed in the thread `source`.
    Command { source: Option<ThreadId> },
}

/// A request in flight.
#[derive(Clone, Debug, PartialEq, Eq)]
struct InFlight {
    /// Which request this is (`State::asked` when it was made).
    at: u64,
    thread: ThreadId,
    /// The excerpt the request was made from: the answer is fresh while the conversation still
    /// yields it.
    history: String,
}

/// The app-scoped state of the feature.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    in_flight: Option<InFlight>,
    asked: u64,
}

impl State {
    /// A recap of `thread` is being generated (the oracle's `ThreadRecapLoadingCell`).
    pub fn generating(&self, thread: &ThreadId) -> bool {
        self.in_flight
            .as_ref()
            .is_some_and(|request| &request.thread == thread)
    }
}

/// What an ask of the host was for: the request number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ctx(pub u64);

/// What `/recap` says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// The recap (oracle `ThreadRecapHistoryCell`): what happened, and what to do next.
    Recap {
        summary: String,
        next_action: Option<String>,
    },
    /// `Could not generate a recap. Please try again.` (error)
    Failed,
    /// `A recap is already being generated.` (error)
    InProgress,
    /// `There is no conversation history to recap.` (error)
    EmptyHistory,
}

/// The answer the model is asked for (`recap_output_schema`).
fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {
                "type": "string",
                "minLength": 1,
                "maxLength": RECAP_MAX_CHARS,
            },
            "next_action": {
                "type": ["string", "null"],
                "maxLength": RECAP_NEXT_MAX_CHARS,
            },
        },
        "required": ["summary", "next_action"],
        "additionalProperties": false,
    })
}

/// `parse_recap`: both keys and no other, a summary that is not empty and within its limit, a
/// next action within its limit (an empty one is none).
fn parse_recap(response: &str) -> Option<(String, Option<String>)> {
    let Value::Object(fields) = serde_json::from_str(response).ok()? else {
        return None;
    };
    if fields.len() != 2 {
        return None;
    }
    let summary = fields.get("summary")?.as_str()?.trim().to_owned();
    if summary.is_empty() || summary.chars().count() > RECAP_MAX_CHARS {
        return None;
    }
    let next_action = match fields.get("next_action")? {
        Value::Null => None,
        Value::String(action) => Some(action.trim().to_owned()).filter(|action| !action.is_empty()),
        _ => return None,
    };
    if next_action
        .as_ref()
        .is_some_and(|action| action.chars().count() > RECAP_NEXT_MAX_CHARS)
    {
        return None;
    }
    Some((summary, next_action))
}

/// The messages of the conversation a reader sees, oldest first: what the user typed and what
/// the assistant said (no tools, reasoning or plans), as history draws their text.
pub fn conversation(thread: ThreadView<'_>) -> Vec<Message> {
    thread
        .core()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::User(prompt) => Some(Message::User(prompt.shown().text.into_owned())),
            Item::Agent { text, .. } => Some(Message::Assistant(text.clone())),
            _ => None,
        })
        .collect()
}

/// The messages the recap reads (`UserHistoryCell`, `AgentMarkdownCell`, `AgentMessageCell`).
fn excerpt(cx: &AppCx<'_>, thread: &ThreadId) -> String {
    let messages = cx
        .view()
        .thread(thread)
        .map(conversation)
        .unwrap_or_default();
    recap_history(&messages)
}

/// `request_recap` for a manual recap.
fn request(state: &mut State, source: Option<ThreadId>, cx: &mut AppCx<'_>) {
    if state.in_flight.is_some() {
        return cx.note(None, Note::InProgress);
    }
    let Some(thread) = source.filter(|id| cx.view().thread(id).is_some()) else {
        return cx.note(None, Note::EmptyHistory);
    };
    let history = excerpt(cx, &thread);
    if history.is_empty() {
        return cx.note(None, Note::EmptyHistory);
    }
    state.asked += 1;
    let at = state.asked;
    cx.structured_turn(
        StructuredTurn {
            kind: StructuredKind::Recap,
            source: thread.clone(),
            model: None,
            effort: None,
            prompt: prompt::prompt(&history),
            schema: output_schema(),
        },
        Ctx(at),
    );
    state.in_flight = Some(InFlight {
        at,
        thread,
        history,
    });
}

impl Feature for Recap {
    const NAME: &'static str = "recap";
    const RUNS: &'static [SlashCommand] = &[SlashCommand::Recap];

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = Never<Self>;
    type Event = Never<Self>;

    /// `/recap`: a summary of the conversation in use.
    fn slash(state: &mut State, slash: Slash, cx: &mut AppCx<'_>) {
        Self::intent(
            state,
            Intent::Command {
                source: slash.source,
            },
            cx,
        );
    }

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let Intent::Command { source } = intent;
        request(state, source, cx);
    }

    /// `handle_recap_started` and `handle_generated_recap`: the request ended; its answer is
    /// dropped when it is stale, a recap when it parses, else the failure line.
    fn structured_done(
        state: &mut State,
        ctx: Ctx,
        result: Result<String, StructuredFailure>,
        cx: &mut AppCx<'_>,
    ) {
        let Some(request) = state.in_flight.take_if(|request| request.at == ctx.0) else {
            return;
        };
        let view = cx.view();
        let fresh = view.active() == Some(&request.thread)
            && view
                .thread(&request.thread)
                .is_some_and(|thread| !thread.busy())
            && excerpt(cx, &request.thread) == request.history;
        if !fresh {
            return;
        }
        let note = match result.ok().as_deref().and_then(parse_recap) {
            Some((summary, next_action)) => Note::Recap {
                summary,
                next_action,
            },
            None => Note::Failed,
        };
        cx.note(Some(request.thread), note);
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::Recap { .. } => Tone::Info,
            Note::Failed | Note::InProgress | Note::EmptyHistory => Tone::Error,
        }
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
