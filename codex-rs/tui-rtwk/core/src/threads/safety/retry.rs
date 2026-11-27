//! Evidence for retrying a safety-buffered turn. A fork request can only come from a terminal
//! source turn with complete, bounded input. The original input is projected in its source
//! conversation, before the new thread exists.

use codex_app_server_protocol::{
    SortDirection, Thread, ThreadForkParams, ThreadHistoryMode, ThreadItem, ThreadItemPayload,
    ThreadItemsListParams, ThreadItemsListResponse, ThreadStatus, ThreadTurnsListParams, ThreadTurnsListResponse,
    Turn, TurnItemsView, TurnStartParams, TurnStatus, UserInput,
};
use codex_protocol::openai_models::ReasoningEffort;

use crate::ids::{ThreadId, Ticket, TurnId};
use crate::rpc::Address;
use crate::threads::UserMessage;

use super::Retry;

const TURN_LIMIT: u32 = 5;
const ITEM_LIMIT: u32 = 100;

/// The source and connection selected when the confirmed operation started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Origin {
    pub(crate) thread: ThreadId,
    pub(crate) turn: TurnId,
    pub(crate) address: Address,
    pub(crate) generation: Ticket,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AwaitingInterrupt {
    pub(crate) origin: Origin,
    model: String,
    message: UserMessage,
    observed: Option<InterruptSignal>,
}

impl AwaitingInterrupt {
    pub(crate) fn new(confirmed: Retry, address: Address, generation: Ticket) -> Self {
        Self {
            origin: Origin {
                thread: confirmed.thread,
                turn: confirmed.turn,
                address,
                generation,
            },
            model: confirmed.model,
            message: confirmed.message,
            observed: None,
        }
    }

    pub(crate) fn message(&self) -> &UserMessage {
        &self.message
    }

