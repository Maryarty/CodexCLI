#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the app's wiring (task core-wiring): the asks the host runs and their answers,
//! the user-verification controller driven by the app, the composer's asks drained after each
//! apply. Expected values are the oracle's (`app/user_verification*.rs`, `clipboard_copy.rs`,
//! `bottom_pane/approval_overlay.rs`); tickets and keys are taken from what the model emitted.

use codex_app_server_protocol::{ServerRequest, UserVerificationProof};
use codex_tui_core::app::effects::{
    Ask, Attempt, ConfigEdit, CopyFormat, CopyStatus, Done, Verified,
};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::RequestKey;
use codex_tui_core::screens::approvals::verify::Cause;
use codex_tui_core::screens::approvals::{Decision, Intent as ApprovalIntent, Prompt};
use codex_tui_core::screens::{ScreenIntent, popovers};
use serde_json::json;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn intent(app: &mut CodexApp, intent: Intent) -> AppFx {
    run(app, AppEv::Intent(intent))
}

fn verification_request(id: u64) -> ServerRequest {
    serde_json::from_value(json!({
        "method": "mcpServer/elicitation/request",
        "id": id,
        "params": {
            "threadId": "thread-1", "turnId": null, "serverName": "deployments",
            "mode": "openai/userVerification", "_meta": null,
            "title": "Approve deployment?",
            "description": "Deploy the reviewed change.",
            "challenge": "AQID"
        }
    }))
    .expect("request")
}

fn exec_request(id: u64) -> ServerRequest {
    serde_json::from_value(json!({
        "method": "item/commandExecution/requestApproval",
        "id": id,
        "params": {
            "threadId": "thread-1", "turnId": "turn-1", "itemId": format!("call-{id}"),
            "startedAtMs": 0, "approvalId": format!("call-{id}"), "environmentId": null,
            "command": "echo hi", "cwd": "/tmp"
        }
    }))
    .expect("request")
}

/// The app with `requests` queued, and their keys in order.
fn app_with(requests: Vec<ServerRequest>) -> (CodexApp, Vec<RequestKey>) {
    let mut app = CodexApp::new();
    for request in requests {
        run(
            &mut app,
            AppEv::Backend(BackendEv::Request(Box::new(request))),
        );
    }
    let keys = app
        .requests()
        .entries
        .iter()
        .map(|e| e.key.clone())
        .collect();
    (app, keys)
}

fn answer(app: &mut CodexApp, key: &RequestKey, decision: Decision) -> AppFx {
    intent(
        app,
        Intent::Screen(ScreenIntent::Approvals(ApprovalIntent::Answer {
            key: key.clone(),
            decision,
        })),
    )
}

fn verify_commands(fx: &AppFx) -> Vec<(RequestKey, Attempt)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::VerifyUser { key, attempt, .. } => Some((key.clone(), *attempt)),
            _ => None,
        })
        .collect()
}

fn resolved(fx: &AppFx) -> Vec<(RequestKey, serde_json::Value)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::ResolveRequest { key, result, .. } => Some((key.clone(), result.clone())),
            _ => None,
        })
        .collect()
}

/// The asks of the `Effect` commands, with their request numbers.
fn effect_asks(fx: &AppFx) -> Vec<(u64, Ask)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
            _ => None,
        })
        .collect()
}

fn waiting(app: &CodexApp, key: &RequestKey) -> bool {
    match app.requests().get(key).and_then(|e| e.prompt.as_deref()) {
        Some(Prompt::UserVerification(p)) => p.waiting,
        other => panic!("not a verification prompt: {other:?}"),
    }
}

/// Begins the one attempt of the one queued verification.
fn begun(app: &mut CodexApp, key: &RequestKey) -> Attempt {
    verify_commands(&answer(app, key, Decision::Verify))
        .remove(0)
        .1
}

fn late_proof() -> UserVerificationProof {
    UserVerificationProof {
        credential_id: "late".into(),
        signature: "late".into(),
    }
}

