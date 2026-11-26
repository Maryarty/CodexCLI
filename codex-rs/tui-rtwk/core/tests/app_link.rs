#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The elicitations that are a link to open (oracle `bottom_pane/app_link_view.rs`,
//! `push_mcp_server_elicitation_request`, `McpServerElicitationFormRequest`): how a request is
//! read (`link::parse`), what the oracle refuses (declined at once, no dialog), and what each
//! answer asks of the app.

use codex_app_server_protocol::McpServerElicitationRequestParams as Params;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::screens::approvals::link::{self, Kind, Parsed};
use codex_tui_core::screens::approvals::{
    Decision, Intent as ApprovalIntent, Prompt, RequestAnswer,
};
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, select};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

fn suggestion(kind: &str, url: &str) -> Value {
    json!({
        "threadId": "thread-1", "turnId": "turn-1", "serverName": "codex_apps",
        "environmentId": null, "mode": "form", "message": "Install Google Calendar",
        "requestedSchema": { "type": "object", "properties": {} },
        "_meta": {
            "codex_approval_kind": "tool_suggestion", "tool_type": "connector",
            "suggest_type": kind, "suggest_reason": "Plan and reference events",
            "tool_id": "connector_google_calendar", "tool_name": "Google Calendar",
            "install_url": url
        }
    })
}

fn url_request(server: &str, url: &str, meta: Value) -> Value {
    json!({
        "threadId": "thread-1", "turnId": "turn-1", "serverName": server,
        "mode": "url", "_meta": meta, "message": "Review the payment details to continue.",
        "url": url, "elicitationId": "payment-123"
    })
}

fn auth_failure() -> Value {
    json!({ "_codex_apps": { "connector_auth_failure": {
        "is_auth_failure": true, "connector_id": "connector_slack", "connector_name": "Slack"
    } } })
}

fn parse(params: Value) -> Parsed {
    link::parse(&serde_json::from_value::<Params>(params).expect("params"))
}

fn link_of(params: Value) -> link::AppLinkPrompt {
    match parse(params) {
        Parsed::Link(prompt) => prompt,
        other => panic!("a link, got {other:?}"),
    }
}

#[test]
fn an_install_suggestion_is_a_link_with_its_app_and_reason() {
    let prompt = link_of(suggestion(
        "install",
        "https://example.test/google-calendar",
    ));
    assert_eq!(prompt.kind, Kind::Install);
    assert_eq!(prompt.server, "codex_apps");
    assert_eq!(prompt.app_id, "connector_google_calendar");
    assert_eq!(prompt.name, "Google Calendar");
    assert_eq!(prompt.reason.as_deref(), Some("Plan and reference events"));
    assert_eq!(prompt.url, "https://example.test/google-calendar");
    assert!(!prompt.installed() && !prompt.browser_flow() && prompt.refreshes_apps());
}

#[test]
fn an_enable_suggestion_is_an_installed_app() {
    let prompt = link_of(suggestion("enable", "https://example.test/google-calendar"));
    assert_eq!(prompt.kind, Kind::Enable);
    assert!(prompt.installed() && !prompt.browser_flow());
}

#[test]
fn the_apps_server_asks_a_sign_in_on_a_chatgpt_host() {
    let prompt = link_of(url_request(
        "codex_apps",
        "https://chatgpt.com/apps/slack/auth",
        auth_failure(),
    ));
    assert_eq!(prompt.kind, Kind::Auth);
    assert_eq!(
        (prompt.app_id.as_str(), prompt.name.as_str()),
        ("connector_slack", "Slack")
    );
    assert!(prompt.browser_flow() && prompt.refreshes_apps());
    // the sign-in of the apps server names a ChatGPT host and an auth failure, nothing else
    for rejected in [
        url_request("codex_apps", "https://evil.example/auth", auth_failure()),
        url_request("codex_apps", "https://chatgpt.com/auth", Value::Null),
        url_request(
            "codex_apps",
            "https://chatgpt.com/auth",
            json!({ "_codex_apps": { "connector_auth_failure": { "is_auth_failure": false } } }),
        ),
    ] {
        assert_eq!(parse(rejected.clone()), Parsed::Reject, "{rejected}");
    }
}

