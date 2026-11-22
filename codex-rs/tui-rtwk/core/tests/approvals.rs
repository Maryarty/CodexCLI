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

#[test]
fn the_banner_lists_the_inactive_threads_that_wait_for_an_approval() {
    use codex_tui_core::screens::approvals::pending_threads;
    let request = |id: u64, thread: &str| {
        exec("")
            .replace("\"id\":1", &format!("\"id\":{id}"))
            .replace("thread-1", thread)
    };
    let mut script =
        String::from("{\"step\":\"seed\",\"seed\":\"session\",\"thread\":\"thread-1\"}\n");
    for (id, thread) in [
        (1, "thread-2"),
        (2, "thread-1"),
        (3, "thread-3"),
        (4, "thread-2"),
    ] {
        let request = request(id, thread);
        script.push_str(&format!(
            "{{\"step\":\"request\",\"message\":{request}}}\n{{\"step\":\"screen\",\"dir\":\"approvals\",\"ev\":{{\"kind\":\"request\",\"request\":{request}}}}}\n"
        ));
    }
    let sandbox = Sandbox::play(&script).expect("plays");
    let threads = [agent("thread-2", Some("Robie"), Some("explorer"))];
    let shown = ThreadId::new("thread-1");
    assert_eq!(
        pending_threads(sandbox.app().requests(), &threads, Some(&shown)),
        vec!["Robie [explorer]".to_string(), "Agent (3)".to_string()],
        "the shown thread has none, a thread is listed once, in the order its first request came"
    );
    assert_eq!(
        pending_threads(
            sandbox.app().requests(),
            &threads,
            Some(&ThreadId::new("thread-2"))
        ),
        vec!["Agent (1)".to_string(), "Agent (3)".to_string()],
    );
}

// ---- family B: the form of an MCP elicitation (`screens::approvals::form`) ----

mod forms {
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::screens::approvals::form::{Action, Entered, FieldInput, Form, FormMode};
    use codex_tui_core::screens::approvals::{Decision, Intent as ApprovalIntent, RequestAnswer};
    use pretty_assertions::assert_eq;
    use serde_json::{Value, json};

    fn actions(form: &Form) -> Vec<Action> {
        match &form.fields[0].input {
            FieldInput::Actions(actions) => actions.clone(),
            other => panic!("not the approval fallback: {other:?}"),
        }
    }

    fn tool_meta(extra: Value) -> Value {
        let mut meta = json!({ "codex_approval_kind": "mcp_tool_call" });
        meta.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        meta
    }

    /// `elicitation_options`: a request with no fields asks Allow, Deny, Cancel.
    #[test]
    fn a_message_only_form_offers_allow_deny_cancel() {
        let form = Form::parse(None, &json!({ "type": "object", "properties": {} })).unwrap();
        assert_eq!(form.mode, FormMode::Approval);
        assert_eq!(
            actions(&form),
            [Action::Allow, Action::Deny, Action::Cancel]
        );
        let form = Form::parse(None, &Value::Null).unwrap();
        assert_eq!(form.mode, FormMode::Approval);
    }

    /// A tool approval has no Deny; the persist choices are the ones its meta offers.
    #[test]
    fn a_tool_approval_offers_the_persist_choices_its_meta_lists() {
        let empty = json!({ "type": "object", "properties": {} });
        let plain = Form::parse(Some(&tool_meta(json!({}))), &empty).unwrap();
        assert_eq!(actions(&plain), [Action::Allow, Action::Cancel]);
        let both = Form::parse(
            Some(&tool_meta(json!({ "persist": ["session", "always"] }))),
            &empty,
        )
        .unwrap();
        assert_eq!(
            actions(&both),
            [
                Action::Allow,
                Action::AllowForSession,
                Action::AlwaysAllow,
                Action::Cancel
            ]
        );
        let one = Form::parse(Some(&tool_meta(json!({ "persist": "always" }))), &empty).unwrap();
        assert_eq!(
            actions(&one),
            [Action::Allow, Action::AlwaysAllow, Action::Cancel]
        );
        // a persist choice of a request that is no tool approval joins Deny
        let other = Form::parse(Some(&json!({ "persist": "session" })), &empty).unwrap();
        assert_eq!(
            actions(&other),
            [
                Action::Allow,
                Action::AllowForSession,
                Action::Deny,
                Action::Cancel
            ]
        );
    }

