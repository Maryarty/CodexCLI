//! The typed asks of a thread beyond turns and history, and the goal fact: review target, goal
//! (`thread/goal/*`), settings patch (`thread/settings/update`). Values only, no prose (the
//! words of `/goal`, `/review` and the pickers are the UI's string tables); the wire encoding
//! of the asks is `ThreadAsk::wire` (ADR-003: thread RPCs exist only through `ThreadFx`).

use codex_app_server_protocol::{
    ApprovalsReviewer, AskForApproval, ReviewTarget as WireTarget, ThreadGoal, ThreadGoalStatus,
    ThreadSettingsUpdateParams,
};
use codex_protocol::config_types::{CollaborationMode, Personality};
use codex_protocol::openai_models::ReasoningEffort;

use crate::rpc::RpcError;

/// What `review/start` reviews (oracle `ReviewTarget`: the review popup's four choices and
/// `/review <instructions>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewTarget {
    /// The working tree: staged, unstaged and untracked files.
    UncommittedChanges,
    /// The changes between the current branch and `branch`.
    BaseBranch { branch: String },
    /// The changes of one commit; `title` is its subject, for the UI.
    Commit { sha: String, title: Option<String> },
    /// Free-form instructions (`/review <text>`).
    Custom { instructions: String },
}

impl ReviewTarget {
    pub(crate) fn wire(&self) -> WireTarget {
        match self {
            ReviewTarget::UncommittedChanges => WireTarget::UncommittedChanges,
            ReviewTarget::BaseBranch { branch } => WireTarget::BaseBranch {
                branch: branch.clone(),
            },
            ReviewTarget::Commit { sha, title } => WireTarget::Commit {
                sha: sha.clone(),
                title: title.clone(),
            },
            ReviewTarget::Custom { instructions } => WireTarget::Custom {
                instructions: instructions.clone(),
            },
        }
    }
}

/// Where a goal stands (oracle `ThreadGoalStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalStatus {
    Active,
    Paused,
    Blocked,
    UsageLimited,
    BudgetLimited,
    Complete,
}

impl GoalStatus {
    pub(crate) fn wire(self) -> ThreadGoalStatus {
        match self {
            GoalStatus::Active => ThreadGoalStatus::Active,
            GoalStatus::Paused => ThreadGoalStatus::Paused,
            GoalStatus::Blocked => ThreadGoalStatus::Blocked,
            GoalStatus::UsageLimited => ThreadGoalStatus::UsageLimited,
            GoalStatus::BudgetLimited => ThreadGoalStatus::BudgetLimited,
            GoalStatus::Complete => ThreadGoalStatus::Complete,
        }
    }

    fn of(status: ThreadGoalStatus) -> GoalStatus {
        match status {
            ThreadGoalStatus::Active => GoalStatus::Active,
            ThreadGoalStatus::Paused => GoalStatus::Paused,
            ThreadGoalStatus::Blocked => GoalStatus::Blocked,
            ThreadGoalStatus::UsageLimited => GoalStatus::UsageLimited,
            ThreadGoalStatus::BudgetLimited => GoalStatus::BudgetLimited,
            ThreadGoalStatus::Complete => GoalStatus::Complete,
        }
    }
}

/// The goal of a thread (`thread/goal/updated`, the answers of `thread/goal/get|set`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goal {
    pub objective: String,
    pub status: GoalStatus,
    pub token_budget: Option<i64>,
    pub tokens_used: i64,
    pub time_used_seconds: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Goal {
    pub(crate) fn of(goal: &ThreadGoal) -> Goal {
        Goal {
            objective: goal.objective.clone(),
            status: GoalStatus::of(goal.status),
            token_budget: goal.token_budget,
            tokens_used: goal.tokens_used,
            time_used_seconds: goal.time_used_seconds,
            created_at: goal.created_at,
            updated_at: goal.updated_at,
        }
    }
}

/// The goal an answer of `thread/goal/get` or `thread/goal/set` carries (`None`: the thread has
/// none); for the slot that decodes the reply.
pub fn goal_of_response(result: serde_json::Value) -> Result<Option<Goal>, RpcError> {
    #[derive(serde::Deserialize)]
    struct Answer {
        goal: Option<ThreadGoal>,
    }
    serde_json::from_value::<Answer>(result)
        .map(|answer| answer.goal.as_ref().map(Goal::of))
        .map_err(|error| RpcError::Malformed(error.to_string()))
}

/// Which settings of a thread a pick changes for the turns after it (`thread/settings/update`:
/// `None` leaves a field as it is; `service_tier: Some(None)` clears the tier).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsPatch {
    pub model: Option<String>,
    pub effort: Option<ReasoningEffort>,
    pub service_tier: Option<Option<String>>,
    /// The permission profile id.
    pub permissions: Option<String>,
    pub approval_policy: Option<AskForApproval>,
    pub approvals_reviewer: Option<ApprovalsReviewer>,
    pub collaboration_mode: Option<CollaborationMode>,
    pub personality: Option<Personality>,
}

impl SettingsPatch {
    pub(crate) fn wire(&self, thread: &str) -> ThreadSettingsUpdateParams {
        ThreadSettingsUpdateParams {
            thread_id: thread.to_owned(),
            model: self.model.clone(),
            effort: self.effort.clone(),
            service_tier: self.service_tier.clone(),
            permissions: self.permissions.clone(),
            approval_policy: self.approval_policy,
            approvals_reviewer: self.approvals_reviewer,
            collaboration_mode: self.collaboration_mode.clone(),
            personality: self.personality,
            ..ThreadSettingsUpdateParams::default()
        }
    }
}
