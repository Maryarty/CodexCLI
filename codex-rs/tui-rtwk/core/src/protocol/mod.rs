//! The mapping between our commands and the app-server protocol: the one
//! place a `Command` becomes a `ClientRequest` and a JSON result becomes a
//! `Reply`. The shell and the host send what `client_request` makes and feed
//! what came back to `reply`; a screen's own RPCs go through its `wire` and
//! `decode` in `screens/<dir>.rs`. Server notifications and requests map to
//! events in `threads::project` and `app::pending_request`.

pub(crate) mod agents;
pub mod import;
pub mod onboarding;
pub(crate) mod requests;
pub(crate) mod targets;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GetAccountRateLimitsResponse;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnInterruptResponse;
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
use crate::rpc::ServerError;
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
        Command::ReadRateLimits { .. } => Some(ClientRequest::GetAccountRateLimits {
            request_id,
            params: None,
        }),
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
        Command::InterruptTurn { thread, turn, .. } => Some(
            crate::threads::ThreadAsk::Interrupt {
                thread: thread.to_string(),
                turn: turn.to_string(),
            }
            .wire(request_id),
        ),
        Command::Screen {
            thread, command, ..
        } => Some(screens::wire(command, thread, request_id)),
        Command::RejectRequest { .. }
        | Command::ResolveRequest { .. }
        | Command::CreateWorktree { .. }
        | Command::BindWorktree { .. }
        | Command::Directories { .. }
        | Command::SaveGrouping { .. }
        | Command::Startup { .. }
        | Command::Import { .. } => None,
    }
}

/// The ticket and reply for `command` out of the request's JSON `result`
/// (or the failure text of the client). `None` for `RejectRequest`.
pub fn reply(
    command: &Command,
    result: Result<serde_json::Value, String>,
) -> Option<(Ticket, Reply)> {
    match command {
        Command::ReadRateLimits { ticket, .. } => Some((
            *ticket,
            match decode::<GetAccountRateLimitsResponse>("account/rateLimits/read", result) {
                Ok(response) => {
                    let plan = response.rate_limits.plan_type;
                    let banner = response.rate_limit_upsell.and_then(|raw| {
                        crate::screens::chat::Banner::parse(raw, response.account_id, plan)
                    });
                    let limits = response
                        .rate_limits_by_limit_id
                        .and_then(|mut limits| limits.remove("codex"))
                        .unwrap_or(response.rate_limits);
                    Reply::account_rate_limits(limits.into(), banner)
                }
                Err(error) => Reply::error(error),
            },
        )),
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
        Command::InterruptTurn { ticket, .. } => Some((
            *ticket,
            match decode::<TurnInterruptResponse>("turn/interrupt", result) {
                Ok(_) => Reply::turn_interrupted(),
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
        | Command::ResolveRequest { .. }
        | Command::CreateWorktree { .. }
        | Command::BindWorktree { .. }
        | Command::Directories { .. }
        | Command::SaveGrouping { .. }
        | Command::Startup { .. }
        | Command::Import { .. } => None,
    }
}

/// Preserve a typed server refusal for the interrupt race before formatting other errors.
pub fn reply_from_server(
    command: &Command,
    result: Result<serde_json::Value, ServerError>,
) -> Option<(Ticket, Reply)> {
    if let (Command::InterruptTurn { ticket, turn, .. }, Err(error)) = (command, &result)
        && error.code == Some(-32602)
        && let Some((expected, actual)) = error
            .message
            .strip_prefix("expected active turn id ")
            .and_then(|message| message.split_once(" but found "))
        && expected.trim_matches('`') == turn.as_str()
    {
        let actual = actual.trim_matches('`');
        if !actual.is_empty() && !actual.contains(char::is_whitespace) && actual != turn.as_str() {
            return Some((*ticket, Reply::interrupt_turn_changed(TurnId::new(actual))));
        }
    }
    reply(command, result.map_err(|error| error.message))
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
