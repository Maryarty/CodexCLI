//! The host's answer to `Ask::ConfigWrite` (the oracle's `write_config_batch`): the edits go to
//! the config file through the app server and, once it has them, the config is loaded again so
//! the facts the session reads follow (the feeds say them again, `feeds::Event::ConfigWritten`).

use std::future::Future;
use std::sync::Arc;

use codex_app_server_protocol::ClientRequest;
use codex_core::config::Config;
use codex_tui_core::app::effects::{ConfigEdit, Done};
use codex_tui_core::protocol::config::{config_write, config_written};
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;

use crate::startup::{Startup, Target};

/// How a write of the config file went.
pub(crate) struct Written {
    /// The answer to the ask.
    pub done: Done,
    /// The config as it stands once the file has the edits: loaded again, for the facts that
    /// follow. None: the write failed, the workspace is remote (its config is not this
    /// machine's) or the config could not be loaded again.
    pub config: Option<Arc<Config>>,
}

/// `Ask::ConfigWrite`: `config/batchWrite` of `edits`, then the session's config again.
pub(crate) async fn write<F, Fut>(
    edits: Vec<ConfigEdit>,
    startup: &Startup,
    ids: &Ids,
    send: F,
) -> Written
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let done = match send(config_write(&edits, ids.next())).await {
        Ok(result) => Done::Config(config_written(result)),
        Err(error) => Done::Config(Err(error.message)),
    };
    let config = if matches!(done, Done::Config(Ok(()))) && startup.target != Target::Remote {
        reload(startup).await
    } else {
        None
    };
    Written { done, config }
}

/// The session's config loaded again and kept as the one in force (`ReloadConfig` does the same
/// at launch; a write of a dialog changes no provider, so no server restarts).
async fn reload(startup: &Startup) -> Option<Arc<Config>> {
    let mut slot = startup.current.write().await;
    let config = Arc::new((startup.reload)().await.ok()?);
    *slot = config.clone();
    Some(config)
}
