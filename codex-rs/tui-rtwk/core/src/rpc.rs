//! Our own RPCs: the table of those in flight and their replies (R1, R3).
//! A `Reply` is opaque: it is read only through `Pending::resolve`, which
//! answers for a ticket this table holds and drops a stale one.

use std::collections::BTreeMap;

use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TurnId;
use crate::screens::ScreenReply;

/// Why a screen's RPC has no reply. A typed value: the words are the UI's
/// string tables'; the texts inside are the server's or the decoder's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RpcError {
    /// The server (or the client) failed the request, with its message.
    Remote(String),
    /// The reply is of another screen than the ticket's.
    Mismatch,
    /// The result did not decode; the decoder's diagnostic.
    Malformed(String),
}

/// A request's failure as the client reports it: the server's JSON-RPC error
/// code when it is one (the shell reads it from the client's typed error,
/// never out of the message), and the text to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerError {
    pub code: Option<i64>,
    pub message: String,
}

impl ServerError {
    /// The server refused the request as malformed or unsupported
    /// (JSON-RPC `-32602..=-32600`: invalid request, method not found,
    /// invalid params).
    pub fn refused(&self) -> bool {
        self.code
            .is_some_and(|code| (-32602..=-32600).contains(&code))
    }
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ServerError {}

impl From<String> for ServerError {
    fn from(message: String) -> Self {
        Self {
            code: None,
            message,
        }
    }
}

impl From<&str> for ServerError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

/// The reply of one of our RPCs, as the shell reports it with the ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply(Answer);

impl Reply {
    pub fn thread_started(thread: ThreadId) -> Self {
        Self(Answer::ThreadStarted { thread })
    }

    pub fn turn_started(thread: ThreadId, turn: TurnId) -> Self {
        Self(Answer::TurnStarted { thread, turn })
    }

    pub fn screen(reply: ScreenReply) -> Self {
        Self(Answer::Screen(reply))
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self(Answer::Failed(RpcError::Remote(message.into())))
    }

    pub fn error(error: RpcError) -> Self {
        Self(Answer::Failed(error))
    }
}

/// What a reply says. Obtained only from `Pending::resolve`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    ThreadStarted { thread: ThreadId },
    TurnStarted { thread: ThreadId, turn: TurnId },
    Screen(ScreenReply),
    Failed(RpcError),
}

/// The RPCs in flight, each with what it was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending<C> {
    entries: BTreeMap<Ticket, C>,
}

impl<C> Default for Pending<C> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl<C> Pending<C> {
    pub fn insert(&mut self, ticket: Ticket, ctx: C) {
        self.entries.insert(ticket, ctx);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, ticket: Ticket) -> bool {
        self.entries.contains_key(&ticket)
    }

    pub fn any(&self, f: impl Fn(&C) -> bool) -> bool {
        self.entries.values().any(f)
    }

    /// The context of `ticket` and what `reply` says, if the ticket is
    /// held; the ticket is spent. A stale ticket gives `None` (R1).
    pub fn resolve(&mut self, ticket: Ticket, reply: Reply) -> Option<(C, Answer)> {
        self.entries.remove(&ticket).map(|ctx| (ctx, reply.0))
    }
}
