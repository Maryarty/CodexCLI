//! Builders of the protocol payloads the fixtures and tests feed: the
//! notifications and requests as the app-server sends them (the shapes
//! codex-tui's `chatwidget/tests/helpers.rs` feeders build). Everything is
//! a `BackendEv`, the form `apply_app` takes.

use codex_app_server_protocol::AgentMessageDeltaNotification;
use codex_app_server_protocol::ItemCompletedNotification;
use codex_app_server_protocol::ItemStartedNotification;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ThreadItem;
use codex_app_server_protocol::ThreadItemPayload;
use codex_app_server_protocol::ToolRequestUserInputParams;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStartedNotification;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::UserInput;

use crate::app::BackendEv;

fn note(notification: ServerNotification) -> BackendEv {
    BackendEv::Notification(Box::new(notification))
}

/// A notification as the wire has it (`{"method":..,"params":..}`): what a script's `notify` step
/// holds, built here for the shapes that have no builder of their own.
fn wire(method: &str, params: serde_json::Value) -> Result<BackendEv, String> {
    serde_json::from_value::<ServerNotification>(serde_json::json!({
        "method": method,
        "params": params,
    }))
    .map(note)
    .map_err(|err| format!("{method}: {err}"))
}

fn turn(id: &str, status: TurnStatus) -> Turn {
    Turn {
        id: id.to_string(),
        root_turn_id: None,
        items: Vec::new(),
        items_view: codex_app_server_protocol::TurnItemsView::NotLoaded,
        status,
        error: None,
        started_at: None,
        completed_at: None,
        duration_ms: None,
    }
}

/// `turn/started`, as the server sends it.
pub fn turn_started(thread: &str, turn_id: &str) -> BackendEv {
    note(ServerNotification::TurnStarted(TurnStartedNotification {
        thread_id: thread.to_string(),
        turn: turn(turn_id, TurnStatus::InProgress),
    }))
}

/// `item/started` of a user message.
pub fn user_item(thread: &str, turn_id: &str, item: &str, text: &str) -> BackendEv {
    note(ServerNotification::ItemStarted(ItemStartedNotification {
        item: ThreadItem::untimed(ThreadItemPayload::UserMessage {
            id: item.to_string(),
            client_id: None,
            content: vec![UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            }],
        }),
        thread_id: thread.to_string(),
        turn_id: turn_id.to_string(),
        started_at_ms: 0,
    }))
}

/// `item/agentMessage/delta`.
pub fn agent_delta(thread: &str, turn_id: &str, item: &str, delta: &str) -> BackendEv {
    note(ServerNotification::AgentMessageDelta(
        AgentMessageDeltaNotification {
            thread_id: thread.to_string(),
            turn_id: turn_id.to_string(),
            item_id: item.to_string(),
            delta: delta.to_string(),
        },
    ))
}

/// `item/completed` of an agent message.
pub fn agent_completed(thread: &str, turn_id: &str, item: &str, text: &str) -> BackendEv {
    note(ServerNotification::ItemCompleted(
        ItemCompletedNotification {
            item: ThreadItem::untimed(ThreadItemPayload::AgentMessage {
                moderation: None,
                id: item.to_string(),
                text: text.to_string(),
                phase: None,
                memory_citation: None,
                delivery: None,
                questions: None,
            }),
            thread_id: thread.to_string(),
            turn_id: turn_id.to_string(),
            content_token_count: None,
            completed_at_ms: 0,
        },
    ))
}

/// `turn/completed`.
pub fn turn_completed(thread: &str, turn_id: &str) -> BackendEv {
    note(ServerNotification::TurnCompleted(
        TurnCompletedNotification {
            thread_id: thread.to_string(),
            turn: turn(turn_id, TurnStatus::Completed),
        },
    ))
}

/// `turn/completed` of a failed turn.
pub fn turn_failed(thread: &str, turn_id: &str) -> BackendEv {
    note(ServerNotification::TurnCompleted(
        TurnCompletedNotification {
            thread_id: thread.to_string(),
            turn: turn(turn_id, TurnStatus::Failed),
        },
    ))
}

/// The server's `error` notification: why a turn goes wrong.
pub fn turn_error(thread: &str, turn_id: &str, message: &str, will_retry: bool) -> BackendEv {
    note(ServerNotification::Error(
        codex_app_server_protocol::ErrorNotification {
            error: codex_app_server_protocol::TurnError {
                message: message.to_string(),
                codex_error_info: None,
                additional_details: None,
                misalignment: None,
            },
            will_retry,
            thread_id: thread.to_string(),
            turn_id: turn_id.to_string(),
        },
    ))
}

/// A `item/tool/requestUserInput` server request, the simplest kind that
/// carries a thread id.
/// `serverRequest/resolved`, as the server sends it when a request is
/// answered or withdrawn.
pub fn server_request_resolved(thread: &str, request_id: i64) -> BackendEv {
    note(ServerNotification::ServerRequestResolved(
        codex_app_server_protocol::ServerRequestResolvedNotification {
            thread_id: thread.to_string(),
            request_id: RequestId::Integer(request_id),
        },
    ))
}

pub fn tool_input_request(thread: &str, turn_id: &str, request_id: i64) -> BackendEv {
    BackendEv::Request(Box::new(ServerRequest::ToolRequestUserInput {
        request_id: RequestId::Integer(request_id),
        params: ToolRequestUserInputParams {
            thread_id: thread.to_string(),
            turn_id: turn_id.to_string(),
            item_id: "tool-1".to_string(),
            questions: Vec::new(),
            is_blocking: true,
            auto_resolution_ms: None,
        },
    }))
}

