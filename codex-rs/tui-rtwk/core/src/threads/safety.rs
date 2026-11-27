//! The thread's safety facts (oracle `chatwidget/safety_buffering.rs`, `safety_recourse.rs`,
//! `misalignment_policy.rs` and the app flows around them): the agent output held back while a
//! safety check runs, the rewrites offered after a refusal, and the stop after a misalignment
//! failure that blocks input.
//!
//! ADR-013: core keeps what the IO and the next decision need, never what is drawn. The prompt
//! sheets ("Giving this request a little extra thought", "Stop this attempt and retry?"), the
//! recourse rows, the precaution and the findings sheet, the composer's disabled placeholder and
//! every word of them are the UI's; it reads `Safety` and says verbs (`Verb`), never gestures.
//! The flows that need the app (the retry on a faster model, the rewrite on a verified fork) are
//! *requests*: core validates them and records `Retry` / `Rewrite` for the app to take.
//! The one writer is `threads::apply` (`chat::ThreadEv::Safety`, the chat's event door, so the hub's
//! `ThreadEv` and `Intent` stay as they are; the verbs come as `chat::Intent::Safety`).

pub mod recourse;
pub(crate) mod retry;

use codex_app_server_protocol::{
    MisalignmentErrorDetails, ModelSafetyBufferingUpdatedNotification, ServerNotification,
    TurnModerationMetadataNotification,
};

pub use recourse::{Domain, Source, Topic};

use super::{Fault, Item, ThreadEv, ThreadFx, ThreadModel, TurnState, UserMessage};
use crate::ids::{ThreadId, Ticket, TurnId};
use crate::screens::cells::Daybreak;
use crate::screens::{ScreenThreadEv, chat};

/// A finding longer than this is not kept (oracle `on_misalignment_error`).
const MAX_FINDINGS: usize = 64 * 1024;
/// A continuation instruction longer than this is never sent.
const MAX_STEER: usize = 1024;

/// The safety check of the running turn that holds its output (`ActiveSafetyBuffering`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buffering {
    pub turn: TurnId,
    /// The faster model the retry is offered with; none when it cannot be offered (the turn was
    /// not submitted here, a model was not named).
    retry: Option<RetryOffer>,
    /// Counts the prompts: a new number is a prompt to show (the first one, or the same check
    /// whose offer of a retry changed), the same number the prompt already shown.
    pub revision: u32,
    message_started: bool,
}

impl Buffering {
    /// Nothing of the answer has come yet (the header and the prompt stay).
    pub fn waiting(&self) -> bool {
        !self.message_started
    }

    /// Exactly the offer the user can explicitly confirm while this check waits.
    pub fn retry_offer(&self) -> Option<&RetryOffer> {
        self.retry.as_ref().filter(|_| self.waiting())
    }
}

/// Identifies the offered model and the lifetime of one buffering check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryOffer {
    generation: Ticket,
    turn: TurnId,
    model: String,
}

impl RetryOffer {
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// What was submitted to a turn (`record_safety_buffering_turn`): the retry sends it again.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Submitted {
    turn: TurnId,
    message: UserMessage,
}

/// A blocked turn's alternatives (`render_failure`): what the UI words as rows under the
/// refusal's cell. The choice of one is `Verb::Rewrite`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub turn: TurnId,
    pub domain: Domain,
    /// What was flagged and what it concerned: the UI words the explanation under the refusal's
    /// cell from them.
    pub source: recourse::Source,
    pub topic: recourse::Topic,
    /// Label and the prompt a new chat starts with.
    pub suggestions: Vec<(String, String)>,
    /// The cyber refusal's wording at the moment: `Disabled` adds the Enable Daybreak row.
    pub daybreak: Daybreak,
}

/// The recourse metadata seen on the running turn, waiting for its refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Pending {
    domain: Domain,
    result: Option<recourse::Recourse>,
    /// The refusal that came before the turn's end: shown (or not) when it completes.
    deferred: Option<Fault>,
}

