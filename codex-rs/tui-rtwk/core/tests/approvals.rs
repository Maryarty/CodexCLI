#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S13: the request a script's `screen` step presents becomes
//! a prompt of the model with its decisions; an answer is a model fact. The
//! expected decisions are the oracle's (`bottom_pane/approval_overlay.rs`:
//! `exec_options`, `permissions_options`, `patch_options`,
//! `elicitation_options`, and `ApprovalRequest::default_available_decisions`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{RequestKey, ServerRequestId, ThreadId};
use codex_tui_core::screens::approvals::{Decision, Intent as ApprovalIntent, Prompt};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use pretty_assertions::assert_eq;

fn play(request: &str) -> Sandbox {
    let script = format!(
        r#"{{"step":"seed","seed":"session","thread":"thread-1"}}
{{"step":"request","message":{request}}}
{{"step":"screen","dir":"approvals","ev":{{"kind":"request","request":{request}}}}}"#
    );
    Sandbox::play(&script).expect("plays")
}

fn exec(extra: &str) -> String {
    format!(
        r#"{{"method":"item/commandExecution/requestApproval","id":1,"params":{{"kind":"command","threadId":"thread-1","turnId":"turn-1","itemId":"call-1","startedAtMs":0,"approvalId":"call-1","environmentId":null,"command":"bash -lc 'echo hello world'","cwd":"/tmp"{extra}}}}}"#
    )
}

fn prompt(sandbox: &Sandbox) -> Prompt {
    let model = &sandbox.app().screens().approvals;
    model
        .prompt(&ThreadId::new("thread-1"), &ServerRequestId::new("1"))
        .expect("a prompt")
        .clone()
}

#[test]
fn exec_prefix_option_emits_execpolicy_amendment() {
    let sandbox = play(&exec(
        r#","proposedExecpolicyAmendment":["echo","hello","world"]"#,
    ));
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptWithPrefix("echo hello world".into()),
            Decision::Cancel
        ]
    );
}

#[test]
fn the_shell_wrapper_is_stripped_from_the_command() {
    let sandbox = play(&exec(""));
    let Prompt::Exec(exec) = prompt(&sandbox) else {
        panic!("an exec prompt");
    };
    assert_eq!(exec.command, "echo hello world");
}

#[test]
fn generic_exec_options_can_offer_allow_for_session() {
    let sandbox = play(&exec(
        r#","availableDecisions":["accept","acceptForSession","cancel"]"#,
    ));
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptForSession,
            Decision::Cancel
        ]
    );
}

#[test]
fn network_exec_options_hide_execpolicy_amendment() {
    let sandbox = play(&exec(
        r#","networkApprovalContext":{"host":"example.com","protocol":"https"},"proposedExecpolicyAmendment":["curl"]"#,
    ));
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptForSession,
            Decision::Cancel
        ]
    );
}

#[test]
fn permissions_options_use_expected_decisions() {
    let sandbox = play(
        r#"{"method":"item/permissions/requestApproval","id":1,"params":{"threadId":"thread-1","turnId":"turn-1","itemId":"test","environmentId":null,"startedAtMs":0,"cwd":"/tmp","reason":"need workspace access","permissions":{"network":{"enabled":true},"fileSystem":{"read":["/tmp/readme.txt"],"write":["/tmp/out.txt"]}}}}"#,
    );
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::GrantForTurn,
            Decision::GrantForTurnStrict,
            Decision::GrantForSession,
            Decision::DenyPermissions
        ]
    );
}

#[test]
fn patch_options_use_expected_decisions() {
    let sandbox = play(
        r#"{"method":"item/fileChange/requestApproval","id":1,"params":{"threadId":"thread-1","turnId":"turn-1","itemId":"call-1","startedAtMs":0}}"#,
    );
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptForSession,
            Decision::Cancel
        ]
    );
}

#[test]
fn an_answer_is_a_model_fact_and_the_prompt_goes() {
    let mut sandbox = play(&exec(""));
    let key = RequestKey {
        backend: Default::default(),
        epoch: sandbox.app().epoch(),
        thread: ThreadId::new("thread-1"),
        id: ServerRequestId::new("1"),
    };
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
        ApprovalIntent::Answer {
            key,
            decision: Decision::Accept,
        },
    ))));
    let model = &sandbox.app().screens().approvals;
    assert!(
        model
            .prompt(&ThreadId::new("thread-1"), &ServerRequestId::new("1"))
            .is_none()
    );
    assert_eq!(model.answered.len(), 1);
    assert_eq!(model.answered[0].decision, Decision::Accept);
}

#[test]
fn a_bad_seed_is_an_error_with_its_line() {
    let err = Sandbox::play(
        r#"{"step":"screen","dir":"approvals","ev":{"kind":"request","request":{"method":"nope"}}}"#,
    )
    .err()
    .expect("fails");
    assert_eq!(err.line, 1);
}

