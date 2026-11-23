//! `/goal`'s values (oracle `app/thread_goal_actions.rs`, `chatwidget/goal_menu.rs`,
//! `goal_display.rs`): how a goal is set, the step of the flow an ask belongs to and the
//! small rules the flow decides by. Values only; the words are `src/features/goal/strings.rs`.

use crate::ids::ThreadId;
use crate::threads::{Goal, GoalStatus};

/// How `/goal <objective>` and the edit prompt set a goal (oracle `ThreadGoalSetMode`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The goal is read first: an unfinished one asks before it is replaced.
    ConfirmIfExists,
    /// The goal is cleared, then set again.
    ReplaceExisting,
    /// The objective changes, the status and the budget stay (the edit prompt).
    UpdateExisting {
        status: GoalStatus,
        token_budget: Option<i64>,
    },
}

/// What a goal is set to (oracle `GoalDraft`; the pastes of the composer are the host's
/// `goal_files`, not modelled here).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub objective: String,
    pub mode: Mode,
}

/// The status the edit prompt keeps (`edited_goal_status`): a finished or budget-limited goal
/// is edited back to active.
pub fn edited_status(status: GoalStatus) -> GoalStatus {
    match status {
        GoalStatus::Active | GoalStatus::BudgetLimited | GoalStatus::Complete => GoalStatus::Active,
        GoalStatus::Paused | GoalStatus::Blocked | GoalStatus::UsageLimited => status,
    }
}

/// An unfinished goal asks before a new objective replaces it
/// (`should_confirm_before_replacing_goal`): a complete one is terminal.
pub fn confirms_replace(goal: &Goal) -> bool {
    goal.status != GoalStatus::Complete
}

/// After a resume, a goal that stopped asks whether to go on
/// (`maybe_prompt_resume_paused_goal_after_resume`).
pub fn offers_resume(goal: &Goal) -> bool {
    matches!(
        goal.status,
        GoalStatus::Paused | GoalStatus::Blocked | GoalStatus::UsageLimited
    )
}

/// The `Commands: …` hint under the goal summary: which of `/goal pause` and `/goal resume`
/// applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandsHint {
    /// `/goal edit, /goal pause, /goal clear`.
    Pause,
    /// `/goal edit, /goal resume, /goal clear`.
    Resume,
    /// `/goal edit, /goal clear`.
    Finished,
}

impl CommandsHint {
    pub fn of(status: GoalStatus) -> Self {
        match status {
            GoalStatus::Active => CommandsHint::Pause,
            GoalStatus::Paused | GoalStatus::Blocked | GoalStatus::UsageLimited => {
                CommandsHint::Resume
            }
            GoalStatus::BudgetLimited | GoalStatus::Complete => CommandsHint::Finished,
        }
    }
}

/// The step of `/goal` an outgoing ask belongs to, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flow {
    /// The bare `/goal`: the goal is shown as a summary.
    Show,
    /// `/goal edit`: the prompt opens on the goal's objective.
    Edit,
    /// After a resume: a stopped goal asks whether to go on.
    AfterResume,
    /// `/goal <objective>` over the goal read first.
    Check(Draft),
    /// A replace: the old goal was cleared, the draft is set next.
    Clearing(Draft),
    /// The draft set (`replacing`: it replaced a goal, which names the failure `replace`).
    Setting { replacing: bool },
    /// `/goal pause|resume` or the resume row: only the status changes.
    Status,
    /// `/goal clear`.
    Clear,
}

/// What an ask of the flow was for: the step, on the thread it was asked of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    pub thread: ThreadId,
    pub flow: Flow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn goal(status: GoalStatus) -> Goal {
        Goal {
            objective: "ship".into(),
            status,
            token_budget: None,
            tokens_used: 0,
            time_used_seconds: 0,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn only_an_unfinished_goal_asks_before_it_is_replaced() {
        assert!(!confirms_replace(&goal(GoalStatus::Complete)));
        for status in [
            GoalStatus::Active,
            GoalStatus::Paused,
            GoalStatus::Blocked,
            GoalStatus::UsageLimited,
            GoalStatus::BudgetLimited,
        ] {
            assert!(confirms_replace(&goal(status)));
        }
    }

    #[test]
    fn the_edit_prompt_keeps_a_stopped_status_and_restarts_a_finished_goal() {
        assert_eq!(edited_status(GoalStatus::Paused), GoalStatus::Paused);
        assert_eq!(edited_status(GoalStatus::Blocked), GoalStatus::Blocked);
        assert_eq!(
            edited_status(GoalStatus::UsageLimited),
            GoalStatus::UsageLimited
        );
        assert_eq!(edited_status(GoalStatus::Complete), GoalStatus::Active);
        assert_eq!(edited_status(GoalStatus::BudgetLimited), GoalStatus::Active);
    }
}