/// The request to retry a buffered turn on the faster model: the app interrupts, forks before
/// the turn and sends `message` again (`retry_safety_buffered_turn`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retry {
    thread: ThreadId,
    turn: TurnId,
    model: String,
    message: UserMessage,
}

impl Retry {
    pub fn turn(&self) -> &TurnId {
        &self.turn
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn message(&self) -> &UserMessage {
        &self.message
    }
}

/// The request to continue with a rewrite on a new chat forked before the blocked turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rewrite {
    pub turn: TurnId,
    pub prompt: String,
}

/// A live review identifies both the failed turn and its version of the findings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewId {
    turn: TurnId,
    revision: u64,
}

/// The findings of a misalignment failure and the way to continue after them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Review {
    id: ReviewId,
    details: MisalignmentErrorDetails,
}

impl Review {
    pub fn id(&self) -> &ReviewId {
        &self.id
    }

    /// What was detected, for the findings page (markdown).
    pub fn findings(&self) -> &str {
        self.details
            .detailed_explanation
            .as_deref()
            .unwrap_or_default()
    }

    /// The instruction a confirmed continuation submits; none when the server gave none (or a
    /// blank one, or one over the bound: it is never truncated).
    pub fn continuation(&self) -> Option<&str> {
        self.details
            .steer
            .as_ref()
            .map(|steer| steer.message.as_str())
            .filter(|message| !message.trim().is_empty() && message.len() <= MAX_STEER)
    }
}

/// A misalignment failure: input is blocked until a new turn starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    /// The turn that failed; none when only the fact is known (no findings, nothing to continue).
    pub turn: Option<TurnId>,
    pub review: Option<Review>,
}

/// The thread's safety state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Safety {
    submitted: Option<Submitted>,
    buffering: Option<Buffering>,
    revisions: u32,
    recourse: Option<Pending>,
    offer: Option<Offer>,
    violation: Option<Violation>,
    reviews: u64,
    retry: Option<Retry>,
    retrying: Option<Ticket>,
    rewrite: Option<Rewrite>,
}

impl Safety {
    /// The check that holds the running turn's output.
    pub fn buffering(&self) -> Option<&Buffering> {
        self.buffering.as_ref()
    }

    /// The status says "Working" with the safety header: output is held and none has come.
    pub fn is_waiting(&self) -> bool {
        self.buffering.as_ref().is_some_and(Buffering::waiting)
    }

    /// A refusal is expected on the running turn and its alternatives are being prepared
    /// ("I can’t help with this request. Preparing alternative suggestions…").
    pub fn is_preparing_alternatives(&self) -> bool {
        self.recourse.is_some() && self.offer.is_none()
    }

    /// The alternatives of the last refused turn.
    pub fn offer(&self) -> Option<&Offer> {
        self.offer.as_ref()
    }

    /// The misalignment failure that blocks input.
    pub fn violation(&self) -> Option<&Violation> {
        self.violation.as_ref()
    }

    /// Ordinary input is rejected (a turn, a command that starts one, the queue); the exempt
    /// ones (interrupt, mode and settings changes, renaming) are never asked of this.
    pub fn blocks_input(&self) -> bool {
        self.violation.is_some()
    }

    /// Whether `review` is the one on screen: a stale verb must not authorize a turn.
    pub fn is_current(&self, review: &ReviewId) -> bool {
        self.current_review(review).is_some()
    }

    fn current_review(&self, id: &ReviewId) -> Option<&Review> {
        self.violation
            .as_ref()?
            .review
            .as_ref()
            .filter(|review| review.id() == id)
    }

    /// The retry the app has to carry out.
    pub fn retry(&self) -> Option<&Retry> {
        self.retry.as_ref()
    }

    /// A confirmed operation still owns the source queue.
    pub fn retrying(&self) -> bool {
        self.retrying.is_some()
    }

