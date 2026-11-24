//! The asks of the `/daemon` menu (oracle `app/daemon_menu.rs` `verify_managed_daemon`,
//! `event_dispatch.rs` `RunDaemonRestart`): whether the connected server is a daemon this CLI
//! manages, and its restart. Both go through the daemon crate's lifecycle commands.

use codex_app_server_daemon::LifecycleCommand;
use codex_tui_core::app::effects::{DaemonAsk, DaemonDone, Done};

/// The answer to `ask`.
pub(crate) async fn run(ask: DaemonAsk) -> Done {
    Done::BackgroundServer(match ask {
        // A discovered socket alone does not prove it belongs to a managed daemon: the version
        // command answers with a backend only where one is managed.
        DaemonAsk::Verify => DaemonDone::Managed(
            codex_app_server_daemon::run(LifecycleCommand::Version)
                .await
                .is_ok_and(|output| output.backend.is_some()),
        ),
        DaemonAsk::Restart => DaemonDone::Restarted(
            codex_app_server_daemon::run(LifecycleCommand::Restart {
                feature_overrides: Default::default(),
            })
            .await
            .map(|_| ())
            .map_err(|error| format!("{error:#}")),
        ),
    })
}