mod verification {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Choosing to verify begins one attempt carrying what the device signs, and the dialog
    /// shows the waiting state (oracle `start_user_verification`, `begin` refuses a second).
    #[test]
    fn choosing_verify_begins_one_attempt_and_waits() {
        let (mut app, keys) = app_with(vec![verification_request(7)]);
        let fx = answer(&mut app, &keys[0], Decision::Verify);
        let started = verify_commands(&fx);
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].0, keys[0]);
        let Command::VerifyUser { params, .. } = &fx.commands()[0] else {
            panic!("a VerifyUser first: {:?}", fx.commands());
        };
        assert_eq!(params.title, "Approve deployment?");
        assert_eq!(params.description, "Deploy the reviewed change.");
        assert_eq!(params.challenge, "AQID");
        assert!(waiting(&app, &keys[0]));
        assert_eq!(
            app.requests()
                .get(&keys[0])
                .and_then(|e| e.prompt.as_deref())
                .map(Prompt::decisions),
            Some(vec![Decision::Cancel]),
            "only cancel is left once verification runs"
        );
        assert_eq!(
            verify_commands(&answer(&mut app, &keys[0], Decision::Verify)),
            vec![],
            "a second choice does not begin another attempt"
        );
    }

    /// The device signed: the proof, and nothing else, answers the elicitation (oracle
    /// `user_verification_controller_uses_the_local_binary_rpc_and_original_response`).
    #[test]
    fn the_proof_answers_the_request() {
        let (mut app, keys) = app_with(vec![verification_request(7)]);
        let attempt = begun(&mut app, &keys[0]);
        let proof = UserVerificationProof {
            credential_id: "private-credential".into(),
            signature: "private-signature".into(),
        };
        let fx = run(
            &mut app,
            AppEv::Verified {
                key: keys[0].clone(),
                attempt,
                result: Verified::Proof(proof),
            },
        );
        assert_eq!(
            resolved(&fx),
            vec![(
                keys[0].clone(),
                json!({
                    "action": "accept",
                    "content": { "credentialId": "private-credential", "signature": "private-signature" },
                    "_meta": null
                })
            )]
        );
        assert!(app.requests().entries.is_empty());
    }

    /// A failure cancels the elicitation with the closed reason only; a payload that is not one of
    /// the closed categories is a service error; the cause is kept for the warning line (oracle
    /// `..._preserves_typed_error_reasons_in_the_rpc_response`, `verification_failure`).
    #[test]
    fn a_failure_cancels_with_the_closed_reason() {
        for (data, reason, cause) in [
            (
                json!({ "type": "invalidRequest", "reason": "invalidParams" }),
                "invalidParams",
                Cause::InvalidParams,
            ),
            (
                json!({ "type": "unavailable", "reason": "credentialMissing" }),
                "credentialMissing",
                Cause::CredentialMissing,
            ),
            (
                json!({ "type": "cancelled", "reason": "userCancelled" }),
                "userCancelled",
                Cause::UserCancelled,
            ),
            (
                json!({ "type": "failed", "reason": "authenticationFailed" }),
                "authenticationFailed",
                Cause::AuthenticationFailed,
            ),
            (
                json!({ "type": "unavailable", "reason": "private native failure" }),
                "serviceError",
                Cause::Service,
            ),
        ] {
            let (mut app, keys) = app_with(vec![verification_request(7)]);
            let attempt = begun(&mut app, &keys[0]);
            let fx = run(
                &mut app,
                AppEv::Verified {
                    key: keys[0].clone(),
                    attempt,
                    result: Verified::Failed(Some(data)),
                },
            );
            assert_eq!(
                resolved(&fx),
                vec![(
                    keys[0].clone(),
                    json!({
                        "action": "cancel",
                        "content": null,
                        "_meta": { "openai/userVerificationReason": reason }
                    })
                )],
                "{reason}"
            );
            assert_eq!(app.verification_failure(&keys[0]), Some(cause), "{reason}");
        }
    }

    /// The call itself failed (an older binary without the method, a lost connection): the
    /// request is cancelled with a service error and the cause is `Unreachable`.
    #[test]
    fn an_unreachable_binary_cancels_with_a_service_error() {
        let (mut app, keys) = app_with(vec![verification_request(7)]);
        let attempt = begun(&mut app, &keys[0]);
        let fx = run(
            &mut app,
            AppEv::Verified {
                key: keys[0].clone(),
                attempt,
                result: Verified::Unreachable,
            },
        );
        assert_eq!(
            resolved(&fx),
            vec![(
                keys[0].clone(),
                json!({
                    "action": "cancel",
                    "content": null,
                    "_meta": { "openai/userVerificationReason": "serviceError" }
                })
            )]
        );
        assert_eq!(app.verification_failure(&keys[0]), Some(Cause::Unreachable));
    }

    /// A late proof after the user cancelled sends nothing (oracle
    /// `..._suppresses_a_late_proof_after_cancellation`).
    #[test]
    fn a_late_proof_after_cancellation_is_dropped() {
        let (mut app, keys) = app_with(vec![verification_request(7)]);
        let attempt = begun(&mut app, &keys[0]);
        let fx = answer(&mut app, &keys[0], Decision::Cancel);
        assert_eq!(resolved(&fx).len(), 1, "the cancel answers the request");
        let fx = run(
            &mut app,
            AppEv::Verified {
                key: keys[0].clone(),
                attempt,
                result: Verified::Proof(late_proof()),
            },
        );
        assert_eq!(fx.commands(), &[], "nothing answers after the cancel");
    }

    /// A result carries the attempt it answers: another request's attempt does not answer this
    /// request while it is still queued.
    #[test]
    fn a_result_of_another_attempt_does_not_answer() {
        let (mut app, keys) = app_with(vec![verification_request(7), verification_request(8)]);
        let _first = begun(&mut app, &keys[0]);
        let second = begun(&mut app, &keys[1]);
        let fx = run(
            &mut app,
            AppEv::Verified {
                key: keys[0].clone(),
                attempt: second,
                result: Verified::Proof(late_proof()),
            },
        );
        assert_eq!(fx.commands(), &[]);
        assert_eq!(app.requests().entries.len(), 2, "both still wait");
    }

    /// The connection was replaced: the in-flight result is void.
    #[test]
    fn a_reconnect_voids_the_attempt() {
        let (mut app, keys) = app_with(vec![verification_request(7)]);
        let attempt = begun(&mut app, &keys[0]);
        app.attach();
        let fx = run(
            &mut app,
            AppEv::Verified {
                key: keys[0].clone(),
                attempt,
                result: Verified::Proof(late_proof()),
            },
        );
        assert_eq!(fx.commands(), &[]);
        assert!(app.requests().entries.is_empty());
    }
}

