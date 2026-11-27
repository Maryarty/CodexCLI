//! Our own RPCs: the table of those in flight and their replies (R1, R3).
//! A `Reply` is opaque: it is read only through `Pending::resolve`, which
//! answers for a ticket this table holds and drops a stale one.

use std::collections::BTreeMap;

use crate::app::worktree::{BindFailure, Checkout};
use crate::ids::{BackendId, ConnEpoch, ThreadId, Ticket, TurnId};
use crate::screens::{ScreenReply, agents};
use crate::threads::ThreadReply;

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
    /// The message never left: its images could not be prepared (oracle
    /// `prepare_images`), with the text to show. The thread gives the message
    /// back whole.
    Prepare(String),
    /// The server refused the turn or steer for the chat's model provider
    /// (`error.data` of a `ModelProviderError`): the chat is read-only.
    Provider(codex_app_server_protocol::ModelProviderError),
}

/// A request's failure as the client reports it: the server's JSON-RPC error
/// code when it is one (the shell reads it from the client's typed error,
/// never out of the message), and the text to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerError {
    pub code: Option<i64>,
    pub message: String,
    /// The JSON-RPC error's `data`, as the server sent it.
    pub data: Option<serde_json::Value>,
}

impl ServerError {
    /// The structured provider failure the server's `data` carries
    /// (oracle `model_provider_error`); other data is not one.
    pub fn model_provider(&self) -> Option<codex_app_server_protocol::ModelProviderError> {
        serde_json::from_value(self.data.clone()?).ok()
    }

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
            data: None,
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
pub struct Reply(ReplyResult);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReplyResult {
    Answer(Answer),
    SubmissionUnconfirmed(String),
    CloudSubmission(CloudSubmission),
}

/// Cloud decides at admission whether input starts a turn or joins one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloudSubmission {
    Started(TurnId),
    Steered(TurnId),
    /// The ID was seen before. The reply says nothing about turn lifecycle.
    Duplicate,
    Unconfirmed(String),
    /// The Cloud worker rejected the request before making a submission.
    Preflight(String),
}

impl Reply {
    fn new(answer: Answer) -> Self {
        Self(ReplyResult::Answer(answer))
    }

    /// The connection failed while a submission was in flight. Receipt still needs verification.
    pub fn submission_unconfirmed(message: impl Into<String>) -> Self {
        Self(ReplyResult::SubmissionUnconfirmed(message.into()))
    }

    pub fn cloud_submission(result: CloudSubmission) -> Self {
        Self(ReplyResult::CloudSubmission(result))
    }

    pub fn thread_started(thread: ThreadId) -> Self {
        Self::thread_started_with(thread, Vec::new())
    }

    /// A thread's start, with the instruction files the session loaded.
    pub fn thread_started_with(
        thread: ThreadId,
        sources: Vec<crate::screens::status::AgentsSource>,
    ) -> Self {
        Self::thread(ThreadReply::SessionStarted {
            thread,
            details: None,
            sources,
        })
    }

    pub fn turn_started(thread: ThreadId, turn: TurnId) -> Self {
        Self::thread(ThreadReply::TurnStarted { thread, turn })
    }

    pub fn rate_limits(limits: crate::screens::status::RateLimits) -> Self {
        Self::account_rate_limits(
            limits,
            None,
            crate::screens::chat::Reserve::default(),
            crate::screens::status::RateRead::default(),
        )
    }

    pub(crate) fn account_rate_limits(
        limits: crate::screens::status::RateLimits,
        banner: Option<crate::screens::chat::Banner>,
        reserve: crate::screens::chat::Reserve,
        read: crate::screens::status::RateRead,
    ) -> Self {
        Self::new(Answer::RateLimits {
            limits,
            banner: banner.map(Box::new),
            reserve,
            read: Box::new(read),
        })
    }

    /// `account/sendAddCreditsNudgeEmail` answered.
    pub(crate) fn add_credits_nudge(outcome: crate::screens::chat::NudgeOutcome) -> Self {
        Self::new(Answer::AddCreditsNudge(outcome))
    }

    pub fn turn_interrupted() -> Self {
        Self::thread(ThreadReply::Interrupted)
    }

    /// The typed result of an operation in the thread RPC catalog.
    pub fn thread(reply: ThreadReply) -> Self {
        Self::new(Answer::Thread(reply))
    }

    pub fn screen(reply: ScreenReply) -> Self {
        // Older fake-host and state fixtures also use the screen reply shape.
        // Give them the same typed result as replies from the live thread RPCs.
        let thread = match reply {
            ScreenReply::Agents(agents::Reply::Resumed(thread)) => ThreadReply::Resumed(thread),
            ScreenReply::Agents(agents::Reply::Forked(thread)) => ThreadReply::Forked(thread),
            ScreenReply::Agents(agents::Reply::Started(thread)) => ThreadReply::Started(thread),
            ScreenReply::Agents(agents::Reply::Read { thread, unread }) => {
                ThreadReply::Read { thread, unread }
            }
            ScreenReply::Agents(agents::Reply::History { turns, next }) => {
                ThreadReply::History { turns, next }
            }
            ScreenReply::Agents(agents::Reply::McpServers(servers)) => {
                ThreadReply::McpServers(servers)
            }
            ScreenReply::Agents(agents::Reply::Temporary { thread, as_asked }) => {
                ThreadReply::Temporary {
                    thread: ThreadId::new(thread),
                    as_asked,
                }
            }
            ScreenReply::Agents(agents::Reply::StructuredTurn { turn }) => {
                ThreadReply::StructuredTurn {
                    turn: TurnId::new(turn),
                }
            }
            ScreenReply::Agents(agents::Reply::Done) => ThreadReply::Done,
            reply => return Self::new(Answer::Screen(reply)),
        };
        Self::thread(thread)
    }