/// A finished turn as `thread/turns/list` returns it: the user's prompt and the agent's answer.
pub fn history_turn(turn_id: &str, prompt: &str, answer: &str) -> Turn {
    let mut turn = turn(turn_id, TurnStatus::Completed);
    turn.items_view = codex_app_server_protocol::TurnItemsView::Full;
    turn.items = vec![
        ThreadItem::untimed(ThreadItemPayload::UserMessage {
            id: [turn_id, "-user"].concat(),
            client_id: None,
            content: vec![UserInput::Text {
                text: prompt.to_string(),
                text_elements: Vec::new(),
            }],
        }),
        ThreadItem::untimed(ThreadItemPayload::AgentMessage {
            moderation: None,
            id: [turn_id, "-agent"].concat(),
            text: answer.to_string(),
            phase: None,
            memory_citation: None,
            delivery: None,
            questions: None,
        }),
    ];
    turn
}

/// The result of `thread/turns/list` (a `reply` step's `result`): `turns` in the order the server
/// sends the page, `next_cursor` the continuation (none on the last page).
pub fn turns_page(
    turns: Vec<Turn>,
    next_cursor: Option<&str>,
    backwards_cursor: Option<&str>,
) -> serde_json::Value {
    serde_json::to_value(codex_app_server_protocol::ThreadTurnsListResponse {
        data: turns,
        next_cursor: next_cursor.map(str::to_string),
        backwards_cursor: backwards_cursor.map(str::to_string),
    })
    .unwrap_or(serde_json::Value::Null)
}

/// `item/completed` of a reasoning item whose summary is `summary` (one part, Markdown): the
/// model's reasoning, which only the detailed transcript shows.
pub fn reasoning_completed(
    thread: &str,
    turn_id: &str,
    item: &str,
    summary: &str,
) -> Result<BackendEv, String> {
    wire(
        "item/completed",
        serde_json::json!({
            "item": {"type": "reasoning", "id": item, "summary": [summary], "content": []},
            "threadId": thread, "turnId": turn_id, "completedAtMs": 0,
        }),
    )
}

/// `item/completed` of a proposed plan (`text`, Markdown).
pub fn plan_completed(
    thread: &str,
    turn_id: &str,
    item: &str,
    text: &str,
) -> Result<BackendEv, String> {
    wire(
        "item/completed",
        serde_json::json!({
            "item": {"type": "plan", "id": item, "text": text},
            "threadId": thread, "turnId": turn_id, "completedAtMs": 0,
        }),
    )
}

/// `turn/plan/updated`: the `update_plan` report, each step with its status (`pending`,
/// `inProgress`, `completed`).
pub fn plan_updated(
    thread: &str,
    turn_id: &str,
    explanation: Option<&str>,
    steps: &[(&str, &str)],
) -> Result<BackendEv, String> {
    let plan: Vec<_> = steps
        .iter()
        .map(|(step, status)| serde_json::json!({"step": step, "status": status}))
        .collect();
    wire(
        "turn/plan/updated",
        serde_json::json!({
            "threadId": thread, "turnId": turn_id,
            "explanation": explanation, "plan": plan,
        }),
    )
}

/// `item/started` of a command that is running (no output, no exit code yet).
pub fn command_started(
    thread: &str,
    turn_id: &str,
    item: &str,
    command: &str,
) -> Result<BackendEv, String> {
    wire(
        "item/started",
        serde_json::json!({
            "item": {
                "type": "commandExecution", "id": item, "command": command, "cwd": "/workspace",
                "processId": null, "source": "agent", "status": "inProgress",
                "commandActions": [], "aggregatedOutput": null, "exitCode": null,
                "durationMs": null,
            },
            "threadId": thread, "turnId": turn_id, "startedAtMs": 0,
        }),
    )
}

/// `item/commandExecution/outputDelta`: more of a running command's output.
pub fn command_output(
    thread: &str,
    turn_id: &str,
    item: &str,
    delta: &str,
) -> Result<BackendEv, String> {
    wire(
        "item/commandExecution/outputDelta",
        serde_json::json!({
            "threadId": thread, "turnId": turn_id, "itemId": item, "delta": delta,
        }),
    )
}

/// `item/completed` of a command that still runs (`inProgress`) with `output` as the output it has
/// now: the server's way to say the whole text of a running command's output, in place of what it
/// had (`item/started` of an item the transcript has does nothing; an upsert replaces it).
pub fn command_progress(
    thread: &str,
    turn_id: &str,
    item: &str,
    command: &str,
    output: &str,
) -> Result<BackendEv, String> {
    wire(
        "item/completed",
        serde_json::json!({
            "item": {
                "type": "commandExecution", "id": item, "command": command, "cwd": "/workspace",
                "processId": null, "source": "agent", "status": "inProgress",
                "commandActions": [], "aggregatedOutput": output, "exitCode": null,
                "durationMs": null,
            },
            "threadId": thread, "turnId": turn_id, "completedAtMs": 0,
        }),
    )
}

/// `item/completed` of a command: its `output` (stdout and stderr as one text) and its exit code.
pub fn command_completed(
    thread: &str,
    turn_id: &str,
    item: &str,
    command: &str,
    output: &str,
    exit_code: i32,
) -> Result<BackendEv, String> {
    let status = if exit_code == 0 {
        "completed"
    } else {
        "failed"
    };
    wire(
        "item/completed",
        serde_json::json!({
            "item": {
                "type": "commandExecution", "id": item, "command": command, "cwd": "/workspace",
                "processId": null, "source": "agent", "status": status,
                "commandActions": [], "aggregatedOutput": output, "exitCode": exit_code,
                "durationMs": null,
            },
            "threadId": thread, "turnId": turn_id, "completedAtMs": 0,
        }),
    )
}
