//! The prompts the account's limits raise as a feature slice (ADR-012, ADR-013; oracle
//! `chatwidget/rate_limits.rs` `on_rate_limit_snapshot_from`, `maybe_show_pending_rate_limit_prompt`,
//! `open_rate_limit_switch_prompt`, `open_workspace_owner_nudge_prompt`,
//! `finish_add_credits_nudge_email_request`, `turn_runtime.rs` `on_rate_limit_error`):
//!
//! - the switch prompt: a thread that has used 90 percent of a window of the plan's own limit is
//!   offered the lower-cost model once its turn is over (`Approaching rate limits`), unless the
//!   user asked not to be reminded, the thread already runs on it, or the workspace has credits to
//!   spend;
//! - the workspace nudge: a member of a workspace whose credits or usage limit ran out is asked,
//!   when the turn fails with the limit error, whether to e-mail the owner; the answer of the
//!   request is a note of the transcript.
//!
//! - the recovery prompt: a switch prompt that is on the screen when a read of the account's
//!   limits says the usage is under the threshold again (and no rolling update overtook the read)
//!   turns into `You're no longer close to your usage limit`, which dismisses itself after five
//!   seconds (a declared wake, `wake_at`), or at once when the user picks `Dismiss`.
//!
//! What the slice keeps is what deciding needs (ADR-013): the stage of the switch prompt, the kind
//! of limit last reached, the credits the last read saw and the seconds the recovery prompt says.
//! The prompts themselves are wishes the surface shows once (`Wish`); what a pick does comes back
//! as a verb (`Intent`). The slice reads the stream the features see (`project`): the rolling
//! turn's failure. The app calls this feature before applying a routed turn completion, so a Plan
//! prompt and a rate-limit offer are decided together. The feature also reads rolling limits and
//! the app's own reads of the limits (`on_rate_limits_read`), which are on no stream. The owner's
//! e-mail is the app's one nudge path (`PublicVerb::NotifyOwner`: its request, its note), and the
//! words of an owner's limit error are the slice's (`fault_words`).

use std::collections::BTreeMap;
use std::rc::Rc;

use codex_app_server_protocol::{
    ClientRequest, CodexErrorInfo, ErrorNotification, ModelListParams, ModelListResponse,
    RateLimitReachedType, RateLimitSnapshot, RequestId,
};

use super::{AppCx, AppView, FeatureStates, Never, PublicVerb, ThreadView};
use crate::app::Now;
use crate::ids::{BackendId, ThreadId, TurnId};
use crate::rpc::{Address, RpcError};
use crate::screens::chat::Nudge;
use crate::screens::select::models::{Effort, RESERVE_MODEL};
use crate::screens::status::{self, RateRead};

/// The marker type of the feature.
pub struct RateLimitPrompt;

/// `NUDGE_MODEL_SLUG` (`LUNA_MODEL`): the lower-cost model the switch prompt offers.
const NUDGE_MODEL: &str = "gpt-6-luna";

/// `RATE_LIMIT_SWITCH_PROMPT_THRESHOLD`: the percent used at which the switch is offered.
const THRESHOLD: i32 = 90;

/// How long the recovery prompt counts down (`tick_rate_limit_recovery_prompt`).
const RECOVERY_MILLIS: i64 = 5_000;

/// `on_rate_limit_error`: what a member of a workspace whose owner ran out is told instead of
/// the server's words.
const OWNER_CREDITS_WORDS: &str =
    "You're out of credits. Your workspace is out of credits. Add credits to continue using Codex.";
const OWNER_USAGE_WORDS: &str = "Usage limit reached. You've reached your usage limit. Increase your limits to continue using codex.";

/// The model the prompt offers, as the catalog describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub model: String,
    /// The catalog's words for the model (empty: the prompt has its own).
    pub description: String,
    pub effort: Effort,
}

/// What the slice's RPC answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The lower-cost model of `model/list`, when the catalog lists it for the picker.
    Candidate(Option<Candidate>),
}

