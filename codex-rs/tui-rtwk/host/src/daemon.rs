//! The asks of the `/daemon` menu (oracle `app/daemon_menu.rs` `verify_managed_daemon`,
//! `event_dispatch.rs` `RunDaemonRestart` / `RunDaemonUpdate`): whether the connected server is a
//! daemon this CLI manages, its restart, and the update that runs once Codex has exited. The
//! first two go through the daemon crate's lifecycle commands; the update is kept in the slot
//! `main` reads (`update::run_daemon_update` runs it).
//!
//! And the launch's compatibility check of the shared server (oracle `daemon_startup.rs`
//! `compatibility_warning`, `daemon_recovery.rs`): the feature settings this session asked for
//! with `-c`, read against what the connected daemon reports. The decisions are core's
//! (`screens::startup::daemon`); this reads the three requests and carries out the restart.

use std::collections::BTreeMap;
use std::future::Future;
use std::time::Duration;

use codex_app_server_protocol::{
    ClientRequest, ConfigReadParams, ExperimentalFeatureListParams,
    ExperimentalFeatureListResponse, ExperimentalFeatureStage,
};
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::startup::DaemonPrompt;
use codex_tui_core::screens::startup::daemon::{
    Reported, Unmet, drop_managed_layers, drop_required, unmet,
};
use codex_tui_rtwk::services::backend::rpc::Ids;

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

/// `daemon_startup.rs::FAILURE_HINT`.
pub(crate) const FAILURE_HINT: &str = "To work without the background server, rerun the same command with --no-daemon (including resume or fork and its arguments).";

/// `daemon_startup.rs::GATEWAY_SIGN_IN_EXCLUSION`.
pub(crate) const GATEWAY_SIGN_IN_EXCLUSION: &str = "custom provider or gateway sign-in";
/// `daemon_startup.rs::WSL_DRVFS_EXCLUSION`.
pub(crate) const WSL_DRVFS_EXCLUSION: &str = "a Windows-mounted WSL CODEX_HOME (DrvFS/9p)";

/// What the config and the sign-in state still exclude from the shared server
/// (`startup_orchestration.rs`): a GCP or gateway sign-in wizard that is due
/// (no login yet) builds the embedded server with the provider it selects.
pub(crate) async fn wizard_exclusion(config: &codex_core::config::Config) -> Option<&'static str> {
    use codex_features::Feature;
    let due = [Feature::GcpSetupWizard, Feature::GatewaySetupWizard]
        .into_iter()
        .any(|feature| {
            // `should_show_provider_setup_wizard` on the embedded target, not signed in.
            config.model_provider.requires_openai_auth
                && crate::startup::wizard_configured(config, feature)
                && config
                    .auth_config()
                    .is_login_method_allowed(codex_protocol::config_types::ForcedLoginMethod::Api)
        });
    if !due {
        return None;
    }
    let signed_in = matches!(config.auth_config().load_auth(false).await, Ok(Some(_)));
    if signed_in {
        return None;
    }
    Some(if config.features.enabled(Feature::GcpSetupWizard) {
        "GCP sign-in"
    } else {
        GATEWAY_SIGN_IN_EXCLUSION
    })
}

/// `daemon_startup.rs::uses_wsl_drvfs`: whether `codex_home` is on a Windows-mounted
/// WSL filesystem, where the daemon's executable and state cannot be written reliably.
pub(crate) fn uses_wsl_drvfs(codex_home: &std::path::Path) -> bool {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        use std::os::fd::AsRawFd;

        if !codex_utils_path::is_wsl() && std::env::var_os("WSL_INTEROP").is_none() {
            return false;
        }
        let Some(directory) = codex_home
            .ancestors()
            .find_map(|path| std::fs::File::open(path).ok())
        else {
            return false;
        };
        let mut stats = std::mem::MaybeUninit::<libc::statfs>::uninit();
        if unsafe { libc::fstatfs(directory.as_raw_fd(), stats.as_mut_ptr()) } != 0 {
            return false;
        }
        unsafe { stats.assume_init() }.f_type == 0x0102_1997
    }
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    {
        let _ = codex_home;
        false
    }
}

/// Why the shared server cannot be used (oracle `CompatibilityError`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mismatch {
    pub reason: String,
    /// The feature settings a restart would use; none where a restart cannot help.
    pub restart_features: Option<BTreeMap<String, bool>>,
}

