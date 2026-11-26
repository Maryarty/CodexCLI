//! The import flow's RPCs as the shell runs them: every source is asked in
//! turn (`externalAgentConfig/detect`), the import is one request
//! (`externalAgentConfig/import`). The client is a closure and the places of
//! the paths are the host's to read, so the host's real client and the fake's
//! run the same code. Core's `protocol::import` makes the requests and reads
//! the results. The answer carries the ticket issued by the app for this ask. The import
//! model's flow generation belongs to the dialog, independently of detection and import replies.
//! Oracle: `external_agent_config_migration/flow.rs`,
//! `app_server_session/external_agent_config.rs`.

use std::future::Future;

use codex_app_server_protocol::ClientRequest;
use codex_tui_core::app::AppEv;
use codex_tui_core::protocol::import as wire;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::import::{Ev, Found, Places, Request, Source};

use crate::services::backend::rpc::Ids;

/// What the oracle's `wrap_err` says of a failed detection: the text a
/// source's failure is told with (the server's own message is only logged).
pub const DETECT_FAILED: &str = "externalAgentConfig/detect failed during external agent import";

/// The same for a failed import.
pub const IMPORT_FAILED: &str = "externalAgentConfig/import failed during external agent import";

/// Reads the places for the paths of the detected items.
pub type PlacesOf = Box<dyn FnOnce(&[String]) -> Places + Send>;

/// What the host reads of the file system for a detection's answer.
pub struct Site {
    /// The project directory of the session: asked when the flow names none
    /// (`config.cwd`).
    pub cwd: Option<String>,
    /// The places the paths of the items are shown from (`~`, the repository
    /// roots of the project directories), read once the items are known.
    pub places: PlacesOf,
}

/// The event of `ask` through `send`.
pub async fn run<F, Fut>(ask: Request, site: Site, ids: Ids, send: F) -> Option<AppEv>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let ev = match ask {
        Request::Detect { request, cwd } => {
            let cwd = cwd.or(site.cwd);
            let mut found = Vec::new();
            for source in Source::ALL {
                let asked = send(wire::detect(source, cwd.as_deref(), ids.next())).await;
                let result = asked
                    .map_err(|_| DETECT_FAILED.to_owned())
                    .and_then(|json| wire::found(json).map_err(|_| DETECT_FAILED.to_owned()));
                found.push(Found { source, result });
            }
            let mut dirs: Vec<String> = Vec::new();
            for item in found
                .iter()
                .filter_map(|found| found.result.as_ref().ok())
                .flatten()
            {
                if let Some(dir) = item.cwd.as_ref().map(|dir| dir.display().to_string())
                    && !dirs.contains(&dir)
                {
                    dirs.push(dir);
                }
            }
            let places = (site.places)(&dirs);
            Ev::Detected {
                request,
                places,
                found,
                selected: None,
            }
        }
        Request::Import {
            request,
            source,
            items,
        } => {
            let asked = send(wire::import(source, items, ids.next())).await;
            let result = asked
                .map_err(|_| IMPORT_FAILED.to_owned())
                .and_then(|json| wire::imported(json).map_err(|_| IMPORT_FAILED.to_owned()));
            Ev::Imported { request, result }
        }
    };
    Some(AppEv::Screen(ScreenEv::Import(ev)))
}