    pub(crate) fn holds_retry(&self, generation: Ticket) -> bool {
        self.retrying == Some(generation)
    }

    /// The rewrite the app has to carry out.
    pub fn rewrite(&self) -> Option<&Rewrite> {
        self.rewrite.as_ref()
    }

    /// The turn ended: the check and what was submitted for it are over (`clear_safety_buffering`).
    pub(super) fn turn_ended(&mut self) {
        self.buffering = None;
        self.submitted = None;
    }

    /// A turn started: what belonged to the one before is gone, a precaution of another turn
    /// with it (`clear_misalignment_for_new_turn`).
    pub(super) fn turn_started(&mut self, turn: &TurnId) {
        self.buffering = None;
        self.recourse = None;
        self.offer = None;
        self.retry = None;
        self.rewrite = None;
        if self
            .violation
            .as_ref()
            .is_some_and(|violation| violation.turn.as_ref().is_none_or(|failed| failed != turn))
        {
            self.violation = None;
        }
    }

    /// A message left for `turn` (`record_safety_buffering_turn`).
    pub(super) fn submitted(&mut self, turn: TurnId, message: UserMessage) {
        self.submitted = Some(Submitted { turn, message });
    }
}

/// What a surface asks of the thread's safety state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    /// Stop the buffered turn and retry according to this exact confirmed offer.
    Retry(RetryOffer),
    /// Start a new chat from the rewrite `prompt` offered for the blocked `turn`.
    Rewrite { turn: TurnId, prompt: String },
    /// Acknowledge the findings of `review` and continue with the server's instruction.
    Continue { review: ReviewId },
    /// The app took the retry or the rewrite it was asked for.
    Taken,
}

/// An event of the safety state of one thread (a variant of `chat::ThreadEv::Safety`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SafetyEv {
    /// `model/safetyBuffering/updated`.
    Buffering {
        turn: TurnId,
        show: bool,
        faster_model: Option<String>,
    },
    /// `turn/moderationMetadata`.
    Moderation {
        turn: TurnId,
        metadata: serde_json::Value,
    },
    Verb(Verb),
    RetryLifecycle(RetryLifecycle),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetryLifecycle {
    Hold { generation: Ticket, offer: RetryOffer },
    Release { generation: Ticket, restore: Option<Box<UserMessage>> },
    RetireOffer,
    Started { turn: TurnId, message: Box<UserMessage> },
}

fn wrap(ev: SafetyEv) -> ThreadEv {
    ThreadEv::Screen(ScreenThreadEv::Chat(chat::ThreadEv::Safety(ev)))
}

/// The notifications of the safety state (`threads::project` asks first).
pub(super) fn project(notification: &ServerNotification) -> Option<(ThreadId, ThreadEv)> {
    match notification {
        ServerNotification::ModelSafetyBufferingUpdated(
            ModelSafetyBufferingUpdatedNotification {
                thread_id,
                turn_id,
                show_buffering_ui,
                faster_model,
                ..
            },
        ) => Some((
            ThreadId::new(thread_id),
            wrap(SafetyEv::Buffering {
                turn: TurnId::new(turn_id),
                show: *show_buffering_ui,
                faster_model: faster_model.clone(),
            }),
        )),
        ServerNotification::TurnModerationMetadata(TurnModerationMetadataNotification {
            thread_id,
            turn_id,
            metadata,
        }) => Some((
            ThreadId::new(thread_id),
            wrap(SafetyEv::Moderation {
                turn: TurnId::new(turn_id),
                metadata: metadata.clone(),
            }),
        )),
        _ => None,
    }
}

/// The last turn of the thread: the running one, else the one that ended last.
fn last_turn(model: &ThreadModel) -> Option<TurnId> {
    match &model.core.turn {
        TurnState::Running(running) => Some(running.turn.clone()),
        TurnState::Idle => model.core.last_ended.clone(),
    }
}

