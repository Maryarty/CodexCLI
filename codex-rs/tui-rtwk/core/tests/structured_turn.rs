#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `AppCx::structured_turn` (ADR-012 "structured turn", oracle `temporary_structured_request.rs`):
//! a slice asks for an isolated request to the model; the app reads the MCP servers, starts an
//! ephemeral thread without tools, runs one turn with the output schema, takes the last assistant
//! message of the stream and lets the thread go. The asking slice (here the recap's) is told the
//! message or why there is none, and the temporary thread is unsubscribed whatever happened. The
//! helpers are the walk of a run for the tests of the slices that ask (`recap`, `rename_prompt`).

use codex_app_server_protocol::{
    AskForApproval, ClientRequest, RequestId, SandboxMode, ServerNotification, ThreadSource,
    ThreadStartParams, UserInput,
};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Now, apply_app};
use codex_tui_core::ids::Ticket;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, ScreenReply, agents};
use codex_tui_core::threads::ThreadAsk;
use codex_tui_core::threads::structured::Kind;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

use crate::recap::{app, notes, recap};
use codex_tui_core::features::recap::Note as RecapNote;

/// The id of the temporary thread the server starts.
pub const TEMP: &str = "temp-1";
/// The id of its turn.
pub const TURN: &str = "turn-temp";

pub fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

/// The thread requests among the commands of a step, with their tickets.
pub fn thread_asks(fx: &AppFx) -> Vec<(Ticket, ThreadAsk)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
                ..
            } => Some((*ticket, ask.clone())),
            _ => None,
        })
        .collect()
}

/// The one thread request of a step.
pub fn the_ask(fx: &AppFx) -> (Ticket, ThreadAsk) {
    let asks = thread_asks(fx);
    let [one] = asks.as_slice() else {
        panic!("expected one thread request, got {asks:?}");
    };
    one.clone()
}

/// The server answers the request `ticket` with `reply`.
pub fn answer(app: &mut CodexApp, ticket: Ticket, reply: agents::Reply) -> AppFx {
    run(
        app,
        AppEv::Reply(ticket, Reply::screen(ScreenReply::Agents(reply))),
    )
}

/// The server refuses the request `ticket`.
pub fn refuse(app: &mut CodexApp, ticket: Ticket, message: &str) -> AppFx {
    run(app, AppEv::Reply(ticket, Reply::failed(message)))
}

/// What the server says on its stream.
pub fn stream(app: &mut CodexApp, message: Value) -> AppFx {
    let notification: ServerNotification = serde_json::from_value(message).expect("a notification");
    run(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
    )
}

/// The temporary thread's assistant message.
pub fn says(app: &mut CodexApp, text: &str) -> AppFx {
    says_in(app, TEMP, TURN, text)
}

/// The assistant message of the turn `turn` of the thread `thread`.
pub fn says_in(app: &mut CodexApp, thread: &str, turn: &str, text: &str) -> AppFx {
    stream(
        app,
        json!({"method": "item/completed", "params": {"threadId": thread, "turnId": turn,
        "completedAtMs": 0, "startedAtMs": 0,
        "item": {"type": "agentMessage", "id": "m1", "text": text, "phase": null}}}),
    )
}

/// The end of the temporary thread's turn (`status`: `completed`, `failed`, `interrupted`).
pub fn ends(app: &mut CodexApp, status: &str) -> AppFx {
    ends_in(app, TEMP, TURN, status)
}

/// The end of the turn `turn` of the thread `thread`.
pub fn ends_in(app: &mut CodexApp, thread: &str, turn: &str, status: &str) -> AppFx {
    stream(
        app,
        json!({"method": "turn/completed", "params": {"threadId": thread, "turn": {
        "id": turn, "items": [], "itemsView": "summary", "status": status, "error": null}}}),
    )
}

/// The walk of a run up to its turn: the MCP servers `servers` are read, the thread starts as
/// asked, its turn starts. What was asked of the server, in order, comes back; the model's answer
/// is next.
pub fn walk(app: &mut CodexApp, first: &AppFx, servers: Value) -> Vec<ThreadAsk> {
    walk_in(app, first, servers, TEMP, TURN)
}

