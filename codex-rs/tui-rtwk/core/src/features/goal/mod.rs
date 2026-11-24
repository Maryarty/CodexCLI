//! `/goal` as a feature slice (ADR-012; oracle `slash_dispatch.rs` `SlashCommand::Goal`,
//! `app/thread_goal_actions.rs`, `chatwidget/goal_menu.rs`): the goal of the thread in use,
//! shown, edited, set, replaced, paused, resumed and cleared. One flow: the asks go out as
//! the feature's RPCs with the thread named, the step an ask belongs to rides with its ticket
//! (`Ctx`), and what the answer settles is said as a note of the thread (`Note`) or asked of the
//! surface as a one-shot (`Wish`: a confirmation, the edit prompt) whose answer comes back as a
//! verb (`Intent::Resume`, `Replace`, `Edit`). How the question looks, and what is typed in it,
//! is the UI half's (`src/features/goal/`); the words are `src/features/goal/strings.rs`.
//!
//! An answer for a thread that is no longer the one in use is dropped
//! (`current_displayed_thread_id`). The composer's pastes (`GoalDraft::pending_pastes`) and the
//! objective files the host writes for a large objective (`goal_files`) are not modelled: the
//! objective goes as typed.
//!
//! The goal itself (`threads::Goal`) is a fact of the thread, projected from the server's
//! notifications (`ThreadEv::Goal`) and read by the status line: it stays in `threads`.

mod model;
mod note;
mod wire;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never, Slash};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::{Goal as GoalFact, GoalStatus, Tone};

pub use model::{
    CommandsHint, Ctx, Draft, Flow, Mode, confirms_replace, edited_status, offers_resume,
};
pub use note::{Action, Failure, Note, UsageHint};
pub use wire::{Ask, Reply};

/// The marker type of the feature.
pub struct Goal;

/// What the user asks of `/goal`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/goal` and `/goal <args>`, typed in the thread `source` (none before the session has
    /// started).
    Command {
        source: Option<ThreadId>,
        args: Option<String>,
    },
    /// `Resume goal` of the question over a paused goal (`AppEvent::SetThreadGoalStatus` with
    /// `Active`).
    Resume { thread: ThreadId },
    /// `Replace current goal` of the question over a new objective
    /// (`AppEvent::SetThreadGoalDraft` with `ReplaceExisting`).
    Replace { thread: ThreadId, objective: String },
    /// The edit prompt submitted: the objective changes, the status and the budget stay.
    Edit {
        thread: ThreadId,
        objective: String,
        status: GoalStatus,
        token_budget: Option<i64>,
    },
}

/// What the goal asks the surface to show, once, for the thread it is about (`Scope::Thread`):
/// the question or the prompt, whose answer is an `Intent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    /// `Resume paused goal?` over the goal `objective` that stopped: answered by
    /// `Intent::Resume`.
    Resume { thread: ThreadId, objective: String },
    /// `Replace goal?` over the new `objective`: answered by `Intent::Replace`.
    Replace { thread: ThreadId, objective: String },
    /// The edit prompt (oracle `show_goal_edit_prompt`: `Edit goal`) over the objective of
    /// `thread`'s goal, which keeps the status and the token budget it had: answered by
    /// `Intent::Edit`.
    Edit {
        thread: ThreadId,
        objective: String,
        status: GoalStatus,
        token_budget: Option<i64>,
    },
}

fn said(cx: &mut AppCx<'_>, thread: Option<ThreadId>, note: Note) {
    cx.note(thread, note);
}

fn ask(cx: &mut AppCx<'_>, thread: &ThreadId, ask: Ask, flow: Flow) {
    let ctx = Ctx {
        thread: thread.clone(),
        flow,
    };
    cx.ask(Some(thread), ask, ctx);
}

/// `AppEvent::SetThreadGoalStatus`.
fn set_status(cx: &mut AppCx<'_>, thread: &ThreadId, status: GoalStatus) {
    let set = Ask::Set {
        objective: None,
        status: Some(status),
        token_budget: None,
    };
    ask(cx, thread, set, Flow::Status);
}

