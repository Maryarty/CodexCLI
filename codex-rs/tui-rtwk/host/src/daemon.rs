//! The asks of the `/daemon` menu (oracle `app/daemon_menu.rs` `verify_managed_daemon`,
//! `event_dispatch.rs` `RunDaemonRestart` / `RunDaemonUpdate`): whether the connected server is a
//! daemon this CLI manages, its restart, and the update that runs once Codex has exited. The
//! first two go through the daemon crate's lifecycle commands; the update is kept in the slot
//! `main` reads (`update::run_daemon_update` runs it).

use codex_app_server_client::EmbeddedNetworkPolicy;
use codex_app_server_daemon::{InstallRequest, LifecycleCommand, UpdateOutput};
use codex_config::LoaderOverrides;
use codex_core::config::ConfigBuilder;
use codex_tui_core::app::effects::{DaemonAsk, DaemonDone, DaemonSource as Source, Done};
use codex_tui_rtwk::services::startup::UpdateAction;

use crate::startup::UpdateSlot;

/// The answer to `ask`; a chosen update is kept in `update` first.
pub(crate) async fn run(ask: DaemonAsk, update: &UpdateSlot) -> Done {
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
        DaemonAsk::Update(source) => {
            // R-F1: kept before the answer: the app ends on it, and `main` reads the slot after.
            *update
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some(UpdateAction::Daemon(source));
            DaemonDone::UpdateQueued
        }
    })
}

/// The update the daemon crate runs for `source` (what `codex app-server daemon update` and its
/// `--from-cli --yes` form run): the output line it prints, none when it was cancelled. The
/// menu's confirmation sheet is the `--yes`. `overrides` are the launch's `-c key=value`.
pub(crate) async fn update_output(
    source: Source,
    overrides: Vec<String>,
) -> Result<Option<String>, String> {
    match source {
        Source::PublicStable => {
            let factory = http_client_factory(overrides).await?;
            let output = codex_app_server_daemon::update(factory)
                .await
                .map_err(|error| format!("{error:#}"))?;
            line(&output).map(Some)
        }
        Source::ThisCli => codex_app_server_daemon::update_from_cli(|request| {
            eprintln!("{}", describe_install(request));
            Ok(true)
        })
        .await
        .map_err(|error| format!("{error:#}"))?
        .as_ref()
        .map(line)
        .transpose(),
    }
}

fn line(output: &UpdateOutput) -> Result<String, String> {
    serde_json::to_string(output).map_err(|error| error.to_string())
}

/// The CLI's `updater_http_client_factory`: the config as the launch reads it, with the
/// embedded network policy bound.
async fn http_client_factory(
    overrides: Vec<String>,
) -> Result<codex_http_client::HttpClientFactory, String> {
    let cli_overrides = codex_utils_cli::CliConfigOverrides {
        raw_overrides: overrides,
    }
    .parse_overrides()?;
    let mut config = ConfigBuilder::default()
        .cli_overrides(cli_overrides)
        .build()
        .await
        .map_err(|error| error.to_string())?;
    EmbeddedNetworkPolicy::load(&LoaderOverrides::default())
        .await
        .bind_config(&mut config);
    Ok(config.http_client_factory())
}

/// The CLI's `describe_install`: what the package copy of `--from-cli` is about to do.
fn describe_install(request: &InstallRequest) -> String {
    let mut message = format!(
        "Replace installed daemon version {} with CLI version {} from {}.\nThe daemon package will be installed in {}.\nThe selected package will be pinned. Run `codex app-server daemon update` to return to production updates.",
        request.installed_version.as_deref().unwrap_or("unknown"),
        request.version,
        request.source.display(),
        request.destination.display()
    );
    if request.restart_required {
        message.push_str(
            "\nThe running daemon will restart; active or queued work may be interrupted.",
        );
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    // R-F1: the update chosen is kept before the answer that lets the app end is given.
    #[tokio::test]
    async fn the_daemon_update_is_stored_before_it_is_acknowledged() {
        let slot = UpdateSlot::default();
        let done = run(DaemonAsk::Update(Source::ThisCli), &slot).await;
        assert!(matches!(
            done,
            Done::BackgroundServer(DaemonDone::UpdateQueued)
        ));
        assert_eq!(
            *slot.lock().unwrap(),
            Some(UpdateAction::Daemon(Source::ThisCli))
        );
    }

    #[test]
    fn the_package_copy_says_what_it_replaces() {
        let request = InstallRequest {
            source: "/pkg".into(),
            version: "0.2.0".to_owned(),
            destination: "/dest".into(),
            installed_version: None,
            restart_required: true,
        };
        let text = describe_install(&request);
        assert!(
            text.starts_with("Replace installed daemon version unknown with CLI version 0.2.0")
        );
        assert!(text.ends_with("active or queued work may be interrupted."));
    }
}
