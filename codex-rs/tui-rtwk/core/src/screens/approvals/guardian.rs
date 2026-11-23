//! The auto-reviewer (the oracle's guardian): the reviews under way, the denials worth a retry, and
//! what a verdict leaves in the history. Oracle: `chatwidget/tool_requests.rs`
//! (`on_guardian_assessment`), `auto_review_denials.rs`, `chatwidget/status_state.rs`
//! (`PendingGuardianReviewStatus`), `chatwidget/protocol_requests.rs` (`on_guardian_review_notification`),
//! `chatwidget/permission_popups.rs` (`approve_recent_auto_review_denial`).
//!
//! The model says WHAT is reviewed and what came of it; the words (the status line, the denial
//! rows, the cells) are the view's (`sheets/approvals/`). The thread's slot is `ThreadPart`
//! (`approvals.rs`); this file is its guardian half, so there is one writer: `ThreadPart::apply`.

use std::collections::VecDeque;

use codex_app_server_protocol as protocol;
use codex_app_server_protocol::ServerNotification;
use codex_protocol::approvals as core;

use super::decided::{Actor, Decided, Outcome, Reviewed, Subject, Verdict};
use super::{ThreadEv, ThreadPart, strip_shell_wrapper};
use crate::ids::ThreadId;

/// Denials kept for `/auto-review` (`MAX_RECENT_DENIALS`).
const MAX_RECENT_DENIALS: usize = 10;

/// The guardian warning that says a review approved is not shown (`protocol.rs`
/// `ServerNotification::GuardianWarning`).
const APPROVED_WARNING: &str = "Automatic approval review approved (";

/// How a review stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    InProgress,
    Approved,
    Denied,
    TimedOut,
    Aborted,
}

/// What a review is about (`GuardianApprovalReviewAction`, reduced to what is shown and kept).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// A command line as the server gave it (an execve joins its arguments).
    Command { line: String },
    /// Input for a running terminal.
    WriteStdin { process: String, input: String },
    /// A patch; the files as the UI spells them.
    ApplyPatch { files: Vec<String> },
    NetworkAccess { target: String },
    McpToolCall {
        server: String,
        tool: String,
        /// The connector's name, shown in place of the server's when there is one.
        connector: Option<String>,
    },
    RequestPermissions { reason: Option<String> },
}

impl Action {
    fn of(action: &protocol::GuardianApprovalReviewAction) -> Self {
        use protocol::GuardianApprovalReviewAction as A;
        match action {
            A::Command { command, .. } => Action::Command { line: command.clone() },
            A::Execve { program, argv, .. } => Action::Command {
                line: if argv.is_empty() {
                    program.clone()
                } else {
                    join_words(argv)
                },
            },
            A::WriteStdin { process_id, stdin, .. } => Action::WriteStdin {
                process: process_id.clone(),
                input: stdin.clone(),
            },
            A::ApplyPatch { files, .. } => {
                // The files as the UI spells them (`render_for_ui`).
                let mut names = Vec::with_capacity(files.len());
                for file in files {
                    names.push(file.render_for_ui());
                }
                Action::ApplyPatch { files: names }
            }
            A::NetworkAccess { target, .. } => Action::NetworkAccess { target: target.clone() },
            A::McpToolCall { server, tool_name, connector_name, .. } => Action::McpToolCall {
                server: server.clone(),
                tool: tool_name.clone(),
                connector: connector_name.clone(),
            },
            A::RequestPermissions { reason, .. } => {
                Action::RequestPermissions { reason: reason.clone() }
            }
        }
    }

    /// What the history says was reviewed (`guardian_command` and the arms after it).
    fn reviewed(&self) -> Result<String, Reviewed> {
        Err(match self {
            Action::Command { line } => return Ok(strip_shell_wrapper(line)),
            Action::WriteStdin { process, input } => Reviewed::Stdin {
                process: process.clone(),
                input: input.clone(),
            },
            Action::ApplyPatch { files } => Reviewed::Patch(files.clone()),
            Action::NetworkAccess { target } => Reviewed::Network(target.clone()),
            Action::McpToolCall { server, tool, .. } => Reviewed::Mcp {
                server: server.clone(),
                tool: tool.clone(),
            },
            Action::RequestPermissions { reason } => Reviewed::Permissions(reason.clone()),
        })
    }
}