mod rpcs {
    use super::*;

    crate::rpc_list! {
        /// What the slice asks the server.
        pub enum Ask => Reply;
        /// `model/list`: the catalog the switch prompt offers its model from
        /// (`lower_cost_preset`).
        Models, Self::Models
            => ModelList(ModelListParams { include_hidden: Some(true), ..Default::default() })
            -> ModelListResponse, |answer| Reply::Candidate(answer
                .data
                .iter()
                .find(|model| !model.hidden && model.model == NUDGE_MODEL)
                .map(|model| Candidate {
                    model: model.model.clone(),
                    description: model.description.clone(),
                    effort: model.default_reasoning_effort.clone(),
                }));
    }
}

pub use rpcs::Ask;

/// The turn whose completion reserved the right to offer a lower-cost model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferId {
    thread: ThreadId,
    turn: TurnId,
    address: Address,
    generation: u64,
}

/// The workspace remediation a failure offered, on the connection that reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NudgeId {
    address: Address,
    thread: ThreadId,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Models(OfferId),
}

/// What the prompts' sheets say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `Switch to {model}`: the thread's next turns use it (`override_turn_context`).
    Switch {
        offer: OfferId,
        model: String,
        effort: Effort,
    },
    /// `Yes` of the workspace nudge: the owner is e-mailed.
    Nudge(NudgeId),
    /// The member declined the request.
    DeclineNudge(NudgeId),
    /// The switch or recovery prompt is closed (picked, dismissed): nothing keeps a sheet for it.
    Closed(OfferId),
    /// The clock reached the moment the recovery prompt's count changes (the feature's own
    /// `wake_at`).
    Tick,
}

/// What the prompt's sheet shows now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// No sheet is wanted.
    Gone,
    /// The offer of the lower-cost model.
    Offer,
    /// `You're no longer close to your usage limit`, dismissing in this many seconds.
    Recovery(u64),
}

/// What the slice shows once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    /// `Approaching rate limits`: offers the lower-cost model.
    Switch {
        offer: OfferId,
        candidate: Candidate,
    },
    /// The workspace owner is asked for credits or for a higher limit.
    Nudge { id: NudgeId, kind: Nudge },
}

/// What a turn failure of the limit says (`RateLimitErrorKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    UsageLimit,
    Generic,
}

/// A change to the remedy the backend last reported.
pub(crate) enum Remedy {
    Rolling(RateLimitReachedType),
    UsageLimitError,
}

/// What a snapshot says that the prompts decide by (`on_rate_limit_snapshot_from`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    /// The snapshot is the plan's own (`codex`) limit.
    pub codex: bool,
    /// The most used of the windows.
    pub used: Option<i32>,
    /// Whether the credits can be spent; none when the snapshot carried none.
    pub credits: Option<bool>,
    pub reached: Option<RateLimitReachedType>,
    pub spend_control: Option<bool>,
}

/// Where the switch prompt is (`RateLimitSwitchPromptState`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Idle,
    /// The usage is high; the prompt waits for the turn to end.
    Pending,
    /// The catalog is asked for the model.
    Asking(OfferId),
    Shown,
    /// A read found the usage low again while the prompt was on the screen: it counts down to
    /// `deadline` (milliseconds since the epoch).
    Recovering {
        deadline: i64,
    },
}

/// Where a snapshot came from: a read of the account's limits (the authoritative kind, which
/// carries the credits and the limit type whole) or a rolling update (the sparse kind).
#[derive(Clone, Copy, Debug)]
struct Read {
    /// A rolling update overtook the read: it is too old to confirm a recovery.
    stale: bool,
    now: i64,
}