impl Mismatch {
    /// A failure that a restart cannot resolve.
    pub fn plain(reason: impl Into<String>) -> Self {
        Mismatch {
            reason: reason.into(),
            restart_features: None,
        }
    }

    /// The text the launch ends with (`CompatibilityError`'s Display).
    pub fn error_text(&self) -> String {
        format!(
            "Cannot use the shared background server: {}.\n{FAILURE_HINT}",
            self.reason
        )
    }

    /// The warning of a launch that runs without the server this time.
    pub fn fallback_warning(&self) -> String {
        format!(
            "Running without the shared background server: {}.",
            self.reason
        )
    }

    /// The recovery menu's data; `managed`: the server is one this CLI manages.
    pub fn prompt(&self, managed: bool) -> DaemonPrompt {
        DaemonPrompt {
            reason: self.reason.clone(),
            restart_features: self
                .restart_features
                .as_ref()
                .map(|features| features.iter().map(|(k, v)| (k.clone(), *v)).collect()),
            managed,
        }
    }
}

impl From<Unmet> for Mismatch {
    fn from(unmet: Unmet) -> Self {
        let state = if unmet.enabled { "enabled" } else { "disabled" };
        Mismatch {
            reason: format!("This session requires {} to be {state}", unmet.name),
            restart_features: Some(unmet.restart_features.into_iter().collect()),
        }
    }
}

/// The settings the command line asks of the shared server (`-c features.<name>=<bool>` and
/// `--enable`/`--disable`): a removed feature is inert and an alias is the feature's own key
/// (the later flag wins, as the config merge has it). File settings and defaults belong to the
/// daemon until it restarts: only explicit flags can ask for a restart.
pub(crate) fn requested_features(
    overrides: &[(String, codex_config::TomlValue)],
) -> BTreeMap<String, bool> {
    codex_config::build_cli_overrides_layer(overrides)
        .get("features")
        .and_then(codex_config::TomlValue::as_table)
        .into_iter()
        .flatten()
        .filter_map(|(name, value)| {
            let name = match codex_features::feature_for_key(name) {
                Some(feature) if feature.stage() == codex_features::Stage::Removed => return None,
                Some(feature) => feature.key(),
                None => name,
            };
            Some((name.to_string(), value.as_bool()?))
        })
        .collect()
}

/// `is_method_not_found`: the server has no such request (older servers refuse an unknown
/// method as an invalid request while reading it).
fn method_not_found(method: &str, error: &ServerError) -> bool {
    error.code == Some(-32601)
        || (error.code == Some(-32600)
            && error.message.contains(method)
            && (error.message.contains("unknown variant")
                || error.message.contains("unknown method")))
}

/// How long the feature list may take (`experimental_features::fetch`).
const LIST_WITHIN: Duration = Duration::from_secs(5);

/// What the server reports of its features (`experimental_features::fetch`): paged, at most ten
/// pages of a hundred, a name once.
async fn reported<F, Fut>(ids: &Ids, send: &F) -> Result<Vec<Reported>, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let discovery = async {
        let mut features: Vec<Reported> = Vec::new();
        let mut cursor = None;
        let mut cursors = std::collections::HashSet::new();
        for _ in 0..10 {
            let value = send(ClientRequest::ExperimentalFeatureList {
                request_id: ids.next(),
                params: ExperimentalFeatureListParams {
                    cursor: cursor.take(),
                    limit: Some(100),
                    thread_id: None,
                },
            })
            .await
            .map_err(|_| "Experimental feature request failed".to_string())?;
            let response: ExperimentalFeatureListResponse = serde_json::from_value(value)
                .map_err(|_| "Experimental feature request failed".to_string())?;
            if response.data.len() > 100 {
                return Err("Experimental feature page exceeds requested limit".to_string());
            }
            for feature in response.data {
                if !features.iter().any(|seen| seen.name == feature.name) {
                    features.push(Reported {
                        removed: feature.stage == ExperimentalFeatureStage::Removed,
                        name: feature.name,
                        enabled: feature.enabled,
                    });
                }
            }
            let Some(next) = response.next_cursor else {
                return Ok(features);
            };
            if !cursors.insert(next.clone()) {
                return Err("Experimental feature pagination repeated a cursor".to_string());
            }
            cursor = Some(next);
        }
        Err("Experimental feature discovery exceeded 10 pages".to_string())
    };
    tokio::time::timeout(LIST_WITHIN, discovery)
        .await
        .unwrap_or_else(|_| Err("Experimental feature discovery timed out".to_string()))
}

