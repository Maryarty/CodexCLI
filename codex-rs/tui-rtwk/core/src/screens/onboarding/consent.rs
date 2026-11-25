//! The folder trust asked over the chat: `/cd` into a folder whose trust is
//! undecided asks the oracle's question (the cancel choice "Keep current
//! directory") before the new thread starts. Oracle: `app/working_directory.rs`
//! (the gate, `confirm_directory_trust` with `TrustCancelAction::CurrentTask`)
//! and `onboarding/directory_trust.rs` (`check_directory_trust`: the lookup,
//! the screen, the write).
//!
//! This is a sub-state of the onboarding model and not `Model::trust`: the
//! launch steps (`due`, `steps`, `settled`) read that one, and a question
//! asked in the middle of the session must not bring the startup gate back.
//! It holds what waits for the answer (`Then`); the app releases it once the
//! question is decided (`take`) and the page over the chat shows the question
//! while `open` says there is one.

use super::{Ask, Model, Request, TrustAsk, TrustCancel, TrustDecision, ask, mint};
use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::threads::ThreadAsk;

/// What goes on once the folder is decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Then {
    /// `/cd`: the session starts in the folder, as `source`'s fork or as a
    /// fresh thread (`ask` is what `/cd` built).
    Cd {
        source: Option<ThreadId>,
        ask: ThreadAsk,
    },
}

/// How the question ended, when it did not end in the user's keeping things
/// as they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The folder needs no more question (trusted, or answered): go on.
    Go(Then),
    /// The folder's trust could not be read: the message, and what waited.
    Failed { message: String, then: Then },
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
    phase: Phase,
}

/// The host answers the question (`Ask::CheckConsent`): `/cd` waits for it.
/// Off, `/cd` starts the session at once (a host that reads no trust).
pub fn arm(model: &mut Model, on: bool) {
    model.consent.armed = on;
}

/// `/cd` asks for the folder `cwd` to be decided before `then` goes on.
/// False when nothing is asked and `then` goes on at once (not armed); true
/// when `then` waits, or is dropped because a question is already open (a
/// second `/cd` while one waits starts nothing).
pub fn begin(model: &mut Model, then: Then, cwd: String, fx: &mut AppFx) -> bool {
    if !model.consent.armed {
        return false;
    }
    if model.consent.phase != Phase::Idle {
        return true;
    }
    let request = mint(model);
    model.consent.phase = Phase::Checking { request, then };
    ask(
        fx,
        Ask::CheckConsent {
            request,
            cwd,
            cancel: TrustCancel::CurrentTask,
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

/// The cancel choice of an open question ("Keep current directory"): what
/// waited is dropped, silently, and the next `/cd` asks again. Not while the
/// write is out. False when no question is open.
pub(super) fn cancel(model: &mut Model) -> bool {
    match &model.consent.phase {
        Phase::Asking { ask, .. } => {
            if ask.decision != Some(TrustDecision::Saving) {
                model.consent.phase = Phase::Idle;
            }
            true
        }
        _ => false,
    }
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