/// The feature's state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    backend: Option<BackendId>,
    stage: Stage,
    generation: u64,
    /// The kind of limit last reached (`codex_rate_limit_reached_type`).
    reached: BTreeMap<BackendId, RateLimitReachedType>,
    spend_control: Option<bool>,
    /// Whether the credits the last read saw can be spent (`rate_limit_snapshots_by_limit_id`;
    /// rolling updates do not write it, they are judged by it).
    credits: Option<bool>,
    /// The model is offered and the offer still stands (the prompt closes when it does not).
    offered: Option<OfferId>,
    /// The failure for which the member can still authorize a request.
    nudge: Option<(NudgeId, Nudge)>,
    /// The seconds the recovery prompt says.
    seconds: u64,
    /// The thread whose limit error the slice has words for, and the words.
    said: Option<(ThreadId, &'static str)>,
}

impl State {
    pub fn nudge_pending(&self, id: &NudgeId) -> bool {
        self.nudge
            .as_ref()
            .is_some_and(|(current, _)| current == id)
    }

    /// What the prompt's sheet shows: the offer, its recovery countdown, or nothing.
    pub fn standing(&self, offer: &OfferId) -> Standing {
        if self.offered.as_ref() != Some(offer) {
            return Standing::Gone;
        }
        match self.stage {
            Stage::Recovering { .. } => Standing::Recovery(self.seconds),
            _ => Standing::Offer,
        }
    }

    fn use_backend(&mut self, backend: BackendId) {
        if self.backend != Some(backend) {
            self.reset();
            self.backend = Some(backend);
        }
    }

    fn reset(&mut self) {
        *self = Self {
            generation: self.generation,
            reached: std::mem::take(&mut self.reached),
            ..Self::default()
        };
    }

    fn dismiss(&mut self) {
        self.offered = None;
        if matches!(self.stage, Stage::Recovering { .. }) {
            self.stage = Stage::Shown;
        }
    }

    fn dismiss_nudge_of(&mut self, backend: BackendId) {
        if self
            .nudge
            .as_ref()
            .is_some_and(|(id, _)| id.address.backend == backend)
        {
            self.nudge = None;
        }
    }

    fn workspace_limit_reached(&self) -> bool {
        self.spend_control == Some(true)
            || matches!(
                self.reached.get(&self.backend.unwrap_or_default()),
                Some(
                    RateLimitReachedType::WorkspaceOwnerCreditsDepleted
                        | RateLimitReachedType::WorkspaceMemberCreditsDepleted
                        | RateLimitReachedType::WorkspaceOwnerUsageLimitReached
                        | RateLimitReachedType::WorkspaceMemberUsageLimitReached
                )
            )
    }

    /// `on_rate_limit_snapshot_from`: a read (`read`) or a rolling update (the sparse kind: what
    /// it does not say is what was known).
    fn take(&mut self, sample: &Sample, read: Option<Read>, hidden: bool, model: Option<&str>) {
        let reading = read.is_some();
        if sample.codex && (reading || sample.spend_control.is_some()) {
            self.spend_control = sample.spend_control;
        }
        let credits = if reading {
            sample.credits
        } else {
            sample.credits.or(self.credits)
        };
        let has_workspace_credits = !self.workspace_limit_reached() && credits == Some(true);
        let replenished = read.is_some_and(|read| !read.stale)
            && sample.used.is_some_and(|used| used < THRESHOLD);
        if sample.codex && (has_workspace_credits || replenished) {
            match self.stage {
                Stage::Pending | Stage::Asking(_) => self.stage = Stage::Idle,
                Stage::Shown if replenished => {
                    // a prompt the user already closed has nothing to count down
                    if let Some(read) = read.filter(|_| self.offered.is_some()) {
                        self.stage = Stage::Recovering {
                            deadline: read.now + RECOVERY_MILLIS,
                        };
                        self.tick(read.now);
                    }
                }
                Stage::Shown => self.offered = None,
                Stage::Idle | Stage::Recovering { .. } => {}
            }
        }
        if reading {
            self.credits = sample.credits;
        }
        let should_warn = sample.codex && !has_workspace_credits;
        if sample.used.is_some_and(|used| used >= THRESHOLD)
            && should_warn
            && !hidden
            && model != Some(NUDGE_MODEL)
            && self.stage == Stage::Idle
        {
            self.stage = Stage::Pending;
        }
    }

