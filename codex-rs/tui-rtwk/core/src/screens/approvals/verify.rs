//! The user-verification controller's state, pure (the oracle's `app/user_verification_requests.rs`
//! and `user_verification_errors.rs`): the challenge of a request stays here, never in a view;
//! an attempt is a generation, so a late result of an attempt that was cancelled, resolved or
//! left behind by a reconnect cannot answer a request that surfaced afterwards.
//!
//! The app owns one `Verifications` (ADR-003: one writer) and drives it: `note` when the request
//! arrives, `begin` when the user chooses to verify (a second `begin` of the same request is
//! refused), `is_pending` when the device's result comes back. What the device (the RPC
//! `userVerification/verify`) does is host work; the words of a failure are the view's.

use std::collections::HashMap;

use codex_app_server_protocol::{
    McpServerElicitationRequest, ServerRequest, UserVerificationCancellationReason,
    UserVerificationErrorDetails, UserVerificationFailureReason,
    UserVerificationInvalidRequestReason, UserVerificationProof,
    UserVerificationUnavailableReason, UserVerificationVerifyParams,
};

use super::{Decision, RequestAnswer};
use crate::ids::{RequestKey, ThreadId};

/// One try to verify; a number that is never used twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttemptId(u64);

/// What `begin` hands the host: the attempt and what the device signs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    pub id: AttemptId,
    pub params: UserVerificationVerifyParams,
}

#[derive(Debug)]
struct Pending {
    params: UserVerificationVerifyParams,
    attempt: Option<AttemptId>,
}

#[derive(Debug, Default)]
pub struct Verifications {
    pending: HashMap<RequestKey, Pending>,
    issued: u64,
}

impl Verifications {
    /// The request arrived: a user verification keeps its challenge here, anything else is left.
    pub fn note(&mut self, key: RequestKey, request: &ServerRequest) {
        let ServerRequest::McpServerElicitationRequest { params, .. } = request else {
            return;
        };
        let McpServerElicitationRequest::UserVerification {
            title,
            description,
            challenge,
            ..
        } = &params.request
        else {
            return;
        };
        self.pending.insert(
            key,
            Pending {
                params: UserVerificationVerifyParams {
                    title: title.clone(),
                    description: description.clone(),
                    challenge: challenge.clone(),
                },
                attempt: None,
            },
        );
    }

    /// The user chose to verify: a new attempt, once per request (`None`: not a pending
    /// verification, or one is under way).
    pub fn begin(&mut self, key: &RequestKey) -> Option<Attempt> {
        let pending = self.pending.get_mut(key)?;
        if pending.attempt.is_some() {
            return None;
        }
        self.issued += 1;
        let id = AttemptId(self.issued);
        pending.attempt = Some(id);
        Some(Attempt {
            id,
            params: pending.params.clone(),
        })
    }

    /// Whether the result of `attempt` may still answer: the request is still pending and this
    /// is its attempt.
    pub fn is_pending(&self, key: &RequestKey, attempt: AttemptId) -> bool {
        self.pending
            .get(key)
            .is_some_and(|pending| pending.attempt == Some(attempt))
    }

    /// The request is answered or the server resolved it: its attempt, if any, is void.
    pub fn remove(&mut self, key: &RequestKey) {
        self.pending.remove(key);
    }

    /// A thread was discarded: its verifications go, those of other threads stay. Returns the
    /// requests that went.
    pub fn cancel_thread(&mut self, thread: &ThreadId) -> Vec<RequestKey> {
        let gone: Vec<RequestKey> = self
            .pending
            .keys()
            .filter(|key| &key.thread == thread)
            .cloned()
            .collect();
        for key in &gone {
            self.pending.remove(key);
        }
        gone
    }

    /// The connection was replaced: nothing of the old one may answer.
    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

/// How a verification ended, what the controller answers the request with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The device signed: the proof goes to the server and nowhere else.
    Accepted(UserVerificationProof),
    /// The user cancelled (Esc, `c`, `x`, in both states).
    Cancelled,
    /// The device could not complete it: the request is cancelled with the closed reason.
    Failed(UserVerificationErrorDetails),
}

impl Outcome {
    /// The answer to the elicitation (oracle `AppServerRequests::take_resolution`): accept with
    /// the proof as `content`; cancel with `_meta` `openai/userVerificationReason` (the closed
    /// reason only, never a display message).
    pub fn answer(&self) -> Option<RequestAnswer> {
        let reason = |reason: serde_json::Value| {
            Some(serde_json::json!({ "openai/userVerificationReason": reason }))
        };
        let (decision, content, meta) = match self {
            Outcome::Accepted(proof) => (Decision::Accept, Some(serde_json::to_value(proof).ok()?), None),
            Outcome::Cancelled => (Decision::Cancel, None, reason("userCancelled".into())),
            Outcome::Failed(details) => (
                Decision::Cancel,
                None,
                reason(serde_json::to_value(details).ok()?.get("reason")?.clone()),
            ),
        };
        Some(RequestAnswer::Elicitation {
            decision,
            content,
            meta,
        })
    }
}

/// Why a verification failed, as the view words it (`Strings::verification_failure`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The request never reached the local binary's verification.
    Unreachable,
    InvalidParams,
    CredentialMissing,
    BiometricsUnavailable,
    ProviderUnavailable,
    UserCancelled,
    Interrupted,
    AuthenticationFailed,
    Timeout,
    /// A provider or service error, or a payload that is not one of the closed categories.
    Service,
}

/// A failure: the cause for the words, the closed details for the server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub cause: Cause,
    pub details: UserVerificationErrorDetails,
}

impl Failure {
    /// The error was not the server's (a transport error): nothing to read.
    pub fn unreachable() -> Failure {
        Failure {
            cause: Cause::Unreachable,
            details: UserVerificationErrorDetails::Failed {
                reason: UserVerificationFailureReason::ServiceError,
            },
        }
    }

    /// The error the server answered with: only the typed `data` counts, its `message` (native
    /// diagnostics) is never read; a payload that is not a closed category is a service error.
    pub fn of_server(data: Option<&serde_json::Value>) -> Failure {
        use UserVerificationErrorDetails as D;
        let details = data.and_then(|data| serde_json::from_value::<D>(data.clone()).ok());
        let cause = match details {
            Some(D::InvalidRequest {
                reason: UserVerificationInvalidRequestReason::InvalidParams,
            }) => Cause::InvalidParams,
            Some(D::Unavailable { reason }) => match reason {
                UserVerificationUnavailableReason::CredentialMissing => Cause::CredentialMissing,
                UserVerificationUnavailableReason::BiometricsUnavailable => {
                    Cause::BiometricsUnavailable
                }
                UserVerificationUnavailableReason::ProviderUnavailable => {
                    Cause::ProviderUnavailable
                }
            },
            Some(D::Cancelled { reason }) => match reason {
                UserVerificationCancellationReason::UserCancelled => Cause::UserCancelled,
                UserVerificationCancellationReason::Interrupted => Cause::Interrupted,
            },
            Some(D::Failed { reason }) => match reason {
                UserVerificationFailureReason::AuthenticationFailed => Cause::AuthenticationFailed,
                UserVerificationFailureReason::Timeout => Cause::Timeout,
                UserVerificationFailureReason::ProviderError
                | UserVerificationFailureReason::ServiceError => Cause::Service,
            },
            None => Cause::Service,
        };
        Failure {
            cause,
            details: details.unwrap_or(D::Failed {
                reason: UserVerificationFailureReason::ServiceError,
            }),
        }
    }
}
