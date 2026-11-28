//! Borrowed thread targets of the incoming app-server events used by the port.

use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;

use crate::ids::BackendId;
use crate::ids::ConnEpoch;
use crate::ids::RequestKey;
use crate::ids::ServerRequestId;
use crate::ids::ThreadId;

/// The thread a notification is about (oracle: `server_notification_thread_target`
/// in `app_server_event_targets.rs`, adapted: only the notifications this
/// app reads).
pub(crate) fn notification_thread(notification: &ServerNotification) -> Option<&str> {
    use ServerNotification as N;
    Some(match notification {
        N::ThreadStarted(n) => &n.thread.id,
        N::ThreadStatusChanged(n) => &n.thread_id,
        N::ThreadNameUpdated(n) => &n.thread_id,
        N::ThreadArchived(n) => &n.thread_id,
        N::ThreadDeleted(n) => &n.thread_id,
        N::ThreadUnarchived(n) => &n.thread_id,
        N::ThreadSettingsUpdated(n) => &n.thread_id,
        N::ThreadGoalUpdated(n) => &n.thread_id,
        N::ThreadGoalCleared(n) => &n.thread_id,
        N::ThreadClosed(n) => &n.thread_id,
        N::ThreadReverted(n) => &n.thread_id,
        N::ThreadReadStateChanged(n) => &n.thread_id,
        N::ThreadTokenUsageUpdated(n) => &n.thread_id,
        N::TurnStarted(n) => &n.thread_id,
        N::TurnRuntimeMetrics(n) => &n.thread_id,
        N::TurnCompleted(n) => &n.thread_id,
        N::TurnPlanUpdated(n) => &n.thread_id,
        N::ItemStarted(n) => &n.thread_id,
        N::ItemCompleted(n) => &n.thread_id,
        N::AgentMessageDelta(n) => &n.thread_id,
        N::PlanDelta(n) => &n.thread_id,
        N::ReasoningSummaryTextDelta(n) => &n.thread_id,
        N::ReasoningSummaryPartAdded(n) => &n.thread_id,
        N::ServerRequestResolved(n) => &n.thread_id,
        N::Error(n) => &n.thread_id,
        N::ItemGuardianApprovalReviewStarted(n) => &n.thread_id,
        N::ItemGuardianApprovalReviewCompleted(n) => &n.thread_id,
        N::GuardianWarning(n) => &n.thread_id,
        N::ModelSafetyBufferingUpdated(n) => &n.thread_id,
        N::TurnModerationMetadata(n) => &n.thread_id,
        _ => return None,
    })
}

/// The request's wire method and borrowed thread ID. The thread-bearing
/// cases follow the oracle's `server_request_thread_id`. The app keeps the
/// server's exact ID, including opaque IDs supplied by alternate backends.
pub(crate) fn request_target(request: &ServerRequest) -> (&'static str, &str) {
    use ServerRequest as R;
    match request {
        R::CommandExecutionRequestApproval { params, .. } => {
            ("item/commandExecution/requestApproval", &params.thread_id)
        }
        R::FileChangeRequestApproval { params, .. } => {
            ("item/fileChange/requestApproval", &params.thread_id)
        }
        R::ToolRequestUserInput { params, .. } => ("item/tool/requestUserInput", &params.thread_id),
        R::McpServerElicitationRequest { params, .. } => {
            ("mcpServer/elicitation/request", &params.thread_id)
        }
        R::PermissionsRequestApproval { params, .. } => {
            ("item/permissions/requestApproval", &params.thread_id)
        }
        R::DynamicToolCall { params, .. } => ("item/tool/call", &params.thread_id),
        R::CurrentTimeRead { params, .. } => ("currentTime/read", &params.thread_id),
        R::ChatgptAuthTokensRefresh { .. } => ("account/chatgptAuthTokens/refresh", ""),
        R::AttestationGenerate { .. } => ("attestation/generate", ""),
        R::ApplyPatchApproval { .. } => ("applyPatchApproval", ""),
        R::ExecCommandApproval { .. } => ("execCommandApproval", ""),
    }
}

/// Preserves the JSON-RPC distinction between numeric and string IDs.
pub(crate) fn request_key(
    backend: BackendId,
    epoch: ConnEpoch,
    thread: &str,
    id: &RequestId,
) -> RequestKey {
    let id = match id {
        RequestId::Integer(id) => id.to_string(),
        RequestId::String(id) => serde_json::to_string(id).unwrap_or_default(),
    };
    RequestKey {
        backend,
        epoch,
        thread: ThreadId::new(thread),
        id: ServerRequestId::new(id),
    }
}
