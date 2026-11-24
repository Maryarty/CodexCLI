//! The small commands' asks (`Ask::Logout`, `Ask::DaybreakSave`, `Ask::IdeContext`,
//! `Ask::DebugConfig`): what the oracle's `event_dispatch` `Logout`, `daybreak.rs` and `/ide` do
//! over the app server, answered as plain data the slice words (`features/small_cmds`).

use std::future::Future;

use codex_app_server_protocol::{
    ClientRequest, ThreadMetadataUpdateParams, ThreadReadParams,
};
use codex_tui_core::app::effects::{ConfigEdit, Done};
use codex_tui_core::features::small_cmds::{
    DaybreakFailure, DaybreakSaved, DefaultFailure, IdeFailure,
};
use codex_tui_core::protocol::config::{config_write, config_written};
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;

use crate::startup::Startup;

const OPEN_IDE_HINT: &str = "Open this project in VS Code or Cursor with the Codex extension active.";

/// `Ask::Logout`: `account/logout`.
pub(crate) async fn logout<F, Fut>(ids: &Ids, send: F) -> Done
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let request = ClientRequest::LogoutAccount {
        request_id: ids.next(),
        params: None,
    };
    Done::LoggedOut(send(request).await.map(|_| ()).map_err(|e| e.message))
}

/// `Ask::DaybreakSave` (`persist_daybreak_selection`): the thread's choice goes to its metadata
/// unless it is ephemeral, then the default preference to the config file. The read of the
/// effective config that follows only feeds the TUI's own copy of the default, which the model
/// does not keep: no `DefaultFailure::Read` comes from here.
pub(crate) async fn daybreak<F, Fut>(
    thread: &str,
    enabled: bool,
    ids: &Ids,
    send: F,
) -> Done
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let read = ClientRequest::ThreadRead {
        request_id: ids.next(),
        params: ThreadReadParams {
            thread_id: thread.to_owned(),
            include_turns: false,
        },
    };
    let ephemeral = match send(read).await {
        Ok(value) => value
            .pointer("/thread/ephemeral")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        Err(error) => return Done::Daybreak(Err(DaybreakFailure::Read(error.message))),
    };
    if !ephemeral {
        let update = ClientRequest::ThreadMetadataUpdate {
            request_id: ids.next(),
            params: ThreadMetadataUpdateParams {
                thread_id: thread.to_owned(),
                daybreak_enabled: Some(enabled),
                project_id: None,
                git_info: None,
            },
        };
        if let Err(error) = send(update).await {
            return Done::Daybreak(Err(DaybreakFailure::Save(error.message)));
        }
    }
    let edits = [ConfigEdit {
        key_path: "daybreak".to_owned(),
        value: serde_json::json!(enabled),
    }];
    let defaults = match send(config_write(&edits, ids.next())).await {
        Ok(result) => config_written(result)
            .err()
            .map(DefaultFailure::Save)
            .into_iter()
            .collect(),
        Err(error) => vec![DefaultFailure::Save(error.message)],
    };
    Done::Daybreak(Ok(DaybreakSaved { defaults }))
}

/// `Ask::IdeContext`: the IDE's ipc socket is not carried yet, so no IDE answers: the oracle's
/// hint for an IDE that is not connected (`IdeContextError::Connect`).
pub(crate) fn ide() -> Done {
    Done::Ide(Err(IdeFailure {
        hint: OPEN_IDE_HINT.to_owned(),
        skip_hint: OPEN_IDE_HINT.to_owned(),
    }))
}

/// `Ask::DebugConfig`: the layers and requirements of the config in force.
pub(crate) async fn debug_config(startup: &Startup) -> Done {
    let config = startup.current.read().await.clone();
    Done::DebugConfig(crate::debug_config::rows(&config))
}
