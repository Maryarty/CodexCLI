//! Our RPCs as the shell sends them: core's `wire` makes the request and
//! reads the result; this adds what is not core's to know. A page of the
//! list or of a history search is one request per source, merged
//! (`Fanout`); the usage fetch gives up after the oracle's timeout.
//! The client is a closure, so the host's real one and the fake's are the
//! same code. Oracle: `agents_overview_discovery.rs`,
//! `agents_overview_usage.rs`, `background_requests.rs`.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_tui_core::app::Command;
use codex_tui_core::ids::Ticket;
use codex_tui_core::protocol;
use codex_tui_core::rpc::Reply;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenCommand;
use codex_tui_core::screens::ScreenReply;
use codex_tui_core::screens::agents::Command as AgentsCommand;
use codex_tui_core::screens::agents::PinReader;
use codex_tui_core::screens::agents::fanout;

use crate::services::agents::workspace::Facts;

/// How long the usage fetch waits (oracle: `THREAD_USAGE_FETCH_TIMEOUT`).
pub const USAGE_TIMEOUT: Duration = Duration::from_secs(65);

/// The text of a failed usage fetch that timed out.
pub const USAGE_TIMED_OUT: &str = "dashboard usage request timed out";

/// Request ids, shared by every request of a connection.
#[derive(Clone, Debug, Default)]
pub struct Ids(Arc<AtomicI64>);

impl Ids {
    pub fn next(&self) -> RequestId {
        RequestId::Integer(self.0.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

/// The reply of `command` through `send`; `None` for a command that is not
/// an RPC. `model` is the `--model` of a new thread.
pub async fn answer<F, Fut>(
    command: Command,
    facts: Facts,
    ids: Ids,
    model: Option<String>,
    send: F,
) -> Option<(Ticket, Reply)>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    answer_with(command, facts, ids, model, USAGE_TIMEOUT, send).await
}

/// As `answer`, with the usage timeout given (tests).
pub async fn answer_with<F, Fut>(
    command: Command,
    facts: Facts,
    ids: Ids,
    model: Option<String>,
    usage_timeout: Duration,
    send: F,
) -> Option<(Ticket, Reply)>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    // A page of the list or of a history search: one request per source,
    // merged by core.
    if let Command::Screen {
        ticket,
        command: ScreenCommand::Agents(page),
        ..
    } = &command
        && let Some(fanout) = fanout(page, facts.cloud)
    {
        let requests = fanout.requests(|| ids.next());
        // The first page of the list also reads the pinned tasks, page after
        // page; the two reads go out together.
        let pins = async {
            if !fanout.reads_pins() {
                return None;
            }
            let mut reader = PinReader::new();
            while let Some(request) = reader.next_request(ids.next()) {
                reader.feed(send(request).await);
            }
            reader.finish()
        };
        let (results, pins) = futures::join!(
            futures::future::join_all(requests.into_iter().map(&send)),
            pins
        );
        let reply = match fanout.reply(results, pins) {
            Ok(reply) => Reply::screen(ScreenReply::Agents(reply)),
            Err(error) => Reply::error(error),
        };
        return Some((*ticket, reply));
    }
    let request = protocol::client_request(&command, ids.next(), model.as_deref())?;
    let usage = matches!(
        &command,
        Command::Screen {
            command: ScreenCommand::Agents(AgentsCommand::Usage { .. }),
            ..
        }
    );
    let result = if usage {
        match tokio::time::timeout(usage_timeout, send(request)).await {
            Ok(result) => result,
            Err(_) => Err(ServerError::from(USAGE_TIMED_OUT)),
        }
    } else {
        send(request).await
    };
    protocol::reply(&command, result.map_err(|error| error.message))
}
