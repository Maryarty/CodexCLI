//! `/goal` (oracle `app/thread_goal_actions.rs`, `chatwidget/goal_menu.rs`, `goal_display.rs`):
//! the two confirmations (`Resume paused goal?`, `Replace goal?`) as typed menus of the select
//! screen, and the asks and the chain of steps behind `/goal`, as values. The words (titles,
//! rows, subtitles, the summary lines, the usage) are the view's (ADR-010); the app carries the
//! flow out (`app/session/goal.rs`).
//!
//! The asks are the thread's (`thread/goal/get|set|clear`) and go as screen commands of this
//! screen with the thread named in them (`Command::Goal`), the way `Command::Settings` does.

use crate::ids::ThreadId;
use crate::threads::{Goal, GoalStatus, ThreadAsk};

/// The rows of `Resume paused goal?` and `Replace goal?`, in order.
pub const RESUME: &str = "goal.resume";
pub const LEAVE: &str = "goal.leave";
pub const REPLACE: &str = "goal.replace";
pub const CANCEL: &str = "goal.cancel";

/// A confirmation of the goal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Menu {
    /// `Resume paused goal?` (subtitle `Goal: {objective}`): rows `Resume goal` (the default) and
    /// `Leave paused`.
    Resume { thread: ThreadId, objective: String },
    /// `Replace goal?` (subtitle `New objective: {objective}`, cut at 200 graphemes by the view):
    /// rows `Replace current goal` and `Cancel`.
    Replace { thread: ThreadId, objective: String },
}

/// What a pick of a row settled (`Cancel` and `Leave paused` only dismiss: nothing is picked).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// `AppEvent::SetThreadGoalStatus` with `Active`.
    Resume { thread: ThreadId },
    /// `AppEvent::SetThreadGoalDraft` with `ReplaceExisting`.
    Replace { thread: ThreadId, objective: String },
}

impl Menu {
    /// The ids of the rows, in order.
    pub fn rows(&self) -> [&'static str; 2] {
        match self {
            Menu::Resume { .. } => [RESUME, LEAVE],
            Menu::Replace { .. } => [REPLACE, CANCEL],
        }
    }

    /// The row `id` chosen: whether it belongs to the menu (it is dismissed) and what it picked.
    pub(crate) fn choose(&self, id: &str) -> Option<Option<Pick>> {
        if !self.rows().contains(&id) {
            return None;
        }
        Some(match (self, id) {
            (Menu::Resume { thread, .. }, RESUME) => Some(Pick::Resume {
                thread: thread.clone(),
            }),
            (Menu::Replace { thread, objective }, REPLACE) => Some(Pick::Replace {
                thread: thread.clone(),
                objective: objective.clone(),
            }),
            _ => None,
        })
    }
}

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
        GoalStatus::Active | GoalStatus::BudgetLimited | GoalStatus::Complete => {
            GoalStatus::Active
        }
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
pub enum Hint {
    /// `/goal edit, /goal pause, /goal clear`.
    Pause,
    /// `/goal edit, /goal resume, /goal clear`.
    Resume,
    /// `/goal edit, /goal clear`.
    Finished,
}

impl Hint {
    pub fn of(status: GoalStatus) -> Self {
        match status {
            GoalStatus::Active => Hint::Pause,
            GoalStatus::Paused | GoalStatus::Blocked | GoalStatus::UsageLimited => Hint::Resume,
            GoalStatus::BudgetLimited | GoalStatus::Complete => Hint::Finished,
        }
    }
}

/// One of the three asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// `thread/goal/get`.
    Get,
    /// `thread/goal/set` as the user.
    Set {
        objective: Option<String>,
        status: Option<GoalStatus>,
        token_budget: Option<Option<i64>>,
    },
    /// `thread/goal/clear` as the user.
    Clear,
}

impl Ask {
    pub(crate) fn thread_ask(&self, thread: &str) -> ThreadAsk {
        let thread = thread.to_owned();
        match self {
            Ask::Get => ThreadAsk::GoalGet { thread },
            Ask::Set {
                objective,
                status,
                token_budget,
            } => ThreadAsk::GoalSet {
                thread,
                objective: objective.clone(),
                status: *status,
                token_budget: *token_budget,
            },
            Ask::Clear => ThreadAsk::GoalClear { thread },
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

    #[test]
    fn a_menu_row_picks_what_it_says_and_a_leave_only_closes() {
        let thread = ThreadId::new("t1");
        let resume = Menu::Resume {
            thread: thread.clone(),
            objective: "ship".into(),
        };
        assert_eq!(
            resume.choose(RESUME),
            Some(Some(Pick::Resume {
                thread: thread.clone()
            }))
        );
        assert_eq!(resume.choose(LEAVE), Some(None));
        assert_eq!(resume.choose(REPLACE), None);
        let replace = Menu::Replace {
            thread: thread.clone(),
            objective: "new".into(),
        };
        assert_eq!(
            replace.choose(REPLACE),
            Some(Some(Pick::Replace {
                thread,
                objective: "new".into()
            }))
        );
        assert_eq!(replace.choose(CANCEL), Some(None));
    }
}
