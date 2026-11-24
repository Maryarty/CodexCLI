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
//! What the slice keeps is what deciding needs (ADR-013): the stage of the switch prompt, the kind
//! of limit last reached and the credits last seen on the stream. The prompts themselves are
//! wishes the surface shows once (`Wish`); what a pick does comes back as a verb (`Intent`). The
//! slice reads the stream the features see (`project`): the rolling `account/rateLimits/updated`
//! notifications, the end of a turn and a turn's failure. The answers of the app's own reads of
//! the limits are not on that stream, so a state only a read told (the limit type of a read, the
//! recovery of the usage after one) reaches the slice with the next rolling update only.

use codex_app_server_protocol::{
    AddCreditsNudgeCreditType, AddCreditsNudgeEmailStatus, ClientRequest, CodexErrorInfo,
    ModelListParams, ModelListResponse, RateLimitReachedType, RequestId,
    SendAddCreditsNudgeEmailParams, SendAddCreditsNudgeEmailResponse, ServerNotification,
};

use super::{AppCx, AppView, PublicVerb};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::chat::{Nudge, NudgeOutcome};
use crate::screens::select::models::{Effort, RESERVE_MODEL};
use crate::threads::Tone;

/// The marker type of the feature.
pub struct RateLimitPrompt;

/// `NUDGE_MODEL_SLUG` (`LUNA_MODEL`): the lower-cost model the switch prompt offers.
const NUDGE_MODEL: &str = "gpt-6-luna";

/// `RATE_LIMIT_SWITCH_PROMPT_THRESHOLD`: the percent used at which the switch is offered.
const THRESHOLD: i32 = 90;

/// The model the prompt offers, as the catalog describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub model: String,
    /// The catalog's words for the model (empty: the prompt has its own).
    pub description: String,
    pub effort: Effort,
}

/// What the slice's two RPCs answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The lower-cost model of `model/list`, when the catalog lists it for the picker.
    Candidate(Option<Candidate>),
    /// `account/sendAddCreditsNudgeEmail`.
    Nudged(NudgeOutcome),
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
        /// `account/sendAddCreditsNudgeEmail`.
        Nudge(Nudge), Self::Nudge(nudge)
            => SendAddCreditsNudgeEmail(SendAddCreditsNudgeEmailParams {
                credit_type: match nudge {
                    Nudge::Credits => AddCreditsNudgeCreditType::Credits,
                    Nudge::UsageLimit => AddCreditsNudgeCreditType::UsageLimit,
                },
            })
            -> SendAddCreditsNudgeEmailResponse, |answer| Reply::Nudged(match answer.status {
                AddCreditsNudgeEmailStatus::Sent => NudgeOutcome::Sent,
                AddCreditsNudgeEmailStatus::CooldownActive => NudgeOutcome::Cooldown,
            });
    }
}

pub use rpcs::Ask;

/// What an ask was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Models,
    Nudge(Nudge),
}

/// What the prompts' sheets say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `Switch to {model}`: the thread's next turns use it (`override_turn_context`).
    Switch { model: String, effort: Effort },
    /// `Yes` of the workspace nudge: the owner is e-mailed.
    Nudge(Nudge),
    /// The switch prompt is closed (picked, dismissed): nothing keeps a sheet for it.
    Closed,
}

/// What the slice shows once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    /// `Approaching rate limits`: offers the lower-cost model.
    Switch(Candidate),
    /// The workspace owner is asked for credits or for a higher limit.
    Nudge(Nudge),
}

/// What a turn failure of the limit says (`RateLimitErrorKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    UsageLimit,
    Generic,
}

/// What the slice makes of the stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// An `account/rateLimits/updated`, as far as the prompts read it.
    Rolling(Sample),
    /// A turn of `thread` ended.
    TurnEnded { thread: ThreadId },
    /// A turn of `thread` failed with a limit error and will not be retried.
    Failed { thread: ThreadId, kind: Failure },
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

