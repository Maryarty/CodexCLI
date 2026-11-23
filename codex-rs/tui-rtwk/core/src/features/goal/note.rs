//! What `/goal` says in the transcript (`thread_goal_actions.rs`, `goal_menu.rs`,
//! `slash_dispatch.rs`): typed notes, their words are `src/features/goal/strings.rs`.

use crate::rpc::RpcError;
use crate::threads::Goal;

/// What `/goal` answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// The usage line (`Usage: /goal [<objective>|clear|edit|pause|resume]`) with the hint
    /// under it.
    Usage(UsageHint),
    /// The bare `/goal`: the goal as a summary (`goal_summary_lines`); the commands under it
    /// are the status's (`model::CommandsHint::of`).
    Summary(Goal),
    /// `Goal {status}` with the usage summary under it: a goal was set or its status changed.
    Status(Goal),
    /// `Goal cleared`.
    Cleared,
    /// `No goal to clear` with `This thread does not currently have a goal.`
    NothingToClear,
    /// `No goal is currently set.` as an error (`/goal edit` over no goal).
    NoGoalToEdit,
    /// An ask failed: `Failed to {action} thread goal: {error}`, or the saved-session
    /// explanation for a temporary thread.
    Failed { action: Action, failure: Failure },
}

/// The hint under the usage line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageHint {
    /// `Example: /goal improve benchmark coverage` (the bare `/goal`, no thread yet).
    Example,
    /// `No goal is currently set.`
    NoGoal,
    /// `The session must start before you can change a goal.`
    StartToChange,
    /// `The session must start before you can set a goal.`
    StartToSet,
    /// `Create a goal before editing it.`
    CreateFirst,
}

/// What the failed ask was doing (the verb of `Failed to {action} thread goal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Read,
    Replace,
    Set,
    Update,
    Clear,
}

/// Why a goal ask failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The thread is temporary: goals need a saved session (`is_ephemeral_thread_goal_error`).
    Ephemeral,
    /// The server's own words.
    Other(String),
}

impl Failure {
    pub fn of(error: &RpcError) -> Self {
        let text = match error {
            RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => {
                text.clone()
            }
            RpcError::Provider(error) => error.to_string(),
            RpcError::Mismatch => String::new(),
        };
        if text.contains("ephemeral thread does not support goals")
            || text.contains("thread goals require a persisted thread; this thread is ephemeral")
        {
            Failure::Ephemeral
        } else {
            Failure::Other(text)
        }
    }
}