    /// A number or a multi-select is not a form field: the plain approval stays.
    #[test]
    fn a_schema_with_a_field_the_form_cannot_take_has_no_form() {
        let number = json!({ "type": "object", "properties": { "n": { "type": "number" } } });
        assert_eq!(Form::parse(None, &number), None);
        let multi = json!({
            "type": "object",
            "properties": { "m": { "type": "array", "items": { "enum": ["a", "b"] } } }
        });
        assert_eq!(Form::parse(None, &multi), None);
        let suggestion = json!({ "codex_approval_kind": "tool_suggestion" });
        assert_eq!(
            Form::parse(
                Some(&suggestion),
                &json!({ "type": "object", "properties": {} })
            ),
            None
        );
    }

    fn content_form() -> Form {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "title": "Name" },
                "tier": { "type": "string", "oneOf": [
                    { "const": "free", "title": "Free" }, { "const": "pro", "title": "Pro" } ] },
                "dry": { "type": "boolean", "title": "Dry run", "default": false }
            },
            "required": ["name", "tier"]
        });
        Form::parse(None, &schema).unwrap()
    }

    /// The answer of a content form: the typed text trimmed, the option's value and the
    /// boolean; a default counts as answered only when the schema gave it.
    #[test]
    fn the_answer_of_a_content_form_is_its_typed_values() {
        let form = content_form();
        assert_eq!(form.mode, FormMode::Content);
        assert_eq!(form.fields.len(), 3);
        // the fields follow the schema's order, so look them up by id
        let at = |id: &str| form.fields.iter().position(|f| f.id == id).unwrap();
        let mut entered: Vec<Option<Entered>> = vec![None; 3];
        entered[at("name")] = Some(Entered::Text("  Ada  ".into()));
        entered[at("tier")] = Some(Entered::Choice(1));
        entered[at("dry")] = Some(Entered::Choice(1));
        assert_eq!(
            form.answer(&entered),
            Ok(RequestAnswer::Elicitation {
                decision: Decision::Accept,
                content: Some(json!({ "name": "Ada", "tier": "pro", "dry": false })),
                meta: None,
            })
        );
    }

    /// Submitting with a required field without a value names the first such field; blank
    /// text is no value.
    #[test]
    fn a_required_field_without_a_value_stops_the_submit_at_the_first() {
        let form = content_form();
        let at = |id: &str| form.fields.iter().position(|f| f.id == id).unwrap();
        let mut entered: Vec<Option<Entered>> = vec![None; 3];
        assert_eq!(form.answer(&entered), Err(at("name").min(at("tier"))));
        assert_eq!(form.unanswered(&entered), 2);
        entered[at("name")] = Some(Entered::Text("   ".into()));
        entered[at("tier")] = Some(Entered::Choice(0));
        assert_eq!(form.first_unanswered(&entered), Some(at("name")));
        assert_eq!(form.unanswered(&entered), 1);
    }

    /// The approval actions answer with the decision and the meta the oracle sends.
    #[test]
    fn approval_actions_answer_with_the_decision_and_the_meta_of_the_oracle() {
        let empty = json!({ "type": "object", "properties": {} });
        let meta = tool_meta(json!({ "persist": ["session", "always"] }));
        let form = Form::parse(Some(&meta), &empty).unwrap();
        let answer = |choice: usize| form.answer(&[Some(Entered::Choice(choice))]).unwrap();
        let elicitation = |decision, meta| RequestAnswer::Elicitation {
            decision,
            content: None,
            meta,
        };
        assert_eq!(answer(0), elicitation(Decision::Accept, None));
        assert_eq!(
            answer(1),
            elicitation(Decision::Accept, Some(json!({ "persist": "session" })))
        );
        assert_eq!(
            answer(2),
            elicitation(Decision::Accept, Some(json!({ "persist": "always" })))
        );
        let user = Some(json!({ "codex_approval_cancellation_reason": "user" }));
        assert_eq!(answer(3), elicitation(Decision::Cancel, user.clone()));
        // Esc and ctrl+c cancel the same way
        assert_eq!(form.cancel(), elicitation(Decision::Cancel, user));
        // a request that is no tool approval cancels without the reason
        let plain = Form::parse(None, &empty).unwrap();
        assert_eq!(plain.cancel(), elicitation(Decision::Cancel, None));
        assert_eq!(
            plain.answer(&[Some(Entered::Choice(1))]),
            Ok(elicitation(Decision::Decline, None))
        );
    }

    /// The prompt lists the first three parameters, long values cut at sixty with `...`,
    /// white space collapsed.
    #[test]
    fn the_parameter_summary_lists_three_and_cuts_long_values() {
        let long = "x".repeat(80);
        let meta = tool_meta(json!({ "tool_params": {
            "b": "two  words\n", "a": 1, "c": long, "d": "never shown" } }));
        let form =
            Form::parse(Some(&meta), &json!({ "type": "object", "properties": {} })).unwrap();
        let summary: Vec<(String, String)> = form
            .summary
            .iter()
            .map(|p| (p.name.clone(), p.value.clone()))
            .collect();
        assert_eq!(
            summary,
            [
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "two words".to_string()),
                ("c".to_string(), format!("{}...", "x".repeat(57))),
            ]
        );
    }

    fn request(schema: Value, meta: Value, id: u64) -> AppEv {
        let request = serde_json::from_value(json!({
            "method": "mcpServer/elicitation/request",
            "id": id,
            "params": {
                "threadId": "thread-1", "turnId": "turn-1", "serverName": "test-server",
                "environmentId": null, "mode": "form", "message": "Choose values",
                "requestedSchema": schema, "_meta": meta
            }
        }))
        .expect("request");
        AppEv::Backend(BackendEv::Request(Box::new(request)))
    }

    fn respond(app: &CodexApp, answer: RequestAnswer) -> AppEv {
        let key = app.requests().head().expect("a request").key.clone();
        AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
            ApprovalIntent::Respond { key, answer },
        )))
    }

    /// The form's answer resolves the request on the wire with the typed content, once; a
    /// second answer for the same key is stale and sends nothing.
    #[test]
    fn a_form_answer_goes_out_once_and_a_stale_answer_sends_nothing() {
        let schema = json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        });
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(schema, Value::Null, 1), &mut fx);
        assert_eq!(app.requests().entries.len(), 1);
        let answer = RequestAnswer::Elicitation {
            decision: Decision::Accept,
            content: Some(json!({ "name": "Ada" })),
            meta: None,
        };
        let ev = respond(&app, answer);
        apply_app(&mut app, ev.clone(), &mut fx);
        apply_app(&mut app, ev, &mut fx);
        assert!(app.requests().entries.is_empty());
        let [Command::ResolveRequest { result, .. }] = fx.commands() else {
            panic!("expected one answer: {:?}", fx.commands());
        };
        assert_eq!(
            *result,
            json!({ "action": "accept", "content": { "name": "Ada" }, "_meta": null })
        );
    }

    /// A schema the form cannot take (a number) falls back to the plain approval: its
    /// Accept answers by a choice and sends no content.
    #[test]
    fn a_schema_without_a_form_is_the_plain_approval_and_answers_by_choice() {
        let schema = json!({
            "type": "object",
            "properties": { "n": { "type": "number" } },
            "required": ["n"]
        });
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(schema, Value::Null, 1), &mut fx);
        let key = app.requests().head().unwrap().key.clone();
        apply_app(
            &mut app,
            AppEv::Intent(Intent::AnswerRequest {
                key,
                answer: RequestAnswer::Choice(Decision::Accept),
            }),
            &mut fx,
        );
        assert!(app.requests().entries.is_empty());
        let [Command::ResolveRequest { result, .. }] = fx.commands() else {
            panic!("expected one answer: {:?}", fx.commands());
        };
        assert_eq!(
            *result,
            json!({ "action": "accept", "content": null, "_meta": null })
        );
    }

    /// A request arriving while a form is open queues behind it: answering the head does
    /// not touch the one behind, which comes up next with its own prompt.
    #[test]
    fn a_request_arriving_while_a_form_is_open_waits_its_turn() {
        let schema = json!({ "type": "object", "properties": {} });
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(schema.clone(), Value::Null, 1), &mut fx);
        let first = app.requests().head().unwrap().key.clone();
        apply_app(&mut app, request(schema, Value::Null, 2), &mut fx);
        assert_eq!(app.requests().entries.len(), 2);
        assert_eq!(app.requests().head().unwrap().key, first);
        let ev = respond(
            &app,
            RequestAnswer::Elicitation {
                decision: Decision::Cancel,
                content: None,
                meta: None,
            },
        );
        apply_app(&mut app, ev, &mut fx);
        assert_eq!(app.requests().entries.len(), 1);
        assert_ne!(app.requests().head().unwrap().key, first);
        assert_eq!(fx.commands().len(), 1);
    }
}