/// `shlex::try_join` for the arguments of an execve: a word with a space, a quote or another
/// character a shell would read gets single quotes.
fn join_words(words: &[String]) -> String {
    words
        .iter()
        .map(|word| {
            let plain = !word.is_empty()
                && word.chars().all(|c| {
                    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ',' | ':' | '+' | '@' | '%' | '=')
                });
            if plain {
                word.clone()
            } else {
                format!("'{}'", word.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One state of a review, from `item/autoApprovalReview/started` or `.../completed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    pub id: String,
    pub turn: String,
    pub status: Status,
    pub action: Action,
    pub rationale: Option<String>,
    /// The core `GuardianAssessmentEvent`, serialized: what `thread/approveGuardianDeniedAction`
    /// carries back.
    event: serde_json::Value,
}

/// A review under way: the status line lists these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reviewing {
    pub id: String,
    pub action: Action,
}

/// A denied review kept for `/auto-review`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Denial {
    pub id: String,
    pub action: Action,
    pub rationale: Option<String>,
    event: serde_json::Value,
}

/// The ten latest denials, newest first (`RecentAutoReviewDenials`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Denials(VecDeque<Denial>);

impl Denials {
    fn push(&mut self, denial: Denial) {
        self.0.retain(|kept| kept.id != denial.id);
        self.0.push_front(denial);
        self.0.truncate(MAX_RECENT_DENIALS);
    }

    fn take(&mut self, id: &str) -> Option<Denial> {
        let at = self.0.iter().position(|denial| denial.id == id)?;
        self.0.remove(at)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Denial> {
        self.0.iter()
    }
}

/// What the thread should append to its history because of an event; the owner of the thread's
/// items reads it with `ThreadPart::take_history` after every apply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum History {
    /// An answer or a verdict (`Item::Decision`).
    Decided(Decided),
    /// A user-input exchange ended: its questions with what was answered (`Notice::Questions`).
    Questioned(Vec<crate::screens::cells::AnsweredQuestion>),
    /// The server's warning, verbatim.
    Warning(String),
    /// The server's file paths of a review could not be localized: the error text it gave.
    Unlocalized(String),
    /// `/auto-review` with no denial kept (an info line with a dim detail).
    NoDenials,
    /// One retry of a denial was approved (an info line with a dim detail).
    RetryApproved,
    /// The denial picked is no longer kept (an error line).
    DenialGone,
}

/// What the part asks the server for; `apply_thread` makes the request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Approve one retry of a denied action: `thread/approveGuardianDeniedAction`.
    ApproveDenied { id: String, event: serde_json::Value },
}

/// The thread event of a guardian notification (the three arms `targets.rs`'s
/// `notification_thread` and `screens::project_thread` need to route).
pub fn project(notification: &ServerNotification) -> Option<(ThreadId, ThreadEv)> {
    match notification {
        ServerNotification::ItemGuardianApprovalReviewStarted(n) => Some((
            ThreadId::new(&n.thread_id),
            review_ev(&n.review_id, &n.turn_id, n.started_at_ms, &n.review, None, &n.action),
        )),
        ServerNotification::ItemGuardianApprovalReviewCompleted(n) => Some((
            ThreadId::new(&n.thread_id),
            review_ev(
                &n.review_id,
                &n.turn_id,
                n.started_at_ms,
                &n.review,
                Some((n.completed_at_ms, n.decision_source)),
                &n.action,
            ),
        )),
        ServerNotification::GuardianWarning(n) => Some((
            ThreadId::new(&n.thread_id),
            ThreadEv::Warning(n.message.clone()),
        )),
        _ => None,
    }
}

/// `on_guardian_review_notification`: the protocol's review as the core's assessment event (kept
/// for the retry), or the error the oracle shows when the paths cannot be localized.
fn review_ev(
    id: &str,
    turn: &str,
    started_at_ms: i64,
    review: &protocol::GuardianApprovalReview,
    completion: Option<(i64, protocol::AutoReviewDecisionSource)>,
    action: &protocol::GuardianApprovalReviewAction,
) -> ThreadEv {
    let core_action: Result<core::GuardianAssessmentAction, _> = action.clone().try_into();
    let core_action = match core_action {
        Ok(action) => action,
        Err(error) => return ThreadEv::Unlocalized(error.to_string()),
    };
    let status = match review.status {
        protocol::GuardianApprovalReviewStatus::InProgress => core::GuardianAssessmentStatus::InProgress,
        protocol::GuardianApprovalReviewStatus::Approved => core::GuardianAssessmentStatus::Approved,
        protocol::GuardianApprovalReviewStatus::Denied => core::GuardianAssessmentStatus::Denied,
        protocol::GuardianApprovalReviewStatus::TimedOut => core::GuardianAssessmentStatus::TimedOut,
        protocol::GuardianApprovalReviewStatus::Aborted => core::GuardianAssessmentStatus::Aborted,
    };
    let event = core::GuardianAssessmentEvent {
        review_reason: None,
        model_context: None,
        id: id.to_string(),
        target_item_id: None,
        plugin_id: None,
        script_path: None,
        turn_id: turn.to_string(),
        started_at_ms,
        completed_at_ms: completion.map(|(at, _)| at),
        status,
        risk_level: review.risk_level.map(|level| match level {
            protocol::GuardianRiskLevel::Low => core::GuardianRiskLevel::Low,
            protocol::GuardianRiskLevel::Medium => core::GuardianRiskLevel::Medium,
            protocol::GuardianRiskLevel::High => core::GuardianRiskLevel::High,
            protocol::GuardianRiskLevel::Critical => core::GuardianRiskLevel::Critical,
        }),
        user_authorization: review.user_authorization.map(|level| match level {
            protocol::GuardianUserAuthorization::Unknown => core::GuardianUserAuthorization::Unknown,
            protocol::GuardianUserAuthorization::Low => core::GuardianUserAuthorization::Low,
            protocol::GuardianUserAuthorization::Medium => core::GuardianUserAuthorization::Medium,
            protocol::GuardianUserAuthorization::High => core::GuardianUserAuthorization::High,
        }),
        rationale: review.rationale.clone(),
        decision_source: completion.map(|(_, source)| match source {
            protocol::AutoReviewDecisionSource::Agent => core::GuardianAssessmentDecisionSource::Agent,
        }),
        action: core_action,
    };
    ThreadEv::Review(Box::new(Review {
        id: id.to_string(),
        turn: turn.to_string(),
        status: match review.status {
            protocol::GuardianApprovalReviewStatus::InProgress => Status::InProgress,
            protocol::GuardianApprovalReviewStatus::Approved => Status::Approved,
            protocol::GuardianApprovalReviewStatus::Denied => Status::Denied,
            protocol::GuardianApprovalReviewStatus::TimedOut => Status::TimedOut,
            protocol::GuardianApprovalReviewStatus::Aborted => Status::Aborted,
        },
        action: Action::of(action),
        rationale: review.rationale.clone(),
        event: serde_json::to_value(&event).unwrap_or(serde_json::Value::Null),
    }))
}

impl Review {
    /// The history record of a verdict that leaves one (denied, timed out); an approval, an
    /// abort and a review still going leave none.
    fn record(&self) -> Option<Decided> {
        let verdict = match self.status {
            Status::Denied => Verdict::Denied,
            Status::TimedOut => Verdict::TimedOut,
            Status::InProgress | Status::Approved | Status::Aborted => return None,
        };
        Some(match self.action.reviewed() {
            Ok(command) => Decided::Request {
                actor: Actor::Guardian,
                subject: Subject::Command(command),
                outcome: match verdict {
                    Verdict::Denied => Outcome::Denied,
                    Verdict::TimedOut => Outcome::TimedOut,
                },
            },
            Err(about) => Decided::Reviewed { verdict, about },
        })
    }
}

impl ThreadPart {
    /// The one place this part changes. What the thread should append comes out of
    /// `take_history`; what the server must be asked comes back.
    pub fn apply(&mut self, ev: ThreadEv) -> Option<Ask> {
        match ev {
            ThreadEv::Decided(decided) => self.outbox.push(History::Decided(decided)),
            ThreadEv::Questioned(questions) => self.outbox.push(History::Questioned(questions)),
            ThreadEv::Review(review) => self.review(*review),
            ThreadEv::Warning(message) => {
                if !message.starts_with(APPROVED_WARNING) {
                    self.outbox.push(History::Warning(message));
                }
            }
            ThreadEv::Unlocalized(error) => self.outbox.push(History::Unlocalized(error)),
            ThreadEv::TurnEnded => self.reviews.clear(),
            ThreadEv::NoDenials => self.outbox.push(History::NoDenials),
            ThreadEv::ApproveDenied { id } => {
                return match self.recent_denials.take(&id) {
                    Some(denial) => {
                        self.outbox.push(History::RetryApproved);
                        Some(Ask::ApproveDenied { id, event: denial.event })
                    }
                    None => {
                        self.outbox.push(History::DenialGone);
                        None
                    }
                };
            }
        }
        None
    }

    fn review(&mut self, review: Review) {
        if review.status == Status::InProgress {
            // A review under way owns the status; parallel ones are listed in arrival order.
            match self.reviews.iter_mut().find(|kept| kept.id == review.id) {
                Some(kept) => kept.action = review.action,
                None => self.reviews.push(Reviewing { id: review.id, action: review.action }),
            }
            return;
        }
        self.reviews.retain(|kept| kept.id != review.id);
        if let Some(record) = review.record() {
            if review.status == Status::Denied {
                self.recent_denials.push(Denial {
                    id: review.id,
                    action: review.action,
                    rationale: review.rationale,
                    event: review.event,
                });
            }
            self.outbox.push(History::Decided(record));
        }
    }

    /// What `apply` left for the thread's history since the last call.
    pub fn take_history(&mut self) -> Vec<History> {
        std::mem::take(&mut self.outbox)
    }
}