/// `walk` of a run whose thread and turn the server calls `thread` and `turn`.
pub fn walk_in(
    app: &mut CodexApp,
    first: &AppFx,
    servers: Value,
    thread: &str,
    turn: &str,
) -> Vec<ThreadAsk> {
    let (ticket, config) = the_ask(first);
    let servers = servers.as_object().cloned().unwrap_or_default();
    let fx = answer(app, ticket, agents::Reply::McpServers(servers));
    let (ticket, start) = the_ask(&fx);
    let fx = answer(
        app,
        ticket,
        agents::Reply::Temporary {
            thread: thread.to_owned(),
            as_asked: true,
        },
    );
    let (ticket, begin) = the_ask(&fx);
    let fx = answer(
        app,
        ticket,
        agents::Reply::StructuredTurn {
            turn: turn.to_owned(),
        },
    );
    assert_eq!(thread_asks(&fx), vec![], "the run waits for the stream now");
    vec![config, start, begin]
}

/// The requests that let the temporary thread go among `fx`'s.
pub fn unsubscribes(fx: &AppFx) -> Vec<String> {
    thread_asks(fx)
        .into_iter()
        .filter_map(|(_, ask)| match ask {
            ThreadAsk::Unsubscribe { thread } => Some(thread),
            _ => None,
        })
        .collect()
}

fn started() -> (CodexApp, AppFx) {
    let mut app = app(&[("fix the bug", "done")]);
    let fx = recap(&mut app);
    (app, fx)
}

const SUMMARY: &str = r#"{"summary": "Fixed it.", "next_action": null}"#;

fn says_recap() -> Vec<RecapNote> {
    vec![RecapNote::Recap {
        summary: "Fixed it.".to_owned(),
        next_action: None,
    }]
}

#[test]
fn the_door_runs_the_requests_in_order_and_answers_the_asking_slice() {
    let (mut app, first) = started();
    let asked = walk(
        &mut app,
        &first,
        json!({"docs": {"command": "x"}, "web": {}}),
    );
    let [
        ThreadAsk::McpServers { .. },
        ThreadAsk::StartTemporary(temporary),
        ThreadAsk::StructuredTurn {
            thread,
            kind,
            prompt,
            schema,
            ..
        },
    ] = asked.as_slice()
    else {
        panic!("the requests of a run: {asked:?}");
    };
    // the thread starts as the source's, with each of the servers read disabled
    assert_eq!(temporary.kind, Kind::Recap);
    assert_eq!(
        temporary.mcp_servers.keys().collect::<Vec<_>>(),
        ["docs", "web"]
    );
    assert_eq!((thread.as_str(), *kind), (TEMP, Kind::Recap));
    assert!(prompt.ends_with("User: fix the bug\n\nAssistant: done"));
    assert_eq!(schema["required"], json!(["summary", "next_action"]));
    assert_eq!(notes(&app), vec![]);

    // the stream carries the message; the end of the turn answers the slice and lets go
    assert_eq!(unsubscribes(&says(&mut app, SUMMARY)), Vec::<String>::new());
    let fx = ends(&mut app, "completed");
    assert_eq!(unsubscribes(&fx), [TEMP]);
    assert_eq!(notes(&app), says_recap());
}

#[test]
fn what_the_stream_said_before_the_turn_was_answered_is_kept() {
    let (mut app, first) = started();
    let (ticket, _) = the_ask(&first);
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::McpServers(Default::default()),
    );
    let (ticket, _) = the_ask(&fx);
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::Temporary {
            thread: TEMP.to_owned(),
            as_asked: true,
        },
    );
    let (ticket, _) = the_ask(&fx);
    // a fast model: the turn is over before `turn/start` answers
    says(&mut app, SUMMARY);
    ends(&mut app, "completed");
    assert_eq!(notes(&app), vec![]);
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::StructuredTurn {
            turn: TURN.to_owned(),
        },
    );
    assert_eq!(unsubscribes(&fx), [TEMP]);
    assert_eq!(notes(&app), says_recap());
}