/// Queued input that follows the turn (`has_queued_follow_up_messages`).
fn has_follow_ups(model: &ThreadModel) -> bool {
    let core = &model.core;
    !core.queue.is_empty() || !core.pending_steers.is_empty() || !core.rejected_steers.is_empty()
}

/// Output of the agent for the checked turn lifts the hold (`mark_..._agent_message_started`).
pub(super) fn observe(model: &mut ThreadModel, ev: &ThreadEv) {
    let turn = match ev {
        ThreadEv::ItemStarted { key, item, .. } | ThreadEv::ItemUpsert { key, item, .. }
            if matches!(item, Item::Agent { .. }) =>
        {
            &key.turn
        }
        ThreadEv::AgentDelta { key, .. } => &key.turn,
        _ => return,
    };
    if let Some(buffering) = model.core.safety.buffering.as_mut()
        && buffering.turn == *turn
    {
        buffering.message_started = true;
    }
}

pub(super) fn apply(model: &mut ThreadModel, ev: SafetyEv, fx: &mut ThreadFx<'_>) {
    match ev {
        SafetyEv::Buffering {
            turn,
            show,
            faster_model,
        } => buffering(model, turn, show, faster_model, fx),
        SafetyEv::Moderation { turn, metadata } => moderation(model, turn, metadata),
        SafetyEv::Verb(verb) => verb_applied(model, verb, fx),
        SafetyEv::RetryLifecycle(step) => retry_step(model, step, fx),
    }
}

fn retry_step(model: &mut ThreadModel, step: RetryLifecycle, fx: &mut ThreadFx<'_>) {
    match step {
        RetryLifecycle::Hold { generation, offer } => {
            if confirmed_retry(model, &offer).is_some() {
                model.core.safety.retrying = Some(generation);
            }
        }
        RetryLifecycle::Release { generation, restore } => {
            if model.core.safety.holds_retry(generation) {
                model.core.safety.retrying = None;
                if let Some(message) = restore {
                    super::give_back(model, vec![*message], false);
                }
            }
        }
        RetryLifecycle::RetireOffer => {
            if let Some(offer) = model.core.safety.buffering.as_mut().and_then(|held| held.retry.as_mut()) {
                offer.generation = fx.mint();
            }
        }
        RetryLifecycle::Started { turn, message } => {
            super::start_turn(model, turn.clone(), fx.now());
            model.core.safety.submitted(turn, *message);
        }
    }
}

fn buffering(
    model: &mut ThreadModel,
    turn: TurnId,
    show: bool,
    faster_model: Option<String>,
    fx: &mut ThreadFx<'_>,
) {
    let running = model.is_running();
    let last = last_turn(model);
    let writer =
        model.has_writer() && !model.slots.chat.ephemeral && !model.slots.chat.parent_owned;
    let safety = &mut model.core.safety;
    if !running
        || safety.recourse.is_some()
        || last.as_ref() != Some(&turn)
        || safety
            .buffering
            .as_ref()
            .is_some_and(|held| held.turn == turn && held.message_started)
    {
        return;
    }
    if !show {
        if safety
            .buffering
            .as_ref()
            .is_some_and(|held| held.turn == turn)
        {
            safety.buffering = None;
        }
        return;
    }
    // The retry is offered for a turn this process submitted, with the model the server named.
    let retry = faster_model.filter(|_| {
        writer
            && safety
                .submitted
                .as_ref()
                .is_some_and(|submitted| submitted.turn == turn)
    });
    let previous = safety.buffering.as_ref().filter(|held| held.turn == turn);
    let previous_model = previous.and_then(|held| held.retry.as_ref().map(RetryOffer::model));
    if previous.is_some() && previous_model == retry.as_deref() {
        return;
    }
    safety.revisions += 1;
    let retry = retry.map(|model| RetryOffer {
        generation: fx.mint(),
        turn: turn.clone(),
        model,
    });
    safety.buffering = Some(Buffering {
        turn,
        retry,
        revision: safety.revisions,
        message_started: false,
    });
}