/// Where the switch prompt is (`RateLimitSwitchPromptState`, without the recovery countdown a
/// read starts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Idle,
    /// The usage is high; the prompt waits for the turn to end.
    Pending,
    /// The catalog is asked for the model.
    Asking,
    Shown,
}

/// The feature's state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    stage: Stage,
    /// The kind of limit last reached (`codex_rate_limit_reached_type`).
    reached: Option<RateLimitReachedType>,
    spend_control: Option<bool>,
    /// Whether the credits last seen can be spent.
    credits: Option<bool>,
    /// The switch prompt is open (its sheet closes when this goes false).
    switch_open: bool,
    /// The nudge e-mail in flight: one at a time.
    sending: Option<Nudge>,
}

impl State {
    /// Whether the switch prompt is to stay open.
    pub fn switch_open(&self) -> bool {
        self.switch_open
    }

    fn workspace_limit_reached(&self) -> bool {
        self.spend_control == Some(true)
            || matches!(
                self.reached,
                Some(
                    RateLimitReachedType::WorkspaceOwnerCreditsDepleted
                        | RateLimitReachedType::WorkspaceMemberCreditsDepleted
                        | RateLimitReachedType::WorkspaceOwnerUsageLimitReached
                        | RateLimitReachedType::WorkspaceMemberUsageLimitReached
                )
            )
    }