/// What of `requested` the connected daemon does not have, in the oracle's order: the legacy
/// managed layers and the managed requirements override the command line, so what they set is
/// not asked of the daemon; nothing left asks nothing; then the server's own list decides.
/// `cwd` is where the project layers are read from.
pub(crate) async fn compatibility<F, Fut>(
    mut requested: BTreeMap<String, bool>,
    cwd: &str,
    ids: &Ids,
    send: &F,
) -> Result<(), Mismatch>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    if requested.is_empty() {
        return Ok(());
    }
    let layers = match send(ClientRequest::ConfigRead {
        request_id: ids.next(),
        params: ConfigReadParams {
            include_layers: true,
            cwd: Some(cwd.to_owned()),
        },
    })
    .await
    {
        Ok(value) => value["layers"].as_array().cloned().unwrap_or_default(),
        // A server without `config/read` has no managed layers.
        Err(error) if method_not_found("config/read", &error) => Vec::new(),
        Err(_) => return Err(Mismatch::plain("could not read daemon configuration")),
    };
    drop_managed_layers(&mut requested, &layers);
    match send(ClientRequest::ConfigRequirementsRead {
        request_id: ids.next(),
        params: None,
    })
    .await
    {
        Ok(value) => {
            if let Some(required) = value["requirements"]["featureRequirements"].as_object() {
                let names: Vec<String> = required.keys().cloned().collect();
                drop_required(&mut requested, &names);
            }
        }
        Err(error) if method_not_found("configRequirements/read", &error) => {}
        Err(_) => {
            return Err(Mismatch::plain(
                "could not check daemon configuration requirements",
            ));
        }
    }
    if requested.is_empty() {
        return Ok(());
    }
    let features = reported(ids, send).await.map_err(Mismatch::plain)?;
    unmet(requested, &features).map_err(Mismatch::from)
}

/// Whether the daemon is one this CLI manages and so may restart: only asked of one whose
/// restart could help (`daemon_recovery.rs::check`); the version command answers with a
/// backend only where one is managed.
pub(crate) async fn managed(mismatch: &Mismatch) -> bool {
    mismatch.restart_features.is_some()
        && codex_app_server_daemon::run(LifecycleCommand::Version)
            .await
            .is_ok_and(|output| output.backend.is_some())
}

/// The one check after a restart (`daemon_recovery.rs`): a connection of its own to the
/// restarted daemon, shut down again once it has been asked.
pub(crate) async fn recheck(requested: BTreeMap<String, bool>, cwd: &str) -> Result<(), Mismatch> {
    let connect_failed = || Mismatch::plain("could not connect to check daemon feature settings");
    let endpoint = crate::in_process::reconnect_endpoint(&crate::target::Mode::Daemon)
        .ok_or_else(connect_failed)?;
    let client = crate::in_process::connect_remote(endpoint)
        .await
        .ok_or_else(connect_failed)?;
    let send = crate::in_process::sender(client.request_handle());
    let result = compatibility(requested, cwd, &Ids::default(), &send).await;
    let _ = client.shutdown().await;
    result
}