// ---- family C: the answers of a request_user_input request (`screens::approvals::input`) ----

mod inputs {
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::screens::approvals::input::{self, Draft, OTHER_ANSWER};
    use codex_tui_core::screens::approvals::{
        Intent as ApprovalIntent, Question, QuestionOption, RequestAnswer,
    };
    use pretty_assertions::assert_eq;
    use serde_json::{Value, json};
    use std::collections::BTreeMap;

    fn option(label: &str) -> QuestionOption {
        QuestionOption {
            label: label.to_string(),
            description: String::new(),
        }
    }

    fn choose(id: &str, is_other: bool) -> Question {
        Question {
            id: id.to_string(),
            header: String::new(),
            text: "Choose.".to_string(),
            is_other,
            is_secret: false,
            options: vec![option("Option 1"), option("Option 2")],
        }
    }

    fn freeform(id: &str) -> Question {
        Question {
            options: Vec::new(),
            ..choose(id, false)
        }
    }

    fn draft(selected: Option<usize>, committed: bool, notes: &str) -> Draft {
        Draft {
            selected,
            committed,
            notes: notes.to_string(),
        }
    }

    fn answers(answer: RequestAnswer) -> BTreeMap<String, Vec<String>> {
        match answer {
            RequestAnswer::UserInput(answers) => answers,
            other => panic!("not a user-input answer: {other:?}"),
        }
    }