#[test]
fn the_last_message_of_the_turn_is_the_answer_and_other_threads_are_not_heard() {
    let (mut app, first) = started();
    walk(&mut app, &first, json!({}));
    // another thread's turn and another turn of this one are not the run's
    stream(
        &mut app,
        json!({"method": "turn/completed", "params": {"threadId": "other", "turn": {
            "id": TURN, "items": [], "itemsView": "summary", "status": "failed",
            "error": null}}}),
    );
    stream(
        &mut app,
        json!({"method": "turn/completed", "params": {"threadId": TEMP, "turn": {
            "id": "turn-other", "items": [], "itemsView": "summary", "status": "failed",
            "error": null}}}),
    );
    says(&mut app, r#"{"summary": "First."}"#);
    says(&mut app, SUMMARY);
    ends(&mut app, "completed");
    assert_eq!(notes(&app), says_recap());
}

/// A run that ends without an answer says the failure and lets the thread go (when there is one).
#[test]
fn a_run_that_ends_without_an_answer_is_the_slices_failure_and_the_thread_is_let_go() {
    // the turn failed, was interrupted, completed without a message, or said too much
    for (message, status) in [
        (Some("x"), "failed"),
        (Some("x"), "interrupted"),
        (None, "completed"),
        (Some(&"x".repeat(9 * 1024)[..]), "completed"),
    ] {
        let (mut app, first) = started();
        walk(&mut app, &first, json!({}));
        // a message that is too long ends the run as it comes, the rest when the turn ends
        let mut unsubscribed = Vec::new();
        if let Some(text) = message {
            unsubscribed.extend(unsubscribes(&says(&mut app, text)));
        }
        unsubscribed.extend(unsubscribes(&ends(&mut app, status)));
        assert_eq!(unsubscribed, [TEMP], "{status}");
        assert_eq!(notes(&app), vec![RecapNote::Failed], "{status}");
    }
}

#[test]
fn a_refused_request_ends_the_run_and_lets_go_only_what_started() {
    // config/read and thread/start refused: no thread to let go
    for refused in 0..2 {
        let (mut app, first) = started();
        let (mut ticket, _) = the_ask(&first);
        if refused == 1 {
            let fx = answer(
                &mut app,
                ticket,
                agents::Reply::McpServers(Default::default()),
            );
            ticket = the_ask(&fx).0;
        }
        let fx = refuse(&mut app, ticket, "no server");
        assert_eq!(thread_asks(&fx), vec![]);
        assert_eq!(notes(&app), vec![RecapNote::Failed]);
    }
    // turn/start refused: the thread started and is let go
    let (mut app, first) = started();
    let (ticket, _) = the_ask(&first);
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::McpServers(Default::default()),
    );
    let (ticket, _) = the_ask(&fx);
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::Temporary {
            thread: TEMP.to_owned(),
            as_asked: true,
        },
    );
    let (ticket, _) = the_ask(&fx);
    let fx = refuse(&mut app, ticket, "no turn");
    assert_eq!(unsubscribes(&fx), [TEMP]);
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

#[test]
fn a_thread_that_did_not_start_read_only_is_let_go_and_never_asked() {
    let (mut app, first) = started();
    let (ticket, _) = the_ask(&first);
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::McpServers(Default::default()),
    );
    let (ticket, _) = the_ask(&fx);
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::Temporary {
            thread: TEMP.to_owned(),
            as_asked: false,
        },
    );
    assert_eq!(
        thread_asks(&fx)
            .into_iter()
            .map(|(_, ask)| ask)
            .collect::<Vec<_>>(),
        vec![ThreadAsk::Unsubscribe {
            thread: TEMP.to_owned()
        }],
        "no turn starts on a thread that could write"
    );
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

