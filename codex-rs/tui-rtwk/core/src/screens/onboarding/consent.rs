//! Folder trust after launch. The app keeps the navigation action while
//! onboarding correlates the host's checks, the question and any trust write.

use super::{Ask, Model, Request, TrustAsk, TrustCancel, TrustDecision, ask};
use crate::app::AppFx;
use crate::ids::ThreadId;

/// How the host finds the folder whose trust is needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// The specified folder, or the launch folder when absent.
    Folder(Option<String>),
    /// The working directory of a saved task.
    Task(ThreadId),
    /// After consenting to a local daemon's task folder, check its latest cwd.
    TaskRecheck {
        thread: ThreadId,
        checked: Vec<String>,
    },
    /// A new conversation on the primary backend, using its own workspace.
    BackendSwitch,
}

/// How the question ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The folder needs no more question (trusted, or answered): go on.
    Go(Option<Checked>),
    Cancelled,
    /// The folder's trust could not be read.
    Failed(String),
}

/// A cwd read by the host that the action needs once its consent is clear.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checked {
    /// After a daemon task's prompt, re-read its latest cwd before resuming.
    Task(String),
    /// The daemon's latest checked cwd is the resume override.
    ResumeIn(String),
    /// Start on the destination backend in this exact directory.
    Destination(String),
}

/// Where the question stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Phase {
    #[default]
    Idle,
    /// The host reads the folder's trust.
    Checking { request: Request, new_route: bool },
    /// The question is on screen; `writing` is the trust write in flight
    /// (the question waits for it: R9).
    Asking {
        ask: TrustAsk,
        checked: Option<Checked>,
        writing: Option<Request>,
    },
    /// Ended, not yet taken by the app.
    Settled(Outcome),
}

/// The sub-state: whether the host answers the question, how many were asked,
/// and where the open one stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Consent {
    armed: bool,
    generation: u64,
    return_on_cancel: bool,
    return_to_center: bool,
    phase: Phase,
}

/// Whether the host answers folder trust lookups.
pub fn arm(model: &mut Model, on: bool) {
    model.consent.armed = on;
}

/// True while a check or question is holding an action.
pub fn pending(model: &Model) -> bool {
    model.consent.phase != Phase::Idle
}

/// Ask the host to check the folder if this host supports checks.
pub fn begin(
    model: &mut Model,
    target: Target,
    cancel: TrustCancel,
    return_on_cancel: bool,
    fx: &mut AppFx,
) -> bool {
    if !model.consent.armed {
        return false;
    }
    model.consent.return_to_center = false;
    model.consent.return_on_cancel = return_on_cancel;
    let request = fx.host_request_number();
    let new_route = !matches!(target, Target::TaskRecheck { .. });
    model.consent.phase = Phase::Checking { request, new_route };
    ask(
        fx,
        Ask::CheckConsent {
            request,
            target,
            cancel,
        },
    );
    true
}

/// The question on screen, if one is.
pub fn open(model: &Model) -> Option<&TrustAsk> {
    match &model.consent.phase {
        Phase::Asking { ask, .. } => Some(ask),
        _ => None,
    }
}

/// Which question is on screen (it counts up with each one): the page that
/// shows it is pushed once per number.
pub fn open_number(model: &Model) -> Option<u64> {
    open(model).map(|_| model.consent.generation)
}

/// The fresh-session question was declined without a session to return to.
pub fn return_to_center(model: &Model) -> bool {
    model.consent.return_to_center
}

/// How the question ended, once: the app goes on with it.
pub fn take(model: &mut Model) -> Option<Outcome> {
    match std::mem::take(&mut model.consent.phase) {
        Phase::Settled(outcome) => Some(outcome),
        phase => {
            model.consent.phase = phase;
            None
        }
    }
}

/// Whether an outcome waits to be taken.
pub fn settled(model: &Model) -> bool {
    matches!(model.consent.phase, Phase::Settled(_))
}