fn moderation(model: &mut ThreadModel, turn: TurnId, metadata: serde_json::Value) {
    let running = model.is_running();
    let last = last_turn(model);
    let follow_ups = has_follow_ups(model);
    // `chat.config_ref().ephemeral`, `blocks_direct_input`
    let withheld = model.slots.chat.ephemeral || model.slots.chat.parent_owned;
    let safety = &mut model.core.safety;
    if !running
        || last.as_ref() != Some(&turn)
        || withheld
        || (safety.recourse.is_some() && follow_ups)
    {
        return;
    }
    let Some((domain, result)) = recourse::parse(metadata) else {
        return;
    };
    let pending = safety.recourse.get_or_insert(Pending {
        domain,
        result: None,
        deferred: None,
    });
    if pending.domain == domain && pending.result.is_none() {
        pending.result = result;
        safety.buffering = None;
    }
}

fn matches_domain(domain: Domain, fault: &Fault) -> bool {
    match domain {
        Domain::Bio => matches!(fault, Fault::SafetyAccessBlock),
        Domain::Cyber => matches!(fault, Fault::CyberPolicy),
    }
}

/// A refusal that comes before the end of the turn waits for it while alternatives are being
/// prepared (`defer_error`). True: the fault is held, nothing else is done with it now.
pub(super) fn defer(model: &mut ThreadModel, turn: &TurnId, fault: &Fault) -> bool {
    let current = last_turn(model).as_ref() == Some(turn);
    match model.core.safety.recourse.as_mut() {
        Some(pending) if current && matches_domain(pending.domain, fault) => {
            pending.deferred = Some(fault.clone());
            true
        }
        _ => false,
    }
}

/// The end of a failed turn: the fault the turn carries or the one held for it, and what the
/// state makes of it. `Some(fault)` is the fault to show as a row; none when it was a
/// misalignment failure (the precaution is the only thing shown).
pub(super) fn on_completed(
    model: &mut ThreadModel,
    turn: &TurnId,
    failed: bool,
    carried: Option<Fault>,
    fx: &mut ThreadFx<'_>,
) -> Option<Fault> {
    let mut pending = model.core.safety.recourse.take();
    let fault = carried.or_else(|| {
        if failed {
            pending.as_mut().and_then(|pending| pending.deferred.take())
        } else {
            None
        }
    })?;
    if matches!(fault, Fault::Misalignment { .. }) {
        misalignment(model, turn, &fault);
        return None;
    }
    if failed {
        render_failure(model, pending, turn, &fault, fx.daybreak());
    }
    Some(fault)
}

/// The refusal is the turn's end and its alternatives are ready: the offer (`render_failure`).
fn render_failure(
    model: &mut ThreadModel,
    pending: Option<Pending>,
    turn: &TurnId,
    fault: &Fault,
    daybreak: Daybreak,
) {
    let Some(Pending {
        domain,
        result: Some(result),
        ..
    }) = pending
    else {
        return;
    };
    if has_follow_ups(model)
        || !matches_domain(domain, fault)
        || last_turn(model).as_ref() != Some(turn)
    {
        return;
    }
    model.core.steers_after_interrupt = false;
    model.core.safety.offer = Some(Offer {
        turn: turn.clone(),
        domain,
        source: result.source,
        topic: result.topic,
        suggestions: result.suggestions,
        daybreak,
    });
}

/// A misalignment failure (`on_misalignment_error`): what the turn carried is kept as a review
/// when the findings are substantive, every queued input is dropped, and input is blocked. A
/// failure of a turn that is not the last one, or the same failure again, changes nothing.
pub(super) fn misalignment(model: &mut ThreadModel, turn: &TurnId, fault: &Fault) {
    let Fault::Misalignment { details, .. } = fault else {
        return;
    };
    if last_turn(model).is_some_and(|last| last != *turn) {
        return;
    }
    let missing = details.is_none();
    // Bound memory-only findings: a blank or oversized explanation is not kept.
    let details = details.clone().filter(|details| {
        details
            .detailed_explanation
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty() && text.len() <= MAX_FINDINGS)
    });
    if let Some(current) = &model.core.safety.violation
        && (current.turn.is_none()
            || (current.turn.as_ref() == Some(turn)
                && (missing
                    || current.review.as_ref().map(|review| &review.details) == details.as_ref())))
    {
        return;
    }
    stop(model, Some(turn.clone()), details);
}

