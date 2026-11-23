//! `/goal` (oracle `slash_dispatch.rs` `SlashCommand::Goal`, `app/thread_goal_actions.rs`): the
//! goal of the thread in use, shown, edited, set, replaced, paused, resumed and cleared. One
//! flow of the app (ADR-003): the asks go out as screen commands of the select screen with the
//! thread named (`select::Command::Goal`), the step an ask belongs to rides with its ticket
//! (`goal::Flow`), and what the answer settles is said as a note of the thread
//! (`GoalNote`) or opened as a confirmation (`Menu::Goal`) or the edit prompt
//! (`PromptKind::GoalEdit`). The words are the view's.
//!
//! An answer for a thread that is no longer the one in use is dropped (`current_displayed_thread_id`).
//! The composer's pastes (`GoalDraft::pending_pastes`) and the objective files the host writes for
//! a large objective (`goal_files`) are not modelled: the objective goes as typed.

use std::rc::Rc;

use super::session_note;
use crate::app::{AppFx, CodexApp, apply_thread};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::dialogs::{Open, Prompt};
use crate::screens::select::goal::{self, Ask, Draft, Flow, Mode};
use crate::screens::select::{self, Menu, Reply};
use crate::screens::{ScreenThreadIntent, chat};
use crate::threads::{
    Goal, GoalAction, GoalFailure, GoalHint, GoalNote, GoalStatus, Note, NoteKey, ThreadEv,
};

fn said(app: &mut CodexApp, thread: Option<ThreadId>, note: GoalNote, fx: &mut AppFx) {
    session_note(app, thread, Note::from(NoteKey::Goal(note)), fx);
}

fn ask(app: &mut CodexApp, thread: &ThreadId, ask: Ask, flow: Flow, fx: &mut AppFx) {
    let command = select::Command::Goal {
        thread: thread.to_string(),
        ask,
    };
    let ctx = select::Ctx::Goal {
        thread: thread.clone(),
        flow,
    };
    crate::app::select::ask(app, command, ctx, fx);
}

/// `/goal` and `/goal <args>` in the thread `source` (none before the session has started).
pub(in crate::app) fn command(
    app: &mut CodexApp,
    source: Option<ThreadId>,
    args: Option<String>,
    fx: &mut AppFx,
) {
    let thread = source.filter(|id| app.threads.get(id).is_some());
    let args = args
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty());
    let Some(args) = args else {
        return match thread {
            Some(thread) => ask(app, &thread, Ask::Get, Flow::Show, fx),
            None => said(app, None, GoalNote::Usage(GoalHint::Example), fx),
        };
    };
    let keyword = args.to_ascii_lowercase();
    match (keyword.as_str(), thread) {
        ("edit", None) => no_goal_to_edit(app, None, fx),
        ("edit", Some(thread)) => ask(app, &thread, Ask::Get, Flow::Edit, fx),
        ("clear" | "pause" | "resume", None) => {
            said(app, None, GoalNote::Usage(GoalHint::StartToChange), fx);
        }
        ("clear", Some(thread)) => ask(app, &thread, Ask::Clear, Flow::Clear, fx),
        ("pause", Some(thread)) => set_status(app, &thread, GoalStatus::Paused, fx),
        ("resume", Some(thread)) => set_status(app, &thread, GoalStatus::Active, fx),
        (_, None) => said(app, None, GoalNote::Usage(GoalHint::StartToSet), fx),
        (_, Some(thread)) => {
            let draft = Draft {
                objective: args,
                mode: Mode::ConfirmIfExists,
            };
            set_draft(app, &thread, draft, fx);
        }
    }
}

/// `maybe_prompt_resume_paused_goal_after_resume`: a resumed thread whose goal stopped asks
/// whether to go on. The app calls it when the resume of a thread in use has been applied.
pub(in crate::app) fn after_resume(app: &mut CodexApp, thread: &ThreadId, fx: &mut AppFx) {
    ask(app, thread, Ask::Get, Flow::AfterResume, fx);
}

/// `AppEvent::SetThreadGoalStatus`.
fn set_status(app: &mut CodexApp, thread: &ThreadId, status: GoalStatus, fx: &mut AppFx) {
    let set = Ask::Set {
        objective: None,
        status: Some(status),
        token_budget: None,
    };
    ask(app, thread, set, Flow::Status, fx);
}

/// `AppEvent::SetThreadGoalDraft` (`set_thread_goal_draft`).
fn set_draft(app: &mut CodexApp, thread: &ThreadId, draft: Draft, fx: &mut AppFx) {
    match draft.mode.clone() {
        Mode::ConfirmIfExists => ask(app, thread, Ask::Get, Flow::Check(draft), fx),
        Mode::ReplaceExisting => ask(app, thread, Ask::Clear, Flow::Clearing(draft), fx),
        Mode::UpdateExisting {
            status,
            token_budget,
        } => {
            let set = Ask::Set {
                objective: Some(draft.objective),
                status: Some(status),
                token_budget: Some(token_budget),
            };
            ask(app, thread, set, Flow::Setting { replacing: false }, fx);
        }
    }
}

/// `show_no_thread_goal_to_edit`: the error, then the usage with its hint.
fn no_goal_to_edit(app: &mut CodexApp, thread: Option<ThreadId>, fx: &mut AppFx) {
    said(app, thread.clone(), GoalNote::NoGoalToEdit, fx);
    said(app, thread, GoalNote::Usage(GoalHint::CreateFirst), fx);
}

fn open_menu(app: &mut CodexApp, menu: goal::Menu, fx: &mut AppFx) {
    select::apply(
        &mut Rc::make_mut(&mut app.screens).select,
        select::Ev::Opened(Menu::Goal(Box::new(menu))),
        fx,
    );
}

