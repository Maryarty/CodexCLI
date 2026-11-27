//! Evidence for retrying a safety-buffered turn. A fork request can only come from a terminal
//! source turn with complete, bounded input. The original input is projected in its source
//! conversation, before the new thread exists.

use codex_app_server_protocol::{
    Thread, ThreadHistoryMode, ThreadItem, ThreadItemPayload, ThreadItemsListResponse,
    ThreadStatus, ThreadTurnsListResponse, Turn, TurnItemsView, TurnStatus, UserInput,
};

use crate::ids::{ThreadId, Ticket, TurnId};
use crate::rpc::Address;
use crate::threads::UserMessage;

use super::Retry;

pub(in crate::threads) const TURN_LIMIT: u32 = 5;
pub(in crate::threads) const ITEM_LIMIT: u32 = 100;

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

    pub(crate) fn request(&self) -> TurnsRequest {
        TurnsRequest {
            thread: self.origin().thread.clone(),
            mode: self.mode,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnsRequest {
    thread: ThreadId,
    mode: ThreadHistoryMode,
}

impl TurnsRequest {
    pub(crate) fn thread(&self) -> &ThreadId {
        &self.thread
    }

    pub(in crate::threads) fn mode(&self) -> ThreadHistoryMode {
        self.mode
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

    pub(crate) fn request(&self) -> ItemsRequest {
        ItemsRequest {
            thread: self.origin().thread.clone(),
            turn: self.origin().turn.clone(),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemsRequest {
    thread: ThreadId,
    turn: TurnId,
}

impl ItemsRequest {
    pub(crate) fn thread(&self) -> &ThreadId {
        &self.thread
    }

    pub(in crate::threads) fn turn(&self) -> &TurnId {
        &self.turn
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

    pub(crate) fn request(&self) -> ForkRequest {
        ForkRequest {
            thread: self.origin.thread.clone(),
            turn: self.origin.turn.clone(),
            mode: self.mode,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForkRequest {
    thread: ThreadId,
    turn: TurnId,
    mode: ThreadHistoryMode,
}

impl ForkRequest {
    pub(crate) fn thread(&self) -> &ThreadId {
        &self.thread
    }

    pub(in crate::threads) fn point(&self) -> (&TurnId, ThreadHistoryMode) {
        (&self.turn, self.mode)
    }
}

pub(crate) struct ReadyToStart {
    fork: ReadyToFork,
    child: Box<Thread>,
}

impl ReadyToStart {
    pub(crate) fn start(
        self,
        client_user_message_id: String,
    ) -> (Box<Thread>, UserMessage, StartRequest) {
        let thread = ThreadId::new(&self.child.id);
        (
            self.child,
            self.fork.message,
            StartRequest {
                thread,
                input: self.fork.input,
                client_user_message_id,
                model: self.fork.model,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartRequest {
    thread: ThreadId,
    input: Vec<UserInput>,
    client_user_message_id: String,
    model: String,
}

// The pinned UserInput variants contain text, paths, IDs, and text ranges.
impl Eq for StartRequest {}

impl StartRequest {
    pub(crate) fn thread(&self) -> &ThreadId {
        &self.thread
    }

    pub(in crate::threads) fn submission(&self) -> (&[UserInput], &str, &str) {
        (&self.input, &self.client_user_message_id, &self.model)
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
    UnexpectedReceipt,
    TimedOut,
}

impl Invalid {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::WrongThread | Self::IneligibleThread => {
                "The source conversation is unavailable for retry."
            }
            Self::WrongFork => "The server did not return the verified fork.",
            Self::WrongTurn | Self::MissingTurn => {
                "The checked turn is missing from the history of the source conversation."
            }
            Self::NotLatest => "The checked turn is no longer the latest turn.",
            Self::IncompleteTurns => "The predecessor of the checked turn could not be verified.",
            Self::TurnRunning => "The checked turn is still in progress.",
            Self::PredecessorRunning => "The previous turn is still in progress.",
            Self::IncompleteInput => {
                "Cannot safely retry a turn whose input exceeds the bounded history page."
            }
            Self::ResponseStarted => "The source turn has already started answering.",
            Self::UnexpectedReceipt => "The server did not acknowledge the retry.",
            Self::TimedOut => "Timed out waiting for confirmation of the retry.",
        }
    }
}

#[cfg(test)]
mod tests;