#[test]
fn live_request_uses_the_original_parameters_even_when_another_backend_reuses_its_id() {
    use codex_tui_core::app::{AppFx, BackendEv, CodexApp, Command, apply_app, apply_app_from};
    use codex_tui_core::ids::BackendId;
    use codex_tui_core::screens::approvals::RequestAnswer;

    let mut app = CodexApp::new();
    let primary = BackendId::default();
    let secondary = BackendId(1);
    app.attach_backend(primary);
    app.attach_backend(secondary);
    let first = serde_json::from_str(&exec(
        r#","proposedExecpolicyAmendment":["echo","hello world"]"#,
    ))
    .expect("request");
    let second = serde_json::from_str(&exec("")).expect("request");
    let mut fx = AppFx::default();
    for (origin, request) in [(primary, first), (secondary, second)] {
        apply_app_from(
            &mut app,
            origin,
            AppEv::Backend(BackendEv::Request(Box::new(request))),
            &mut fx,
        );
    }
    assert_eq!(app.requests().entries.len(), 2);
    let key = app.requests().head().expect("first").key.clone();

    let answer = || {
        AppEv::Intent(Intent::AnswerRequest {
            key: key.clone(),
            answer: RequestAnswer::Choice(Decision::AcceptWithPrefix("echo hello world".into())),
        })
    };
    apply_app(&mut app, answer(), &mut fx);
    apply_app(&mut app, answer(), &mut fx);
    let [command @ Command::ResolveRequest { result, .. }] = fx.commands() else {
        panic!("expected one keyed answer: {:?}", fx.commands());
    };
    assert_eq!(app.backend_of(command), primary);
    assert_eq!(
        result["decision"]["acceptWithExecpolicyAmendment"]["execpolicy_amendment"],
        serde_json::json!(["echo", "hello world"])
    );
    assert_eq!(
        app.requests().head().expect("second").key.backend,
        secondary
    );
}

#[test]
fn reconnect_drops_the_old_request_key_even_if_the_new_server_reuses_its_id() {
    use codex_tui_core::app::{AppFx, BackendEv, CodexApp, apply_app};
    use codex_tui_core::screens::agents;

    let mut app = CodexApp::new();
    let request = || {
        let request = serde_json::from_str(&exec("")).expect("request");
        AppEv::Backend(BackendEv::Request(Box::new(request)))
    };
    let mut fx = AppFx::default();
    apply_app(&mut app, request(), &mut fx);
    let old = app.requests().head().expect("old request").key.clone();
    apply_app(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
        &mut fx,
    );
    apply_app(&mut app, request(), &mut fx);
    apply_app(
        &mut app,
        AppEv::Intent(Intent::DismissRequest(old.clone())),
        &mut fx,
    );
    assert!(fx.commands().is_empty());
    assert_ne!(app.requests().head().expect("new request").key, old);
}

#[test]
fn a_network_request_offers_the_allow_rule_it_proposes() {
    // The oracle's `network_exec_options`: just this once, for the
    // conversation, the proposed allow rule, then the cancel.
    let sandbox = play(&exec(
        r#","networkApprovalContext":{"host":"example.com","protocol":"https"},"proposedNetworkPolicyAmendments":[{"host":"example.com","action":"allow"}]"#,
    ));
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptForSession,
            Decision::NetworkRule {
                host: "example.com".into(),
                allow: true
            },
            Decision::Cancel
        ]
    );
}

#[test]
fn a_command_with_additional_permissions_lists_its_rule_and_asks_once() {
    let sandbox = play(&exec(
        r#","additionalPermissions":{"network":{"enabled":true},"fileSystem":{"read":["/tmp/readme.txt"],"write":["/tmp/out.txt"]}}"#,
    ));
    let Prompt::Exec(exec) = prompt(&sandbox) else {
        panic!("an exec prompt");
    };
    assert_eq!(exec.decisions, vec![Decision::Accept, Decision::Cancel]);
    assert_eq!(exec.permission_rule.len(), 3, "network, read and write");
}

fn agent(
    id: &str,
    nickname: Option<&str>,
    role: Option<&str>,
) -> codex_app_server_protocol::Thread {
    serde_json::from_value(serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 0,
        "updatedAt": 0, "recencyAt": 0, "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": nickname, "agentRole": role, "gitInfo": null, "name": null,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

#[test]
fn a_request_of_another_thread_is_named_like_the_agent_picker_names_it() {
    use codex_tui_core::screens::approvals::thread_label;
    let shown = ThreadId::new("thread-1");
    let other = ThreadId::new("thread-2");
    let threads = [agent("thread-2", Some("Robie"), Some("explorer"))];
    assert_eq!(
        thread_label(&threads, &other, Some(&shown)).as_deref(),
        Some("Robie [explorer]")
    );
    assert_eq!(
        thread_label(&threads, &shown, Some(&shown)),
        None,
        "the shown thread has no label"
    );
    // No nickname: the id's last group tells the agents spawned together apart.
    let uuid = "019a0000-0000-7000-8000-00000000cafe";
    let nameless = [agent(uuid, None, None)];
    assert_eq!(
        thread_label(&nameless, &ThreadId::new(uuid), Some(&shown)).as_deref(),
        Some("Agent (00000000cafe)")
    );
}

#[test]
fn a_seeded_label_reaches_the_prompt_the_queue_holds() {
    // A fixture's `label` is what the app-server request cannot carry: the
    // `Presented` event sets the prompt of the request already queued.
    let request = exec("");
    let script = format!(
        r#"{{"step":"seed","seed":"session","thread":"thread-1"}}
{{"step":"request","message":{request}}}
{{"step":"screen","dir":"approvals","ev":{{"kind":"request","request":{request},"label":"Robie [explorer]"}}}}"#
    );
    let sandbox = Sandbox::play(&script).expect("plays");
    let head = sandbox.app().requests().head().expect("a request");
    let prompt = head.prompt.as_ref().expect("a prompt");
    assert_eq!(prompt.thread_label(), Some("Robie [explorer]"));
}