    /// `tick_rate_limit_recovery_prompt`: the seconds the prompt says are the seconds that are
    /// left, rounded up; none left, the prompt goes.
    fn tick(&mut self, now: i64) {
        let Stage::Recovering { deadline } = self.stage else {
            return;
        };
        let remaining = deadline - now;
        if remaining <= 0 {
            self.stage = Stage::Shown;
            self.offered = None;
            self.seconds = 0;
        } else {
            self.seconds =
                u64::try_from(remaining.div_euclid(1000) + i64::from(remaining % 1000 != 0))
                    .unwrap_or(1);
        }
    }

    /// A backend banner is on the screen (`show_backend_banner`): it replaces the switch prompt,
    /// which is asked again once the banner is gone, "not the user's decision to dismiss".
    fn yield_to_banner(&mut self) {
        if matches!(self.stage, Stage::Asking(_))
            || (self.offered.is_some() && self.stage == Stage::Shown)
        {
            self.stage = Stage::Pending;
            self.offered = None;
        }
    }

    fn owner_nudge(&self, backend: BackendId) -> Option<Nudge> {
        match self.reached.get(&backend) {
            Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted) => Some(Nudge::Credits),
            Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached) => Some(Nudge::UsageLimit),
            _ => None,
        }
    }

    /// What the owner of a workspace that ran out is told (`on_rate_limit_error`).
    fn owner_words(&self, backend: BackendId) -> Option<&'static str> {
        match self.reached.get(&backend) {
            Some(RateLimitReachedType::WorkspaceOwnerCreditsDepleted) => Some(OWNER_CREDITS_WORDS),
            Some(RateLimitReachedType::WorkspaceOwnerUsageLimitReached) => Some(OWNER_USAGE_WORDS),
            _ => None,
        }
    }
}

/// A backend banner is showing for the model in use (`has_applicable_backend_banner`): the
/// prompts wait for it.
pub(crate) fn banner_shows(view: AppView<'_>) -> bool {
    let model = model_in_use(view);
    view.facts()
        .backend_banner()
        .is_some_and(|banner| banner.is_visible(model.as_deref()))
}

/// The model the thread in use runs on: its own settings, else the config's.
fn model_in_use(view: AppView<'_>) -> Option<String> {
    view.active_thread()
        .and_then(|thread| thread.slots().chat.model.clone())
        .or_else(|| view.screens().chat.status.model.clone())
}

pub(crate) fn failure_of(error: &ErrorNotification) -> Option<Failure> {
    match error.error.codex_error_info.as_ref()? {
        CodexErrorInfo::UsageLimitExceeded => Some(Failure::UsageLimit),
        CodexErrorInfo::ResponseTooManyFailedAttempts {
            http_status_code: Some(429),
        } => Some(Failure::Generic),
        _ => None,
    }
}

/// Record a new known reason. A sparse notification cannot undo a known remedy and an
/// initial observation cannot contradict the read that supplied the existing banner.
pub(crate) fn change_remedy(
    states: &mut Rc<FeatureStates>,
    backend: BackendId,
    update: Remedy,
) -> bool {
    use RateLimitReachedType as Reached;
    let previous = states.rate_limit_prompt.reached.get(&backend).copied();
    let next = match (update, previous) {
        (Remedy::Rolling(reached), _) => reached,
        (Remedy::UsageLimitError, Some(Reached::WorkspaceOwnerCreditsDepleted)) => {
            Reached::WorkspaceOwnerUsageLimitReached
        }
        (Remedy::UsageLimitError, Some(Reached::WorkspaceMemberCreditsDepleted)) => {
            Reached::WorkspaceMemberUsageLimitReached
        }
        _ => return false,
    };
    if previous == Some(next) {
        return false;
    }
    let state = &mut Rc::make_mut(states).rate_limit_prompt;
    state.reached.insert(backend, next);
    if previous.is_some() {
        state.dismiss_nudge_of(backend);
        return true;
    }
    false
}