/// The host's answer to `Ask::CheckConsent`: the question to ask, or none
/// when the folder needs none (a stale answer is dropped).
pub(super) fn answered(
    model: &mut Model,
    request: Request,
    question: Option<TrustAsk>,
    checked: Option<Checked>,
) {
    let Phase::Checking {
        request: asked,
        new_route,
    } = model.consent.phase
    else {
        return;
    };
    if asked != request {
        return;
    }
    model.consent.phase = match question {
        None => Phase::Settled(Outcome::Go(checked)),
        Some(ask) => {
            model.consent.generation += u64::from(new_route);
            Phase::Asking {
                ask,
                checked,
                writing: None,
            }
        }
    };
}

fn checking(model: &Model, request: Request) -> bool {
    matches!(model.consent.phase, Phase::Checking { request: asked, .. } if asked == request)
}

pub(super) fn waits_for(model: &Model, request: Request) -> bool {
    checking(model, request) || writes(model, request)
}

/// The host could not read the folder's trust.
pub(super) fn failed(model: &mut Model, request: Request, message: String) {
    if checking(model, request) {
        model.consent.phase = Phase::Settled(Outcome::Failed(message));
    }
}

/// The first choice of an open question: a restricted folder opens as it is,
/// trust is written and waited for. False when no question is open.
pub(super) fn trust(model: &mut Model, fx: &mut AppFx) -> bool {
    let Phase::Asking {
        ask: question,
        writing,
        ..
    } = &mut model.consent.phase
    else {
        return false;
    };
    if question.decision.is_some() {
        return true;
    }
    question.error = None;
    if question.restricted {
        question.decision = Some(TrustDecision::Opened);
        settle(model);
        return true;
    }
    question.decision = Some(TrustDecision::Saving);
    let request = fx.host_request_number();
    *writing = Some(request);
    ask(
        fx,
        Ask::WriteTrust {
            request,
            target: question.target.clone(),
        },
    );
    true
}

/// Cancel the question unless a write is in flight. True if it consumed the choice.
pub(super) fn cancel(model: &mut Model) -> bool {
    match &model.consent.phase {
        Phase::Asking { ask, .. } if ask.decision == Some(TrustDecision::Saving) => {}
        Phase::Checking { .. } | Phase::Asking { .. } | Phase::Settled(_) => {
            model.consent.return_to_center = model.consent.return_on_cancel;
            model.consent.phase = Phase::Settled(Outcome::Cancelled);
        }
        _ => return false,
    }
    true
}

/// The write `request` is confirmed: the folder is trusted and what waited
/// goes on. False when it is not this question's write.
pub(super) fn saved(model: &mut Model, request: Request) -> bool {
    if !writes(model, request) {
        return false;
    }
    if let Phase::Asking { ask, .. } = &mut model.consent.phase {
        ask.decision = Some(TrustDecision::Trusted);
    }
    settle(model);
    true
}

/// The write `request` failed: the question stays, with the message.
pub(super) fn save_failed(model: &mut Model, request: Request, message: String) -> bool {
    if !writes(model, request) {
        return false;
    }
    if let Phase::Asking { ask, writing, .. } = &mut model.consent.phase {
        *writing = None;
        ask.decision = None;
        ask.error = Some(message);
    }
    true
}

fn writes(model: &Model, request: Request) -> bool {
    matches!(
        &model.consent.phase,
        Phase::Asking { writing: Some(w), .. } if *w == request
    )
}

/// The question was answered in favour: what waited goes on.
fn settle(model: &mut Model) {
    if let Phase::Asking { checked, .. } = std::mem::take(&mut model.consent.phase) {
        model.consent.phase = Phase::Settled(Outcome::Go(checked));
    }
}

/// The connection was lost: the read or the write will not be answered, so
/// the question and what waited are dropped, and a late answer with it.
pub fn abandon(model: &mut Model) {
    model.consent.return_to_center = false;
    model.consent.phase = Phase::Idle;
}