    /// `options_can_submit_empty_when_unanswered`: a question nobody committed sends an empty
    /// list, and every question id is in the answer.
    #[test]
    fn an_unanswered_question_sends_an_empty_list() {
        let questions = [choose("q1", false), freeform("q2")];
        let drafts = [draft(Some(0), false, ""), draft(None, false, "typed")];
        assert_eq!(
            answers(input::answer(&questions, &drafts)),
            BTreeMap::from([
                ("q1".to_string(), Vec::new()),
                ("q2".to_string(), Vec::new())
            ])
        );
    }

    /// `skipped_option_questions_count_as_unanswered`, `highlighted_option_questions_are_unanswered`,
    /// `freeform_requires_enter_with_text_to_mark_answered`: only a committed answer counts.
    #[test]
    fn only_a_committed_answer_counts() {
        let questions = [choose("q1", false), choose("q2", false), freeform("q3")];
        let drafts = [
            draft(None, false, ""),
            draft(Some(1), false, ""),
            draft(None, false, "Draft"),
        ];
        assert_eq!(input::unanswered(&questions, &drafts), 3);
        assert_eq!(input::first_unanswered(&questions, &drafts), Some(0));
        let drafts = [
            draft(Some(0), true, ""),
            draft(Some(1), true, ""),
            draft(None, true, "Draft"),
        ];
        assert_eq!(input::unanswered(&questions, &drafts), 0);
        assert_eq!(input::first_unanswered(&questions, &drafts), None);
        // an option question committed with no selection is not answered
        assert!(!input::answered(&questions[0], &draft(None, true, "")));
    }