/// The app has routed a terminal error and retired any superseded banner.
pub(crate) fn failed(
    states: &mut FeatureStates,
    thread: ThreadId,
    address: Address,
    kind: Option<Failure>,
    can_nudge: bool,
    cx: &mut AppCx<'_>,
) {
    let state = &mut states.rate_limit_prompt;
    state.said = kind
        .and_then(|_| state.owner_words(address.backend))
        .map(|words| (thread.clone(), words));
    if cx.view().active() != Some(&thread) {
        return;
    }
    if banner_shows(cx.view()) {
        state.yield_to_banner();
        return;
    }
    if can_nudge && let Some(nudge) = kind.and_then(|_| state.owner_nudge(address.backend)) {
        if state.nudge.as_ref().is_some_and(|(id, current)| {
            id.thread == thread && id.address == address && *current == nudge
        }) {
            return;
        }
        state.generation = state.generation.wrapping_add(1);
        let id = NudgeId {
            address,
            thread: thread.clone(),
            generation: state.generation,
        };
        state.nudge = Some((id.clone(), nudge));
        cx.wish(thread, Wish::Nudge { id, kind: nudge });
    }
}

pub(crate) fn nudge_origin(states: &FeatureStates) -> Option<(&ThreadId, Address)> {
    let (id, _) = states.rate_limit_prompt.nudge.as_ref()?;
    Some((&id.thread, id.address))
}

pub(crate) fn dismiss_nudge(states: &mut Rc<FeatureStates>) {
    Rc::make_mut(states).rate_limit_prompt.nudge = None;
}

fn sample_of(snapshot: &RateLimitSnapshot) -> Sample {
    Sample {
        codex: snapshot
            .limit_id
            .as_deref()
            .is_none_or(|id| id.eq_ignore_ascii_case("codex")),
        used: snapshot
            .primary
            .iter()
            .chain(&snapshot.secondary)
            .map(|window| window.used_percent)
            .max(),
        credits: snapshot
            .credits
            .as_ref()
            .map(|credits| credits.unlimited || credits.has_credits),
        reached: snapshot.rate_limit_reached_type,
        spend_control: snapshot.spend_control_reached,
    }
}

/// Rolling updates from the displayed backend, routed by the app.
pub(crate) fn rolling(
    states: &mut FeatureStates,
    backend: BackendId,
    snapshot: &RateLimitSnapshot,
    cx: &AppCx<'_>,
) {
    let state = &mut states.rate_limit_prompt;
    state.use_backend(backend);
    if banner_shows(cx.view()) {
        state.yield_to_banner();
    }
    let hidden = cx.view().facts().tui().hide_rate_limit_model_nudge;
    let model = model_in_use(cx.view());
    state.take(&sample_of(snapshot), None, hidden, model.as_deref());
}

/// A replaced transport or account invalidates the operations belonging to it.
pub(crate) fn retire(states: &mut Rc<FeatureStates>, backend: BackendId) {
    let current = &states.rate_limit_prompt;
    if current.backend == Some(backend)
        || current
            .nudge
            .as_ref()
            .is_some_and(|(id, _)| id.address.backend == backend)
    {
        let state = &mut Rc::make_mut(states).rate_limit_prompt;
        state.dismiss_nudge_of(backend);
        if state.backend == Some(backend) {
            state.reset();
        }
    }
}

/// Forget the last remedy when the app also forgets the backend's banner.
pub(crate) fn forget_remedy(states: &mut Rc<FeatureStates>, backend: BackendId) {
    if states.rate_limit_prompt.reached.contains_key(&backend) {
        Rc::make_mut(states)
            .rate_limit_prompt
            .reached
            .remove(&backend);
    }
}

/// The conversation is still ready to act on the completion that made the offer.
pub(crate) fn origin_available(thread: ThreadView<'_>, turn: &TurnId) -> bool {
    thread.core().last_ended() == Some(turn)
        && !thread.busy()
        && !thread.stream_lost()
        && thread.has_writer()
}

