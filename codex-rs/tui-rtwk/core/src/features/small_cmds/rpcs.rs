//! The RPCs of `/logout` and `/daybreak`: `account/logout`, and a thread's Daybreak choice by
//! `thread/metadata/update`, then the default preference in the config file (oracle
//! `AppServerSession::logout_account`, `persist_daybreak_selection`).

use codex_app_server_protocol::{ConfigWriteResponse, ThreadMetadataUpdateParams};

use crate::app::effects::ConfigEdit;
use crate::ids::ThreadId;
use crate::protocol::config::config_write_params;

/// What the server answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// Done: nothing the slice reads.
    Done,
}

crate::rpc_list! {
    /// What the slice asks the server.
    pub enum Command => Reply, request thread: &ThreadId;
    /// `account/logout`.
    Logout, Self::Logout
        => LogoutAccount(None)
        -> serde_json::Value, |_| Reply::Done;
    /// `thread/metadata/update` with the thread's Daybreak choice.
    SetDaybreak(bool), Self::SetDaybreak(enabled)
        => ThreadMetadataUpdate(ThreadMetadataUpdateParams {
            thread_id: thread.to_string(),
            daybreak_enabled: Some(*enabled),
            project_id: None,
            git_info: None,
        })
        -> serde_json::Value, |_| Reply::Done;
    /// `config/batchWrite` of the default Daybreak preference.
    SaveDefault(bool), Self::SaveDefault(enabled)
        => ConfigBatchWrite(config_write_params(&[ConfigEdit {
            key_path: "daybreak".to_owned(),
            value: serde_json::json!(enabled),
        }]))
        -> ConfigWriteResponse, |_| Reply::Done;
}
