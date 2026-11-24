#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of the app's wiring (task core-wiring): the asks the host runs and their answers,
//! the user-verification controller driven by the app, the composer's asks drained after each
//! apply. Expected values are the oracle's (`app/user_verification*.rs`, `clipboard_copy.rs`,
//! `bottom_pane/approval_overlay.rs`); tickets and keys are taken from what the model emitted.

use codex_app_server_protocol::{ServerRequest, UserVerificationProof};
use codex_tui_core::app::effects::{
    Ask, Attempt, ConfigEdit, CopyDestination, CopyFormat, CopyStatus, Done, Verified,
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
            destination: CopyDestination::Clipboard,
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
                done: Done::Copy(Ok(CopyStatus::Pending(1))),
            },
        );
        assert_eq!(app.effect(5), Some(&Done::Copy(Ok(CopyStatus::Pending(1)))));
        run(
            &mut app,
            AppEv::Effect {
                request: 5,
                done: Done::Copy(Ok(CopyStatus::Confirmed)),
            },
        );
        assert_eq!(
            app.effect(5),
            Some(&Done::Copy(Ok(CopyStatus::Confirmed))),
            "a later answer of the same request replaces the earlier"
        );
        assert_eq!(
            app.latest_effect(),
            Some((5, &Done::Copy(Ok(CopyStatus::Confirmed))))
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
        ServerNotification, ThreadBackgroundTerminalsCleanParams, ThreadSettingsUpdateParams,
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

    #[test]
    fn background_terminals_are_cleaned() {
        assert_eq!(
            wire(ThreadAsk::CleanTerminals { thread: T.into() }),
            ClientRequest::ThreadBackgroundTerminalsClean {
                request_id: RequestId::Integer(1),
                params: ThreadBackgroundTerminalsCleanParams {
                    thread_id: T.into(),
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

/// The approvals slot of a thread through the app: the auto-reviewer's notifications reach the
/// thread, what they leave is appended to its history in order, an answer is recorded once the
/// request is resolved, and `/auto-review` asks the server for one retry. Expected values are the
/// oracle's (`chatwidget/tool_requests.rs` `on_guardian_assessment`, `auto_review_denials.rs`,
/// `permission_popups.rs`, `bottom_pane/approval_overlay.rs` `handle_exec_decision`).
mod guardian {
    use codex_app_server_protocol::{ClientRequest, RequestId, ServerNotification};
    use codex_tui_core::app::{
        AppEv, AppFx, AutoReview, BackendEv, CodexApp, Command, Intent, apply_app,
    };
    use codex_tui_core::fixtures::builders::{turn_completed, turn_started};
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::approvals::Decision;
    use codex_tui_core::screens::approvals::decided::{Actor, Decided, Outcome, Subject};
    use codex_tui_core::screens::{self, ScreenIntent};
    use codex_tui_core::threads::{Item, ReviewLine};
    use pretty_assertions::assert_eq;

    const T: &str = "thread-1";

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

    fn notify(app: &mut CodexApp, method: &str, params: &str) -> AppFx {
        let notification: ServerNotification =
            serde_json::from_str(&format!(r#"{{"method":"{method}","params":{params}}}"#))
                .expect("a notification");
        run(
            app,
            AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        )
    }

    /// A review of `id` in the state `status` of the command `command` (an in-progress review
    /// has no completion).
    fn review(app: &mut CodexApp, id: &str, status: &str, command: &str) -> AppFx {
        let (method, done) = if status == "inProgress" {
            ("item/autoApprovalReview/started", String::new())
        } else {
            (
                "item/autoApprovalReview/completed",
                r#","completedAtMs":1,"decisionSource":"agent""#.to_string(),
            )
        };
        notify(
            app,
            method,
            &format!(
                r#"{{"threadId":"{T}","turnId":"turn-1","startedAtMs":0{done},"reviewId":"{id}","targetItemId":null,"review":{{"status":"{status}","riskLevel":null,"userAuthorization":null,"rationale":"too risky"}},"action":{{"type":"command","source":"shell","command":{command:?},"cwd":"/tmp"}}}}"#
            ),
        )
    }

    fn history(app: &CodexApp) -> Vec<Item> {
        let thread = app.active_thread();
        let transcript = thread.transcript();
        (0..transcript.len())
            .filter_map(|index| transcript.get(index).map(|(_, item)| item.clone()))
            .collect()
    }

    fn guardian_denied(command: &str) -> Item {
        Item::Decision(Decided::Request {
            actor: Actor::Guardian,
            subject: Subject::Command(command.into()),
            outcome: Outcome::Denied,
        })
    }

    #[test]
    fn a_review_under_way_is_listed_and_a_denial_leaves_a_decision() {
        let mut app = ready();
        review(&mut app, "g1", "inProgress", "rm -rf /tmp/a");
        assert_eq!(
            app.active_thread().screen_slots().approvals.reviews.len(),
            1
        );
        assert_eq!(history(&app), vec![]);
        review(&mut app, "g1", "denied", "rm -rf /tmp/a");
        let thread = app.active_thread();
        let slot = &thread.screen_slots().approvals;
        assert!(slot.reviews.is_empty());
        assert_eq!(slot.recent_denials.iter().count(), 1);
        assert_eq!(history(&app), vec![guardian_denied("rm -rf /tmp/a")]);
    }

    #[test]
    fn an_approved_review_leaves_nothing() {
        let mut app = ready();
        review(&mut app, "g1", "inProgress", "ls");
        review(&mut app, "g1", "approved", "ls");
        assert_eq!(history(&app), vec![]);
        assert!(
            app.active_thread()
                .screen_slots()
                .approvals
                .reviews
                .is_empty()
        );
    }

    /// The reviewer's warning is a warning of the detailed transcript; the one that says an
    /// action was approved is not shown at all.
    #[test]
    fn the_reviewers_warning_is_kept_but_not_an_approval_notice() {
        let mut app = ready();
        notify(
            &mut app,
            "guardianWarning",
            r#"{"threadId":"thread-1","message":"Automatic approval review approved (risk: low): ls"}"#,
        );
        assert_eq!(history(&app), vec![]);
        notify(
            &mut app,
            "guardianWarning",
            r#"{"threadId":"thread-1","message":"Automatic approval review denied the requested action."}"#,
        );
        assert_eq!(
            history(&app),
            vec![Item::Warning {
                message: "Automatic approval review denied the requested action.".into()
            }]
        );
    }

    /// The reviews of a turn are over with it (oracle `on_task_complete`).
    #[test]
    fn the_end_of_the_turn_ends_the_reviews() {
        let mut app = ready();
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        review(&mut app, "g1", "inProgress", "ls");
        assert_eq!(
            app.active_thread().screen_slots().approvals.reviews.len(),
            1
        );
        run(&mut app, AppEv::Backend(turn_completed(T, "turn-1")));
        assert!(
            app.active_thread()
                .screen_slots()
                .approvals
                .reviews
                .is_empty()
        );
    }

    fn auto_review(app: &mut CodexApp, what: AutoReview) -> AppFx {
        run(
            app,
            AppEv::Intent(Intent::AutoReview {
                thread: ThreadId::new(T),
                what,
            }),
        )
    }

    #[test]
    fn auto_review_without_denials_says_so() {
        let mut app = ready();
        auto_review(&mut app, AutoReview::NoDenials);
        assert_eq!(history(&app), vec![Item::Review(ReviewLine::NoDenials)]);
    }

    /// A pick approves one retry: the server is asked once with the denial's assessment, the
    /// history says so; the same denial is gone afterwards.
    #[test]
    fn a_picked_denial_is_approved_once() {
        let mut app = ready();
        review(&mut app, "g1", "denied", "rm -rf /tmp/a");
        let fx = auto_review(&mut app, AutoReview::Approve { id: "g1".into() });
        let wires: Vec<ClientRequest> = fx
            .commands()
            .iter()
            .filter_map(|command| match command {
                Command::Screen {
                    thread, command, ..
                } => Some(screens::wire(command, thread, RequestId::Integer(1))),
                _ => None,
            })
            .collect();
        let [ClientRequest::ThreadApproveGuardianDeniedAction { params, .. }] = wires.as_slice()
        else {
            panic!("one approveGuardianDeniedAction, got {wires:?}");
        };
        assert_eq!(params.thread_id, T);
        assert_eq!(params.event["id"], "g1");
        assert_eq!(params.event["status"], "denied");
        assert_eq!(params.event["action"]["command"], "rm -rf /tmp/a");
        assert_eq!(
            history(&app),
            vec![
                guardian_denied("rm -rf /tmp/a"),
                Item::Review(ReviewLine::RetryApproved)
            ]
        );
        let again = auto_review(&mut app, AutoReview::Approve { id: "g1".into() });
        assert!(
            again
                .commands()
                .iter()
                .all(|command| !matches!(command, Command::Screen { .. })),
            "a denial retried once is gone"
        );
        assert_eq!(
            history(&app).last(),
            Some(&Item::Review(ReviewLine::DenialGone))
        );
    }

    fn exec_request(id: u64) -> codex_app_server_protocol::ServerRequest {
        serde_json::from_value(serde_json::json!({
            "method": "item/commandExecution/requestApproval",
            "id": id,
            "params": {
                "threadId": T, "turnId": "turn-1", "itemId": format!("call-{id}"),
                "startedAtMs": 0, "approvalId": format!("call-{id}"), "environmentId": null,
                "command": "echo hi", "cwd": "/tmp"
            }
        }))
        .expect("request")
    }

    fn answer(app: &mut CodexApp, decision: Decision) {
        let key = app.requests().entries[0].key.clone();
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
                codex_tui_core::screens::approvals::Intent::Answer { key, decision },
            ))),
        );
    }

    /// An answer is recorded once the request is resolved (the oracle's decision cell): the
    /// user's, with the command, for a request of the thread that is shown.
    #[test]
    fn an_answered_request_of_the_shown_thread_is_recorded() {
        let mut app = ready();
        run(
            &mut app,
            AppEv::Backend(BackendEv::Request(Box::new(exec_request(1)))),
        );
        answer(&mut app, Decision::Accept);
        assert_eq!(
            history(&app),
            vec![Item::Decision(Decided::Request {
                actor: Actor::User,
                subject: Subject::Command("echo hi".into()),
                outcome: Outcome::Approved,
            })]
        );
    }

    /// Esc / ctrl+c on the dialog answers with the cancelling decision, which is recorded too.
    #[test]
    fn a_cancelled_request_is_recorded_as_aborted() {
        let mut app = ready();
        run(
            &mut app,
            AppEv::Backend(BackendEv::Request(Box::new(exec_request(1)))),
        );
        run(&mut app, AppEv::Intent(Intent::CancelRequests));
        assert_eq!(
            history(&app),
            vec![Item::Decision(Decided::Request {
                actor: Actor::User,
                subject: Subject::Command("echo hi".into()),
                outcome: Outcome::Aborted,
            })]
        );
    }
}

/// What the server said about an assistant message (oracle `AgentMarkdownCell::phase`): kept on
/// the item, none for a streamed message, and what a restored turn's snapshot says.
mod phase {
    use codex_app_server_protocol::{
        ItemCompletedNotification, ServerNotification, ThreadItem, ThreadItemPayload,
    };
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, apply_app};
    use codex_tui_core::fixtures::builders::{agent_completed, agent_delta, turn_started};
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::threads::{Item, MessagePhase};
    use pretty_assertions::assert_eq;

    const T: &str = "thread-1";

    fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    }

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

    fn completed(item: &str, text: &str, phase: Option<MessagePhase>) -> AppEv {
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::ItemCompleted(ItemCompletedNotification {
                item: ThreadItem::untimed(ThreadItemPayload::AgentMessage {
                    moderation: None,
                    id: item.to_string(),
                    text: text.to_string(),
                    phase,
                    memory_citation: None,
                    delivery: None,
                    questions: None,
                }),
                thread_id: T.to_string(),
                turn_id: "turn-1".to_string(),
                content_token_count: None,
                completed_at_ms: 0,
            }),
        )))
    }

    fn agents(app: &CodexApp) -> Vec<(String, Option<MessagePhase>)> {
        let thread = app.active_thread();
        let transcript = thread.transcript();
        (0..transcript.len())
            .filter_map(|index| match transcript.get(index)?.1 {
                Item::Agent { text, phase } => Some((text.clone(), phase.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_completed_message_keeps_its_phase() {
        let mut app = ready();
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        run(
            &mut app,
            completed("a", "looking", Some(MessagePhase::Commentary)),
        );
        run(
            &mut app,
            completed("b", "done", Some(MessagePhase::FinalAnswer)),
        );
        run(&mut app, completed("c", "old model", None));
        assert_eq!(
            agents(&app),
            vec![
                ("looking".to_string(), Some(MessagePhase::Commentary)),
                ("done".to_string(), Some(MessagePhase::FinalAnswer)),
                ("old model".to_string(), None),
            ]
        );
    }

    #[test]
    fn a_streamed_message_has_none_until_it_completes() {
        let mut app = ready();
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        run(
            &mut app,
            AppEv::Backend(agent_delta(T, "turn-1", "a", "par")),
        );
        assert_eq!(agents(&app), vec![("par".to_string(), None)]);
        run(
            &mut app,
            completed("a", "partial", Some(MessagePhase::PartialAnswer)),
        );
        assert_eq!(
            agents(&app),
            vec![("partial".to_string(), Some(MessagePhase::PartialAnswer))]
        );
    }

    #[test]
    fn the_untagged_builder_leaves_it_none() {
        let mut app = ready();
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        run(
            &mut app,
            AppEv::Backend(agent_completed(T, "turn-1", "a", "hi")),
        );
        assert_eq!(agents(&app), vec![("hi".to_string(), None)]);
    }
}

/// How a folded turn counts its calls (oracle `tool_call_labels().len()` and `ToolGroup`).
mod calls {
    use std::sync::Arc;

    use codex_tui_core::threads::{
        ExplorationAction, Item, PatchActivity, ShellClass, ToolActivity, ToolKind, ToolStatus,
    };
    use pretty_assertions::assert_eq;

    fn shell(
        actions: Vec<ExplorationAction>,
        exit_code: Option<i32>,
        user_shell: bool,
        status: ToolStatus,
    ) -> Item {
        Item::Tool(Arc::new(ToolActivity {
            kind: ToolKind::Shell {
                command: "ls".into(),
                compound: actions.len() > 1,
                actions,
                exit_code,
                duration_ms: None,
                user_shell,
                process_id: None,
                unified_exec: false,
            },
            status,
            output: None,
        }))
    }

    fn read() -> Vec<ExplorationAction> {
        vec![ExplorationAction::Read(vec!["a.rs".into()])]
    }

    #[test]
    fn a_models_shell_command_is_one_call_and_the_users_is_none() {
        let ran = shell(Vec::new(), Some(0), false, ToolStatus::Completed);
        let user = shell(Vec::new(), Some(0), true, ToolStatus::Completed);
        assert_eq!((ran.call_count(), user.call_count()), (1, 0));
    }

    #[test]
    fn messages_and_notices_add_no_call() {
        let agent = Item::Agent {
            text: "x".into(),
            phase: None,
        };
        assert_eq!(agent.call_count(), 0);
        let patch = Item::Patch(Arc::new(PatchActivity {
            changes: Vec::new(),
            status: ToolStatus::Completed,
        }));
        assert_eq!(patch.call_count(), 1);
    }

    #[test]
    fn a_finished_command_counts_as_explored_or_ran() {
        let explored = shell(read(), Some(0), false, ToolStatus::Completed);
        let ran = shell(Vec::new(), Some(0), false, ToolStatus::Completed);
        assert_eq!(explored.shell_class(), Some(ShellClass::Explored));
        assert_eq!(ran.shell_class(), Some(ShellClass::Ran));
    }

    #[test]
    fn a_command_that_runs_failed_or_is_the_users_counts_as_neither() {
        let running = shell(read(), None, false, ToolStatus::Running);
        let failed = shell(read(), Some(1), false, ToolStatus::Failed);
        let declined = shell(Vec::new(), None, false, ToolStatus::Declined);
        let user = shell(Vec::new(), Some(0), true, ToolStatus::Completed);
        for item in [running, failed, declined, user] {
            assert_eq!(item.shell_class(), None, "{item:?}");
        }
    }
}

/// The state of the older pages of a thread's history (oracle `TranscriptHistoryState`
/// `Complete`, `Partial`, `LoadingOlder`, `Failed`).
mod history_state {
    use codex_app_server_protocol::Thread;
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::fixtures::script::build;
    use codex_tui_core::ids::{ThreadId, Ticket};
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::agents::{
        Command as AgentsCommand, Ev, Page, Reply as AgentsReply,
    };
    use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenReply};
    use codex_tui_core::threads::{HistoryState, ThreadAsk};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    const A: &str = "thread-1";
    const B: &str = "00000000-0000-0000-0000-000000000002";

    fn thread(id: &str) -> Thread {
        serde_json::from_value(json!({
            "id": id, "environments": null, "extra": null, "sessionId": id,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
            "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
            "status": {"type": "idle"}, "path": null,
            "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
            "canAcceptDirectInput": true, "threadSource": null,
            "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
            "daybreakEnabled": null, "turns": []
        }))
        .expect("a thread")
    }

    fn send(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    }

    /// B listed beside the seeded chat thread, opened and resumed: its first page was asked.
    fn opened() -> (CodexApp, Ticket) {
        let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
        let mut app = build(&format!("{script}\n")).expect("plays").app;
        send(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
                threads: vec![thread(A), thread(B)],
                unread: Vec::new(),
                next_cursor: None,
                append: false,
                pins: None,
            }))),
        );
        let fx = send(
            &mut app,
            AppEv::Intent(Intent::OpenThread(ThreadId::new(B))),
        );
        let resume = ticket_of(
            &fx,
            |ask| matches!(ask, ThreadAsk::Resume { thread } if thread == B),
        )
        .expect("a resume");
        let fx = send(
            &mut app,
            AppEv::Reply(
                resume,
                Reply::screen(ScreenReply::Agents(AgentsReply::Resumed(Box::new(thread(
                    B,
                ))))),
            ),
        );
        let first = ticket_of(&fx, |ask| {
            matches!(ask, ThreadAsk::History { cursor: None, .. })
        })
        .expect("the first page");
        (app, first)
    }

    fn ticket_of(fx: &AppFx, wanted: impl Fn(&ThreadAsk) -> bool) -> Option<Ticket> {
        fx.commands().iter().find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(AgentsCommand::Thread(ask)),
                ..
            } if wanted(ask) => Some(*ticket),
            _ => None,
        })
    }

    fn page(next: Option<&str>) -> Reply {
        Reply::screen(ScreenReply::Agents(AgentsReply::History {
            turns: Vec::new(),
            next: next.map(str::to_owned),
        }))
    }

    fn state(app: &CodexApp) -> HistoryState {
        app.thread(&ThreadId::new(B))
            .expect("thread B")
            .history_state()
    }

    fn scroll(app: &mut CodexApp) -> AppFx {
        send(
            app,
            AppEv::Intent(Intent::LoadOlderHistory(ThreadId::new(B))),
        )
    }

    fn older(fx: &AppFx) -> Option<Ticket> {
        ticket_of(fx, |ask| {
            matches!(
                ask,
                ThreadAsk::History {
                    cursor: Some(_),
                    ..
                }
            )
        })
    }

    #[test]
    fn a_thread_with_older_pages_is_partial_until_the_last_is_read() {
        let (mut app, first) = opened();
        send(&mut app, AppEv::Reply(first, page(Some("older"))));
        assert_eq!(state(&app), HistoryState::Partial);
        let ask = older(&scroll(&mut app)).expect("the older page");
        send(&mut app, AppEv::Reply(ask, page(None)));
        assert_eq!(state(&app), HistoryState::Complete);
    }

    #[test]
    fn asking_for_an_older_page_is_loading_until_it_answers() {
        let (mut app, first) = opened();
        send(&mut app, AppEv::Reply(first, page(Some("older"))));
        let ask = older(&scroll(&mut app)).expect("the older page");
        assert_eq!(state(&app), HistoryState::Loading);
        send(&mut app, AppEv::Reply(ask, page(Some("oldest"))));
        assert_eq!(state(&app), HistoryState::Partial);
    }

    #[test]
    fn a_failed_page_is_failed_and_asking_again_retries_the_same_cursor() {
        let (mut app, first) = opened();
        send(&mut app, AppEv::Reply(first, page(Some("older"))));
        let ask = older(&scroll(&mut app)).expect("the older page");
        send(&mut app, AppEv::Reply(ask, Reply::failed("server said no")));
        assert_eq!(state(&app), HistoryState::Failed);
        let retry = scroll(&mut app);
        assert_eq!(state(&app), HistoryState::Loading);
        let cursor = retry.commands().iter().find_map(|command| match command {
            Command::Screen {
                command:
                    ScreenCommand::Agents(AgentsCommand::Thread(ThreadAsk::History { cursor, .. })),
                ..
            } => cursor.clone(),
            _ => None,
        });
        assert_eq!(cursor.as_deref(), Some("older"));
    }

    #[test]
    fn a_lost_connection_fails_the_page_it_was_waiting_for() {
        let (mut app, first) = opened();
        send(&mut app, AppEv::Reply(first, page(Some("older"))));
        older(&scroll(&mut app)).expect("the older page");
        send(&mut app, AppEv::Screen(ScreenEv::Agents(Ev::Reconnecting)));
        assert_eq!(state(&app), HistoryState::Failed);
    }

    #[test]
    fn a_failed_first_read_is_not_a_failed_history() {
        let (mut app, first) = opened();
        send(&mut app, AppEv::Reply(first, Reply::failed("no")));
        assert_eq!(state(&app), HistoryState::Complete);
    }
}

