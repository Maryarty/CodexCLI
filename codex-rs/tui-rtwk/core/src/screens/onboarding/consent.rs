//! Folder trust for navigation after launch. The onboarding model holds the
//! action until the host's lookup and any trust write have finished. A
//! separate route shows the question without restarting onboarding.

use super::{Ask, Model, Request, TrustAsk, TrustCancel, TrustDecision, ask, mint};
use crate::app::AppFx;
use crate::features::FeatureCtx;
use crate::ids::ThreadId;
use crate::threads::ThreadAsk;

/// How the host finds the folder whose trust is needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// The specified folder, or the launch folder when absent.
    Folder(Option<String>),
    /// The working directory of a saved task.
    Task(ThreadId),
}

/// What goes on once the folder is decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Then {
    /// `/cd`: the session starts in the folder, as `source`'s fork or as a
    /// fresh thread (`ask` is what `/cd` built).
    Cd {
        source: Option<ThreadId>,
        ask: ThreadAsk,
    },
    Open(ThreadId),
    New {
        cwd: Option<String>,
    },
    Fresh {
        feature: Option<FeatureCtx>,
    },
}

/// How the question ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The folder needs no more question (trusted, or answered): go on.
    Go(Then),
    Cancelled(Then),
    /// The folder's trust could not be read: the message, and what waited.
    Failed {
        message: String,
        then: Then,
    },
}

/// Where the question stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Phase {
    #[default]
    Idle,
    /// The host reads the folder's trust.
    Checking { request: Request, then: Then },
    /// The question is on screen; `writing` is the trust write in flight
    /// (the question waits for it: R9).
    Asking {
        ask: TrustAsk,
        then: Then,
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

/// Hold the action while the folder is checked. Returns the action only if
/// the host has no consent lookup. Repeated requests while one is held are dropped.
pub fn begin(
    model: &mut Model,
    then: Then,
    target: Target,
    cancel: TrustCancel,
    fx: &mut AppFx,
) -> Option<Then> {
    if !model.consent.armed {
        return Some(then);
    }
    if pending(model) {
        return None;
    }
    model.consent.return_to_center = false;
    let request = mint(model);
    model.consent.phase = Phase::Checking { request, then };
    ask(
        fx,
        Ask::CheckConsent {
            request,
            target,
            cancel,
        },
    );
    None
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
pub(super) fn answered(model: &mut Model, request: Request, question: Option<TrustAsk>) {
    let Phase::Checking { request: asked, .. } = &model.consent.phase else {
        return;
    };
    if *asked != request {
        return;
    }
    let Phase::Checking { then, .. } = std::mem::take(&mut model.consent.phase) else {
        return;
    };
    model.consent.phase = match question {
        None => Phase::Settled(Outcome::Go(then)),
        Some(ask) => {
            model.consent.generation += 1;
            Phase::Asking {
                ask,
                then,
                writing: None,
            }
        }
    };
}

/// The host could not read the folder's trust.
pub(super) fn failed(model: &mut Model, request: Request, message: String) {
    let Phase::Checking { request: asked, .. } = &model.consent.phase else {
        return;
    };
    if *asked != request {
        return;
    }
    let Phase::Checking { then, .. } = std::mem::take(&mut model.consent.phase) else {
        return;
    };
    model.consent.phase = Phase::Settled(Outcome::Failed { message, then });
}

/// The first choice of an open question: a restricted folder opens as it is,
/// trust is written and waited for. False when no question is open.
pub(super) fn trust(model: &mut Model, fx: &mut AppFx) -> bool {
    let request = mint_if_asking(model);
    let Phase::Asking { ask: question, .. } = &mut model.consent.phase else {
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
    let target = question.target.clone();
    if let Phase::Asking { writing, .. } = &mut model.consent.phase {
        *writing = request;
    }
    if let Some(request) = request {
        ask(fx, Ask::WriteTrust { request, target });
    }
    true
}

fn mint_if_asking(model: &mut Model) -> Option<Request> {
    matches!(model.consent.phase, Phase::Asking { .. }).then(|| mint(model))
}

/// Cancel the question unless a write is in flight. True if it consumed the choice.
pub(super) fn cancel(model: &mut Model) -> bool {
    let Phase::Asking { ask, .. } = &model.consent.phase else {
        return false;
    };
    if ask.decision != Some(TrustDecision::Saving)
        && let Phase::Asking { then, .. } = std::mem::take(&mut model.consent.phase)
    {
        model.consent.return_to_center = matches!(then, Then::Fresh { .. });
        model.consent.phase = Phase::Settled(Outcome::Cancelled(then));
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
    if let Phase::Asking { then, .. } = std::mem::take(&mut model.consent.phase) {
        model.consent.phase = Phase::Settled(Outcome::Go(then));
    }
}

/// The connection was lost: the read or the write will not be answered, so
/// the question and what waited are dropped, and a late answer with it.
pub(super) fn lost(model: &mut Model) {
    if matches!(
        model.consent.phase,
        Phase::Checking { .. } | Phase::Asking { .. }
    ) {
        model.consent.phase = Phase::Idle;
    }
}