mod cancel_requests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Ctrl+C: the request shown is cancelled, the dialog's queue goes unanswered (oracle
    /// `ApprovalOverlay::cancel_current_request` clears its queue).
    #[test]
    fn answers_the_shown_request_and_drops_the_queue() {
        let (mut app, keys) = app_with(vec![exec_request(1), exec_request(2), exec_request(3)]);
        let fx = intent(&mut app, Intent::CancelRequests);
        let answered = resolved(&fx);
        assert_eq!(answered.len(), 1);
        assert_eq!(answered[0].0, keys[0]);
        assert_eq!(answered[0].1["decision"], json!("cancel"));
        assert!(app.requests().entries.is_empty(), "the queue is dropped");
    }

    #[test]
    fn nothing_queued_does_nothing() {
        let mut app = CodexApp::new();
        assert_eq!(intent(&mut app, Intent::CancelRequests).commands(), &[]);
    }
}

mod effects {
    use super::*;
    use pretty_assertions::assert_eq;

    /// An ask becomes one command under the number the UI chose; the answer is read back under it.
    #[test]
    fn an_ask_goes_out_and_its_answer_is_kept_under_the_request() {
        let mut app = CodexApp::new();
        let ask = Ask::Copy {
            text: "hello".into(),
            format: CopyFormat::Markdown,
        };
        let fx = intent(
            &mut app,
            Intent::Effect {
                request: 5,
                ask: ask.clone(),
            },
        );
        assert_eq!(effect_asks(&fx), vec![(5, ask)]);
        assert_eq!(app.effect(5), None, "not answered yet");
        run(
            &mut app,
            AppEv::Effect {
                request: 5,
                done: Done::Copy(CopyStatus::Pending(1)),
            },
        );
        assert_eq!(app.effect(5), Some(&Done::Copy(CopyStatus::Pending(1))));
        run(
            &mut app,
            AppEv::Effect {
                request: 5,
                done: Done::Copy(CopyStatus::Confirmed),
            },
        );
        assert_eq!(
            app.effect(5),
            Some(&Done::Copy(CopyStatus::Confirmed)),
            "a later answer of the same request replaces the earlier"
        );
        assert_eq!(
            app.latest_effect(),
            Some((5, &Done::Copy(CopyStatus::Confirmed)))
        );
    }

