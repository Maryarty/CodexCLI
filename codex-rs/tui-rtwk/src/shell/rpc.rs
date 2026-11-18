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
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_tui_core::app::Command;
use codex_tui_core::ids::Ticket;
use codex_tui_core::rpc::Reply;

use super::workspace::Facts;

/// How long the usage fetch waits (oracle: `THREAD_USAGE_FETCH_TIMEOUT`).
pub const USAGE_TIMEOUT: Duration = Duration::from_secs(65);

/// The text of a failed usage fetch that timed out.
pub const USAGE_TIMED_OUT: &str = "dashboard usage request timed out";

/// Request ids, shared by every request of a connection.
#[derive(Clone, Debug, Default)]
pub struct Ids(Arc<AtomicI64>);

impl Ids {
    pub fn next(&self) -> RequestId {
        todo!("ids")
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
    Fut: Future<Output = Result<serde_json::Value, String>>,
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
    Fut: Future<Output = Result<serde_json::Value, String>>,
{
    let _ = (command, facts, ids, model, usage_timeout, send);
    todo!("answer")
}