/// The refusal notices a failed turn leaves, the streamed plan, and the notices the host
/// announces (oracle `chatwidget/turn_runtime.rs`, `plan_stream`, `app/startup.rs`).
mod notices {
    use codex_app_server_protocol::ServerNotification;
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, apply_app};
    use codex_tui_core::fixtures::builders::{turn_error, turn_started};
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::ScreenThreadEv;
    use codex_tui_core::screens::cells::Daybreak;
    use codex_tui_core::screens::chat::ThreadEv as ChatThreadEv;
    use codex_tui_core::threads::daybreak::{Access, ModelAccess, Programs};
    use codex_tui_core::threads::{Item, Notice, ThreadEv};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    const T: &str = "thread-1";

    fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    }

    fn start(app: &mut CodexApp) {
        let fx = run(app, AppEv::Connected);
        let ticket = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a StartThread");
        run(
            app,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
        );
    }

    fn ready() -> CodexApp {
        let mut app = CodexApp::new();
        start(&mut app);
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        app
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

    fn items(app: &CodexApp) -> Vec<Item> {
        let thread = app.active_thread();
        let transcript = thread.transcript();
        (0..transcript.len())
            .filter_map(|index| transcript.get(index).map(|(_, item)| item.clone()))
            .collect()
    }

    fn error(app: &mut CodexApp, message: &str, info: Option<&str>) {
        notify(
            app,
            "error",
            json!({
                "error": {"message": message, "codexErrorInfo": info, "additionalDetails": null},
                "willRetry": false, "threadId": T, "turnId": "turn-1"
            }),
        );
    }

    fn eligible(model: &str, standard: bool, daybreak: bool) -> Access {
        Access {
            eligible: true,
            models: vec![ModelAccess {
                model: model.to_owned(),
                programs: Some(Programs { standard, daybreak }),
            }],
        }
    }

    #[test]
    fn a_cyber_refusal_is_a_notice_worded_apply_until_the_host_knows_better() {
        let mut app = ready();
        error(&mut app, "refused", Some("cyberPolicy"));
        assert_eq!(
            items(&app),
            vec![Item::Notice(Notice::CyberPolicy(Daybreak::Apply))]
        );
    }

    #[test]
    fn the_wording_follows_the_catalog_and_the_threads_setting() {
        let mut app = CodexApp::new();
        start(&mut app);
        run(
            &mut app,
            AppEv::Screen(codex_tui_core::screens::ScreenEv::Chat(
                codex_tui_core::screens::chat::Ev::Status(Box::new(
                    codex_tui_core::screens::chat::StatusFacts {
                        model: Some("gpt-5.6".into()),
                        ..Default::default()
                    },
                )),
            )),
        );
        run(&mut app, AppEv::Daybreak(eligible("gpt-5.6", true, true)));
        run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
        error(&mut app, "refused", Some("cyberPolicy"));
        assert_eq!(
            items(&app),
            vec![Item::Notice(Notice::CyberPolicy(Daybreak::Disabled))]
        );
        // switched on for the thread: the next refusal words as Enabled
        codex_tui_core::app::apply_thread(
            &mut app,
            &ThreadId::new(T),
            ThreadEv::Screen(ScreenThreadEv::Chat(ChatThreadEv::Metadata {
                name: None,
                forked_from: None,
                cwd: "/tmp".into(),
                path: None,
                parent_owned: false,
                ephemeral: false,
                model: Some("gpt-5.6".into()),
                effort: None,
                provider: "openai".into(),
                daybreak: Some(true),
                resumed: false,
                fresh: false,
            })),
            &mut AppFx::default(),
        );
        run(&mut app, AppEv::Backend(turn_started(T, "turn-2")));
        notify(
            &mut app,
            "error",
            json!({
                "error": {"message": "refused", "codexErrorInfo": "cyberPolicy", "additionalDetails": null},
                "willRetry": false, "threadId": T, "turnId": "turn-2"
            }),
        );
        assert_eq!(
            items(&app).last(),
            Some(&Item::Notice(Notice::CyberPolicy(Daybreak::Enabled)))
        );
    }

    // Oracle `is_bio_policy_error`: the message itself, or the JSON error object around it.
    #[test]
    fn a_biology_refusal_is_the_safety_block_in_either_form() {
        let mut app = ready();
        error(
            &mut app,
            "This content was flagged for possible biological risk. More words.",
            None,
        );
        assert_eq!(items(&app), vec![Item::Notice(Notice::SafetyAccessBlock)]);
        let mut app = ready();
        error(
            &mut app,
            r#"{"error":{"code":"bio_policy","message":"x"}}"#,
            None,
        );
        assert_eq!(items(&app), vec![Item::Notice(Notice::SafetyAccessBlock)]);
        let mut app = ready();
        error(&mut app, "something else", None);
        assert_eq!(
            items(&app),
            vec![Item::Error {
                message: "something else".into()
            }]
        );
    }

    // The turn's own report of an error the `error` notification already gave ends quietly.
    #[test]
    fn the_failed_turn_does_not_repeat_its_refusal() {
        let mut app = ready();
        error(&mut app, "refused", Some("cyberPolicy"));
        run(
            &mut app,
            AppEv::Backend(turn_error(T, "turn-1", "unused", true)),
        );
        notify(
            &mut app,
            "turn/completed",
            json!({
                "threadId": T,
                "turn": {
                    "id": "turn-1", "items": [], "status": "failed",
                    "error": {"message": "refused", "codexErrorInfo": "cyberPolicy", "additionalDetails": null},
                    "startedAt": null, "completedAt": null, "durationMs": null
                }
            }),
        );
        assert_eq!(items(&app).len(), 1);
    }

    #[test]
    fn the_plan_streams_in_deltas_and_the_completed_item_replaces_it() {
        let mut app = ready();
        for delta in ["# Pl", "an\n- one"] {
            notify(
                &mut app,
                "item/plan/delta",
                json!({"threadId": T, "turnId": "turn-1", "itemId": "plan-1", "delta": delta}),
            );
        }
        assert_eq!(
            items(&app),
            vec![Item::Plan {
                text: "# Plan\n- one".into()
            }]
        );
        notify(
            &mut app,
            "item/completed",
            json!({
                "threadId": T, "turnId": "turn-1", "completedAtMs": 0, "contentTokenCount": null,
                "item": {"type": "plan", "id": "plan-1", "text": "# Plan\n- one\n- two"}
            }),
        );
        assert_eq!(
            items(&app),
            vec![Item::Plan {
                text: "# Plan\n- one\n- two".into()
            }]
        );
    }

    // Oracle `OpenUrlInBrowser`: a browser that did not open the page is an error line.
    #[test]
    fn a_browser_that_did_not_open_is_an_error_line_of_the_shown_conversation() {
        let mut app = CodexApp::new();
        run(
            &mut app,
            AppEv::ErrorLine {
                message: "Failed to open browser for https://x.test: no display".into(),
            },
        );
        start(&mut app);
        assert_eq!(items(&app), vec![], "no conversation, no line");
        run(
            &mut app,
            AppEv::ErrorLine {
                message: "Failed to open browser for https://x.test: no display".into(),
            },
        );
        assert_eq!(
            items(&app),
            vec![Item::Error {
                message: "Failed to open browser for https://x.test: no display".into()
            }]
        );
    }

    fn update() -> Notice {
        Notice::UpdateAvailable {
            current: "0.1.0".into(),
            latest: "0.2.0".into(),
            command: Some("npm install -g @openai/codex".into()),
        }
    }

    #[test]
    fn an_announcement_goes_into_the_shown_conversation() {
        let mut app = CodexApp::new();
        start(&mut app);
        run(&mut app, AppEv::Announce(update()));
        assert_eq!(items(&app), vec![Item::Notice(update())]);
    }

    #[test]
    fn an_announcement_before_any_conversation_waits_for_the_first() {
        let mut app = CodexApp::new();
        let version = Notice::ServerVersion {
            message: "The server runs 0.3.0".into(),
            offer_update: true,
        };
        run(&mut app, AppEv::Announce(version.clone()));
        run(&mut app, AppEv::Announce(update()));
        start(&mut app);
        assert_eq!(
            items(&app),
            vec![Item::Notice(version), Item::Notice(update())]
        );
    }
}

