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
    sandbox
        .app()
        .requests()
        .entries
        .iter()
        .find(|entry| entry.key.thread.as_str() == "thread-1" && entry.key.id.as_str() == "1")
        .and_then(|entry| entry.prompt.as_deref())
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
fn an_answer_takes_the_request_out_of_the_queue() {
    let mut sandbox = play(&exec(""));
    let key = RequestKey {
        backend: Default::default(),
        epoch: sandbox.app().epoch(),
        thread: ThreadId::new("thread-1"),
        id: ServerRequestId::new("1"),
    };
    assert!(sandbox.app().requests().contains(&key));
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
        ApprovalIntent::Answer {
            key,
            decision: Decision::Accept,
        },
    ))));
    assert!(sandbox.app().requests().entries.is_empty());
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
            answer: RequestAnswer::Choice(Decision::AcceptWithPrefix("echo 'hello world'".into())),
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

    fn draft(choice: Option<usize>, committed: bool, notes: &str) -> Draft {
        Draft {
            choice,
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

    /// Cancel answers the elicitation as cancelled and says why (oracle
    /// `user_verification_cancellation_invalidates_the_inflight_result`: `_meta`
    /// `openai/userVerificationReason: userCancelled`); accepting needs the device's proof, so a
    /// plain Accept sends nothing and keeps the request; Verify only starts the controller (one
    /// `VerifyUser`, no answer to the server).
    #[test]
    fn cancel_answers_and_accept_without_a_proof_does_not() {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, request(), &mut fx);
        for decision in [Decision::Accept, Decision::Decline] {
            let ev = answer(&app, decision);
            apply_app(&mut app, ev, &mut fx);
        }
        assert_eq!(app.requests().entries.len(), 1);
        assert!(fx.commands().is_empty(), "{:?}", fx.commands());
        let ev = answer(&app, Decision::Verify);
        apply_app(&mut app, ev, &mut fx);
        assert_eq!(app.requests().entries.len(), 1);
        assert!(
            matches!(
                fx.commands(),
                [Command::Effect {
                    ask: codex_tui_core::app::effects::Ask::VerifyUser { .. },
                    ..
                }]
            ),
            "{:?}",
            fx.commands()
        );
        let mut fx = AppFx::default();
        let ev = answer(&app, Decision::Cancel);
        apply_app(&mut app, ev, &mut fx);
        assert!(app.requests().entries.is_empty());
        let [Command::ResolveRequest { result, .. }] = fx.commands() else {
            panic!("expected one answer: {:?}", fx.commands());
        };
        assert_eq!(
            *result,
            json!({
                "action": "cancel",
                "content": null,
                "_meta": { "openai/userVerificationReason": "userCancelled" }
            })
        );
    }
}