#[test]
fn an_answer_of_the_wrong_shape_is_a_refusal() {
    let (mut app, first) = started();
    let (ticket, _) = the_ask(&first);
    let fx = answer(&mut app, ticket, agents::Reply::Done);
    assert_eq!(thread_asks(&fx), vec![]);
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

/// The clock ends a phase that takes longer than 30 seconds (`STRUCTURED_TURN_TIMEOUT`): the
/// turn is interrupted, the thread let go, the slice told.
#[test]
fn a_turn_that_takes_too_long_is_interrupted_and_let_go() {
    let mut app = app(&[("fix the bug", "done")]);
    run(&mut app, AppEv::Clock(Now::at_secs(1_000)));
    let first = recap(&mut app);
    let fx = {
        walk(&mut app, &first, json!({}));
        run(&mut app, AppEv::Clock(Now::at_secs(1_010)))
    };
    // the turn started at 1000 s (the clock stood still): late at 1030 s
    assert_eq!(fx.wake().map(|wake| wake.at()), Some(Now::at_secs(1_030)));
    assert_eq!(
        thread_asks(&run(&mut app, AppEv::Clock(Now::at_secs(1_029)))),
        vec![]
    );
    let fx = run(&mut app, AppEv::Clock(Now::at_secs(1_030)));
    let asks: Vec<ThreadAsk> = thread_asks(&fx).into_iter().map(|(_, ask)| ask).collect();
    assert_eq!(
        asks,
        vec![
            ThreadAsk::Interrupt {
                thread: TEMP.to_owned(),
                turn: TURN.to_owned()
            },
            ThreadAsk::Unsubscribe {
                thread: TEMP.to_owned()
            },
        ]
    );
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
    assert_eq!(fx.wake(), None, "nothing waits any more");
    // the late message finds no run
    says(&mut app, SUMMARY);
    ends(&mut app, "completed");
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

#[test]
fn a_request_that_is_not_answered_in_time_ends_the_run() {
    let mut app = app(&[("fix the bug", "done")]);
    run(&mut app, AppEv::Clock(Now::at_secs(1_000)));
    let first = recap(&mut app);
    let (ticket, _) = the_ask(&first);
    let fx = run(&mut app, AppEv::Clock(Now::at_secs(1_030)));
    assert_eq!(thread_asks(&fx), vec![], "no thread to let go yet");
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
    // the answer that comes later is dropped
    let fx = answer(
        &mut app,
        ticket,
        agents::Reply::McpServers(Default::default()),
    );
    assert_eq!(thread_asks(&fx), vec![]);
}

#[test]
fn a_connection_that_is_lost_ends_a_run_that_waits_for_the_stream() {
    let (mut app, first) = started();
    walk(&mut app, &first, json!({}));
    let fx = run(&mut app, AppEv::Disconnected);
    assert_eq!(
        unsubscribes(&fx),
        Vec::<String>::new(),
        "the server that knew the thread is gone"
    );
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

#[test]
fn a_reconnect_fails_the_request_in_flight() {
    let (mut app, first) = started();
    let _ = the_ask(&first);
    app.attach();
    app.assert_consistent();
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

fn thread_json(id: &str) -> Value {
    json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "", "ephemeral": true, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null, "cwd": "/tmp/project",
        "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
        "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null, "turns": []
    })
}

fn started_json(sandbox: Value) -> Value {
    json!({
        "thread": thread_json(TEMP), "model": "gpt-5", "modelProvider": "openai",
        "serviceTier": null, "cwd": "/tmp/project", "instructionSources": [],
        "approvalPolicy": "never", "approvalsReviewer": "user", "sandbox": sandbox,
        "reasoningEffort": null
    })
}

/// The server answers the thread request of `fx` with its `result` (JSON).
fn serve(app: &mut CodexApp, fx: &AppFx, result: Value) -> AppFx {
    let command = fx
        .commands()
        .iter()
        .find(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Agents(agents::Command::Thread(_)),
                    ..
                }
            )
        })
        .cloned()
        .expect("a thread request");
    let (ticket, reply) =
        codex_tui_core::protocol::reply(&command, Ok(result)).expect("a command with a reply");
    run(app, AppEv::Reply(ticket, reply))
}

/// The server's JSON of each answer decodes to what the run reads: the servers of the config,
/// the thread's id with whether it started read-only, the turn's id.
#[test]
fn the_servers_answers_decode_to_what_the_run_reads() {
    let (mut app, first) = started();
    let fx = serve(
        &mut app,
        &first,
        json!({"config": {"mcp_servers": {"docs": {"command": "x"}}}, "origins": {}}),
    );
    let (_, ThreadAsk::StartTemporary(temporary)) = the_ask(&fx) else {
        panic!("the thread starts after the config is read");
    };
    assert_eq!(temporary.mcp_servers.keys().collect::<Vec<_>>(), ["docs"]);
    let fx = serve(
        &mut app,
        &fx,
        started_json(json!({"type": "readOnly", "networkAccess": false})),
    );
    let (_, ThreadAsk::StructuredTurn { thread, .. }) = the_ask(&fx) else {
        panic!("the turn starts on the thread");
    };
    assert_eq!(thread, TEMP);
    let fx = serve(
        &mut app,
        &fx,
        json!({"turn": {"id": TURN, "items": [], "itemsView": "summary",
            "status": "inProgress", "error": null}}),
    );
    assert_eq!(thread_asks(&fx), vec![]);
    says(&mut app, SUMMARY);
    ends(&mut app, "completed");
    assert_eq!(notes(&app), says_recap());
}

/// A thread that came back with a sandbox that can write is not asked anything.
#[test]
fn a_thread_the_server_started_with_write_access_is_let_go() {
    let (mut app, first) = started();
    let fx = serve(&mut app, &first, json!({"config": {}, "origins": {}}));
    let fx = serve(
        &mut app,
        &fx,
        started_json(json!({"type": "dangerFullAccess"})),
    );
    assert_eq!(unsubscribes(&fx), [TEMP]);
    assert_eq!(notes(&app), vec![RecapNote::Failed]);
}