mod tui_settings {
    use codex_tui_core::app::facts::TuiSettings;
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, apply_app};
    use pretty_assertions::assert_eq;

    // `tui.collapse_turn_details` is false until the host reports the config's value, and the
    // last report holds.
    #[test]
    fn the_collapse_setting_is_what_the_host_last_reported() {
        let mut app = CodexApp::new();
        assert_eq!(app.tui(), TuiSettings::default());
        assert!(!app.tui().collapse_turn_details);
        let mut fx = AppFx::default();
        let on = TuiSettings {
            collapse_turn_details: true,
            ..TuiSettings::default()
        };
        apply_app(&mut app, AppEv::Tui(on), &mut fx);
        assert_eq!(app.tui(), on);
        apply_app(&mut app, AppEv::Tui(TuiSettings::default()), &mut fx);
        assert!(!app.tui().collapse_turn_details);
    }
}

mod host_asks {
    use codex_tui_core::app::effects::{Ask, Done};
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::ids::ThreadId;
    use pretty_assertions::assert_eq;

    fn ask(app: &mut CodexApp, request: u64, ask: Ask) -> Vec<(u64, Ask)> {
        let mut fx = AppFx::default();
        apply_app(app, AppEv::Intent(Intent::Effect { request, ask }), &mut fx);
        fx.commands()
            .iter()
            .filter_map(|command| match command {
                Command::Effect { request, ask, .. } => Some((*request, ask.clone())),
                _ => None,
            })
            .collect()
    }