    /// `on_rate_limit_snapshot_from` for a rolling update (the sparse kind: what it does not
    /// say is what was known).
    fn rolling(&mut self, sample: &Sample, hidden: bool, model: Option<&str>) {
        if sample.codex && sample.spend_control.is_some() {
            self.spend_control = sample.spend_control;
        }
        if sample.reached.is_some() {
            self.reached = sample.reached;
        }
        if sample.credits.is_some() {
            self.credits = sample.credits;
        }
        let has_workspace_credits = !self.workspace_limit_reached() && self.credits == Some(true);
        if sample.codex && has_workspace_credits {
            match self.stage {
                Stage::Pending | Stage::Asking => self.stage = Stage::Idle,
                Stage::Shown => self.switch_open = false,
                Stage::Idle => {}
            }
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

    /// `on_rate_limit_error`: the limit the failure names, a usage limit being what a credits
    /// limit is when the error says so.
    fn failed(&mut self, kind: Failure) -> Option<Nudge> {
        let usage = kind == Failure::UsageLimit;
        self.reached = self.reached.map(|reached| match (usage, reached) {
            (true, RateLimitReachedType::WorkspaceOwnerCreditsDepleted) => {
                RateLimitReachedType::WorkspaceOwnerUsageLimitReached
            }
            (true, RateLimitReachedType::WorkspaceMemberCreditsDepleted) => {
                RateLimitReachedType::WorkspaceMemberUsageLimitReached
            }
            (_, other) => other,
        });
        match self.reached {
            Some(RateLimitReachedType::WorkspaceMemberCreditsDepleted) => Some(Nudge::Credits),
            Some(RateLimitReachedType::WorkspaceMemberUsageLimitReached) => {
                Some(Nudge::UsageLimit)
            }
            _ => None,
        }
    }
}

/// The model the thread in use runs on: its own settings, else the config's.
fn model_in_use(view: AppView<'_>) -> Option<String> {
    view.active_thread()
        .and_then(|thread| thread.slots().chat.model.clone())
        .or_else(|| view.screens().chat.status.model.clone())
}

fn failure_of(info: &CodexErrorInfo) -> Option<Failure> {
    match info {
        CodexErrorInfo::UsageLimitExceeded => Some(Failure::UsageLimit),
        CodexErrorInfo::ResponseTooManyFailedAttempts {
            http_status_code: Some(429),
        } => Some(Failure::Generic),
        _ => None,
    }
}

fn sample_of(snapshot: &codex_app_server_protocol::RateLimitSnapshot) -> Sample {
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

/// `maybe_show_pending_rate_limit_prompt`, at the end of a turn.
fn maybe_show(state: &mut State, cx: &mut AppCx<'_>) {
    if state.stage != Stage::Pending {
        return;
    }
    let model = model_in_use(cx.view());
    if cx.view().facts().tui().hide_rate_limit_model_nudge
        || matches!(model.as_deref(), Some(NUDGE_MODEL | RESERVE_MODEL))
    {
        state.stage = Stage::Idle;
        return;
    }
    state.stage = Stage::Asking;
    cx.ask(None, Ask::Models, Ctx::Models);
}

feature! {
    impl Feature for RateLimitPrompt {
        const NAME: &'static str = "rate_limit_prompt";
        type State = State;
        type Intent = Intent;
        type Command = Ask;
        type Reply = Reply;
        type Ctx = Ctx;
        type Note = Note;
        type Wish = Wish;
        type Event = Event;
        {
            fn project(notification: &ServerNotification) -> Option<Event> {
                match notification {
                    ServerNotification::AccountRateLimitsUpdated(update) => {
                        Some(Event::Rolling(sample_of(&update.rate_limits)))
                    }
                    ServerNotification::TurnCompleted(done) => Some(Event::TurnEnded {
                        thread: ThreadId::new(done.thread_id.clone()),
                    }),
                    ServerNotification::Error(error) if !error.will_retry => {
                        let kind = failure_of(error.error.codex_error_info.as_ref()?)?;
                        Some(Event::Failed {
                            thread: ThreadId::new(error.thread_id.clone()),
                            kind,
                        })
                    }
                    _ => None,
                }
            }

            fn on_event(state: &mut State, event: Event, cx: &mut AppCx<'_>) {
                match event {
                    Event::Rolling(sample) => {
                        let hidden = cx.view().facts().tui().hide_rate_limit_model_nudge;
                        let model = model_in_use(cx.view());
                        state.rolling(&sample, hidden, model.as_deref());
                    }
                    Event::TurnEnded { thread } => {
                        if cx.view().active() == Some(&thread) {
                            maybe_show(state, cx);
                        }
                    }
                    Event::Failed { thread, kind } => {
                        let nudge = state.failed(kind);
                        if cx.view().active() == Some(&thread)
                            && state.sending.is_none()
                            && let Some(nudge) = nudge
                        {
                            cx.wish(thread, Wish::Nudge(nudge));
                        }
                    }
                }
            }

            fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
                match intent {
                    Intent::Switch { model, effort } => {
                        state.switch_open = false;
                        cx.verb(PublicVerb::OverrideTurn {
                            model,
                            effort: Some(effort),
                        });
                    }
                    Intent::Nudge(nudge) => {
                        if state.sending.is_none() {
                            state.sending = Some(nudge);
                            cx.ask(None, Ask::Nudge(nudge), Ctx::Nudge(nudge));
                        }
                    }
                    Intent::Closed => state.switch_open = false,
                }
            }

            fn answered(
                state: &mut State,
                ctx: Ctx,
                result: Result<Reply, RpcError>,
                cx: &mut AppCx<'_>,
            ) {
                match (ctx, result) {
                    (Ctx::Models, Ok(Reply::Candidate(Some(candidate))))
                        if state.stage == Stage::Asking =>
                    {
                        state.stage = Stage::Shown;
                        state.switch_open = true;
                        if let Some(thread) = cx.view().active().cloned() {
                            cx.wish(thread, Wish::Switch(candidate));
                        }
                    }
                    (Ctx::Models, _) => {
                        if state.stage == Stage::Asking {
                            state.stage = Stage::Idle;
                        }
                    }
                    (Ctx::Nudge(nudge), result) => {
                        state.sending = None;
                        let outcome = match result {
                            Ok(Reply::Nudged(outcome)) => outcome,
                            _ => NudgeOutcome::Failed,
                        };
                        cx.note(None, Note::Nudged { nudge, outcome });
                    }
                }
            }

            fn note_tone(note: &Note) -> Tone {
                match note {
                    Note::Nudged { .. } => Tone::Info,
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

/// What the slice says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// The e-mail to the workspace owner was answered
    /// (`finish_add_credits_nudge_email_request`).
    Nudged { nudge: Nudge, outcome: NudgeOutcome },
}
