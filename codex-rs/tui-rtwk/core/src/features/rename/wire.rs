//! The RPC of `/rename`: `thread/name/set` of the thread the ask is about (oracle
//! `app/thread_title.rs`).

use codex_app_server_protocol::{
    ClientRequest, RequestId, ThreadSetNameParams, ThreadSetNameResponse,
};

use crate::ids::ThreadId;
use crate::rpc::RpcError;

/// The one ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `thread/name/set`.
    SetName(String),
}

/// What it answered with: nothing the feature reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Ack,
}

pub(super) fn request(
    command: &Command,
    thread: &ThreadId,
    request_id: RequestId,
) -> ClientRequest {
    match command {
        Command::SetName(name) => ClientRequest::ThreadSetName {
            request_id,
            params: ThreadSetNameParams {
                thread_id: thread.to_string(),
                name: name.clone(),
            },
        },
    }
}

pub(super) fn reply(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    match command {
        Command::SetName(_) => serde_json::from_value::<ThreadSetNameResponse>(result)
            .map(|_| Reply::Ack)
            .map_err(|error| RpcError::Malformed(error.to_string())),
    }
}