#[test]
fn another_servers_url_is_a_generic_link_with_no_app_name() {
    let prompt = link_of(url_request(
        "payments",
        "https://payments.example/checkout/123",
        Value::Null,
    ));
    assert_eq!(prompt.kind, Kind::Url);
    assert_eq!(prompt.server, "payments");
    assert_eq!(prompt.name, "");
    assert_eq!(
        prompt.reason.as_deref(),
        Some("Review the payment details to continue.")
    );
    assert_eq!(prompt.url, "https://payments.example/checkout/123");
    // another server's links are not the apps' own: no directory is read when it is done
    assert!(prompt.browser_flow() && !prompt.refreshes_apps());
}

#[test]
fn a_url_is_shown_as_the_oracle_normalizes_it() {
    let prompt = link_of(url_request(
        "payments",
        "https://Payments.Example",
        Value::Null,
    ));
    assert_eq!(prompt.url, "https://payments.example/");
}

/// What `validate_external_url` refuses, in both the suggestion and the URL request.
#[test]
fn a_link_the_oracle_refuses_is_rejected() {
    for url in [
        "javascript:alert(1)",
        "https://",
        "https:",
        "mailto:someone@example.test",
        "",
        "not a URL",
        "http://example.test/install",
        "file:///tmp/connector",
        "custom://example.test/install",
        "https://user:password@example.test/install",
        "https://user@example.test/install",
    ] {
        assert_eq!(
            parse(suggestion("install", url)),
            Parsed::Reject,
            "install {url:?}"
        );
        assert_eq!(
            parse(suggestion("enable", url)),
            Parsed::Reject,
            "enable {url:?}"
        );
        assert_eq!(
            parse(url_request("payments", url, Value::Null)),
            Parsed::Reject,
            "url {url:?}"
        );
    }
}

#[test]
fn what_is_not_a_link_request_is_left_to_the_other_dialogs() {
    // a form with no tool suggestion
    let mut plain = suggestion("install", "https://example.test/x");
    plain["_meta"] = Value::Null;
    assert_eq!(parse(plain), Parsed::NotALink);
    // a suggestion that names no install URL
    let mut nameless = suggestion("install", "https://example.test/x");
    nameless["_meta"]
        .as_object_mut()
        .unwrap()
        .remove("install_url");
    assert_eq!(parse(nameless), Parsed::NotALink);
    // the device verification is its own dialog
    let verification = json!({
        "threadId": "thread-1", "turnId": "turn-1", "serverName": "deployments",
        "mode": "openai/userVerification", "_meta": null, "title": "Approve?",
        "description": "Approve deploying the reviewed release?", "challenge": "challenge"
    });
    assert_eq!(parse(verification), Parsed::NotALink);
}

fn request(id: u64, params: Value) -> AppEv {
    let request = serde_json::from_value(json!({
        "method": "mcpServer/elicitation/request", "id": id, "params": params
    }))
    .expect("request");
    AppEv::Backend(BackendEv::Request(Box::new(request)))
}

fn send(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn approvals(intent: ApprovalIntent) -> AppEv {
    AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(intent)))
}

fn head(app: &CodexApp) -> codex_tui_core::ids::RequestKey {
    app.requests().head().expect("a request").key.clone()
}

/// The results the app sent for the requests, in order.
fn results(fx: &AppFx) -> Vec<Value> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::ResolveRequest { result, .. } => Some(result.clone()),
            _ => None,
        })
        .collect()
}

fn refreshes_apps(fx: &AppFx) -> bool {
    fx.commands().iter().any(|command| {
        matches!(
            command,
            Command::Screen {
                command: ScreenCommand::Select(select::Command::Apps { .. }),
                ..
            }
        )
    })
}

#[test]
fn a_refused_link_is_declined_at_once_and_never_queued() {
    let mut app = CodexApp::new();
    let fx = send(
        &mut app,
        request(5, suggestion("install", "http://example.test/install")),
    );
    assert!(app.requests().entries.is_empty());
    assert_eq!(
        results(&fx),
        [json!({ "action": "decline", "content": null, "_meta": null })]
    );
}