    // The clipboard reads, the export file and the Desktop link are asks like the copy: one
    // command under the request the UI chose, the answer kept under it.
    #[test]
    fn the_reads_the_export_and_the_desktop_link_are_asks() {
        let mut app = CodexApp::new();
        let export = Ask::Export {
            thread: ThreadId::new("thread-1"),
            path: "~/talk.md".into(),
            cwd: "/work".into(),
            markdown: "# Codex conversation\n".into(),
        };
        for (request, kind) in [
            (1, Ask::ClipboardText),
            (
                3,
                Ask::OpenDesktop {
                    thread: ThreadId::new("thread-1"),
                },
            ),
            (4, export),
        ] {
            assert_eq!(ask(&mut app, request, kind.clone()), vec![(request, kind)]);
        }
        let mut fx = AppFx::default();
        for (request, done) in [
            (1, Done::ClipboardText(Ok("pasted".into()))),
            (3, Done::Opened(Err("not installed".into()))),
            (4, Done::Export(Ok("/work/talk.md".into()))),
        ] {
            apply_app(&mut app, AppEv::Effect { request, done }, &mut fx);
        }
        assert_eq!(
            app.effect(1),
            Some(&Done::ClipboardText(Ok("pasted".into())))
        );
        assert_eq!(
            app.effect(3),
            Some(&Done::Opened(Err("not installed".into())))
        );
    }