/// The controller's state (`approvals::verify`): oracle `app/user_verification_requests_tests.rs`
/// and `user_verification_errors_tests.rs`.
mod verification_controller {
    use codex_app_server_protocol::{
        ServerRequest, UserVerificationErrorDetails, UserVerificationFailureReason,
        UserVerificationProof, UserVerificationUnavailableReason,
    };
    use codex_tui_core::ids::{ConnEpoch, RequestKey, ServerRequestId, ThreadId};
    use codex_tui_core::screens::approvals::verify::{Cause, Failure, Outcome, Verifications};
    use codex_tui_core::screens::approvals::{Decision, RequestAnswer};
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn request(id: u64) -> ServerRequest {
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

    fn key(thread: &str, epoch: u64, id: u64) -> RequestKey {
        RequestKey {
            backend: Default::default(),
            epoch: ConnEpoch(epoch),
            thread: ThreadId::new(thread),
            id: ServerRequestId::new(id.to_string()),
        }
    }

    /// An attempt is begun once per request and carries what the device signs, from the request.
    #[test]
    fn an_attempt_begins_once_and_carries_the_challenge() {
        let mut verifications = Verifications::default();
        let key = key("thread-1", 1, 7);
        assert_eq!(verifications.begin(&key), None, "nothing noted yet");
        verifications.note(key.clone(), &request(7));
        let attempt = verifications.begin(&key).expect("pending request");
        assert_eq!(attempt.params.title, "Approve deployment?");
        assert_eq!(attempt.params.description, "Deploy the reviewed change.");
        assert_eq!(attempt.params.challenge, "AQID");
        assert_eq!(verifications.begin(&key), None, "one attempt at a time");
        assert!(verifications.is_pending(&key, attempt.id));
    }

    /// A request that is not a user verification is not kept (the controller never saw it).
    #[test]
    fn another_request_is_not_noted() {
        let mut verifications = Verifications::default();
        let other: ServerRequest = serde_json::from_value(json!({
            "method": "item/tool/requestUserInput", "id": 1,
            "params": { "threadId": "thread-1", "turnId": "t", "itemId": "i", "questions": [] }
        }))
        .expect("request");
        verifications.note(key("thread-1", 1, 1), &other);
        assert_eq!(verifications.begin(&key("thread-1", 1, 1)), None);
    }

    /// Cancelled or resolved by the server: the request leaves, the in-flight result is void.
    #[test]
    fn answering_or_resolving_voids_the_inflight_result() {
        let mut verifications = Verifications::default();
        let key = key("thread-1", 1, 7);
        verifications.note(key.clone(), &request(7));
        let attempt = verifications.begin(&key).expect("attempt");
        verifications.remove(&key);
        assert!(!verifications.is_pending(&key, attempt.id));
        assert_eq!(verifications.begin(&key), None);
    }

    /// A discarded thread voids only its own attempts.
    #[test]
    fn a_discarded_thread_voids_only_its_attempts() {
        let mut verifications = Verifications::default();
        let (gone, kept) = (key("thread-1", 1, 7), key("thread-2", 1, 7));
        verifications.note(gone.clone(), &request(7));
        verifications.note(kept.clone(), &request(7));
        let gone_attempt = verifications.begin(&gone).expect("attempt");
        let kept_attempt = verifications.begin(&kept).expect("attempt");
        assert_eq!(
            verifications.cancel_thread(&ThreadId::new("thread-1")),
            vec![gone.clone()]
        );
        assert!(!verifications.is_pending(&gone, gone_attempt.id));
        assert!(verifications.is_pending(&kept, kept_attempt.id));
    }

    /// After a reconnect the old connection's proof cannot answer the request that comes again
    /// with the same id: the new attempt is another generation, and a request of the old epoch
    /// is another key.
    #[test]
    fn a_reconnect_cannot_accept_a_previous_connections_proof() {
        let mut verifications = Verifications::default();
        let key = key("thread-1", 1, 7);
        verifications.note(key.clone(), &request(7));
        let previous = verifications.begin(&key).expect("first");
        verifications.clear();
        assert!(!verifications.is_pending(&key, previous.id));
        verifications.note(key.clone(), &request(7));
        let current = verifications.begin(&key).expect("second");
        assert_ne!(previous.id, current.id);
        assert!(!verifications.is_pending(&key, previous.id));
        assert!(verifications.is_pending(&key, current.id));
        // the same id on the next connection is another request
        assert!(!verifications.is_pending(&self::key("thread-1", 2, 7), current.id));
    }

    /// Accept sends the proof as `content` and no `_meta`; cancel and failure send the closed
    /// reason only (oracle `take_resolution`, `user_verification_proof_is_sent_to_the_server_*`).
    #[test]
    fn the_outcomes_answer_the_elicitation() {
        let proof = UserVerificationProof {
            credential_id: "private-credential".into(),
            signature: "private-signature".into(),
        };
        assert_eq!(
            Outcome::Accepted(proof).answer(),
            Some(RequestAnswer::Elicitation {
                decision: Decision::Accept,
                content: Some(json!({
                    "credentialId": "private-credential",
                    "signature": "private-signature"
                })),
                meta: None,
            })
        );
        assert_eq!(
            Outcome::Cancelled.answer(),
            Some(RequestAnswer::Elicitation {
                decision: Decision::Cancel,
                content: None,
                meta: Some(json!({ "openai/userVerificationReason": "userCancelled" })),
            })
        );
        assert_eq!(
            Outcome::Failed(UserVerificationErrorDetails::Unavailable {
                reason: UserVerificationUnavailableReason::BiometricsUnavailable,
            })
            .answer(),
            Some(RequestAnswer::Elicitation {
                decision: Decision::Cancel,
                content: None,
                meta: Some(json!({ "openai/userVerificationReason": "biometricsUnavailable" })),
            })
        );
    }

    /// The failure is read from the typed `data` only; the native diagnostics of `message` are
    /// never looked at, and a payload outside the closed categories is a service error.
    #[test]
    fn a_failure_reads_the_typed_reason_and_nothing_else() {
        let of = |data: serde_json::Value| Failure::of_server(Some(&data)).cause;
        let unavailable = |reason: &str| json!({ "type": "unavailable", "reason": reason });
        assert_eq!(
            of(unavailable("credentialMissing")),
            Cause::CredentialMissing
        );
        assert_eq!(
            of(unavailable("biometricsUnavailable")),
            Cause::BiometricsUnavailable
        );
        assert_eq!(
            of(unavailable("providerUnavailable")),
            Cause::ProviderUnavailable
        );
        assert_eq!(
            of(json!({ "type": "invalidRequest", "reason": "invalidParams" })),
            Cause::InvalidParams
        );
        assert_eq!(
            of(json!({ "type": "cancelled", "reason": "userCancelled" })),
            Cause::UserCancelled
        );
        assert_eq!(
            of(json!({ "type": "cancelled", "reason": "interrupted" })),
            Cause::Interrupted
        );
        assert_eq!(
            of(json!({ "type": "failed", "reason": "authenticationFailed" })),
            Cause::AuthenticationFailed
        );
        assert_eq!(
            of(json!({ "type": "failed", "reason": "timeout" })),
            Cause::Timeout
        );
        assert_eq!(
            of(json!({ "type": "failed", "reason": "providerError" })),
            Cause::Service
        );
        assert_eq!(of(unavailable("secret native failure")), Cause::Service);
        let malformed = Failure::of_server(Some(&unavailable("secret native failure")));
        assert_eq!(
            malformed.details,
            UserVerificationErrorDetails::Failed {
                reason: UserVerificationFailureReason::ServiceError
            }
        );
        assert_eq!(Failure::of_server(None).cause, Cause::Service);
        assert_eq!(Failure::unreachable().cause, Cause::Unreachable);
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
        assert_eq!(
            auto_resolution(false, false, SECS(0)),
            AutoResolution::Disabled
        );
        assert_eq!(
            auto_resolution(false, false, SECS(500)),
            AutoResolution::Disabled
        );
    }

    // :1881 auto_resolution_hides_timer_during_grace_period
    #[test]
    fn the_grace_period_hides_the_countdown() {
        assert_eq!(
            auto_resolution(true, false, SECS(0)),
            AutoResolution::HiddenGrace {
                remaining: SECS(60)
            }
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
            AutoResolution::VisibleCountdown {
                remaining: SECS(60)
            }
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
        assert_eq!(
            auto_resolution(true, true, SECS(0)),
            AutoResolution::Disabled
        );
        assert_eq!(
            auto_resolution(true, true, SECS(200)),
            AutoResolution::Disabled
        );
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

/// What the transcript keeps of an answer (the oracle's `new_approval_decision_cell` and the
/// plain permissions line, `approval_overlay.rs` `handle_exec_decision` and
/// `handle_permissions_decision`): the model says WHAT happened, the cell says it in words.
mod decided {
    use codex_tui_core::screens::approvals::decided::{Actor, Decided, Grant, Outcome, Subject};
    use pretty_assertions::assert_eq;

    use super::*;

    fn network(command: &str, context: &str) -> String {
        format!(
            r#"{{"method":"item/commandExecution/requestApproval","id":1,"params":{{"kind":"command","threadId":"thread-1","turnId":"turn-1","itemId":"call-1","startedAtMs":0,"approvalId":"call-1","environmentId":null,"command":"{command}","cwd":"/tmp"{context}}}}}"#
        )
    }

    fn user(subject: Subject, outcome: Outcome) -> Option<Decided> {
        Some(Decided::Request {
            actor: Actor::User,
            subject,
            outcome,
        })
    }

    fn command(text: &str) -> Subject {
        Subject::Command(text.to_string())
    }

    fn target(text: &str) -> Subject {
        Subject::Network {
            target: text.to_string(),
        }
    }

    // :2384 exec_history_cell_wraps_with_two_space_indent (the command, shell wrapper stripped)
    #[test]
    fn an_exec_answer_names_the_command_and_what_the_user_chose() {
        let sandbox = play(&exec(r#","proposedExecpolicyAmendment":["echo","hello"]"#));
        let prompt = prompt(&sandbox);
        let of = |decision: Decision| Decided::of(&prompt, &decision);
        let run = || command("echo hello world");
        assert_eq!(of(Decision::Accept), user(run(), Outcome::Approved));
        assert_eq!(
            of(Decision::AcceptForSession),
            user(run(), Outcome::ApprovedForSession)
        );
        assert_eq!(
            of(Decision::AcceptWithPrefix("echo hello".into())),
            user(run(), Outcome::ApprovedWithPrefix("echo hello".into()))
        );
        assert_eq!(of(Decision::Decline), user(run(), Outcome::Denied));
        assert_eq!(of(Decision::Cancel), user(run(), Outcome::Aborted));
    }

    #[test]
    fn a_decision_the_request_does_not_take_records_nothing() {
        let sandbox = play(&exec(""));
        let prompt = prompt(&sandbox);
        for decision in [
            Decision::AllowHost,
            Decision::Verify,
            Decision::GrantForTurn,
            Decision::DenyPermissions,
        ] {
            assert_eq!(Decided::of(&prompt, &decision), None, "{decision:?}");
        }
    }

    // network_access_command_history_uses_target_without_structured_context
    #[test]
    fn a_network_access_command_names_its_target() {
        let sandbox = play(&network("network-access https://example.com:8443", ""));
        assert_eq!(
            Decided::of(&prompt(&sandbox), &Decision::Accept),
            user(target("https://example.com:8443"), Outcome::Approved)
        );
    }

    // network_exec_approval_history_describes_one_time_host_allowance: no target in the
    // command, so the context's scheme and host are it
    #[test]
    fn a_network_context_names_its_scheme_and_host() {
        let context = |protocol: &str| {
            format!(r#","networkApprovalContext":{{"host":"example.com","protocol":"{protocol}"}}"#)
        };
        for (protocol, scheme) in [
            ("http", "http"),
            ("https", "https"),
            ("socks5Tcp", "socks5-tcp"),
            ("socks5Udp", "socks5-udp"),
        ] {
            let sandbox = play(&network("curl example.com", &context(protocol)));
            assert_eq!(
                Decided::of(&prompt(&sandbox), &Decision::Accept),
                user(
                    target(&format!("{scheme}://example.com")),
                    Outcome::Approved
                ),
                "{protocol}"
            );
        }
    }

    // network_exec_approval_history_describes_canceled_host_request: the command's own target
    // wins over the context
    #[test]
    fn the_target_of_the_command_wins_over_the_context() {
        let sandbox = play(&network(
            "network-access socks5-tcp://example.com:1080",
            r#","networkApprovalContext":{"host":"example.com","protocol":"socks5Tcp"}"#,
        ));
        assert_eq!(
            Decided::of(&prompt(&sandbox), &Decision::Cancel),
            user(target("socks5-tcp://example.com:1080"), Outcome::Aborted)
        );
    }

    #[test]
    fn a_network_rule_names_the_rule_and_keeps_the_target() {
        let sandbox = play(&network(
            "curl example.com",
            r#","networkApprovalContext":{"host":"example.com","protocol":"https"}"#,
        ));
        let rule = |allow| Decision::NetworkRule {
            host: "example.com".into(),
            allow,
        };
        let prompt = prompt(&sandbox);
        assert_eq!(
            Decided::of(&prompt, &rule(true)),
            user(
                target("https://example.com"),
                Outcome::NetworkRule {
                    host: "example.com".into(),
                    allow: true
                }
            )
        );
        assert_eq!(
            Decided::of(&prompt, &rule(false)),
            user(
                target("https://example.com"),
                Outcome::NetworkRule {
                    host: "example.com".into(),
                    allow: false
                }
            )
        );
    }

    // `if request.thread_label().is_none()`: the answer of another thread's request is not
    // the shown thread's history
    #[test]
    fn a_request_of_another_thread_leaves_no_record() {
        let sandbox = play(&exec(""));
        let prompt = prompt(&sandbox).with_thread_label(Some("Robie".into()));
        assert_eq!(Decided::of(&prompt, &Decision::Accept), None);
    }

    // handle_permissions_decision: a plain line, the four grants
    #[test]
    fn a_permissions_answer_says_what_was_granted() {
        let sandbox = play(
            r#"{"method":"item/permissions/requestApproval","id":1,"params":{"threadId":"thread-1","turnId":"turn-1","itemId":"test","environmentId":null,"startedAtMs":0,"cwd":"/tmp","reason":"need workspace access","permissions":{"network":{"enabled":true}}}}"#,
        );
        let prompt = prompt(&sandbox);
        for (decision, grant) in [
            (Decision::GrantForTurn, Grant::Turn),
            (Decision::GrantForTurnStrict, Grant::Strict),
            (Decision::GrantForSession, Grant::Session),
            (Decision::DenyPermissions, Grant::Nothing),
        ] {
            assert_eq!(
                Decided::of(&prompt, &decision),
                Some(Decided::Permissions(grant)),
                "{decision:?}"
            );
        }
        assert_eq!(Decided::of(&prompt, &Decision::Accept), None);
    }

    // a patch, a form and a question leave no decision cell (the oracle records the two above)
    #[test]
    fn a_patch_records_no_decision() {
        let sandbox = play(
            r#"{"method":"item/fileChange/requestApproval","id":1,"params":{"threadId":"thread-1","turnId":"turn-1","itemId":"call-1","startedAtMs":0}}"#,
        );
        assert_eq!(Decided::of(&prompt(&sandbox), &Decision::Accept), None);
    }
}

/// Which queued request is shown next (the oracle's view stack, `bottom_pane/mod.rs`
/// 2085-2130, and the queue of each view): a request of another kind stacks over the one
/// shown; the requests of one kind wait in one view, approvals newest first
/// (`approval_overlay.rs` `advance_queue` pops a `Vec`), forms and questions oldest first.
mod order {
    use codex_tui_core::screens::approvals::order::{Family, to_show};
    use pretty_assertions::assert_eq;

    use super::*;

    const A: Family = Family::Approval;
    const E: Family = Family::Elicitation;
    const Q: Family = Family::UserInput;

    fn shown(queue: &[(&'static str, Family)]) -> Option<&'static str> {
        to_show(queue).copied()
    }

    // approval_overlay.rs:507 advance_queue
    #[test]
    fn approvals_come_newest_first() {
        assert_eq!(shown(&[("a1", A), ("a2", A), ("a3", A)]), Some("a3"));
        assert_eq!(shown(&[("a1", A), ("a2", A)]), Some("a2"));
        assert_eq!(shown(&[("a1", A)]), Some("a1"));
    }

    // mcp_server_elicitation.rs:2497 queues_requests_fifo
    #[test]
    fn elicitations_come_oldest_first() {
        assert_eq!(shown(&[("e1", E), ("e2", E)]), Some("e1"));
    }

    // request_user_input/mod.rs:1770 queued_requests_are_fifo
    #[test]
    fn questions_come_oldest_first() {
        assert_eq!(shown(&[("q1", Q), ("q2", Q), ("q3", Q)]), Some("q1"));
    }

    // bottom_pane/mod.rs push_approval_request, push_mcp_server_elicitation_request,
    // push_user_input_request: a new kind is a new view on top
    #[test]
    fn a_request_of_another_kind_stacks_over_the_others() {
        assert_eq!(shown(&[("a1", A), ("e1", E)]), Some("e1"));
        assert_eq!(shown(&[("e1", E), ("a1", A)]), Some("a1"));
        assert_eq!(
            shown(&[("a1", A), ("a2", A), ("q1", Q), ("q2", Q)]),
            Some("q1")
        );
        // once the view on top is done, the earlier one is the top again
        assert_eq!(shown(&[("a1", A), ("a2", A)]), Some("a2"));
    }

    #[test]
    fn nothing_queued_shows_nothing() {
        assert_eq!(shown(&[]), None);
    }

    #[test]
    fn the_family_follows_the_prompt() {
        let sandbox = play(&exec(""));
        assert_eq!(Family::of(Some(&prompt(&sandbox))), A);
        let sandbox = play(
            r#"{"method":"item/fileChange/requestApproval","id":1,"params":{"threadId":"thread-1","turnId":"turn-1","itemId":"call-1","startedAtMs":0}}"#,
        );
        assert_eq!(Family::of(Some(&prompt(&sandbox))), A);
        assert_eq!(Family::of(None), Family::Other);
    }
}

// ---- guardian (auto-review): chatwidget/tool_requests.rs `on_guardian_assessment`,
// auto_review_denials.rs, permission_popups.rs `open_auto_review_denials_popup` and
// `approve_recent_auto_review_denial`, status_state.rs `PendingGuardianReviewStatus`.

mod guardian {
    use codex_app_server_protocol::ServerNotification;
    use codex_tui_core::screens::approvals::decided::{
        Actor, Decided, Outcome, Reviewed, Subject, Verdict,
    };
    use codex_tui_core::screens::approvals::guardian::{Action, Ask, History, project};
    use codex_tui_core::screens::approvals::{ThreadEv, ThreadPart};
    use pretty_assertions::assert_eq;

    fn notification(method: &str, params: &str) -> ServerNotification {
        serde_json::from_str(&format!(r#"{{"method":"{method}","params":{params}}}"#))
            .expect("a notification")
    }

    fn review(id: &str, status: &str, action: &str, rationale: Option<&str>) -> ServerNotification {
        let rationale = rationale.map_or("null".to_string(), |r| format!("{r:?}"));
        let done = if status == "inProgress" {
            String::new()
        } else {
            r#","completedAtMs":1,"decisionSource":"agent""#.to_string()
        };
        let method = if status == "inProgress" {
            "item/autoApprovalReview/started"
        } else {
            "item/autoApprovalReview/completed"
        };
        notification(
            method,
            &format!(
                r#"{{"threadId":"thread-1","turnId":"turn-1","startedAtMs":0{done},"reviewId":"{id}","targetItemId":null,"review":{{"status":"{status}","riskLevel":null,"userAuthorization":null,"rationale":{rationale}}},"action":{action}}}"#
            ),
        )
    }

    fn command(text: &str) -> String {
        format!(r#"{{"type":"command","source":"shell","command":{text:?},"cwd":"/tmp"}}"#)
    }

    const STDIN: &str = r#"{"type":"writeStdin","approvalId":"a","processId":"42","stdin":"confirm\n","cwd":"/tmp"}"#;

    /// Applies `notification` to `part` the way the app does: projected for its thread.
    fn feed(part: &mut ThreadPart, notification: &ServerNotification) -> Option<Ask> {
        let (thread, ev) = project(notification).expect("a guardian notification");
        assert_eq!(thread.as_str(), "thread-1");
        part.apply(ev)
    }

    #[test]
    fn only_the_three_guardian_notifications_are_projected() {
        let turn = notification(
            "turn/started",
            r#"{"threadId":"thread-1","turn":{"id":"turn-1","items":[],"status":"inProgress"}}"#,
        );
        assert!(project(&turn).is_none());
        let warning = notification(
            "guardianWarning",
            r#"{"threadId":"thread-1","message":"Automatic approval review denied"}"#,
        );
        assert!(project(&warning).is_some());
        assert!(project(&review("g", "inProgress", &command("ls"), None)).is_some());
        assert!(project(&review("g", "denied", &command("ls"), None)).is_some());
    }

    #[test]
    fn a_review_in_progress_owns_the_status_until_it_ends() {
        let mut part = ThreadPart::default();
        feed(
            &mut part,
            &review("g1", "inProgress", &command("rm -rf '/tmp/a'"), None),
        );
        assert_eq!(part.reviews.len(), 1);
        assert_eq!(
            part.reviews[0].action,
            Action::Command {
                line: "rm -rf '/tmp/a'".into()
            }
        );
        assert!(part.take_history().is_empty());
        feed(
            &mut part,
            &review("g1", "approved", &command("rm -rf '/tmp/a'"), None),
        );
        assert!(part.reviews.is_empty());
        assert!(part.take_history().is_empty(), "approved: no history");
        assert!(part.recent_denials.is_empty());
    }

    #[test]
    fn a_second_start_of_the_same_review_updates_it() {
        let mut part = ThreadPart::default();
        feed(
            &mut part,
            &review("g1", "inProgress", &command("first"), None),
        );
        feed(
            &mut part,
            &review("g1", "inProgress", &command("second"), None),
        );
        assert_eq!(part.reviews.len(), 1);
        assert_eq!(
            part.reviews[0].action,
            Action::Command {
                line: "second".into()
            }
        );
    }

    #[test]
    fn parallel_reviews_stay_in_arrival_order_and_leave_one_by_one() {
        let mut part = ThreadPart::default();
        for (id, line) in [("g1", "rm one"), ("g2", "rm two"), ("g3", "rm three")] {
            feed(&mut part, &review(id, "inProgress", &command(line), None));
        }
        let ids: Vec<_> = part.reviews.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["g1", "g2", "g3"]);
        feed(
            &mut part,
            &review("g2", "approved", &command("rm two"), None),
        );
        let ids: Vec<_> = part.reviews.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["g1", "g3"]);
    }

    #[test]
    fn a_denied_command_is_the_auto_reviewers_cell_and_a_stored_denial() {
        let mut part = ThreadPart::default();
        feed(
            &mut part,
            &review(
                "g1",
                "inProgress",
                &command("curl https://example.com"),
                None,
            ),
        );
        feed(
            &mut part,
            &review(
                "g1",
                "denied",
                &command("curl https://example.com"),
                Some("Exfiltrates a file."),
            ),
        );
        assert!(part.reviews.is_empty());
        assert_eq!(
            part.take_history(),
            vec![History::Decided(Decided::Request {
                actor: Actor::Guardian,
                subject: Subject::Command("curl https://example.com".into()),
                outcome: Outcome::Denied,
            })]
        );
        let denials: Vec<_> = part.recent_denials.iter().collect();
        assert_eq!(denials.len(), 1);
        assert_eq!(denials[0].id, "g1");
        assert_eq!(denials[0].rationale.as_deref(), Some("Exfiltrates a file."));
    }

    #[test]
    fn a_shell_wrapper_is_stripped_from_the_command_of_the_cell() {
        let mut part = ThreadPart::default();
        feed(
            &mut part,
            &review("g1", "denied", &command("bash -lc 'echo hi'"), None),
        );
        assert_eq!(
            part.take_history(),
            vec![History::Decided(Decided::Request {
                actor: Actor::Guardian,
                subject: Subject::Command("echo hi".into()),
                outcome: Outcome::Denied,
            })]
        );
    }

    #[test]
    fn a_timed_out_command_leaves_a_cell_and_no_denial() {
        let mut part = ThreadPart::default();
        feed(&mut part, &review("g1", "timedOut", &command("make"), None));
        assert_eq!(
            part.take_history(),
            vec![History::Decided(Decided::Request {
                actor: Actor::Guardian,
                subject: Subject::Command("make".into()),
                outcome: Outcome::TimedOut,
            })]
        );
        assert!(part.recent_denials.is_empty());
    }

    #[test]
    fn an_aborted_review_leaves_nothing_but_the_status_goes() {
        let mut part = ThreadPart::default();
        feed(
            &mut part,
            &review("g1", "inProgress", &command("make"), None),
        );
        feed(&mut part, &review("g1", "aborted", &command("make"), None));
        assert!(part.reviews.is_empty());
        assert!(part.take_history().is_empty());
        assert!(part.recent_denials.is_empty());
    }

    #[test]
    fn what_is_not_a_command_has_its_own_subject() {
        let cases = [
            (
                STDIN.to_string(),
                Reviewed::Stdin { process: "42".into(), input: "confirm\n".into() },
            ),
            (
                r#"{"type":"applyPatch","cwd":"/tmp","files":["/tmp/a.rs","/tmp/b.rs"]}"#.into(),
                Reviewed::Patch(vec!["/tmp/a.rs".into(), "/tmp/b.rs".into()]),
            ),
            (
                r#"{"type":"mcpToolCall","server":"docs","toolName":"search","connectorId":null,"connectorName":"Docs","toolTitle":null}"#.into(),
                Reviewed::Mcp { server: "docs".into(), tool: "search".into() },
            ),
            (
                r#"{"type":"networkAccess","target":"https://example.com:443","host":"example.com","protocol":"https","port":443}"#.into(),
                Reviewed::Network("https://example.com:443".into()),
            ),
            (
                r#"{"type":"requestPermissions","reason":"Need write access.","permissions":{"network":null,"fileSystem":null}}"#.into(),
                Reviewed::Permissions(Some("Need write access.".into())),
            ),
        ];
        for (action, about) in cases {
            for (status, verdict) in [("denied", Verdict::Denied), ("timedOut", Verdict::TimedOut)]
            {
                let mut part = ThreadPart::default();
                feed(&mut part, &review("g", status, &action, None));
                assert_eq!(
                    part.take_history(),
                    vec![History::Decided(Decided::Reviewed {
                        verdict,
                        about: about.clone()
                    })],
                    "{action} {status}"
                );
            }
        }
    }

    #[test]
    fn an_execve_is_a_command_of_its_arguments() {
        let mut part = ThreadPart::default();
        let execve = r#"{"type":"execve","source":"shell","program":"rm","argv":["rm","-rf","a b"],"cwd":"/tmp"}"#;
        feed(&mut part, &review("g", "inProgress", execve, None));
        assert_eq!(
            part.reviews[0].action,
            Action::Command {
                line: "rm -rf 'a b'".into()
            }
        );
        let empty = r#"{"type":"execve","source":"shell","program":"ls","argv":[],"cwd":"/tmp"}"#;
        feed(&mut part, &review("h", "inProgress", empty, None));
        assert_eq!(
            part.reviews[1].action,
            Action::Command { line: "ls".into() }
        );
    }

    #[test]
    fn the_ten_most_recent_denials_stay_newest_first() {
        let mut part = ThreadPart::default();
        for n in 0..12 {
            feed(
                &mut part,
                &review(
                    &format!("review-{n}"),
                    "denied",
                    &command(&format!("rm {n}")),
                    None,
                ),
            );
        }
        let ids: Vec<_> = part.recent_denials.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "review-11",
                "review-10",
                "review-9",
                "review-8",
                "review-7",
                "review-6",
                "review-5",
                "review-4",
                "review-3",
                "review-2"
            ]
        );
    }

    #[test]
    fn a_denial_seen_again_moves_to_the_front_without_a_copy() {
        let mut part = ThreadPart::default();
        for n in 0..3 {
            feed(
                &mut part,
                &review(&format!("r{n}"), "denied", &command("x"), None),
            );
        }
        feed(
            &mut part,
            &review("r0", "denied", &command("x"), Some("again")),
        );
        let ids: Vec<_> = part.recent_denials.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["r0", "r2", "r1"]);
    }

    #[test]
    fn the_end_of_a_turn_drops_the_reviews_and_keeps_the_denials() {
        let mut part = ThreadPart::default();
        feed(
            &mut part,
            &review("stale", "inProgress", &command("rm stale"), None),
        );
        feed(
            &mut part,
            &review("old", "denied", &command("rm old"), None),
        );
        part.take_history();
        assert!(part.apply(ThreadEv::TurnEnded).is_none());
        assert!(part.reviews.is_empty());
        assert_eq!(part.recent_denials.iter().count(), 1);
    }

    #[test]
    fn a_warning_is_history_unless_the_review_approved() {
        let mut part = ThreadPart::default();
        for (message, shown) in [
            ("Automatic approval review denied (risk: high): no.", true),
            (
                "Automatic approval review timed out while evaluating the requested approval.",
                true,
            ),
            (
                "Automatic approval review approved (risk: low, authorization: high): ok.",
                false,
            ),
        ] {
            let warning = notification(
                "guardianWarning",
                &format!(r#"{{"threadId":"thread-1","message":{message:?}}}"#),
            );
            feed(&mut part, &warning);
            assert_eq!(
                part.take_history(),
                if shown {
                    vec![History::Warning(message.into())]
                } else {
                    vec![]
                },
                "{message}"
            );
        }
    }

    #[test]
    fn approving_a_denial_takes_it_and_asks_the_server_with_its_event() {
        let mut part = ThreadPart::default();
        feed(
            &mut part,
            &review("g1", "denied", &command("rm x"), Some("Destructive.")),
        );
        part.take_history();
        let ask = part.apply(ThreadEv::ApproveDenied { id: "g1".into() });
        let Some(Ask::ApproveDenied { event, id }) = ask else {
            panic!("an ask, got {ask:?}");
        };
        assert_eq!(id, "g1");
        assert_eq!(event["id"], "g1");
        assert_eq!(event["status"], "denied");
        assert_eq!(event["rationale"], "Destructive.");
        assert_eq!(event["action"]["command"], "rm x");
        assert!(part.recent_denials.is_empty());
        assert_eq!(part.take_history(), vec![History::RetryApproved]);
    }

    #[test]
    fn approving_a_gone_denial_says_so_and_asks_nothing() {
        let mut part = ThreadPart::default();
        assert!(
            part.apply(ThreadEv::ApproveDenied { id: "g1".into() })
                .is_none()
        );
        assert_eq!(part.take_history(), vec![History::DenialGone]);
    }

    #[test]
    fn no_denials_is_an_info_line() {
        let mut part = ThreadPart::default();
        assert!(part.apply(ThreadEv::NoDenials).is_none());
        assert_eq!(part.take_history(), vec![History::NoDenials]);
    }

    #[test]
    fn a_path_that_cannot_be_localized_is_an_error_line_with_the_servers_text() {
        let mut part = ThreadPart::default();
        assert!(
            part.apply(ThreadEv::Unlocalized("bad path".into()))
                .is_none()
        );
        assert_eq!(
            part.take_history(),
            vec![History::Unlocalized("bad path".into())]
        );
    }
}

/// The three guardian notifications through the app's own router: `project_thread` and
/// `notification_thread` carry them to the thread's slot.
mod through_the_app {
    use codex_tui_core::fixtures::sandbox::Sandbox;
    use codex_tui_core::ids::ThreadId;
    use pretty_assertions::assert_eq;

    const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

    fn notify(method: &str, params: &str) -> String {
        format!(r#"{{"step":"notify","message":{{"method":"{method}","params":{params}}}}}"#)
    }

    fn review(id: &str, status: &str, rationale: &str) -> String {
        let (method, done) = if status == "inProgress" {
            ("item/autoApprovalReview/started", "")
        } else {
            (
                "item/autoApprovalReview/completed",
                r#","completedAtMs":1,"decisionSource":"agent""#,
            )
        };
        notify(
            method,
            &format!(
                r#"{{"threadId":"thread-1","turnId":"turn-1","startedAtMs":0{done},"reviewId":"{id}","targetItemId":null,"review":{{"status":"{status}","riskLevel":null,"userAuthorization":null,"rationale":{rationale}}},"action":{{"type":"command","source":"shell","command":"rm -rf x","cwd":"/tmp"}}}}"#
            ),
        )
    }

    #[test]
    #[ignore = "needs(chat-live, popups): `screens::project_thread` has no `approvals::project` arm and `protocol/targets.rs` `notification_thread` names none of ItemGuardianApprovalReviewStarted/Completed, GuardianWarning (checked green with those four lines added and reverted)"]
    fn a_review_under_way_and_a_denial_reach_the_slot_of_their_thread() {
        let script = format!(
            "{SESSION}\n{}\n{}\n",
            review("g1", "inProgress", "null"),
            review("g2", "denied", r#""Destructive.""#)
        );
        let sandbox = Sandbox::play(&script).expect("plays");
        let thread = sandbox
            .app()
            .thread(&ThreadId::new("thread-1"))
            .expect("the thread");
        let part = &thread.screen_slots().approvals;
        let running: Vec<_> = part
            .reviews
            .iter()
            .map(|review| review.id.as_str())
            .collect();
        let kept: Vec<_> = part
            .recent_denials
            .iter()
            .map(|denial| denial.id.as_str())
            .collect();
        assert_eq!((running, kept), (vec!["g1"], vec!["g2"]));
    }
}

/// What an answered user-input request leaves in the thread's history (oracle
/// `bottom_pane/request_user_input/mod.rs` `submit_answers` and `submit_empty_auto_resolution`:
/// a `RequestUserInputResultCell` inserted into history, `interrupted: false`).
mod question_results {
    use std::collections::BTreeMap;

    use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::rpc::Reply;
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::screens::approvals::{Intent as ApprovalIntent, RequestAnswer, input};
    use codex_tui_core::screens::cells::AnsweredQuestion;
    use codex_tui_core::threads::{Item, Notice};
    use pretty_assertions::assert_eq;
    use serde_json::json;

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
            AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
        );
        app
    }

    fn ask(app: &mut CodexApp) {
        let request = serde_json::from_value(json!({
            "method": "item/tool/requestUserInput", "id": 1,
            "params": {
                "threadId": "thread-1", "turnId": "turn-1", "itemId": "call-1",
                "isBlocking": true, "autoResolutionMs": null,
                "questions": [
                    { "id": "choice", "header": "Approach",
                      "question": "Which approach should we use for the next step?",
                      "isOther": true, "isSecret": false,
                      "options": [{ "label": "Keep it small", "description": "Use the helper" }] },
                    { "id": "token", "header": "Token", "question": "Paste the token.",
                      "isOther": false, "isSecret": true, "options": null },
                    { "id": "skipped", "header": "Notes", "question": "Anything else?",
                      "isOther": false, "isSecret": false, "options": null }
                ]
            }
        }))
        .expect("request");
        run(app, AppEv::Backend(BackendEv::Request(Box::new(request))));
    }

    fn respond(app: &mut CodexApp, answer: RequestAnswer) {
        let key = app.requests().head().expect("a request").key.clone();
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
                ApprovalIntent::Respond { key, answer },
            ))),
        );
    }

    fn history(app: &CodexApp) -> Vec<Item> {
        let thread = app.active_thread();
        let transcript = thread.transcript();
        (0..transcript.len())
            .filter_map(|index| transcript.get(index).map(|(_, item)| item.clone()))
            .collect()
    }

    fn question(text: &str, secret: bool, has_options: bool, answers: &[&str]) -> AnsweredQuestion {
        AnsweredQuestion {
            question: text.to_owned(),
            secret,
            has_options,
            answers: answers.iter().map(|answer| (*answer).to_owned()).collect(),
        }
    }

    /// The submit leaves every question in the history with what it was answered: the option,
    /// the typed note after it, a secret one as it is (the cell masks it), and an empty list for
    /// a question the user left alone.
    #[test]
    fn a_submitted_exchange_is_recorded_with_its_answers() {
        let mut app = ready();
        ask(&mut app);
        respond(
            &mut app,
            RequestAnswer::UserInput(BTreeMap::from([
                (
                    "choice".to_owned(),
                    vec![
                        "Keep it small".to_owned(),
                        "user_note: Preserve it".to_owned(),
                    ],
                ),
                ("token".to_owned(), vec!["user_note: hunter2".to_owned()]),
                ("skipped".to_owned(), Vec::new()),
            ])),
        );
        assert!(app.requests().entries.is_empty());
        assert_eq!(
            history(&app),
            vec![Item::Notice(Notice::Questions {
                questions: vec![
                    question(
                        "Which approach should we use for the next step?",
                        false,
                        true,
                        &["Keep it small", "user_note: Preserve it"],
                    ),
                    question("Paste the token.", true, false, &["user_note: hunter2"]),
                    question("Anything else?", false, false, &[]),
                ],
                interrupted: false,
            })]
        );
    }

    /// A request that ran out answers empty (`submit_empty_auto_resolution`): the cell still
    /// lands, every question unanswered, `interrupted` stays false.
    #[test]
    fn an_auto_resolved_request_is_recorded_with_every_question_unanswered() {
        let mut app = ready();
        ask(&mut app);
        respond(&mut app, input::empty_answer());
        let history = history(&app);
        let [
            Item::Notice(Notice::Questions {
                questions,
                interrupted,
            }),
        ] = history.as_slice()
        else {
            panic!("one result cell expected: {history:?}");
        };
        assert!(!interrupted);
        assert_eq!(questions.len(), 3);
        assert!(questions.iter().all(|question| !question.answered()));
    }

    /// An answer that does not fit the pending request is dropped: it leaves nothing behind.
    #[test]
    fn a_stale_answer_leaves_no_result() {
        let mut app = ready();
        ask(&mut app);
        respond(
            &mut app,
            RequestAnswer::UserInput(BTreeMap::from([("choice".to_owned(), Vec::new())])),
        );
        assert_eq!(app.requests().entries.len(), 1);
        assert_eq!(history(&app), Vec::new());
    }
}
