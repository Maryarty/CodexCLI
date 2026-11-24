//! The RPCs of `/goal`: `thread/goal/get|set|clear` of the thread the ask is about (oracle
//! `thread_goal_get`, `thread_goal_set`, `thread_goal_clear`).

use codex_app_server_protocol::{
    ThreadGoalClearParams, ThreadGoalClearResponse, ThreadGoalGetParams, ThreadGoalGetResponse,
    ThreadGoalMutationOrigin, ThreadGoalSetParams, ThreadGoalSetResponse,
};

use crate::ids::ThreadId;
use crate::threads::{Goal, GoalStatus};

/// What an ask answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The goal a `thread/goal/get` or `thread/goal/set` answered with (none: the thread has
    /// none).
    Goal(Option<Goal>),
    /// `thread/goal/clear`: whether a goal was there to clear.
    Cleared(bool),
}

crate::rpc_list! {
    /// One of the three asks.
    pub enum Ask => Reply, request thread: &ThreadId;
    /// `thread/goal/get`.
    Get, Self::Get
        => ThreadGoalGet(ThreadGoalGetParams { thread_id: thread.to_string() })
        -> ThreadGoalGetResponse, |answer| Reply::Goal(answer.goal.as_ref().map(Goal::of));
    /// `thread/goal/set` as the user: a new objective, a status (`/goal pause|resume`), a token
    /// budget (`Some(None)` removes it).
    Set {
        objective: Option<String>,
        status: Option<GoalStatus>,
        token_budget: Option<Option<i64>>,
    }, Self::Set { objective, status, token_budget }
        => ThreadGoalSet(ThreadGoalSetParams {
            origin: Some(ThreadGoalMutationOrigin::User),
            thread_id: thread.to_string(),
            objective: objective.clone(),
            status: status.map(GoalStatus::wire),
            token_budget: *token_budget,
        })
        -> ThreadGoalSetResponse, |answer| Reply::Goal(Some(Goal::of(&answer.goal)));
    /// `thread/goal/clear` as the user.
    Clear, Self::Clear
        => ThreadGoalClear(ThreadGoalClearParams {
            origin: Some(ThreadGoalMutationOrigin::User),
            thread_id: thread.to_string(),
        })
        -> ThreadGoalClearResponse, |answer| Reply::Cleared(answer.cleared);
}