    /// The option's text, then the notes as `user_note: ...` (trimmed); the other row sends its
    /// own text; a freeform answer is the note alone.
    #[test]
    fn a_committed_answer_sends_the_option_and_its_notes() {
        let questions = [choose("q1", true), choose("q2", true), freeform("q3")];
        let drafts = [
            draft(Some(1), true, "  because  "),
            draft(Some(2), true, ""),
            draft(None, true, "free text"),
        ];
        assert_eq!(
            answers(input::answer(&questions, &drafts)),
            BTreeMap::from([
                (
                    "q1".to_string(),
                    vec!["Option 2".to_string(), "user_note: because".to_string()]
                ),
                ("q2".to_string(), vec![OTHER_ANSWER.to_string()]),
                ("q3".to_string(), vec!["user_note: free text".to_string()]),
            ])
        );
        assert_eq!(input::choices(&questions[0]), 3);
        assert_eq!(input::choices(&choose("q", false)), 2);
        assert_eq!(input::choices(&freeform("q")), 0);
        // the other row exists only on a question that has options
        let other_freeform = Question {
            is_other: true,
            ..freeform("q")
        };
        assert_eq!(input::choices(&other_freeform), 0);
        assert_eq!(input::label(&questions[0], 3), None);
    }

    fn request(id: u64, questions: Value) -> AppEv {
        let request = serde_json::from_value(json!({
            "method": "item/tool/requestUserInput",
            "id": id,
            "params": {
                "threadId": "thread-1", "turnId": "turn-1", "itemId": format!("call-{id}"),
                "isBlocking": true, "autoResolutionMs": null, "questions": questions
            }
        }))
        .expect("request");
        AppEv::Backend(BackendEv::Request(Box::new(request)))
    }

    fn two_questions() -> Value {
        json!([
            { "id": "q1", "header": "A", "question": "Choose.", "isOther": false, "isSecret": false,
              "options": [{ "label": "Option 1", "description": "" }] },
            { "id": "q2", "header": "B", "question": "Say.", "isOther": false, "isSecret": true,
              "options": null }
        ])
    }

    /// The prompt keeps what the answer needs: the secret flag too.
    #[test]
    fn the_prompt_holds_the_questions_with_their_secret_flag() {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(1, two_questions()), &mut fx);
        let entry = app.requests().head().expect("queued");
        let Some(codex_tui_core::screens::approvals::Prompt::UserInput(prompt)) =
            entry.prompt.as_deref()
        else {
            panic!("no user-input prompt: {:?}", entry.prompt);
        };
        assert_eq!(prompt.questions.len(), 2);
        assert!(!prompt.questions[0].is_secret);
        assert!(prompt.questions[1].is_secret);
    }

    fn respond(app: &CodexApp, answer: RequestAnswer) -> AppEv {
        let key = app.requests().head().expect("a request").key.clone();
        AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
            ApprovalIntent::Respond { key, answer },
        )))
    }

    /// The submit resolves the request on the wire once; an answer that misses a question id is
    /// stale and sends nothing, the request stays.
    #[test]
    fn a_submit_goes_out_once_and_an_answer_missing_a_question_sends_nothing() {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(1, two_questions()), &mut fx);
        let partial = RequestAnswer::UserInput(BTreeMap::from([("q1".to_string(), Vec::new())]));
        let ev = respond(&app, partial);
        apply_app(&mut app, ev, &mut fx);
        assert_eq!(app.requests().entries.len(), 1);
        assert!(fx.commands().is_empty());

        let whole = input::answer(
            &[choose("q1", false), freeform("q2")],
            &[draft(Some(0), true, "n"), draft(None, true, "x")],
        );
        let ev = respond(&app, whole);
        apply_app(&mut app, ev.clone(), &mut fx);
        apply_app(&mut app, ev, &mut fx);
        assert!(app.requests().entries.is_empty());
        let [Command::ResolveRequest { result, .. }] = fx.commands() else {
            panic!("expected one answer: {:?}", fx.commands());
        };
        assert_eq!(
            *result,
            json!({ "answers": {
                "q1": { "answers": ["Option 1", "user_note: n"] },
                "q2": { "answers": ["user_note: x"] }
            } })
        );
    }

    /// A request that ran out answers with no answer at all: an empty map on the wire (the
    /// oracle's `submit_empty_auto_resolution`), not one empty list per question; the request
    /// leaves the queue.
    #[test]
    fn a_request_that_ran_out_resolves_with_an_empty_map() {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(1, two_questions()), &mut fx);
        let ev = respond(&app, input::empty_answer());
        apply_app(&mut app, ev, &mut fx);
        assert!(app.requests().entries.is_empty());
        let [Command::ResolveRequest { result, .. }] = fx.commands() else {
            panic!("expected one answer: {:?}", fx.commands());
        };
        assert_eq!(*result, json!({ "answers": {} }));
    }
}