fn failed(
    app: &mut CodexApp,
    thread: &ThreadId,
    action: GoalAction,
    error: &RpcError,
    fx: &mut AppFx,
) {
    let failure = GoalFailure::of(error);
    said(
        app,
        Some(thread.clone()),
        GoalNote::Failed { action, failure },
        fx,
    );
}

/// The answer of an ask of the flow `flow` on `thread`.
pub(in crate::app) fn answered(
    app: &mut CodexApp,
    thread: ThreadId,
    flow: Flow,
    result: Result<Reply, RpcError>,
    fx: &mut AppFx,
) {
    if app.session.active.as_ref() != Some(&thread) {
        return;
    }
    match (flow, result) {
        // A read after a resume that failed is only logged.
        (Flow::AfterResume, Err(_)) => {}
        (Flow::Show | Flow::Edit | Flow::Check(_), Err(error)) => {
            failed(app, &thread, GoalAction::Read, &error, fx);
        }
        (Flow::Clearing(_), Err(error)) => failed(app, &thread, GoalAction::Replace, &error, fx),
        (Flow::Setting { replacing }, Err(error)) => {
            let action = if replacing {
                GoalAction::Replace
            } else {
                GoalAction::Set
            };
            failed(app, &thread, action, &error, fx);
        }
        (Flow::Status, Err(error)) => failed(app, &thread, GoalAction::Update, &error, fx),
        (Flow::Clear, Err(error)) => failed(app, &thread, GoalAction::Clear, &error, fx),

        (Flow::Show, Ok(Reply::Goal(Some(goal)))) => {
            said(app, Some(thread), GoalNote::Summary(goal), fx);
        }
        (Flow::Show, Ok(Reply::Goal(None))) => {
            said(app, Some(thread), GoalNote::Usage(GoalHint::NoGoal), fx);
        }
        (Flow::Edit, Ok(Reply::Goal(Some(goal)))) => {
            Rc::make_mut(&mut app.screens).dialogs.open =
                Some(Open::Prompt(Prompt::goal_edit(thread, &goal)));
        }
        (Flow::Edit, Ok(Reply::Goal(None))) => no_goal_to_edit(app, Some(thread), fx),
        (Flow::AfterResume, Ok(Reply::Goal(Some(goal)))) => {
            if goal::offers_resume(&goal) {
                let menu = goal::Menu::Resume {
                    thread,
                    objective: goal.objective,
                };
                open_menu(app, menu, fx);
            }
        }
        (Flow::Check(draft), Ok(Reply::Goal(Some(goal)))) => {
            if goal::confirms_replace(&goal) {
                let menu = goal::Menu::Replace {
                    thread,
                    objective: draft.objective,
                };
                open_menu(app, menu, fx);
            } else {
                replace(app, &thread, draft.objective, fx);
            }
        }
        (Flow::Check(draft), Ok(Reply::Goal(None))) => {
            set_objective(app, &thread, draft.objective, false, fx);
        }
        (Flow::Clearing(draft), Ok(_)) => set_objective(app, &thread, draft.objective, true, fx),
        (Flow::Setting { .. }, Ok(Reply::Goal(Some(goal)))) => {
            said_status(app, thread.clone(), goal, fx);
            // `maybe_send_next_queued_input`: what waited for the goal goes on.
            let resume =
                ThreadEv::ScreenIntent(ScreenThreadIntent::Chat(chat::ThreadIntent::QueueResume));
            apply_thread(app, &thread, resume, fx);
        }
        (Flow::Status, Ok(Reply::Goal(Some(goal)))) => said_status(app, thread, goal, fx),
        (Flow::Clear, Ok(Reply::GoalCleared(true))) => {
            said(app, Some(thread), GoalNote::Cleared, fx);
        }
        (Flow::Clear, Ok(Reply::GoalCleared(false))) => {
            said(app, Some(thread), GoalNote::NothingToClear, fx);
        }
        // An answer of another shape than its ask: nothing to say.
        (_, Ok(_)) => {}
    }
}

fn said_status(app: &mut CodexApp, thread: ThreadId, goal: Goal, fx: &mut AppFx) {
    said(app, Some(thread), GoalNote::Status(goal), fx);
}

/// A new goal over none (or a finished one): active, no budget.
fn set_objective(
    app: &mut CodexApp,
    thread: &ThreadId,
    objective: String,
    replacing: bool,
    fx: &mut AppFx,
) {
    let set = Ask::Set {
        objective: Some(objective),
        status: Some(GoalStatus::Active),
        token_budget: None,
    };
    ask(app, thread, set, Flow::Setting { replacing }, fx);
}

/// A new objective over a goal that is finished (no confirmation) or that the user agreed to
/// replace: the goal is cleared first.
fn replace(app: &mut CodexApp, thread: &ThreadId, objective: String, fx: &mut AppFx) {
    let draft = Draft {
        objective,
        mode: Mode::ReplaceExisting,
    };
    set_draft(app, thread, draft, fx);
}

/// What a confirmation settled.
pub(in crate::app) fn picked(app: &mut CodexApp, pick: goal::Pick, fx: &mut AppFx) {
    match pick {
        goal::Pick::Resume { thread } => set_status(app, &thread, GoalStatus::Active, fx),
        goal::Pick::Replace { thread, objective } => replace(app, &thread, objective, fx),
    }
}

/// The edit prompt, submitted: the objective changes, the status and the budget stay.
pub(in crate::app) fn edited(
    app: &mut CodexApp,
    thread: ThreadId,
    status: GoalStatus,
    token_budget: Option<i64>,
    objective: String,
    fx: &mut AppFx,
) {
    let draft = Draft {
        objective,
        mode: Mode::UpdateExisting {
            status,
            token_budget,
        },
    };
    set_draft(app, &thread, draft, fx);
}