/// The first request the thread asks of `fx` made into the wire request (`thread/start`,
/// `turn/start`, `config/read`).
fn wire(fx: &AppFx) -> ClientRequest {
    let command = fx
        .commands()
        .iter()
        .find(|c| {
            matches!(
                c,
                Command::Screen {
                    command: ScreenCommand::Agents(agents::Command::Thread(_)),
                    ..
                }
            )
        })
        .expect("a thread request");
    codex_tui_core::protocol::client_request(command, RequestId::Integer(1), None)
        .expect("a wire request")
}

fn start_params(request: ClientRequest) -> ThreadStartParams {
    match request {
        ClientRequest::ThreadStart { params, .. } => params,
        other => panic!("not a thread/start: {other:?}"),
    }
}

/// The requests of a recap's run, as the server gets them: the config is read for the thread's
/// directory; the thread is ephemeral, has no tools and no environment, disables each MCP server
/// of the config, and is read-only whatever the profile in use (`:workspace` is not a named one);
/// its turn follows the schema.
#[test]
fn the_requests_of_a_recap_run_are_isolated_and_read_only() {
    let (mut app, first) = started();
    let ClientRequest::ConfigRead { params, .. } = wire(&first) else {
        panic!("the config is read first");
    };
    assert!(!params.include_layers);
    let fx = serve(
        &mut app,
        &first,
        json!({"config": {"mcp_servers": {"docs": {"command": "x"}}}, "origins": {}}),
    );
    let params = start_params(wire(&fx));
    assert_eq!(params.ephemeral, Some(true));
    assert_eq!(params.permissions, None);
    assert_eq!(params.sandbox, Some(SandboxMode::ReadOnly));
    assert_eq!(params.approval_policy, None);
    assert_eq!(params.environments, Some(vec![]));
    assert_eq!(params.dynamic_tools, Some(vec![]));
    assert_eq!(
        params.thread_source,
        Some(ThreadSource::Feature("system".to_owned()))
    );
    let config = params.config.expect("a config");
    assert_eq!(config["mcp_servers"], json!({"docs": {"enabled": false}}));
    assert_eq!(config["features.shell_tool"], json!(false));
    assert_eq!(config["default_permissions"], json!(":read-only"));
    assert_eq!(config["web_search"], json!("disabled"));
    let fx = serve(
        &mut app,
        &fx,
        started_json(json!({"type": "readOnly", "networkAccess": false})),
    );
    let ClientRequest::TurnStart { params, .. } = wire(&fx) else {
        panic!("the turn starts next");
    };
    assert_eq!(params.thread_id, TEMP);
    assert_eq!(
        params.permissions, None,
        "a recap's turn keeps the thread's"
    );
    assert_eq!(
        params.output_schema.expect("a schema")["required"],
        json!(["summary", "next_action"])
    );
    assert!(matches!(
        params.input.as_slice(),
        [UserInput::Text { text, .. }] if text.ends_with("Assistant: done")
    ));
}

/// A title thread asks for the built-in read-only profile and never asks for approval, and so
/// does its turn.
#[test]
fn the_requests_of_a_title_run_are_read_only_by_name_and_never_ask() {
    let mut app = crate::rename_prompt::talked_app();
    let first = crate::rename_prompt::open_title(&mut app);
    let fx = serve(&mut app, &first, json!({"config": {}, "origins": {}}));
    let params = start_params(wire(&fx));
    assert_eq!(params.permissions.as_deref(), Some(":read-only"));
    assert_eq!(params.sandbox, None);
    assert_eq!(params.approval_policy, Some(AskForApproval::Never));
    assert_eq!(
        params.thread_source,
        Some(ThreadSource::Feature("thread_title".to_owned()))
    );
    let config = params.config.expect("a config");
    assert_eq!(config["mcp_servers"], json!({}));
    assert!(!config.contains_key("default_permissions"));
    let fx = serve(
        &mut app,
        &fx,
        started_json(json!({"type": "readOnly", "networkAccess": false})),
    );
    let ClientRequest::TurnStart { params, .. } = wire(&fx) else {
        panic!("the turn starts next");
    };
    assert_eq!(params.permissions.as_deref(), Some(":read-only"));
    assert_eq!(params.runtime_workspace_roots, Some(vec![]));
}
