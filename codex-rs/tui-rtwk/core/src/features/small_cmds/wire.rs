//! The RPC of `/logout` and `/daybreak`: `account/logout`, and a thread's Daybreak choice by
//! `thread/metadata/update`, then the default preference in the config file (oracle `AppServerSession::logout_account`, `persist_daybreak_selection`).

use codex_app_server_protocol::{ClientRequest, RequestId, ThreadMetadataUpdateParams};

use crate::app::effects::ConfigEdit;
use crate::ids::ThreadId;
use crate::protocol::config::{config_write, config_written};
use crate::rpc::RpcError;

/// What the slice asks the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `account/logout`.
    Logout,
    /// `thread/metadata/update` with the thread's Daybreak choice.
    SetDaybreak(bool),
    /// `config/batchWrite` of the default Daybreak preference.
    SaveDefault(bool),
}

/// What the server answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// Done: nothing the slice reads.
    Done,
}

pub(super) fn request(
    command: &Command,
    thread: &ThreadId,
    request_id: RequestId,
) -> ClientRequest {
    match command {
        Command::Logout => ClientRequest::LogoutAccount {
            request_id,
            params: None,
        },
        Command::SetDaybreak(enabled) => ClientRequest::ThreadMetadataUpdate {
            request_id,
            params: ThreadMetadataUpdateParams {
                thread_id: thread.to_string(),
                daybreak_enabled: Some(*enabled),
                project_id: None,
                git_info: None,
            },
        },
        Command::SaveDefault(enabled) => config_write(
            &[ConfigEdit {
                key_path: "daybreak".to_owned(),
                value: serde_json::json!(enabled),
            }],
            request_id,
        ),
    }
}

pub(super) fn reply(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    match command {
        Command::SaveDefault(_) => config_written(result)
            .map(|()| Reply::Done)
            .map_err(RpcError::Malformed),
        Command::Logout | Command::SetDaybreak(_) => Ok(Reply::Done),
    }
}