    /// An answer of another request does not answer this one; the oldest answers are pushed out.
    #[test]
    fn answers_are_kept_per_request_and_bounded() {
        let mut app = CodexApp::new();
        for request in 0..40u64 {
            run(
                &mut app,
                AppEv::Effect {
                    request,
                    done: Done::Config(Ok(())),
                },
            );
        }
        assert_eq!(app.effect(0), None, "pushed out");
        assert_eq!(app.effect(39), Some(&Done::Config(Ok(()))));
        assert_eq!(app.effect(1000), None);
    }

    /// The config edits are values with a dotted key path (oracle `config/batchWrite` edits).
    #[test]
    fn a_config_write_carries_its_edits() {
        let mut app = CodexApp::new();
        let edit = ConfigEdit {
            key_path: "tui.theme".into(),
            value: json!("dracula"),
        };
        let ask = Ask::ConfigWrite { edits: vec![edit] };
        let fx = intent(
            &mut app,
            Intent::Effect {
                request: 1,
                ask: ask.clone(),
            },
        );
        assert_eq!(effect_asks(&fx), vec![(1, ask)]);
    }
}

mod composer_asks {
    use super::*;
    use pretty_assertions::assert_eq;

    /// What the composer's popovers ask of the host (a submitted prompt for the history file) is
    /// drained after the apply into one command each, in order, and not twice.
    #[test]
    fn the_popovers_asks_become_commands_once() {
        let mut app = CodexApp::new();
        let fx = intent(
            &mut app,
            Intent::Screen(ScreenIntent::Popovers(popovers::Intent::Record(
                "hello".into(),
            ))),
        );
        let asks: Vec<_> = fx
            .commands()
            .iter()
            .filter_map(|command| match command {
                Command::Popovers { ask, .. } => Some(ask.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(asks, vec![popovers::Ask::Append("hello".into())]);
        assert!(app.screens().popovers.pending_asks().is_empty());
        let fx = intent(&mut app, Intent::CancelRequests);
        assert_eq!(fx.commands(), &[], "taken once");
    }
}

/// The thread asks beyond turns (group B): the wire request of review, goal, background
/// terminals, settings patch and the guardian retry, and the goal the notifications and answers
/// set. Expected values are the oracle's (`app_server_session.rs`: `review_start`,
/// `thread_goal_set|clear|get`, `thread_background_terminals_clean`, `thread_settings_update`,
/// `thread_approve_guardian_denied_action`).
mod thread_asks {
    use codex_app_server_protocol::{
        ClientRequest, RequestId, ReviewDelivery, ReviewStartParams, ReviewTarget as Wire,
        ServerNotification, ThreadApproveGuardianDeniedActionParams,
        ThreadBackgroundTerminalsCleanParams, ThreadGoalClearParams, ThreadGoalGetParams,
        ThreadGoalMutationOrigin, ThreadGoalSetParams, ThreadGoalStatus,
        ThreadSettingsUpdateParams,
    };
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, apply_app};
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::{Reply, RpcError};
    use codex_tui_core::screens::agents::{self, Command as AgentsCommand};
    use codex_tui_core::threads::{
        Goal, GoalStatus, ReviewTarget, SettingsPatch, ThreadAsk, goal_of_response,
    };
    use pretty_assertions::assert_eq;
    use serde_json::json;

    const T: &str = "thread-1";

    fn wire(ask: ThreadAsk) -> ClientRequest {
        agents::wire(
            &AgentsCommand::Thread(ask),
            &ThreadId::new(T),
            RequestId::Integer(1),
        )
    }

    #[test]
    fn a_review_starts_inline_on_the_thread() {
        for (target, wire_target) in [
            (ReviewTarget::UncommittedChanges, Wire::UncommittedChanges),
            (
                ReviewTarget::BaseBranch {
                    branch: "main".into(),
                },
                Wire::BaseBranch {
                    branch: "main".into(),
                },
            ),
            (
                ReviewTarget::Commit {
                    sha: "abc123".into(),
                    title: Some("Fix it".into()),
                },
                Wire::Commit {
                    sha: "abc123".into(),
                    title: Some("Fix it".into()),
                },
            ),
            (
                ReviewTarget::Custom {
                    instructions: "look at the diff".into(),
                },
                Wire::Custom {
                    instructions: "look at the diff".into(),
                },
            ),
        ] {
            assert_eq!(
                wire(ThreadAsk::Review {
                    thread: T.into(),
                    target
                }),
                ClientRequest::ReviewStart {
                    request_id: RequestId::Integer(1),
                    params: ReviewStartParams {
                        thread_id: T.into(),
                        target: wire_target,
                        delivery: Some(ReviewDelivery::Inline),
                    },
                }
            );
        }
    }

    /// A goal is set and cleared as the user (the server takes a goal change without that
    /// provenance as the model's); a token budget of `Some(None)` removes it.
    #[test]
    fn goals_are_set_cleared_and_read_as_the_user() {
        assert_eq!(
            wire(ThreadAsk::GoalSet {
                thread: T.into(),
                objective: Some("ship it".into()),
                status: Some(GoalStatus::Paused),
                token_budget: Some(None),
            }),
            ClientRequest::ThreadGoalSet {
                request_id: RequestId::Integer(1),
                params: ThreadGoalSetParams {
                    origin: Some(ThreadGoalMutationOrigin::User),
                    thread_id: T.into(),
                    objective: Some("ship it".into()),
                    status: Some(ThreadGoalStatus::Paused),
                    token_budget: Some(None),
                },
            }
        );
        assert_eq!(
            wire(ThreadAsk::GoalClear { thread: T.into() }),
            ClientRequest::ThreadGoalClear {
                request_id: RequestId::Integer(1),
                params: ThreadGoalClearParams {
                    origin: Some(ThreadGoalMutationOrigin::User),
                    thread_id: T.into(),
                },
            }
        );
        assert_eq!(
            wire(ThreadAsk::GoalGet { thread: T.into() }),
            ClientRequest::ThreadGoalGet {
                request_id: RequestId::Integer(1),
                params: ThreadGoalGetParams {
                    thread_id: T.into(),
                },
            }
        );
    }

    #[test]
    fn background_terminals_are_cleaned_and_a_denial_is_retried() {
        assert_eq!(
            wire(ThreadAsk::CleanTerminals { thread: T.into() }),
            ClientRequest::ThreadBackgroundTerminalsClean {
                request_id: RequestId::Integer(1),
                params: ThreadBackgroundTerminalsCleanParams {
                    thread_id: T.into(),
                },
            }
        );
        let event = json!({ "id": "review-1", "status": "denied" });
        assert_eq!(
            wire(ThreadAsk::ApproveGuardianDenied {
                thread: T.into(),
                event: event.clone()
            }),
            ClientRequest::ThreadApproveGuardianDeniedAction {
                request_id: RequestId::Integer(1),
                params: ThreadApproveGuardianDeniedActionParams {
                    thread_id: T.into(),
                    event,
                },
            }
        );
    }

    /// Only the fields a pick sets go out; the rest keep the thread's own.
    #[test]
    fn a_settings_patch_sends_only_what_it_sets() {
        let patch = SettingsPatch {
            model: Some("gpt-5".into()),
            service_tier: Some(None),
            ..SettingsPatch::default()
        };
        assert_eq!(
            wire(ThreadAsk::SettingsUpdate {
                thread: T.into(),
                patch
            }),
            ClientRequest::ThreadSettingsUpdate {
                request_id: RequestId::Integer(1),
                params: ThreadSettingsUpdateParams {
                    thread_id: T.into(),
                    model: Some("gpt-5".into()),
                    service_tier: Some(None),
                    ..ThreadSettingsUpdateParams::default()
                },
            }
        );
    }

    fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    }

    /// The app with its thread started, as after `Connected` and the reply.
    fn ready() -> CodexApp {
        let mut app = CodexApp::new();
        let fx = run(&mut app, AppEv::Connected);
        let ticket = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a StartThread");
        run(
            &mut app,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
        );
        app
    }

    fn goal_json(objective: &str, status: &str) -> serde_json::Value {
        json!({
            "threadId": T, "objective": objective, "status": status, "tokenBudget": 5000,
            "tokensUsed": 120, "timeUsedSeconds": 30, "createdAt": 10, "updatedAt": 20
        })
    }

    fn notify(app: &mut CodexApp, method: &str, params: serde_json::Value) {
        let notification: ServerNotification =
            serde_json::from_value(json!({ "method": method, "params": params }))
                .expect("notification");
        run(
            app,
            AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        );
    }

    fn expected(objective: &str, status: GoalStatus) -> Goal {
        Goal {
            objective: objective.into(),
            status,
            token_budget: Some(5000),
            tokens_used: 120,
            time_used_seconds: 30,
            created_at: 10,
            updated_at: 20,
        }
    }

    /// `thread/goal/updated|cleared` is the thread's goal: stored once in the `ThreadModel`.
    #[test]
    fn updated_and_cleared_notifications_set_and_clear_the_goal() {
        let mut app = ready();
        assert_eq!(app.active_thread().goal(), None);
        notify(
            &mut app,
            "thread/goal/updated",
            json!({ "threadId": T, "turnId": null, "goal": goal_json("ship it", "active") }),
        );
        assert_eq!(
            app.active_thread().goal(),
            Some(&expected("ship it", GoalStatus::Active))
        );
        notify(
            &mut app,
            "thread/goal/updated",
            json!({ "threadId": T, "turnId": "turn-1", "goal": goal_json("ship it", "budgetLimited") }),
        );
        assert_eq!(
            app.active_thread().goal(),
            Some(&expected("ship it", GoalStatus::BudgetLimited))
        );
        notify(&mut app, "thread/goal/cleared", json!({ "threadId": T }));
        assert_eq!(app.active_thread().goal(), None);
    }

    #[test]
    fn the_answers_of_get_and_set_decode_to_the_goal() {
        assert_eq!(
            goal_of_response(json!({ "goal": goal_json("ship it", "paused") })),
            Ok(Some(expected("ship it", GoalStatus::Paused)))
        );
        assert_eq!(goal_of_response(json!({ "goal": null })), Ok(None));
        assert!(matches!(
            goal_of_response(json!({ "goal": { "objective": 3 } })),
            Err(RpcError::Malformed(_))
        ));
    }
}