    /// The RPC reply and its matching terminal event can arrive in either order.
    pub(crate) fn observe(mut self, signal: InterruptSignal) -> Interruption {
        if self.observed.is_some_and(|prior| prior != signal) {
            Interruption::Ready(ReadSource(self))
        } else {
            self.observed = Some(signal);
            Interruption::Waiting(self)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InterruptSignal {
    Acknowledged,
    TurnInterrupted,
}

pub(crate) enum Interruption {
    Waiting(AwaitingInterrupt),
    Ready(ReadSource),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadSource(AwaitingInterrupt);

impl ReadSource {
    pub(crate) fn origin(&self) -> &Origin {
        &self.0.origin
    }

    pub(crate) fn verify(self, source: &Thread) -> Result<ReadTurns, Invalid> {
        if self.origin().thread.as_str() != source.id {
            return Err(Invalid::WrongThread);
        }
        if source.ephemeral
            || source.attached_to_thread_id.is_some()
            || source.can_accept_direct_input == Some(false)
        {
            return Err(Invalid::IneligibleThread);
        }
        Ok(ReadTurns {
            attempt: self.0,
            mode: source.history_mode,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadTurns {
    attempt: AwaitingInterrupt,
    mode: ThreadHistoryMode,
}

impl ReadTurns {
    pub(crate) fn origin(&self) -> &Origin {
        &self.attempt.origin
    }

    pub(crate) fn params(&self) -> ThreadTurnsListParams {
        ThreadTurnsListParams {
            thread_id: self.origin().thread.to_string(),
            cursor: None,
            limit: Some(TURN_LIMIT),
            sort_direction: Some(SortDirection::Desc),
            items_view: Some(match self.mode {
                ThreadHistoryMode::Legacy => TurnItemsView::Full,
                ThreadHistoryMode::Paginated => TurnItemsView::NotLoaded,
            }),
        }
    }

    pub(crate) fn verify(self, page: ThreadTurnsListResponse) -> Result<History, Invalid> {
        let [latest, older @ ..] = page.data.as_slice() else {
            return Err(Invalid::MissingTurn);
        };
        if latest.id != self.origin().turn.as_str() {
            return Err(
                if older
                    .iter()
                    .any(|turn| turn.id == self.origin().turn.as_str())
                {
                    Invalid::NotLatest
                } else {
                    Invalid::MissingTurn
                },
            );
        }
        // A continuation cursor with no predecessor leaves the predecessor unknown.
        if older.is_empty() && page.next_cursor.is_some() {
            return Err(Invalid::IncompleteTurns);
        }
        validate_terminal(latest, older.first())?;
        match self.mode {
            ThreadHistoryMode::Legacy => {
                if latest.items_view != TurnItemsView::Full {
                    return Err(Invalid::IncompleteInput);
                }
                Ok(History::Complete(ReadyToFork::from_items(
                    self.attempt,
                    &latest.items,
                    self.mode,
                )?))
            }
            ThreadHistoryMode::Paginated => Ok(History::NeedsItems(ReadItems(self.attempt))),
        }
    }
}

fn validate_terminal(latest: &Turn, previous: Option<&Turn>) -> Result<(), Invalid> {
    if latest.status == TurnStatus::InProgress {
        return Err(Invalid::TurnRunning);
    }
    if previous.is_some_and(|turn| turn.status == TurnStatus::InProgress) {
        return Err(Invalid::PredecessorRunning);
    }
    Ok(())
}

pub(crate) enum History {
    Complete(ReadyToFork),
    NeedsItems(ReadItems),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadItems(AwaitingInterrupt);

impl ReadItems {
    pub(crate) fn origin(&self) -> &Origin {
        &self.0.origin
    }

    pub(crate) fn params(&self) -> ThreadItemsListParams {
        ThreadItemsListParams {
            thread_id: self.origin().thread.to_string(),
            turn_id: Some(self.origin().turn.to_string()),
            cursor: None,
            limit: Some(ITEM_LIMIT),
            sort_direction: Some(SortDirection::Asc),
            defer_output: None,
        }
    }

    pub(crate) fn verify(self, page: ThreadItemsListResponse) -> Result<ReadyToFork, Invalid> {
        if page.next_cursor.is_some() || page.data.len() > ITEM_LIMIT as usize {
            return Err(Invalid::IncompleteInput);
        }
        let mut items = Vec::with_capacity(page.data.len());
        for entry in page.data {
            if entry.turn_id != self.origin().turn.as_str() {
                return Err(Invalid::WrongTurn);
            }
            items.push(ThreadItem::try_from(entry).map_err(|_| Invalid::IncompleteInput)?);
        }
        ReadyToFork::from_items(self.0, &items, ThreadHistoryMode::Paginated)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReadyToFork {
    origin: Origin,
    model: String,
    message: UserMessage,
    input: Vec<UserInput>,
    mode: ThreadHistoryMode,
}

impl ReadyToFork {
    fn from_items(
        attempt: AwaitingInterrupt,
        items: &[ThreadItem],
        mode: ThreadHistoryMode,
    ) -> Result<Self, Invalid> {
        if items.len() > ITEM_LIMIT as usize {
            return Err(Invalid::IncompleteInput);
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut extra = Vec::new();
        let mut first_user = true;
        for item in items {
            if !seen.insert(item.id()) {
                return Err(Invalid::IncompleteInput);
            }
            match &item.payload {
                ThreadItemPayload::AgentMessage { text, .. } if !text.is_empty() => {
                    return Err(Invalid::ResponseStarted);
                }
                ThreadItemPayload::UserMessage { .. } if first_user => {
                    first_user = false;
                }
                ThreadItemPayload::UserMessage { content, .. } => {
                    extra.push(UserInput::Text {
                        text: "\n".to_owned(),
                        text_elements: Vec::new(),
                    });
                    extra.extend(content.iter().cloned());
                }
                _ => {}
            }
        }
        let source = attempt.origin.thread.as_str();
        let mut input = crate::protocol::turn_input_in(&attempt.message, Some(source));
        input.extend(extra);
        Ok(Self {
            origin: attempt.origin,
            model: attempt.model,
            message: attempt.message,
            input,
            mode,
        })
    }

    pub(crate) fn origin(&self) -> &Origin {
        &self.origin
    }

    pub(crate) fn params(&self) -> ThreadForkParams {
        // Leave permission parameters absent so the server inherits the source's settings.
        ThreadForkParams {
            thread_id: self.origin.thread.to_string(),
            before_turn_id: Some(self.origin.turn.to_string()),
            defer_goal_continuation: true,
            exclude_turns: self.mode == ThreadHistoryMode::Paginated,
            ..ThreadForkParams::default()
        }
    }

    pub(crate) fn forked(self, child: Box<Thread>) -> Result<ReadyToStart, Invalid> {
        let source = self.origin.thread.as_str();
        if child.id.is_empty()
            || child.id == source
            || child.forked_from_id.as_deref() != Some(source)
            || child.ephemeral
            || child.attached_to_thread_id.is_some()
            || child.can_accept_direct_input == Some(false)
            || child.status != ThreadStatus::Idle
        {
            return Err(Invalid::WrongFork);
        }
        Ok(ReadyToStart { fork: self, child })
    }
}

pub(crate) struct ReadyToStart {
    fork: ReadyToFork,
    child: Box<Thread>,
}

impl ReadyToStart {
    pub(crate) fn origin(&self) -> &Origin {
        self.fork.origin()
    }

    pub(crate) fn child(&self) -> &Thread {
        &self.child
    }

    /// The fork's effective permissions stay in force because the turn carries no overrides.
    pub(crate) fn start(self, client_user_message_id: String) -> (Box<Thread>, UserMessage, TurnStartParams) {
        let thread_id = self.child.id.clone();
        (
            self.child,
            self.fork.message,
            TurnStartParams {
                thread_id,
                input: self.fork.input,
                client_user_message_id: Some(client_user_message_id),
                model: Some(self.fork.model),
                effort: Some(ReasoningEffort::Low),
                ..TurnStartParams::default()
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Invalid {
    WrongThread,
    WrongFork,
    IneligibleThread,
    WrongTurn,
    MissingTurn,
    NotLatest,
    IncompleteTurns,
    TurnRunning,
    PredecessorRunning,
    IncompleteInput,
    ResponseStarted,
}

#[cfg(test)]
mod tests;
