//! The two confirmations of `/goal` (oracle `chatwidget/goal_menu.rs`,
//! `app/thread_goal_actions.rs`): `Resume paused goal?` and `Replace goal?`, as the typed menus
//! of the generic selection dialog. The rows are `src/features/goal/` (ADR-010).

use crate::features::Choice;
use crate::ids::ThreadId;

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

    /// The row `id` chosen: a row of another menu does nothing, a leave only closes, the others
    /// pick.
    pub(super) fn choose(&self, id: &str) -> Choice<Menu, Pick> {
        if !self.rows().contains(&id) {
            return Choice::Stay;
        }
        match (self, id) {
            (Menu::Resume { thread, .. }, RESUME) => Choice::Pick(Pick::Resume {
                thread: thread.clone(),
            }),
            (Menu::Replace { thread, objective }, REPLACE) => Choice::Pick(Pick::Replace {
                thread: thread.clone(),
                objective: objective.clone(),
            }),
            _ => Choice::Dismiss,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_menu_row_picks_what_it_says_and_a_leave_only_closes() {
        let thread = ThreadId::new("t1");
        let resume = Menu::Resume {
            thread: thread.clone(),
            objective: "ship".into(),
        };
        assert_eq!(
            resume.choose(RESUME),
            Choice::Pick(Pick::Resume {
                thread: thread.clone()
            })
        );
        assert_eq!(resume.choose(LEAVE), Choice::Dismiss);
        assert_eq!(resume.choose(REPLACE), Choice::Stay);
        let replace = Menu::Replace {
            thread: thread.clone(),
            objective: "new".into(),
        };
        assert_eq!(
            replace.choose(REPLACE),
            Choice::Pick(Pick::Replace {
                thread,
                objective: "new".into()
            })
        );
        assert_eq!(replace.choose(CANCEL), Choice::Dismiss);
    }
}
