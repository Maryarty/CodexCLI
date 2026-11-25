//! The mapping between our commands and the app-server protocol: the one
//! place a `Command` becomes a `ClientRequest` and a JSON result becomes a
//! `Reply`. The shell and the host send what `client_request` makes and feed
//! what came back to `reply`; a screen's own RPCs go through its `wire` and
//! `decode` in `screens/<dir>.rs`. Server notifications and requests map to
//! events in `threads::project` and `app::pending_request`.

pub(crate) mod agents;
pub mod config;
pub mod import;
pub mod onboarding;
pub(crate) mod requests;
mod rpc_list;
pub(crate) mod targets;

use codex_app_server_protocol::AddCreditsNudgeCreditType;
use codex_app_server_protocol::AddCreditsNudgeEmailStatus;
use codex_app_server_protocol::ByteRange;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::GetAccountRateLimitsResponse;
use codex_app_server_protocol::ImageReference;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::SendAddCreditsNudgeEmailParams;
use codex_app_server_protocol::SendAddCreditsNudgeEmailResponse;
use codex_app_server_protocol::TextElement;
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
use crate::threads::ToolMention;
use crate::threads::UserMessage;
use std::path::PathBuf;

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
        Command::SendAddCreditsNudge { nudge, .. } => {
            Some(ClientRequest::SendAddCreditsNudgeEmail {
                request_id,
                params: SendAddCreditsNudgeEmailParams {
                    credit_type: match nudge {
                        screens::chat::Nudge::Credits => AddCreditsNudgeCreditType::Credits,
                        screens::chat::Nudge::UsageLimit => AddCreditsNudgeCreditType::UsageLimit,
                    },
                },
            })
        }
        Command::StartThread { model: chosen, .. } => Some(ClientRequest::ThreadStart {
            request_id,
            params: ThreadStartParams {
                model: chosen.clone().or_else(|| model.map(str::to_string)),
                ..ThreadStartParams::default()
            },
        }),
        Command::StartTurn {
            thread,
            message,
            misalignment_override,
            ..
        } => Some(ClientRequest::TurnStart {
            request_id,
            params: TurnStartParams {
                thread_id: thread.to_string(),
                input: turn_input(message),
                responsesapi_client_metadata: misalignment_override.map(|timestamp| {
                    std::collections::HashMap::from([(
                        "misalignment_override".to_owned(),
                        serde_json::json!({ "timestamp": timestamp }).to_string(),
                    )])
                }),
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
        Command::Feature {
            thread, command, ..
        } => Some(crate::features::wire(command, thread, request_id)),
        Command::RejectRequest { .. }
        | Command::ResolveRequest { .. }
        | Command::CreateWorktree { .. }
        | Command::BindWorktree { .. }
        | Command::Directories { .. }
        | Command::SaveGrouping { .. }
        | Command::Startup { .. }
        | Command::OpenUrl { .. }
        | Command::Import { .. }
        | Command::Popovers { .. }
        | Command::Usage { .. }
        | Command::Effect { .. }
        | Command::VerifyUser { .. }
        | Command::ReconnectIdleThread { .. } => None,
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
                    let reserve = crate::screens::chat::Reserve::of(&response);
                    let read = crate::screens::status::RateRead::from_response(&response);
                    let plan = response.rate_limits.plan_type;
                    let banner = response.rate_limit_upsell.and_then(|raw| {
                        crate::screens::chat::Banner::parse(raw, response.account_id, plan)
                    });
                    let limits = response
                        .rate_limits_by_limit_id
                        .and_then(|mut limits| limits.remove("codex"))
                        .unwrap_or(response.rate_limits);
                    Reply::account_rate_limits(limits.into(), banner, reserve, read)
                }
                Err(error) => Reply::error(error),
            },
        )),
        Command::SendAddCreditsNudge { ticket, .. } => Some((
            *ticket,
            match decode::<SendAddCreditsNudgeEmailResponse>(
                "account/sendAddCreditsNudgeEmail",
                result,
            ) {
                Ok(response) => Reply::add_credits_nudge(match response.status {
                    AddCreditsNudgeEmailStatus::Sent => screens::chat::NudgeOutcome::Sent,
                    AddCreditsNudgeEmailStatus::CooldownActive => {
                        screens::chat::NudgeOutcome::Cooldown
                    }
                }),
                // a failed request is told as the outcome, not as an error row
                Err(_) => Reply::add_credits_nudge(screens::chat::NudgeOutcome::Failed),
            },
        )),
        Command::StartThread { ticket, .. } => Some((
            *ticket,
            match decode::<ThreadStartResponse>("thread/start", result) {
                Ok(response) => {
                    // The oracle's split (`compose_agents_summary`): a path in
                    // the host's own convention is shown relative to the
                    // directory, any other as the server spells it.
                    let sources = response
                        .instruction_source_path_uris()
                        .into_iter()
                        .map(|uri| match uri.to_abs_path() {
                            Ok(path) => screens::status::AgentsSource::Native(
                                path.as_path().display().to_string(),
                            ),
                            Err(_) => screens::status::AgentsSource::Foreign(
                                uri.inferred_native_path_string(),
                            ),
                        })
                        .collect();
                    Reply::thread_started_with(ThreadId::new(response.thread.id), sources)
                }
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
        Command::Feature {
            ticket, command, ..
        } => Some((
            *ticket,
            match result {
                Ok(json) => match crate::features::decode(command, json) {
                    Ok(reply) => Reply::feature(reply),
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
        | Command::OpenUrl { .. }
        | Command::Import { .. }
        | Command::Popovers { .. }
        | Command::Usage { .. }
        | Command::Effect { .. }
        | Command::VerifyUser { .. }
        | Command::ReconnectIdleThread { .. } => None,
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
    // A turn or a steer the server refuses for the chat's model provider keeps
    // the typed failure: the chat is read-only and says why (oracle `show_provider_error`).
    if let Err(error) = &result
        && let Some(provider) = error.model_provider()
    {
        match command {
            Command::StartTurn { ticket, .. }
            | Command::Screen {
                ticket,
                command: screens::ScreenCommand::Chat(screens::chat::Command::Steer { .. }),
                ..
            }
            // the resume of a thread whose provider is refused opens it read-only
            | Command::Screen {
                ticket,
                command:
                    screens::ScreenCommand::Agents(screens::agents::Command::Thread(
                        crate::threads::ThreadAsk::Resume { .. }
                        | crate::threads::ThreadAsk::ResumeAt { .. },
                    )),
                ..
            } => return Some((*ticket, Reply::error(RpcError::Provider(provider)))),
            _ => {}
        }
    }
    // A refusal that is an answer of a feature's ask (the feature says which).
    if let (
        Command::Feature {
            ticket,
            command: asked,
            ..
        },
        Err(error),
    ) = (command, &result)
        && let Some(answer) = crate::features::refusal(asked, error)
    {
        return Some((*ticket, Reply::feature(answer)));
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

/// The input of a turn (`input_submission.rs`): the remote images first, then the local ones,
/// then the text with a text element for each local image's placeholder.
pub(crate) fn turn_input(message: &UserMessage) -> Vec<UserInput> {
    let mut input: Vec<UserInput> = message
        .remote_images
        .iter()
        .map(|url| UserInput::Image {
            image: ImageReference::Inline { url: url.clone() },
            detail: None,
        })
        .collect();
    input.extend(message.images.iter().map(|image| UserInput::LocalImage {
        path: image.path.clone(),
        detail: None,
    }));
    if !message.text.is_empty() {
        let mut elements: Vec<TextElement> = message
            .images
            .iter()
            .filter_map(|image| {
                let placeholder = message.text.get(image.range.clone())?;
                Some(TextElement::new(
                    ByteRange {
                        start: image.range.start,
                        end: image.range.end,
                    },
                    Some(placeholder.to_string()),
                ))
            })
            .collect();
        elements.sort_by_key(|element| element.byte_range.start);
        input.push(UserInput::Text {
            text: message.text.clone(),
            text_elements: elements,
        });
    }
    // the tools the text names, after it (`input_submission.rs`: skills, then plugins and apps)
    input.extend(message.mentions.iter().map(|mention| match mention {
        ToolMention::Skill { name, path } => UserInput::Skill {
            name: name.clone(),
            path: PathBuf::from(path),
        },
        ToolMention::Mention { name, path } => UserInput::Mention {
            name: name.clone(),
            path: path.clone(),
        },
    }));
    input
}
