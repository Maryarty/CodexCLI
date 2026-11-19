//! The mapping between our commands and the app-server protocol: the one
//! place a `Command` becomes a `ClientRequest` and a JSON result becomes a
//! `Reply`. The shell and the host send what `client_request` makes and feed
//! what came back to `reply`; a screen's own RPCs go through its `wire` and
//! `decode` in `screens/<dir>.rs`. Server notifications and requests map to
//! events in `threads::project` and `app::pending_request`.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;

use crate::app::Command;
use crate::ids::RequestKey;
use crate::ids::ThreadId;
use crate::ids::Ticket;
use crate::ids::TurnId;
use crate::rpc::Reply;
use crate::rpc::RpcError;
use crate::screens;

/// The wire request of `command`, under `request_id`. `model` is the
/// `--model` override for a new thread. `None` for `RejectRequest`, which is
/// an answer to the server, not a request (`reject_target`).
pub fn client_request(
    command: &Command,
    request_id: RequestId,
    model: Option<&str>,
) -> Option<ClientRequest> {
    match command {
        Command::StartThread { .. } => Some(ClientRequest::ThreadStart {
            request_id,
            params: ThreadStartParams {
                model: model.map(str::to_string),
                ..ThreadStartParams::default()
            },
        }),
        Command::StartTurn { thread, text, .. } => Some(ClientRequest::TurnStart {
            request_id,
            params: TurnStartParams {
                thread_id: thread.to_string(),
                input: vec![UserInput::Text {
                    text: text.clone(),
                    text_elements: Vec::new(),
                }],
                ..TurnStartParams::default()
            },
        }),
        Command::Screen {
            thread, command, ..
        } => Some(screens::wire(command, thread, request_id)),
        Command::RejectRequest { .. }
        | Command::Worktree { .. }
        | Command::Directories { .. }
        | Command::SaveGrouping { .. } => None,
    }
}

/// The ticket and reply for `command` out of the request's JSON `result`
/// (or the failure text of the client). `None` for `RejectRequest`.
pub fn reply(
    command: &Command,
    result: Result<serde_json::Value, String>,
) -> Option<(Ticket, Reply)> {
    match command {
        Command::StartThread { ticket, .. } => Some((
            *ticket,
            match decode::<ThreadStartResponse>("thread/start", result) {
                Ok(response) => Reply::thread_started(ThreadId::new(response.thread.id)),
                Err(error) => Reply::error(error),
            },
        )),
        Command::StartTurn { ticket, thread, .. } => Some((
            *ticket,
            match decode::<TurnStartResponse>("turn/start", result) {
                Ok(response) => Reply::turn_started(thread.clone(), TurnId::new(response.turn.id)),
                Err(error) => Reply::error(error),
            },
        )),
        Command::Screen {
            ticket, command, ..
        } => Some((
            *ticket,
            match result {
                Ok(json) => match screens::decode(command, json) {
                    Ok(reply) => Reply::screen(reply),
                    Err(error) => Reply::error(error),
                },
                Err(message) => Reply::failed(message),
            },
        )),
        Command::RejectRequest { .. }
        | Command::Worktree { .. }
        | Command::Directories { .. }
        | Command::SaveGrouping { .. } => None,
    }
}

fn decode<T: serde::de::DeserializeOwned>(
    method: &str,
    result: Result<serde_json::Value, String>,
) -> Result<T, RpcError> {
    let json = result.map_err(|error| RpcError::Remote(format!("{method}: {error}")))?;
    serde_json::from_value(json).map_err(|error| RpcError::Malformed(format!("{method}: {error}")))
}

/// The JSON-RPC id of the server request `key` stands for, to answer it.
pub fn reject_target(key: &RequestKey) -> Option<RequestId> {
    serde_json::from_str::<RequestId>(key.id.as_str()).ok()
}

/// The error a dismissed request is rejected with.
pub fn dismissal() -> JSONRPCErrorError {
    JSONRPCErrorError {
        code: -32000,
        data: None,
        message: "dismissed by the user".to_string(),
    }
}