/// `AppEvent::SetThreadGoalDraft` (`set_thread_goal_draft`).
fn set_draft(cx: &mut AppCx<'_>, thread: &ThreadId, draft: Draft) {
    match draft.mode.clone() {
        Mode::ConfirmIfExists => ask(cx, thread, Ask::Get, Flow::Check(draft)),
        Mode::ReplaceExisting => ask(cx, thread, Ask::Clear, Flow::Clearing(draft)),
        Mode::UpdateExisting {
            status,
            token_budget,
        } => {
            let set = Ask::Set {
                objective: Some(draft.objective),
                status: Some(status),
                token_budget: Some(token_budget),
            };
            ask(cx, thread, set, Flow::Setting { replacing: false });
        }
    }
}

/// `show_no_thread_goal_to_edit`: the error, then the usage with its hint.
fn no_goal_to_edit(cx: &mut AppCx<'_>, thread: Option<ThreadId>) {
    said(cx, thread.clone(), Note::NoGoalToEdit);
    said(cx, thread, Note::Usage(UsageHint::CreateFirst));
}

fn failed(cx: &mut AppCx<'_>, thread: &ThreadId, action: Action, error: &RpcError) {
    let failure = Failure::of(error);
    said(cx, Some(thread.clone()), Note::Failed { action, failure });
}

/// A new goal over none (or a finished one): active, no budget.
fn set_objective(cx: &mut AppCx<'_>, thread: &ThreadId, objective: String, replacing: bool) {
    let set = Ask::Set {
        objective: Some(objective),
        status: Some(GoalStatus::Active),
        token_budget: None,
    };
    ask(cx, thread, set, Flow::Setting { replacing });
}

/// A new objective over a goal that is finished (no confirmation) or that the user agreed to
/// replace: the goal is cleared first.
fn replace(cx: &mut AppCx<'_>, thread: &ThreadId, objective: String) {
    let draft = Draft {
        objective,
        mode: Mode::ReplaceExisting,
    };
    set_draft(cx, thread, draft);
}

fn command(cx: &mut AppCx<'_>, source: Option<ThreadId>, args: Option<String>) {
    let thread = source.filter(|id| cx.view().thread(id).is_some());
    let args = args
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty());
    let Some(args) = args else {
        return match thread {
            Some(thread) => ask(cx, &thread, Ask::Get, Flow::Show),
            None => said(cx, None, Note::Usage(UsageHint::Example)),
        };
    };
    let keyword = args.to_ascii_lowercase();
    match (keyword.as_str(), thread) {
        ("edit", None) => no_goal_to_edit(cx, None),
        ("edit", Some(thread)) => ask(cx, &thread, Ask::Get, Flow::Edit),
        ("clear" | "pause" | "resume", None) => {
            said(cx, None, Note::Usage(UsageHint::StartToChange));
        }
        ("clear", Some(thread)) => ask(cx, &thread, Ask::Clear, Flow::Clear),
        ("pause", Some(thread)) => set_status(cx, &thread, GoalStatus::Paused),
        ("resume", Some(thread)) => set_status(cx, &thread, GoalStatus::Active),
        (_, None) => said(cx, None, Note::Usage(UsageHint::StartToSet)),
        (_, Some(thread)) => {
            let draft = Draft {
                objective: args,
                mode: Mode::ConfirmIfExists,
            };
            set_draft(cx, &thread, draft);
        }
    }
}

fn said_status(cx: &mut AppCx<'_>, thread: ThreadId, goal: GoalFact) {
    said(cx, Some(thread), Note::Status(goal));
}

impl Feature for Goal {
    const NAME: &'static str = "goal";
    const RUNS: &'static [SlashCommand] = &[SlashCommand::Goal];

    type State = ();
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Ask;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Wish;
    type Interest = Never<Self>;
    type Event = Never<Self>;

    /// `/goal`: the goal shown, set, paused, resumed, cleared or edited.
    fn slash(state: &mut (), slash: Slash, cx: &mut AppCx<'_>) {
        let source = slash.source.clone();
        let args = slash.into_args();
        Self::intent(state, Intent::Command { source, args }, cx);
    }