/// A failed turn that is not the latest one when its history is replayed carries no precaution;
/// the latest one blocks input with nothing to continue from (`on_misalignment_policy_violation`).
pub(super) fn replayed(model: &mut ThreadModel) {
    if model.core.safety.violation.is_none() {
        stop(model, None, None);
    }
}

fn stop(model: &mut ThreadModel, turn: Option<TurnId>, details: Option<MisalignmentErrorDetails>) {
    let core = &mut model.core;
    core.queue.clear();
    core.pending_steers.clear();
    core.rejected_steers.clear();
    core.steers_after_interrupt = false;
    let safety = &mut core.safety;
    safety.reviews += 1;
    let review = turn.clone().zip(details).map(|(turn, details)| Review {
        id: ReviewId {
            turn,
            revision: safety.reviews,
        },
        details,
    });
    safety.violation = Some(Violation { turn, review });
    safety.buffering = None;
    safety.recourse = None;
    safety.retry = None;
    safety.rewrite = None;
}

/// The owner checks the same offer at dispatch before any interrupt can be emitted.
pub(crate) fn confirmed_retry(model: &ThreadModel, offer: &RetryOffer) -> Option<Retry> {
    let safety = &model.core.safety;
    if safety.retrying()
        || !model.is_running()
        || !model.has_writer()
        || model.slots.chat.ephemeral
        || model.slots.chat.parent_owned
        || safety.buffering()?.retry_offer() != Some(offer)
    {
        return None;
    }
    let submitted = safety
        .submitted
        .as_ref()
        .filter(|submitted| submitted.turn == offer.turn)?;
    Some(Retry {
        thread: model.id().clone(),
        turn: offer.turn.clone(),
        model: offer.model.clone(),
        message: submitted.message.clone(),
    })
}

fn verb_applied(model: &mut ThreadModel, verb: Verb, fx: &mut ThreadFx<'_>) {
    match verb {
        Verb::Retry(offer) => {
            if model.core.safety.retry.is_none() {
                model.core.safety.retry = confirmed_retry(model, &offer);
            }
        }
        Verb::Rewrite { turn, prompt } => {
            let offered = model.core.safety.offer.as_ref().is_some_and(|offer| {
                offer.turn == turn
                    && offer
                        .suggestions
                        .iter()
                        .any(|(_, suggestion)| *suggestion == prompt)
            });
            if offered
                && last_turn(model).as_ref() == Some(&turn)
                && !model.is_running()
                && !model.core.outgoing.any(super::is_start_turn)
                && !has_follow_ups(model)
            {
                model.core.safety.rewrite = Some(Rewrite { turn, prompt });
            }
        }
        Verb::Continue { review } => {
            let current = model
                .core
                .safety
                .current_review(&review)
                .filter(|_| last_turn(model).is_none_or(|last| last == review.turn));
            let Some(message) = current.and_then(Review::continuation).map(str::to_owned) else {
                return;
            };
            if !model.has_writer()
                || model.slots.chat.parent_owned
                || model.is_running()
                || model.core.outgoing.any(super::is_start_turn)
            {
                return;
            }
            // The turn that went wrong is not sent again: the instruction goes as a new turn with
            // the acknowledgement the server asks for, outside the queue that stays blocked.
            let stamp = fx.now().saturating_mul(1000);
            let _ = fx.start_acknowledged(UserMessage::text(message), stamp);
        }
        Verb::Taken => {
            model.core.safety.retry = None;
            model.core.safety.rewrite = None;
        }
    }
}