    /// The reply of a feature slice's RPC.
    pub fn feature(reply: crate::features::FeatureReply) -> Self {
        Self::new(Answer::Feature(reply))
    }

    pub fn worktree_created(result: Result<Checkout, String>) -> Self {
        Self::new(Answer::WorktreeCreated(result))
    }

    pub fn worktree_bound(result: Result<(), BindFailure>) -> Self {
        Self::new(Answer::WorktreeBound(result))
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self::new(Answer::Failed(RpcError::Remote(message.into())))
    }

    /// The images of a turn or steer could not be prepared: nothing was sent.
    pub fn prepare_failed(message: impl Into<String>) -> Self {
        Self::new(Answer::Failed(RpcError::Prepare(message.into())))
    }

    pub fn error(error: RpcError) -> Self {
        Self::new(Answer::Failed(error))
    }
}

/// What a reply says. Obtained only from `Pending::resolve`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Thread(ThreadReply),
    RateLimits {
        limits: crate::screens::status::RateLimits,
        banner: Option<Box<crate::screens::chat::Banner>>,
        reserve: crate::screens::chat::Reserve,
        /// Every bucket of the read and its reset credits.
        read: Box<crate::screens::status::RateRead>,
    },
    AddCreditsNudge(crate::screens::chat::NudgeOutcome),
    Screen(ScreenReply),
    Feature(crate::features::FeatureReply),
    WorktreeCreated(Result<Checkout, String>),
    WorktreeBound(Result<(), BindFailure>),
    Failed(RpcError),
}

/// The connection a request was sent on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Address {
    pub backend: BackendId,
    pub epoch: ConnEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry<C> {
    address: Address,
    context: C,
}

/// The RPCs in flight, each with its address and operation context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending<C> {
    entries: BTreeMap<Ticket, Entry<C>>,
}

impl<C> Default for Pending<C> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl<C> Pending<C> {
    pub(crate) fn insert(&mut self, ticket: Ticket, address: Address, context: C) {
        self.entries.insert(ticket, Entry { address, context });
    }

    pub(crate) fn address(&self, ticket: Ticket) -> Option<Address> {
        self.entries.get(&ticket).map(|entry| entry.address)
    }

    pub(crate) fn context(&self, ticket: Ticket) -> Option<&C> {
        self.entries.get(&ticket).map(|entry| &entry.context)
    }

    pub(crate) fn tickets_on(&self, backend: BackendId) -> impl Iterator<Item = Ticket> + '_ {
        self.entries.iter().filter_map(move |(&ticket, entry)| {
            (entry.address.backend == backend).then_some(ticket)
        })
    }

    pub(crate) fn drop_ticket(&mut self, ticket: Ticket) {
        self.entries.remove(&ticket);
    }

    /// The tickets in flight, in order.
    pub fn tickets(&self) -> impl Iterator<Item = Ticket> + '_ {
        self.entries.keys().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (Ticket, &mut C)> {
        self.entries
            .iter_mut()
            .map(|(ticket, entry)| (*ticket, &mut entry.context))
    }

    pub fn contains(&self, ticket: Ticket) -> bool {
        self.entries.contains_key(&ticket)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Drops the tickets whose context `keep` refuses: their requests were
    /// superseded and will never answer.
    pub fn retain(&mut self, keep: impl Fn(&C) -> bool) {
        self.entries.retain(|_, entry| keep(&entry.context));
    }

    pub(crate) fn retain_tickets(&mut self, keep: impl Fn(Ticket, &C) -> bool) {
        self.entries
            .retain(|ticket, entry| keep(*ticket, &entry.context));
    }

    pub(crate) fn find_mut(&mut self, predicate: impl Fn(&C) -> bool) -> Option<&mut C> {
        self.entries
            .values_mut()
            .map(|entry| &mut entry.context)
            .find(|ctx| predicate(ctx))
    }

    pub fn any(&self, f: impl Fn(&C) -> bool) -> bool {
        self.entries.values().any(|entry| f(&entry.context))
    }

    pub(crate) fn any_on(&self, backend: BackendId, f: impl Fn(&C) -> bool) -> bool {
        self.entries
            .values()
            .any(|entry| entry.address.backend == backend && f(&entry.context))
    }

    /// The context of `ticket` and what `reply` says, if the ticket is
    /// held; the ticket is spent. A stale ticket gives `None` (R1).
    pub(crate) fn resolve(&mut self, ticket: Ticket, reply: Reply) -> Option<(C, Answer)> {
        let (ctx, result) = self.resolve_reply(ticket, reply)?;
        let answer = match result {
            ReplyResult::Answer(answer) => answer,
            ReplyResult::SubmissionUnconfirmed(message) => {
                Answer::Failed(RpcError::Remote(message))
            }
            ReplyResult::CloudSubmission(_) => Answer::Failed(RpcError::Mismatch),
        };
        Some((ctx, answer))
    }

    pub(crate) fn resolve_reply(
        &mut self,
        ticket: Ticket,
        reply: Reply,
    ) -> Option<(C, ReplyResult)> {
        self.entries
            .remove(&ticket)
            .map(|entry| (entry.context, reply.0))
    }
}