/// A new turn can arrive while the sheet has the input. An accepted thread transition that
/// supersedes its completion takes the old offer down without waiting for a user's key.
pub(crate) fn revalidate(states: &mut std::rc::Rc<FeatureStates>, thread: ThreadView<'_>) {
    if states
        .rate_limit_prompt
        .offered
        .as_ref()
        .is_some_and(|offer| thread.id() == &offer.thread && !origin_available(thread, &offer.turn))
    {
        std::rc::Rc::make_mut(states).rate_limit_prompt.dismiss();
    }
}

/// Whether the offer can be made on the currently displayed thread.
fn can_offer(view: AppView<'_>) -> bool {
    !view.facts().tui().hide_rate_limit_model_nudge
        && !matches!(
            model_in_use(view).as_deref(),
            Some(NUDGE_MODEL | RESERVE_MODEL)
        )
}

/// Both completion prompts are decided before the thread publishes its Plan question. The
/// oracle skips that question when an eligible offer is pending, even if the catalog then has
/// no lower-cost model. Settling or cancelling this reservation never reopens the Plan question.
pub(crate) fn reserve(
    states: &mut FeatureStates,
    thread: &ThreadId,
    turn: &TurnId,
    address: Address,
    cx: &mut AppCx<'_>,
) -> crate::screens::chat::plan_mode::PromptPriority {
    use crate::screens::chat::plan_mode::PromptPriority;
    let state = &mut states.rate_limit_prompt;
    state.use_backend(address.backend);
    state.said = None;
    if matches!(state.stage, Stage::Asking(_)) {
        state.stage = Stage::Pending;
    }
    if state.stage != Stage::Pending {
        return PromptPriority::Ordinary;
    }
    if !can_offer(cx.view()) {
        state.stage = Stage::Idle;
        return PromptPriority::Ordinary;
    }
    if !banner_shows(cx.view()) {
        state.generation = state.generation.wrapping_add(1);
        let offer = OfferId {
            thread: thread.clone(),
            turn: turn.clone(),
            address,
            generation: state.generation,
        };
        state.stage = Stage::Asking(offer.clone());
        cx.ask(Some(thread), Ask::Models, Ctx::Models(offer));
    }
    PromptPriority::RateLimit
}

/// What a read says that the prompts decide by: the plan's own bucket, none when the read named
/// no such bucket.
fn sample_of_read(read: &RateRead) -> Option<Sample> {
    let sample: &status::Sample = read.codex.as_ref()?;
    Some(Sample {
        codex: sample.codex,
        used: sample
            .primary
            .iter()
            .chain(&sample.secondary)
            .map(|used| used.percent)
            .max(),
        credits: sample.credits_usable,
        reached: sample.reached_type,
        spend_control: sample.spend_control_reached,
    })
}

