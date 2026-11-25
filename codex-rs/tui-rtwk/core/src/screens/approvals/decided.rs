//! What the thread's history keeps of an answer (the oracle's `new_approval_decision_cell` and
//! the plain permissions line, `bottom_pane/approval_overlay.rs` `handle_exec_decision` and
//! `handle_permissions_decision`).
//!
//! The model says WHAT happened: who decided (the user, or the auto-reviewer), about what (a
//! command, or network access to a target) and how. The words, the colours and the 80-grapheme
//! snippet are the view's (`sheets/approvals/decision_cell.rs`).
//!
//! The oracle records an answer only for an exec request (commands and network access) and for a
//! permissions request, and only when the request came from the thread that is shown (no thread
//! label). A patch, a form and a question leave nothing.

use super::{Decision, ExecPrompt, Prompt};

/// Who decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    User,
    /// The auto-reviewer (the guardian).
    Guardian,
}

/// What the request was about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    /// A command, shell wrapper stripped. Empty: the cell says "this request".
    Command(String),
    /// Network access to `scheme://host[:port]`.
    Network { target: String },
}

/// How it was decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Approved,
    ApprovedForSession,
    /// Approved, and commands that start with this prefix from now on.
    ApprovedWithPrefix(String),
    /// A persistent network rule: the host allowed, or denied and saved.
    NetworkRule {
        host: String,
        allow: bool,
    },
    Denied,
    /// The review did not finish in time (the auto-reviewer's).
    TimedOut,
    /// The user canceled the request.
    Aborted,
}

/// What a permissions request granted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grant {
    Nothing,
    Turn,
    /// For the turn, with strict auto-review.
    Strict,
    Session,
}

/// What the auto-reviewer concluded without letting the action through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Denied,
    TimedOut,
}

/// What the auto-reviewer judged when it was not a command (`new_guardian_denied_*_request`,
/// `new_guardian_timed_out_*_request`): each has its own sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reviewed {
    /// A patch touching these files.
    Patch(Vec<String>),
    /// Input for the terminal of this process; `input` is the raw text.
    Stdin {
        process: String,
        input: String,
    },
    Mcp {
        server: String,
        tool: String,
    },
    Network(String),
    Permissions(Option<String>),
}

/// One answer in the thread's history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decided {
    Request {
        actor: Actor,
        subject: Subject,
        outcome: Outcome,
    },
    Permissions(Grant),
    /// The auto-reviewer refused, or ran out of time on, something that is not a command.
    Reviewed {
        verdict: Verdict,
        about: Reviewed,
    },
}

impl Decided {
    /// What answering `prompt` with `decision` leaves in the history, if anything.
    pub fn of(prompt: &Prompt, decision: &Decision) -> Option<Decided> {
        if prompt.thread_label().is_some() {
            return None;
        }
        match prompt {
            Prompt::Exec(exec) => Some(Decided::Request {
                actor: Actor::User,
                subject: subject_of(exec),
                outcome: outcome_of(decision)?,
            }),
            Prompt::Permissions(_) => Some(Decided::Permissions(match decision {
                Decision::GrantForTurn => Grant::Turn,
                Decision::GrantForTurnStrict => Grant::Strict,
                Decision::GrantForSession => Grant::Session,
                Decision::DenyPermissions => Grant::Nothing,
                _ => return None,
            })),
            Prompt::Patch(_)
            | Prompt::Elicitation(_)
            | Prompt::UserInput(_)
            | Prompt::UserVerification(_)
            | Prompt::AppLink(_) => None,
        }
    }
}

fn subject_of(exec: &ExecPrompt) -> Subject {
    match &exec.network_target {
        Some(target) => Subject::Network {
            target: target.clone(),
        },
        None => Subject::Command(exec.command.clone()),
    }
}

/// `command_decision_to_review_decision`: the exec decisions the app answers with. `AllowHost`
/// has no answer on the wire, so it has no record.
fn outcome_of(decision: &Decision) -> Option<Outcome> {
    Some(match decision {
        Decision::Accept => Outcome::Approved,
        Decision::AcceptForSession => Outcome::ApprovedForSession,
        Decision::AcceptWithPrefix(prefix) => Outcome::ApprovedWithPrefix(prefix.clone()),
        Decision::NetworkRule { host, allow } => Outcome::NetworkRule {
            host: host.clone(),
            allow: *allow,
        },
        Decision::Decline => Outcome::Denied,
        Decision::Cancel => Outcome::Aborted,
        _ => return None,
    })
}
