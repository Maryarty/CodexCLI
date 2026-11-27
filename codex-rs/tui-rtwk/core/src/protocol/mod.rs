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
use codex_app_server_protocol::UserInput;

use crate::app::Command;
use crate::ids::RequestKey;
use crate::ids::Ticket;
use crate::rpc::Reply;
use crate::rpc::RpcError;
use crate::rpc::ServerError;
use crate::screens;
use crate::threads::ToolMention;
use crate::threads::UserMessage;
use crate::threads::asks::{ThreadRpc, changed_turn};

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
        Command::StartThread { .. } | Command::StartTurn { .. } | Command::InterruptTurn { .. } => {
            lifecycle_ask(command).map(|(_, _, ask)| ask.request(model, request_id))
        }
        Command::Screen {
            thread, command, ..
        } => Some(screens::wire(command, thread, request_id)),
        Command::Feature {
            thread, command, ..
        } => Some(crate::features::wire(command, thread, request_id)),
        Command::RejectRequest { .. } | Command::ResolveRequest { .. } | Command::Effect { .. } => {
            None
        }
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
        Command::StartThread { .. } | Command::StartTurn { .. } | Command::InterruptTurn { .. } => {
            lifecycle_ask(command).map(|(ticket, method, ask)| {
                let reply = match result {
                    Ok(json) => ask.reply(json).map_err(|error| match error {
                        RpcError::Malformed(error) => {
                            RpcError::Malformed(format!("{method}: {error}"))
                        }
                        error => error,
                    }),
                    Err(message) => Err(RpcError::Remote(format!("{method}: {message}"))),
                };
                (ticket, reply.map_or_else(Reply::error, Reply::thread))
            })
        }
        Command::Screen {
            ticket,
            command: screens::ScreenCommand::Agents(screens::agents::Command::Thread(ask)),
            ..
        } => Some((
            *ticket,
            match result {
                Ok(json) => {
                    agents::decode_thread(ask, json).map_or_else(Reply::error, Reply::thread)
                }
                Err(message) => Reply::failed(message),
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
        Command::RejectRequest { .. } | Command::ResolveRequest { .. } | Command::Effect { .. } => {
            None
        }
    }
}

/// Preserve server refusals that prove the outcome of a request before formatting other errors.
pub fn reply_from_server(
    command: &Command,
    result: Result<serde_json::Value, ServerError>,
) -> Option<(Ticket, Reply)> {
    if let (Command::InterruptTurn { ticket, turn, .. }, Err(error)) = (command, &result)
        && let Some(reply) = changed_turn(turn, error)
    {
        return Some((*ticket, Reply::thread(reply)));
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
    // The JSON-RPC request itself was rejected, so no turn can have been admitted.
    if let (Command::StartTurn { ticket, .. }, Err(error)) = (command, &result)
        && error.refused()
    {
        return Some((
            *ticket,
            Reply::start_rejected(format!("turn/start: {error}")),
        ));
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

/// The app's launch and input effects are projected onto the same catalog as
/// requests from a screen. The input can stay borrowed for request construction.
fn lifecycle_ask(command: &Command) -> Option<(Ticket, &'static str, ThreadRpc<&UserMessage>)> {
    Some(match command {
        Command::StartThread {
            ticket, model, cwd, ..
        } => (
            *ticket,
            "thread/start",
            ThreadRpc::StartSession {
                model: model.clone(),
                cwd: cwd.clone(),
            },
        ),
        Command::StartTurn {
            ticket,
            thread,
            message,
            client_id,
            misalignment_override,
            ide_prompt,
        } => (
            *ticket,
            "turn/start",
            ThreadRpc::StartUserTurn {
                thread: thread.clone(),
                message,
                client_id: client_id.clone(),
                misalignment_override: *misalignment_override,
                ide_prompt: ide_prompt.clone(),
            },
        ),
        Command::InterruptTurn {
            ticket,
            thread,
            turn,
        } => (
            *ticket,
            "turn/interrupt",
            ThreadRpc::Interrupt {
                thread: thread.to_string(),
                turn: turn.to_string(),
            },
        ),
        _ => return None,
    })
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
/// then the text with a text element for each local image's placeholder. The tasks the text
/// names add their context to the text sent.
///
/// `current` is the thread the turn goes to: its own task mention is no reference
/// (`apply_task_references`). Task mentions are always on: the oracle turns them off only for a
/// thread resumed from a directory without the capability file, a state this port has not.
pub(crate) fn turn_input_in(message: &UserMessage, current: Option<&str>) -> Vec<UserInput> {
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
        let image_ranges: Vec<_> = message
            .images
            .iter()
            .map(|image| image.range.clone())
            .collect();
        let context = crate::kit::mentions::with_task_context(
            &message.text,
            &message.tasks,
            current,
            &image_ranges,
        );
        let element = |range: std::ops::Range<usize>, text: &str| {
            let placeholder = text.get(range.clone())?;
            Some(TextElement::new(
                ByteRange {
                    start: range.start,
                    end: range.end,
                },
                Some(placeholder.to_string()),
            ))
        };
        let mut elements: Vec<TextElement> = message
            .images
            .iter()
            .filter_map(|image| {
                let range = match &context {
                    Some(context) => context.map(image.range.clone()),
                    None => image.range.clone(),
                };
                element(range, context.as_ref().map_or(&message.text, |c| &c.text))
            })
            .collect();
        // a task mention is an element too: its link in the text sent (else `@title` as typed),
        // `@title` its placeholder
        let sent = context
            .as_ref()
            .map_or(message.text.as_str(), |c| c.text.as_str());
        for (index, task) in message.tasks.iter().enumerate() {
            let range = context
                .as_ref()
                .map_or_else(|| task.range.clone(), |c| c.tasks[index].clone());
            if sent.get(range.clone()).is_some() {
                elements.push(TextElement::new(
                    ByteRange {
                        start: range.start,
                        end: range.end,
                    },
                    Some(format!("@{}", task.title)),
                ));
            }
        }
        elements.sort_by_key(|element| element.byte_range.start);
        input.push(UserInput::Text {
            text: context.map_or_else(|| message.text.clone(), |context| context.text),
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