#[test]
fn a_valid_link_is_queued_as_a_prompt_and_asks_nothing() {
    let mut app = CodexApp::new();
    let fx = send(
        &mut app,
        request(5, suggestion("install", "https://example.test/install")),
    );
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    let entry = app.requests().head().expect("queued");
    assert!(
        matches!(entry.prompt.as_deref(), Some(Prompt::AppLink(link)) if link.kind == Kind::Install)
    );
}

#[test]
fn opening_the_page_asks_the_browser_for_the_validated_url_and_keeps_the_request() {
    let mut app = CodexApp::new();
    send(
        &mut app,
        request(5, suggestion("install", "https://Example.test/install")),
    );
    let key = head(&app);
    let fx = send(&mut app, approvals(ApprovalIntent::OpenLink { key }));
    assert!(matches!(
        fx.commands(),
        [Command::Effect { ask: codex_tui_core::app::effects::Ask::OpenUrl { url, .. }, .. }] if url == "https://example.test/install"
    ));
    assert_eq!(app.requests().entries.len(), 1);
}

#[test]
fn finishing_an_apps_link_reads_the_directory_again_then_accepts_with_no_content() {
    let mut app = CodexApp::new();
    send(
        &mut app,
        request(5, suggestion("install", "https://example.test/install")),
    );
    let key = head(&app);
    let fx = send(
        &mut app,
        approvals(ApprovalIntent::LinkDone { key: key.clone() }),
    );
    assert!(refreshes_apps(&fx));
    assert_eq!(
        results(&fx),
        [json!({ "action": "accept", "content": null, "_meta": null })]
    );
    assert!(app.requests().entries.is_empty());
    // a second finish for the same request is stale: nothing is sent
    let again = send(&mut app, approvals(ApprovalIntent::LinkDone { key }));
    assert!(again.commands().is_empty(), "{:?}", again.commands());
}

#[test]
fn finishing_a_link_of_another_server_does_not_refresh_the_apps() {
    let mut app = CodexApp::new();
    send(
        &mut app,
        request(
            7,
            url_request(
                "payments",
                "https://payments.example/checkout/123",
                Value::Null,
            ),
        ),
    );
    let key = head(&app);
    let fx = send(&mut app, approvals(ApprovalIntent::LinkDone { key }));
    assert!(!refreshes_apps(&fx), "{:?}", fx.commands());
    assert_eq!(
        results(&fx),
        [json!({ "action": "accept", "content": null, "_meta": null })]
    );
}

#[test]
fn enabling_the_app_writes_the_switch_before_the_request_is_accepted() {
    let mut app = CodexApp::new();
    send(
        &mut app,
        request(6, suggestion("enable", "https://example.test/install")),
    );
    let key = head(&app);
    let fx = send(&mut app, approvals(ApprovalIntent::LinkEnable { key }));
    let commands = fx.commands();
    let written = commands.iter().position(|command| {
        matches!(
            command,
            Command::Screen {
                command: ScreenCommand::Select(select::Command::SetApp { id, enabled: true }),
                ..
            } if id == "connector_google_calendar"
        )
    });
    let accepted = commands
        .iter()
        .position(|command| matches!(command, Command::ResolveRequest { .. }));
    assert!(written.is_some() && written < accepted, "{commands:?}");
}

#[test]
fn a_decline_and_a_cancel_both_answer_decline() {
    for cancel in [false, true] {
        let mut app = CodexApp::new();
        send(
            &mut app,
            request(5, suggestion("install", "https://example.test/install")),
        );
        let key = head(&app);
        let ev = if cancel {
            AppEv::Intent(Intent::CancelRequests)
        } else {
            AppEv::Intent(Intent::AnswerRequest {
                key,
                answer: RequestAnswer::Choice(Decision::Decline),
            })
        };
        let fx = send(&mut app, ev);
        assert_eq!(
            results(&fx),
            [json!({ "action": "decline", "content": null, "_meta": null })],
            "cancel: {cancel}"
        );
        assert!(app.requests().entries.is_empty());
        assert!(!refreshes_apps(&fx));
    }
}
