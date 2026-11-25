//! Converts a user's answer using the parameters of the exact pending request.

use codex_app_server_protocol::CommandExecutionApprovalDecision as ExecDecision;
use codex_app_server_protocol::CommandExecutionRequestApprovalResponse;
use codex_app_server_protocol::FileChangeApprovalDecision;
use codex_app_server_protocol::FileChangeRequestApprovalResponse;
use codex_app_server_protocol::GrantedPermissionProfile;
use codex_app_server_protocol::McpServerElicitationAction;
use codex_app_server_protocol::McpServerElicitationRequest;
use codex_app_server_protocol::McpServerElicitationRequestResponse;
use codex_app_server_protocol::NetworkPolicyAmendment;
use codex_app_server_protocol::NetworkPolicyRuleAction;
use codex_app_server_protocol::PermissionGrantScope;
use codex_app_server_protocol::PermissionsRequestApprovalResponse;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ToolRequestUserInputAnswer;
use codex_app_server_protocol::ToolRequestUserInputResponse;
use serde_json::Value;

use crate::screens::approvals::{Decision, Prompt, RequestAnswer};

pub(crate) fn answer(
    request: &ServerRequest,
    prompt: Option<&Prompt>,
    answer: RequestAnswer,
) -> Option<Value> {
    use Decision as D;
    use RequestAnswer as A;
    use ServerRequest as R;

    match (request, answer) {
        (R::CommandExecutionRequestApproval { params, .. }, A::Choice(choice)) => {
            let decision = match choice {
                D::Accept => ExecDecision::Accept,
                D::AcceptForSession => ExecDecision::AcceptForSession,
                D::Decline => ExecDecision::Decline,
                D::Cancel => ExecDecision::Cancel,
                D::NetworkRule { ref host, allow } => ExecDecision::ApplyNetworkPolicyAmendment {
                    network_policy_amendment: NetworkPolicyAmendment {
                        host: host.clone(),
                        action: if allow {
                            NetworkPolicyRuleAction::Allow
                        } else {
                            NetworkPolicyRuleAction::Deny
                        },
                    },
                },
                D::AcceptWithPrefix(ref prefix) => {
                    let amendment =
                        params
                            .proposed_execpolicy_amendment
                            .iter()
                            .chain(params.available_decisions.iter().flatten().filter_map(
                                |decision| {
                                    if let ExecDecision::AcceptWithExecpolicyAmendment {
                                        execpolicy_amendment,
                                    } = decision
                                    {
                                        Some(execpolicy_amendment)
                                    } else {
                                        None
                                    }
                                },
                            ))
                            .find(|amendment| {
                                crate::screens::approvals::escape_command(
                                    amendment.command.iter().map(String::as_str),
                                ) == *prefix
                            })?;
                    ExecDecision::AcceptWithExecpolicyAmendment {
                        execpolicy_amendment: amendment.clone(),
                    }
                }
                _ => return None,
            };
            if let Some(available) = &params.available_decisions {
                if !available.contains(&decision) {
                    return None;
                }
            } else if !matches!(choice, D::Decline) && !prompt?.decisions().contains(&choice) {
                return None;
            }
            serde_json::to_value(CommandExecutionRequestApprovalResponse { decision }).ok()
        }
        (R::FileChangeRequestApproval { .. }, A::Choice(choice)) => {
            let decision = match choice {
                D::Accept => FileChangeApprovalDecision::Accept,
                D::AcceptForSession => FileChangeApprovalDecision::AcceptForSession,
                D::Decline => FileChangeApprovalDecision::Decline,
                D::Cancel => FileChangeApprovalDecision::Cancel,
                _ => return None,
            };
            serde_json::to_value(FileChangeRequestApprovalResponse { decision }).ok()
        }
        (R::PermissionsRequestApproval { params, .. }, A::Choice(choice)) => {
            let scope = match choice {
                D::GrantForSession => PermissionGrantScope::Session,
                D::GrantForTurn | D::GrantForTurnStrict | D::DenyPermissions => {
                    PermissionGrantScope::Turn
                }
                _ => return None,
            };
            let permissions = if choice == D::DenyPermissions {
                GrantedPermissionProfile::default()
            } else {
                GrantedPermissionProfile {
                    network: params.permissions.network.clone(),
                    file_system: params.permissions.file_system.clone(),
                }
            };
            serde_json::to_value(PermissionsRequestApprovalResponse {
                permissions,
                scope,
                strict_auto_review: (choice == D::GrantForTurnStrict).then_some(true),
            })
            .ok()
        }
        (R::McpServerElicitationRequest { params, .. }, A::Choice(choice)) => {
            let mut meta = None;
            let content = match (&params.request, &choice) {
                (
                    McpServerElicitationRequest::Form {
                        requested_schema, ..
                    },
                    D::Accept,
                ) if requested_schema.properties.is_empty() => Some(serde_json::json!({})),
                // Accepting a user verification carries the device's proof: the controller
                // answers, never a plain choice. Cancelling it says why, as the controller's
                // `verify::Outcome::Cancelled` does.
                (McpServerElicitationRequest::UserVerification { .. }, D::Cancel) => {
                    meta = Some(serde_json::json!({
                        "openai/userVerificationReason": "userCancelled"
                    }));
                    None
                }
                (McpServerElicitationRequest::UserVerification { .. }, _) => return None,
                (_, D::Accept | D::Decline | D::Cancel) => None,
                _ => return None,
            };
            elicitation(choice, content, meta)
        }
        (
            R::McpServerElicitationRequest { .. },
            A::Elicitation {
                decision,
                content,
                meta,
            },
        ) => elicitation(decision, content, meta),
        (R::ToolRequestUserInput { params, .. }, A::UserInput(answers)) => {
            // No answer at all is the one that a request which ran out sends (the oracle's
            // `submit_empty_auto_resolution`: an empty map); else every question is answered.
            if !answers.is_empty()
                && (params.questions.len() != answers.len()
                    || params
                        .questions
                        .iter()
                        .any(|question| !answers.contains_key(&question.id)))
            {
                return None;
            }
            serde_json::to_value(ToolRequestUserInputResponse {
                answers: answers
                    .into_iter()
                    .map(|(id, answers)| (id, ToolRequestUserInputAnswer { answers }))
                    .collect(),
            })
            .ok()
        }
        _ => None,
    }
}

fn elicitation(choice: Decision, content: Option<Value>, meta: Option<Value>) -> Option<Value> {
    let action = match choice {
        Decision::Accept => McpServerElicitationAction::Accept,
        Decision::Decline => McpServerElicitationAction::Decline,
        Decision::Cancel => McpServerElicitationAction::Cancel,
        _ => return None,
    };
    serde_json::to_value(McpServerElicitationRequestResponse {
        action,
        content,
        meta,
    })
    .ok()
}
