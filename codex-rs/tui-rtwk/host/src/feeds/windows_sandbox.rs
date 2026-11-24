//! The Windows sandbox slice's host half: its configuration, requirements and readiness (nothing
//! off Windows), and the word that the setup the user started is over.

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::AppEv;
use codex_tui_core::features::windows_sandbox::Fact;

use super::{Event, Feed, FeedCx};
use crate::in_process::sender;

/// The environment at start; the completion of a setup when the server announces it.
pub(crate) fn feed(event: &Event<'_>, cx: &FeedCx<'_>, out: &mut Vec<Feed>) {
    match event {
        Event::Startup => {
            let (cwd, ids, send) = (
                cx.config.cwd.display().to_string(),
                cx.ids.clone(),
                sender(cx.handle.clone()),
            );
            let remote_workspace = cx.remote_workspace;
            out.push(Feed::Job(Box::pin(async move {
                let (config, requirements, ready) =
                    crate::windows_sandbox::fetch(cwd, &ids, send).await?;
                Some(AppEv::Feature(
                    Fact::environment(&config, requirements, remote_workspace, ready).into(),
                ))
            })));
        }
        Event::Notification(ServerNotification::WindowsSandboxSetupCompleted(done)) => {
            out.push(Feed::fact(Fact::completed(done)));
        }
        _ => {}
    }
}