    fn intent(_state: &mut (), intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Command { source, args } => command(cx, source, args),
            Intent::Resume { thread } => set_status(cx, &thread, GoalStatus::Active),
            Intent::Replace { thread, objective } => replace(cx, &thread, objective),
            Intent::Edit {
                thread,
                objective,
                status,
                token_budget,
            } => {
                let objective = objective.trim().to_owned();
                if objective.is_empty() {
                    return;
                }
                let mode = Mode::UpdateExisting {
                    status,
                    token_budget,
                };
                set_draft(cx, &thread, Draft { objective, mode });
            }
        }
    }

    /// The answer of an ask of the flow `ctx.flow` on `ctx.thread`.
    fn answered(_state: &mut (), ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        let Ctx { thread, flow } = ctx;
        if cx.view().active() != Some(&thread) {
            return;
        }
        match (flow, result) {
            // A read after a resume that failed is only logged.
            (Flow::AfterResume, Err(_)) => {}
            (Flow::Show | Flow::Edit | Flow::Check(_), Err(error)) => {
                failed(cx, &thread, Action::Read, &error);
            }
            (Flow::Clearing(_), Err(error)) => failed(cx, &thread, Action::Replace, &error),
            (Flow::Setting { replacing }, Err(error)) => {
                let action = if replacing {
                    Action::Replace
                } else {
                    Action::Set
                };
                failed(cx, &thread, action, &error);
            }
            (Flow::Status, Err(error)) => failed(cx, &thread, Action::Update, &error),
            (Flow::Clear, Err(error)) => failed(cx, &thread, Action::Clear, &error),

            (Flow::Show, Ok(Reply::Goal(Some(goal)))) => {
                said(cx, Some(thread), Note::Summary(goal));
            }
            (Flow::Show, Ok(Reply::Goal(None))) => {
                said(cx, Some(thread), Note::Usage(UsageHint::NoGoal));
            }
            (Flow::Edit, Ok(Reply::Goal(Some(goal)))) => {
                let wish = Wish::Edit {
                    thread: thread.clone(),
                    objective: goal.objective,
                    status: edited_status(goal.status),
                    token_budget: goal.token_budget,
                };
                cx.wish(thread, wish);
            }
            (Flow::Edit, Ok(Reply::Goal(None))) => no_goal_to_edit(cx, Some(thread)),
            (Flow::AfterResume, Ok(Reply::Goal(Some(goal)))) => {
                if offers_resume(&goal) {
                    let wish = Wish::Resume {
                        thread: thread.clone(),
                        objective: goal.objective,
                    };
                    cx.wish(thread, wish);
                }
            }
            (Flow::Check(draft), Ok(Reply::Goal(Some(goal)))) => {
                if confirms_replace(&goal) {
                    let wish = Wish::Replace {
                        thread: thread.clone(),
                        objective: draft.objective,
                    };
                    cx.wish(thread, wish);
                } else {
                    replace(cx, &thread, draft.objective);
                }
            }
            (Flow::Check(draft), Ok(Reply::Goal(None))) => {
                set_objective(cx, &thread, draft.objective, false);
            }
            (Flow::Clearing(draft), Ok(_)) => set_objective(cx, &thread, draft.objective, true),
            (Flow::Setting { .. }, Ok(Reply::Goal(Some(goal)))) => {
                said_status(cx, thread.clone(), goal);
                // `maybe_send_next_queued_input`: what waited for the goal goes on.
                cx.settled(&thread);
            }
            (Flow::Status, Ok(Reply::Goal(Some(goal)))) => said_status(cx, thread, goal),
            (Flow::Clear, Ok(Reply::Cleared(true))) => {
                said(cx, Some(thread), Note::Cleared);
            }
            (Flow::Clear, Ok(Reply::Cleared(false))) => {
                said(cx, Some(thread), Note::NothingToClear);
            }
            // An answer of another shape than its ask: nothing to say.
            (_, Ok(_)) => {}
        }
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::NoGoalToEdit | Note::Failed { .. } => Tone::Error,
            _ => Tone::Info,
        }
    }

    /// `maybe_prompt_resume_paused_goal_after_resume`: a resumed thread whose goal stopped asks
    /// whether to go on. Runs when the resume of a thread in use has been applied.
    fn on_resumed(_state: &mut (), thread: &ThreadId, cx: &mut AppCx<'_>) {
        ask(cx, thread, Ask::Get, Flow::AfterResume);
    }

    fn wire(command: &Ask, thread: &ThreadId, id: RequestId) -> ClientRequest {
        wire::request(command, thread, id)
    }

    fn decode(command: &Ask, result: serde_json::Value) -> Result<Reply, RpcError> {
        wire::reply(command, result)
    }
}