/// `bottom_pane/user_verification.rs`: the request a server makes for a device verification.
mod user_verification {
    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::screens::approvals::{
        Decision, Intent as ApprovalIntent, Prompt, UserVerificationPrompt,
    };
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn request() -> AppEv {
        let request = serde_json::from_value(json!({
            "method": "mcpServer/elicitation/request",
            "id": 12,
            "params": {
                "threadId": "thread-1", "turnId": "turn-1", "serverName": "deployments",
                "mode": "openai/userVerification", "_meta": null,
                "title": "Production deployment needs your approval.",
                "description": "Approve deploying the reviewed release?",
                "challenge": "c2VjcmV0"
            }
        }))
        .expect("request");
        AppEv::Backend(BackendEv::Request(Box::new(request)))
    }

    fn answer(app: &CodexApp, decision: Decision) -> AppEv {
        let key = app.requests().head().expect("a request").key.clone();
        AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
            ApprovalIntent::Answer { key, decision },
        )))
    }

    #[test]
    fn the_request_is_a_prompt_that_keeps_the_challenge_out() {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(), &mut fx);
        let prompt = app.requests().head().and_then(|r| r.prompt.clone());
        assert_eq!(
            prompt.as_deref(),
            Some(&Prompt::UserVerification(UserVerificationPrompt {
                // No thread is shown yet: the request carries the label of its own.
                thread_label: Some("Agent (1)".into()),
                server: "deployments".into(),
                title: "Production deployment needs your approval.".into(),
                description: "Approve deploying the reviewed release?".into(),
                waiting: false,
            }))
        );
        assert_eq!(
            prompt.expect("a prompt").decisions(),
            vec![Decision::Verify, Decision::Cancel]
        );
    }

    #[test]
    fn once_the_verification_runs_only_cancel_is_left() {
        let waiting = Prompt::UserVerification(UserVerificationPrompt {
            waiting: true,
            ..UserVerificationPrompt::default()
        });
        assert_eq!(waiting.decisions(), vec![Decision::Cancel]);
    }

    /// Cancel answers the elicitation as cancelled; accepting needs the device's proof, so a
    /// plain Accept (or Verify, which only starts the controller) sends nothing and keeps the
    /// request.
    #[test]
    fn cancel_answers_and_accept_without_a_proof_does_not() {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(), &mut fx);
        for decision in [Decision::Accept, Decision::Decline, Decision::Verify] {
            let ev = answer(&app, decision);
            apply_app(&mut app, ev, &mut fx);
        }
        assert_eq!(app.requests().entries.len(), 1);
        assert!(fx.commands().is_empty(), "{:?}", fx.commands());
        let ev = answer(&app, Decision::Cancel);
        apply_app(&mut app, ev, &mut fx);
        assert!(app.requests().entries.is_empty());
        let [Command::ResolveRequest { result, .. }] = fx.commands() else {
            panic!("expected one answer: {:?}", fx.commands());
        };
        assert_eq!(
            *result,
            json!({ "action": "cancel", "content": null, "_meta": null })
        );
    }
}