feature! {
    impl Feature for RateLimitPrompt {
        const NAME: &'static str = "rate_limit_prompt";
        type State = State;
        type Intent = Intent;
        type Command = Ask;
        type Reply = Reply;
        type Ctx = Ctx;
        type Note = Never<Self>;
        type Wish = Wish;
        type Event = Never<Self>;
        {
            fn on_rate_limits_read(
                state: &mut State,
                backend: BackendId,
                read: &RateRead,
                cx: &mut AppCx<'_>,
            ) {
                let sample = sample_of_read(read);
                if let Some(sample) = &sample {
                    if let Some(reached) = sample.reached {
                        state.reached.insert(backend, reached);
                    } else if sample.codex {
                        state.reached.remove(&backend);
                    }
                }
                if !read.active {
                    return;
                }
                state.use_backend(backend);
                if banner_shows(cx.view()) {
                    state.yield_to_banner();
                }
                let Some(sample) = sample else {
                    return;
                };
                let hidden = cx.view().facts().tui().hide_rate_limit_model_nudge;
                let model = model_in_use(cx.view());
                let at = Read {
                    stale: read.overtaken,
                    now: cx.view().facts().now().millis(),
                };
                state.take(&sample, Some(at), hidden, model.as_deref());
            }

            fn fault_words(
                state: &State,
                thread: &ThreadId,
                _message: &str,
                _view: &AppView<'_>,
            ) -> Option<String> {
                state
                    .said
                    .as_ref()
                    .filter(|(said, _)| said == thread)
                    .map(|(_, words)| (*words).to_owned())
            }

            fn on_shown(state: &mut State, thread: &ThreadId, cx: &mut AppCx<'_>) {
                state.use_backend(cx.view().screens().agents.owner_of(thread.as_str()));
                if matches!(&state.stage, Stage::Asking(offer) if offer.thread != *thread) {
                    // Another turn may claim the still-applicable hint. The old response
                    // cannot open a sheet for the new conversation.
                    state.stage = Stage::Pending;
                }
                if state.offered.as_ref().is_some_and(|offer| offer.thread != *thread) {
                    state.offered = None;
                    state.stage = Stage::Shown;
                }
            }

            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::Switch { offer, model, effort }
                        if state.standing(&offer) == Standing::Offer
                            && cx.view().active() == Some(&offer.thread) =>
                    {
                        state.dismiss();
                        cx.verb(PublicVerb::OverrideTurn {
                            thread: offer.thread,
                            turn: offer.turn,
                            address: offer.address,
                            model,
                            effort: Some(effort),
                        });
                    }
                    Intent::Nudge(id) if state.nudge_pending(&id)
                        && cx.view().active() == Some(&id.thread)
                        && !banner_shows(cx.view()) =>
                    {
                        if let Some((_, nudge)) = state.nudge.take() {
                            cx.verb(PublicVerb::NotifyOwner(nudge));
                        }
                    }
                    Intent::DeclineNudge(id) if state.nudge_pending(&id) => {
                        state.nudge = None;
                    }
                    Intent::Closed(offer) if state.offered.as_ref() == Some(&offer) => {
                        state.dismiss();
                    }
                    Intent::Switch { .. } | Intent::Closed(_) |
                        Intent::Nudge(_) | Intent::DeclineNudge(_) => {}
                    Intent::Tick => state.tick(cx.view().facts().now().millis()),
                }
            }

            fn wake_at(state: &State) -> Option<(Now, Intent)> {
                match state.stage {
                    // the moment the count says one second less (the last one: the deadline)
                    Stage::Recovering { deadline } if state.offered.is_some() => {
                        let seconds = i64::try_from(state.seconds).unwrap_or(1);
                        Some((
                            Now::at_millis(deadline - (seconds - 1) * 1000),
                            Intent::Tick,
                        ))
                    }
                    _ => None,
                }
            }

            fn answered(
                state: &mut State,
                ctx: Ctx,
                result: Result<Reply, RpcError>,
                cx: &mut AppCx<'_>,
            ) {
                let Ctx::Models(offer) = ctx;
                if !matches!(&state.stage, Stage::Asking(current) if *current == offer) {
                    return;
                }
                state.stage = Stage::Idle;
                let view = cx.view();
                if banner_shows(view) {
                    state.stage = Stage::Pending;
                    return;
                }
                if let Ok(Reply::Candidate(Some(candidate))) = result
                    && view.active() == Some(&offer.thread)
                    && !view.navigation_pending()
                    && view.active_thread().is_some_and(|thread| origin_available(thread, &offer.turn))
                    && can_offer(view)
                {
                    state.stage = Stage::Shown;
                    state.offered = Some(offer.clone());
                    cx.wish(offer.thread.clone(), Wish::Switch { offer, candidate });
                }
            }

            fn wire(command: &Ask, _thread: &ThreadId, id: RequestId) -> ClientRequest {
                command.request(id)
            }

            fn decode(command: &Ask, result: serde_json::Value) -> Result<Reply, RpcError> {
                command.reply(result)
            }
        }
    }
}