/// The restart of the managed daemon with `features` (`restart_with_features`): the failure's
/// text with the hint on how to work without the server. Its output is not drawn.
pub(crate) async fn restart_with(features: &BTreeMap<String, bool>) -> Result<(), String> {
    codex_app_server_daemon::restart_with_features(features)
        .await
        .map(|_| ())
        .map_err(|error| format!("{error:#}\n{FAILURE_HINT}"))
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

    /// The oracle's table (`daemon_startup_tests.rs`), over a fake server: the `-c` flags of a
    /// scenario, what the daemon reports, and the reason and the restart a mismatch has.
    /// (`unreachable daemon` needs no case: the host is connected before it asks.)
    #[tokio::test]
    async fn daemon_feature_compatibility_respects_required_and_optional_attachment() {
        use codex_app_server_protocol::ClientRequest;
        use serde_json::json;
        let overrides = |scenario: &str| -> Vec<String> {
            match scenario {
                "matching" => vec![
                    "features.instant_interrupt=true",
                    "features.remote_compaction_v2=true",
                    "features.shell_tool=true",
                    "features.zz_missing=false",
                ],
                "explicit enable" => vec!["features.api_key_model_discovery=true"],
                "mixed flags" => vec![
                    "features.zz_overridden=true",
                    "features={api_key_model_discovery=false,code_mode_host=false,instant_interrupt=true}",
                    "features.code_mode_host=true",
                ],
                "missing enabled" => vec!["features.zz_missing=true"],
                _ => vec!["features.api_key_model_discovery=false"],
            }
            .into_iter()
            .map(str::to_owned)
            .collect()
        };
        for scenario in [
            "matching",
            "conflicting client",
            "explicit enable",
            "mixed flags",
            "missing enabled",
            "managed",
            "legacy managed",
            "old requirements server",
            "unsupported RPC",
        ] {
            let parsed = codex_utils_cli::CliConfigOverrides {
                raw_overrides: overrides(scenario),
            }
            .parse_overrides()
            .expect("overrides");
            let features: Vec<serde_json::Value> = [
                ("api_key_model_discovery", scenario != "explicit enable", "beta"),
                ("code_mode_host", false, "beta"),
                ("instant_interrupt", true, "beta"),
                ("remote_compaction_v2", false, "stable"),
                ("shell_tool", false, "removed"),
            ]
            .iter()
            .map(|(name, enabled, stage)| {
                json!({"name": name, "stage": stage, "enabled": enabled, "defaultEnabled": false})
            })
            .collect();
            let send = |request: ClientRequest| {
                let reply = match request {
                    ClientRequest::ConfigRead { params, .. } => {
                        assert!(params.include_layers);
                        Ok(json!({"config": {}, "origins": {}, "layers": [{
                            "name": {"type": "legacyManagedConfigTomlFromFile"},
                            "config": if scenario == "legacy managed" {
                                json!({"features": {"api_key_model_discovery": true}})
                            } else {
                                json!({})
                            },
                        }]}))
                    }
                    ClientRequest::ConfigRequirementsRead { .. }
                        if scenario == "old requirements server" =>
                    {
                        Err(ServerError {
                            code: Some(-32600),
                            message: "configRequirements/read failed: Invalid request: unknown variant `configRequirements/read` (code -32600)".to_owned(),
                            data: None,
                        })
                    }
                    ClientRequest::ConfigRequirementsRead { .. } => Ok(json!({"requirements":
                        if scenario == "managed" {
                            json!({"featureRequirements": {"api_key_model_discovery": true}})
                        } else {
                            json!(null)
                        }
                    })),
                    ClientRequest::ExperimentalFeatureList { params, .. } => {
                        assert_eq!(params.thread_id, None);
                        if scenario == "unsupported RPC" {
                            Err(ServerError {
                                code: Some(-32601),
                                message: "method not found".to_owned(),
                                data: None,
                            })
                        } else {
                            Ok(json!({"data": features, "nextCursor": null}))
                        }
                    }
                    other => panic!("unexpected request {other:?}"),
                };
                std::future::ready(reply)
            };
            let result =
                compatibility(requested_features(&parsed), "/work", &Ids::default(), &send).await;
            if matches!(scenario, "matching" | "managed" | "legacy managed") {
                assert_eq!(result, Ok(()), "{scenario}");
                continue;
            }
            let error = result.expect_err(scenario);
            let reason = match scenario {
                "explicit enable" => "This session requires api_key_model_discovery to be enabled",
                "conflicting client" | "old requirements server" | "mixed flags" => {
                    "This session requires api_key_model_discovery to be disabled"
                }
                "missing enabled" => "This session requires zz_missing to be enabled",
                _ => "Experimental feature request failed",
            };
            assert_eq!(error.reason, reason, "{scenario}");
            assert_eq!(
                error.restart_features.is_some(),
                scenario != "unsupported RPC",
                "{scenario}"
            );
            if scenario == "mixed flags" {
                assert_eq!(
                    error.restart_features,
                    Some(std::collections::BTreeMap::from([
                        ("api_key_model_discovery".to_owned(), false),
                        ("code_mode_host".to_owned(), true),
                    ]))
                );
            }
            assert_eq!(
                error.error_text(),
                format!("Cannot use the shared background server: {reason}.\n{FAILURE_HINT}")
            );
            // The oracle's snapshot `daemon_feature_mismatch_error`.
            if scenario == "explicit enable" {
                assert_eq!(
                    error.error_text(),
                    "Cannot use the shared background server: This session requires api_key_model_discovery to be enabled.\nTo work without the background server, rerun the same command with --no-daemon (including resume or fork and its arguments)."
                );
            }
        }
    }

    // The menu restarts only a managed server with settings to restart with, and the launch
    // that runs without the server says why.
    #[test]
    fn the_menu_restarts_only_a_managed_server_with_settings() {
        let mismatch = Mismatch {
            reason: "r".into(),
            restart_features: Some(BTreeMap::from([
                ("a".to_owned(), true),
                ("b".to_owned(), false),
            ])),
        };
        assert!(mismatch.prompt(true).can_restart());
        assert!(!mismatch.prompt(false).can_restart());
        assert_eq!(
            mismatch.prompt(true).restart_features,
            Some(vec![("a".to_owned(), true), ("b".to_owned(), false)])
        );
        assert!(!Mismatch::plain("r").prompt(true).can_restart());
        assert_eq!(
            Mismatch::plain("r").fallback_warning(),
            "Running without the shared background server: r."
        );
    }

    // A launch that asked for nothing asks the server nothing (the connection is the check).
    #[tokio::test]
    async fn no_requested_feature_asks_the_server_nothing() {
        let send =
            |request: ClientRequest| -> std::future::Ready<Result<serde_json::Value, ServerError>> {
                panic!("unexpected request {request:?}")
            };
        assert_eq!(
            compatibility(Default::default(), "/work", &Ids::default(), &send).await,
            Ok(())
        );
    }

    // A server that cannot read its configuration is not guessed at.
    #[tokio::test]
    async fn a_failed_configuration_read_names_what_could_not_be_read() {
        let send = |_: ClientRequest| {
            std::future::ready(Err::<serde_json::Value, _>(ServerError {
                code: Some(-32603),
                message: "boom".to_owned(),
                data: None,
            }))
        };
        let requested = std::collections::BTreeMap::from([("a".to_owned(), true)]);
        let error = compatibility(requested, "/work", &Ids::default(), &send)
            .await
            .expect_err("a failure");
        assert_eq!(error.reason, "could not read daemon configuration");
        assert_eq!(error.restart_features, None);
    }

    async fn config_with(
        home: &std::path::Path,
        overrides: &[(&str, bool)],
    ) -> codex_core::config::Config {
        let overrides = overrides
            .iter()
            .map(|(key, value)| (key.to_string(), codex_config::TomlValue::Boolean(*value)))
            .collect::<Vec<_>>();
        ConfigBuilder::default()
            .codex_home(home.to_path_buf())
            .fallback_cwd(Some(home.to_path_buf()))
            .cli_overrides(overrides)
            .build()
            .await
            .expect("a config")
    }

    /// A due gateway sign-in builds the embedded server with the provider it selects: the
    /// shared one is left out until a login exists.
    #[tokio::test]
    async fn gateway_wizard_due_without_auth_excludes_the_daemon() {
        let home = tempfile::tempdir().expect("a temp dir");
        let wizard = [("features.gateway_setup_wizard", true)];
        let config = config_with(home.path(), &wizard).await;
        assert_eq!(
            wizard_exclusion(&config).await,
            Some(GATEWAY_SIGN_IN_EXCLUSION)
        );
        let gcp = [
            ("features.gateway_setup_wizard", true),
            ("features.gcp_setup_wizard", true),
        ];
        let config = config_with(home.path(), &gcp).await;
        assert_eq!(wizard_exclusion(&config).await, Some("GCP sign-in"));
        // No wizard on: nothing to exclude.
        let config = config_with(home.path(), &[]).await;
        assert_eq!(wizard_exclusion(&config).await, None);
        // A login ends the exclusion.
        {
            // (`std::fs::write` is the shell's: R12)
            use std::io::Write as _;
            std::fs::File::create(home.path().join("auth.json"))
                .and_then(|mut file| {
                    file.write_all(br#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-test"}"#)
                })
                .expect("auth.json");
        }
        let config = config_with(home.path(), &wizard).await;
        assert_eq!(wizard_exclusion(&config).await, None);
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn drvfs_is_false_off_linux() {
        assert!(!uses_wsl_drvfs(std::path::Path::new("/")));
    }

    /// The notice's words (`daemon_exclusion_warning`).
    #[test]
    fn the_fallback_notice_uses_the_oracles_words() {
        assert_eq!(
            crate::target::exclusion_warning(WSL_DRVFS_EXCLUSION),
            "Running without the shared background server: a Windows-mounted WSL CODEX_HOME (DrvFS/9p) requires embedded mode."
        );
    }
}
