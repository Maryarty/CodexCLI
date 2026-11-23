//! The RPCs of `/goal`: `thread/goal/get|set|clear` of the thread the ask is about (oracle
//! `thread_goal_get`, `thread_goal_set`, `thread_goal_clear`).

use codex_app_server_protocol::{
    ClientRequest, RequestId, ThreadGoalClearParams, ThreadGoalGetParams, ThreadGoalMutationOrigin,
    ThreadGoalSetParams,
};

use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::{Goal, GoalStatus, goal_of_response};

/// One of the three asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// `thread/goal/get`.
    Get,
    /// `thread/goal/set` as the user: a new objective, a status (`/goal pause|resume`), a token
    /// budget (`Some(None)` removes it).
    Set {
        objective: Option<String>,
        status: Option<GoalStatus>,
        token_budget: Option<Option<i64>>,
    },
    /// `thread/goal/clear` as the user.
    Clear,
}

/// What an ask answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The goal a `thread/goal/get` or `thread/goal/set` answered with (none: the thread has
    /// none).
    Goal(Option<Goal>),
    /// `thread/goal/clear`: whether a goal was there to clear.
    Cleared(bool),
}

pub(super) fn request(ask: &Ask, thread: &ThreadId, request_id: RequestId) -> ClientRequest {
    let thread_id = thread.to_string();
    match ask {
        Ask::Get => ClientRequest::ThreadGoalGet {
            request_id,
            params: ThreadGoalGetParams { thread_id },
        },
        Ask::Set {
            objective,
            status,
            token_budget,
        } => ClientRequest::ThreadGoalSet {
            request_id,
            params: ThreadGoalSetParams {
                origin: Some(ThreadGoalMutationOrigin::User),
                thread_id,
                objective: objective.clone(),
                status: status.map(GoalStatus::wire),
                token_budget: *token_budget,
            },
        },
        Ask::Clear => ClientRequest::ThreadGoalClear {
            request_id,
            params: ThreadGoalClearParams {
                origin: Some(ThreadGoalMutationOrigin::User),
                thread_id,
            },
        },
    }
}

pub(super) fn reply(ask: &Ask, result: serde_json::Value) -> Result<Reply, RpcError> {
    match ask {
        Ask::Clear => {
            #[derive(serde::Deserialize)]
            struct Cleared {
                cleared: bool,
            }
            serde_json::from_value::<Cleared>(result)
                .map(|answer| Reply::Cleared(answer.cleared))
                .map_err(|error| RpcError::Malformed(error.to_string()))
        }
        Ask::Get | Ask::Set { .. } => goal_of_response(result).map(Reply::Goal),
    }
}