    // A failed copy keeps the host's text for the "failed to copy" line.
    #[test]
    fn a_failed_copy_keeps_the_hosts_text() {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(
            &mut app,
            AppEv::Effect {
                request: 9,
                done: Done::Copy(Err("clipboard unavailable".into())),
            },
            &mut fx,
        );
        assert_eq!(
            app.effect(9),
            Some(&Done::Copy(Err("clipboard unavailable".into())))
        );
    }
}

// Oracle `chatwidget/replay.rs` and `tests/history_replay.rs`: what a resumed thread's loaded turns
// show and which of their user messages seed the composer's history.
mod replay {
    use codex_app_server_protocol::{ThreadItem, ThreadItemPayload, Turn, TurnStatus, UserInput};
    use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, apply_app, apply_thread};
    use codex_tui_core::fixtures::builders::history_turn;
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::backtrack::Mention;
    use codex_tui_core::threads::{Item, history_page};
    use pretty_assertions::assert_eq;

    const T: &str = "thread-1";

    fn app() -> CodexApp {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, AppEv::Connected, &mut fx);
        let ticket = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::StartThread { ticket, .. } => Some(*ticket),
                _ => None,
            })
            .expect("a StartThread");
        apply_app(
            &mut app,
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
            &mut AppFx::default(),
        );
        app
    }

    /// The server's page: newest turn first.
    fn load(app: &mut CodexApp, newest_first: &[Turn]) {
        apply_thread(
            app,
            &ThreadId::new(T),
            history_page(newest_first, None),
            &mut AppFx::default(),
        );
    }

    fn user(id: &str, inputs: Vec<UserInput>) -> ThreadItem {
        ThreadItem::untimed(ThreadItemPayload::UserMessage {
            id: id.to_owned(),
            client_id: None,
            content: inputs,
        })
    }

    fn text(text: &str) -> UserInput {
        UserInput::Text {
            text: text.to_owned(),
            text_elements: Vec::new(),
        }
    }

    fn entered(id: &str) -> ThreadItem {
        ThreadItem::untimed(ThreadItemPayload::EnteredReviewMode {
            id: id.to_owned(),
            review: "changes against main".to_owned(),
        })
    }

    fn exited(id: &str) -> ThreadItem {
        ThreadItem::untimed(ThreadItemPayload::ExitedReviewMode {
            id: id.to_owned(),
            review: "looks fine".to_owned(),
        })
    }

    fn submitted(app: &CodexApp) -> Vec<String> {
        app.active_thread()
            .replayed_submissions()
            .entries()
            .iter()
            .map(|submission| submission.prompt.text.clone())
            .collect()
    }

    fn shown_users(app: &CodexApp) -> Vec<String> {
        let thread = app.active_thread();
        let transcript = thread.transcript();
        (0..transcript.len())
            .filter_map(|index| match transcript.get(index) {
                Some((_, Item::User(prompt))) => Some(prompt.text.clone()),
                _ => None,
            })
            .collect()
    }

    // `replayed_user_messages_seed_composer_history`: the entries are oldest first and keep the
    // mention the input carried.
    #[test]
    fn replayed_user_messages_seed_the_composer_history_oldest_first() {
        let mut app = app();
        let mut first = history_turn("turn-1", "x", "ok");
        first.items[0] = user(
            "user-1",
            vec![
                text("use $sample"),
                UserInput::Mention {
                    name: "Sample Plugin".into(),
                    path: "plugin://sample@test".into(),
                },
            ],
        );
        let mut second = history_turn("turn-2", "x", "ok");
        second.items[0] = user(
            "user-2",
            vec![
                text("use $google-calendar"),
                UserInput::Mention {
                    name: "Google Calendar".into(),
                    path: "app://google_calendar".into(),
                },
            ],
        );
        load(&mut app, &[second, first]);
        assert_eq!(submitted(&app), ["use $sample", "use $google-calendar"]);
        assert_eq!(
            app.active_thread().replayed_submissions().entries()[1]
                .prompt
                .mentions,
            vec![Mention {
                skill: false,
                name: "Google Calendar".into(),
                path: "app://google_calendar".into(),
            }]
        );
        assert_eq!(shown_users(&app), ["use $sample", "use $google-calendar"]);
    }

    // `replayed_review_prompt_does_not_seed_composer_history`: the message after the review
    // started is the review prompt; the one after it ended is the user's.
    #[test]
    fn a_replayed_review_prompt_adds_no_row_and_seeds_nothing() {
        let mut app = app();
        let mut review = history_turn("turn-1", "x", "ok");
        review.items = vec![
            entered("review-start"),
            user(
                "review-prompt",
                vec![text(
                    "Review the code changes against the base branch 'main'.",
                )],
            ),
            exited("review-end"),
        ];
        let mut after = history_turn("turn-2", "x", "ok");
        after.items[0] = user("user-2", vec![text("thanks")]);
        load(&mut app, &[after, review]);
        assert_eq!(submitted(&app), ["thanks"]);
        assert_eq!(shown_users(&app), ["thanks"]);
    }

    // `replayed_nested_review_prompts_do_not_render_or_seed_composer_history`: an interrupted turn
    // that follows a finished review and says the same message twice is hidden.
    #[test]
    fn a_hidden_nested_review_turn_shows_no_user_message_and_counts_as_completed() {
        let mut app = app();
        let mut review = history_turn("turn-1", "x", "ok");
        review.items = vec![entered("review-start"), exited("review-end")];
        let mut nested = history_turn("turn-2", "x", "ok");
        nested.status = TurnStatus::Interrupted;
        nested.completed_at = None;
        nested.items = vec![
            user("nested-1", vec![text("Review this")]),
            user("nested-2", vec![text("Review this")]),
        ];
        load(&mut app, &[nested, review]);
        assert_eq!(submitted(&app), Vec::<String>::new());
        assert_eq!(shown_users(&app), Vec::<String>::new());
        assert!(!app.active_thread().is_running());
    }

    // The same turns as two messages that differ are a user's own turn.
    #[test]
    fn an_interrupted_turn_of_two_different_messages_is_not_hidden() {
        let mut app = app();
        let mut review = history_turn("turn-1", "x", "ok");
        review.items = vec![entered("review-start"), exited("review-end")];
        let mut next = history_turn("turn-2", "x", "ok");
        next.status = TurnStatus::Interrupted;
        next.completed_at = None;
        next.items = vec![
            user("u-1", vec![text("one")]),
            user("u-2", vec![text("two")]),
        ];
        load(&mut app, &[next, review]);
        assert_eq!(submitted(&app), ["one", "two"]);
    }

    // An older page goes in front; a page seen again adds nothing.
    #[test]
    fn an_older_page_goes_in_front_and_the_generation_says_so() {
        let mut app = app();
        let mut newer = history_turn("turn-2", "x", "ok");
        newer.items[0] = user("user-2", vec![text("second")]);
        let mut older = history_turn("turn-1", "x", "ok");
        older.items[0] = user("user-1", vec![text("first")]);
        load(&mut app, std::slice::from_ref(&newer));
        let generation = app.active_thread().replayed_submissions().generation();
        load(&mut app, std::slice::from_ref(&older));
        assert_eq!(submitted(&app), ["first", "second"]);
        let after = app.active_thread().replayed_submissions().generation();
        assert!(after > generation);
        load(&mut app, std::slice::from_ref(&older));
        assert_eq!(submitted(&app), ["first", "second"]);
        assert_eq!(
            app.active_thread().replayed_submissions().generation(),
            after
        );
    }

    // `replayed_in_progress_turn_marks_task_running`.
    #[test]
    fn a_replayed_turn_in_progress_marks_the_thread_running() {
        let mut app = app();
        let mut turn = history_turn("turn-1", "go", "working");
        turn.status = TurnStatus::InProgress;
        turn.completed_at = None;
        load(&mut app, &[turn]);
        assert!(app.active_thread().is_running());
        assert_eq!(submitted(&app), ["go"]);
    }
}