mod auto_resolution {
    use std::time::Duration;

    use codex_tui_core::screens::approvals::input::{
        AutoResolution, auto_resolution, remaining_text,
    };
    use pretty_assertions::assert_eq;

    use super::*;

    const SECS: fn(u64) -> Duration = Duration::from_secs;

    // oracle request_user_input/mod.rs:1829 auto_resolution_absent_has_no_timer, :1849
    // auto_resolution_uses_is_blocking_without_auto_resolution_ms
    #[test]
    fn a_blocking_request_has_no_timer() {
        assert_eq!(auto_resolution(false, false, SECS(0)), AutoResolution::Disabled);
        assert_eq!(auto_resolution(false, false, SECS(500)), AutoResolution::Disabled);
    }

    // :1881 auto_resolution_hides_timer_during_grace_period
    #[test]
    fn the_grace_period_hides_the_countdown() {
        assert_eq!(
            auto_resolution(true, false, SECS(0)),
            AutoResolution::HiddenGrace { remaining: SECS(60) }
        );
        assert_eq!(
            auto_resolution(true, false, SECS(59)),
            AutoResolution::HiddenGrace { remaining: SECS(1) }
        );
    }

    // :1912 auto_resolution_visible_countdown_snapshot, :1978 auto_resolution_expiry_emits_empty_answer
    #[test]
    fn the_countdown_shows_for_a_minute_then_the_request_is_due() {
        assert_eq!(
            auto_resolution(true, false, SECS(60)),
            AutoResolution::VisibleCountdown { remaining: SECS(60) }
        );
        assert_eq!(
            auto_resolution(true, false, SECS(119)),
            AutoResolution::VisibleCountdown { remaining: SECS(1) }
        );
        assert_eq!(auto_resolution(true, false, SECS(120)), AutoResolution::Due);
    }

    // :2011 auto_resolution_key_interaction_snoozes_timer, :2038 ..._paste_...
    #[test]
    fn a_key_or_a_paste_stops_the_timer_for_good() {
        assert_eq!(auto_resolution(true, true, SECS(0)), AutoResolution::Disabled);
        assert_eq!(auto_resolution(true, true, SECS(200)), AutoResolution::Disabled);
    }

    #[test]
    fn the_remaining_time_counts_a_started_second_whole() {
        assert_eq!(remaining_text(SECS(60)), "1m 00s");
        assert_eq!(remaining_text(SECS(61)), "1m 01s");
        assert_eq!(remaining_text(Duration::from_millis(59_500)), "1m 00s");
        assert_eq!(remaining_text(SECS(59)), "59s");
        assert_eq!(remaining_text(Duration::from_millis(500)), "1s");
        assert_eq!(remaining_text(SECS(0)), "0s");
    }

    #[test]
    fn only_a_request_that_says_it_is_not_blocking_answers_itself() {
        let request = |blocking: &str| {
            format!(
                r#"{{"method":"item/tool/requestUserInput","id":1,"params":{{"threadId":"thread-1","turnId":"turn-1","itemId":"call-1"{blocking},"questions":[{{"id":"q1","header":"H","question":"Q?","isOther":false,"isSecret":false,"options":null}}]}}}}"#
            )
        };
        let can = |blocking: &str| match prompt(&play(&request(blocking))) {
            Prompt::UserInput(input) => input.can_auto_resolve,
            other => panic!("not a question: {other:?}"),
        };
        assert!(!can(""));
        assert!(!can(r#","isBlocking":true"#));
        assert!(can(r#","isBlocking":false"#));
    }
}
