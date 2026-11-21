//! The import flow's host side (S06): the project directory of the session
//! and the places the paths of the detected items are shown from (the home
//! directory, the git root of each project directory), read from the file
//! system; the RPCs are the shared runner's. Oracle:
//! `diff_render.rs::display_path_for` (`get_git_repo_root`,
//! `relativize_to_home`), `event_dispatch.rs` (the detection's `cwd` is
//! `config.cwd`).

use std::future::Future;
use std::path::Path;

use codex_app_server_protocol::ClientRequest;
use codex_tui_core::app::AppEv;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::import::{Places, Request};
use codex_tui_rtwk::services::backend::import::{Site, run as run_flow};
use codex_tui_rtwk::services::backend::rpc::Ids;

use crate::startup::Startup;

/// The places of `dirs`: the home directory and, of each project directory,
/// the root of its git repository (none when it is in none).
fn places(dirs: &[String]) -> Places {
    Places {
        home: std::env::var_os("HOME").map(|home| Path::new(&home).display().to_string()),
        repos: dirs
            .iter()
            .filter_map(|dir| {
                let root = codex_git_utils::get_git_repo_root(Path::new(dir))?;
                Some((dir.clone(), root.display().to_string()))
            })
            .collect(),
    }
}

/// The event of `ask` over `send`: the detection asks about the session's
/// project directory unless the flow names one.
pub async fn run<F, Fut>(startup: Startup, ask: Request, ids: Ids, send: F) -> Option<AppEv>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let site = Site {
        cwd: Some(startup.config.cwd.display().to_string()),
        places: Box::new(places),
    };
    run_flow(ask, site, ids, send).await
}
