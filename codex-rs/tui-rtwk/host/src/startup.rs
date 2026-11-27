//! The startup page's host side (S01-S04): reads the launch facts (the
//! update offered, the sign-in due and what it offers, the folder trust)
//! and runs the page's asks (the trust write, the update's dismissal, the
//! logins through the app server's account RPCs); the update chosen runs
//! once the app has ended (`main`). Core decides
//! (`protocol::onboarding`); this only reads and writes. Oracle: `lib.rs`
//! (`get_login_status`, the trust check at launch), `config_update.rs`,
//! `updates.rs`, `updates_cache.rs`, `update_action.rs`, the CLI's
//! `run_update_action`, `should_show_provider_setup_wizard`, and
//! `onboarding/auth.rs` (`start_chatgpt_login`, `save_api_key`,
//! `cancel_login_attempt`, `show_chatgpt_login_in_browser`).

use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::daemon::Mismatch;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::EnvironmentInfoParams;
use codex_config::ConfigLayerSource;
use codex_config::default_project_root_markers;
use codex_config::loader::discover_project_root;
use codex_config::loader::normalized_project_trust_keys;
use codex_config::loader::project_trust_key;
use codex_core::config::Config;
use codex_exec_server::LOCAL_ENVIRONMENT_ID;
use codex_exec_server::LOCAL_FS;
use codex_features::Feature;
use codex_git_utils::resolve_root_git_project_for_trust;
use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_http_client::RouteAwareClientPool;
use codex_install_context::InstallContext;
use codex_install_context::InstallMethod;
use codex_install_context::StandalonePlatform;
use codex_login::default_client::default_headers;
use codex_login::read_openai_api_key_from_env;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_tui_core::app::AppEv;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::protocol::onboarding as wire;
use codex_tui_core::rpc::RpcError;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::chat::Ev as ChatEv;
use codex_tui_core::screens::onboarding::auth::{AuthResult, Offer, Providers};
use codex_tui_core::screens::onboarding::consent::{Checked, Target as ConsentTarget};
use codex_tui_core::screens::onboarding::gateway::Logins;
use codex_tui_core::screens::onboarding::session::Resumed;
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts, TrustAsk, TrustCancel, Welcome};
use codex_tui_core::screens::startup::{DaemonChoice, Prompt};
use codex_tui_core::threads::{ThreadAsk, ThreadReply};
use codex_tui_rtwk::services::backend::rpc::Ids;
pub use codex_tui_rtwk::services::startup::Browser;
use codex_tui_rtwk::services::startup::UpdateAction;
use codex_tui_rtwk::services::startup::dismiss_version;
use codex_tui_rtwk::services::startup::ensure_npm_version_ready;
use codex_tui_rtwk::services::startup::latest_from_release_tag;
use codex_tui_rtwk::services::startup::open_in_browser;
use codex_tui_rtwk::services::startup::read_version_cache;
use codex_tui_rtwk::services::startup::store_latest_version;
use codex_tui_rtwk::services::startup::version_cache_needs_refresh;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::LegacyAppPathString;
use codex_utils_path_uri::PathConvention;
use codex_utils_path_uri::PathUri;

/// The update the user chose to run, kept for `main` to run after the
/// terminal is restored.
pub type UpdateSlot = Arc<Mutex<Option<UpdateAction>>>;

/// The message a launch that failed ends with, kept for `main` to print once
/// the terminal is restored (as `update`: set before the event that ends the
/// app is sent, so it is there when the app is gone).
pub type FailureSlot = Arc<Mutex<Option<String>>>;

/// The compatibility failure the launch's recovery menu was opened for, kept for the answer
/// (`Ask::DaemonChoice`) to act on: the menu carries what is drawn, the host what is done.
pub type DaemonSlot = Arc<Mutex<Option<Mismatch>>>;

/// An established local account can edit a draft while the host reads the startup facts.
/// The file check is only a presentation hint. The normal account and trust checks still
/// decide when a session may start.
pub fn can_show_draft(cli: &codex_tui_rtwk::Cli) -> bool {
    if cli.agents || cli.resume || cli.session.is_some() || cli.remote.is_some() || cli.oss {
        return false;
    }
    std::env::var(codex_login::CODEX_ACCESS_TOKEN_ENV_VAR)
        .is_ok_and(|value| !value.trim().is_empty())
        || codex_core::config::find_codex_home()
            .is_ok_and(|home| home.as_path().join("auth.json").is_file())
}

/// What the startup asks need of the host.
#[derive(Clone)]
pub struct Startup {
    pub config: Arc<Config>,
    /// The physical screen mode chosen before the terminal opened.
    pub alternate_screen: bool,
    /// The server is not embedded (a daemon): the trust key is the folder as
    /// it was read (`remote_trust_key`), and the write says so.
    pub connected: bool,
    /// Whether the folder trust is asked at launch: not for the agents
    /// overview, not for a remote workspace (lib.rs: "The shared overview
    /// does not open a workspace").
    pub check_trust: bool,
    /// `--cd` for a remote workspace (`remote_cwd_override`): the folder on
    /// the remote host whose trust is asked. A remote workspace without one
    /// is not asked.
    pub remote_cwd: Option<PathBuf>,
    /// `--cd` was given (a resumed or forked session then opens here:
    /// `effective_resume_cwd_mode`).
    pub cwd_override: bool,
    /// The provider `--oss` chose (`model_provider_override`): the server's
    /// own pin (`required_model_provider`) wins over it.
    pub oss_provider: Option<String>,
    /// The command line carries a prompt (`cli.prompt` non-empty): no update is offered
    /// (lib.rs `skip_update_prompt`); the cache refresh and the banner still run.
    pub launch_prompt: bool,
    pub update: UpdateSlot,
    /// Which server the TUI talks to: what the provider wizards and the
    /// browser login are offered for (`AppServerTarget`).
    pub target: Target,
    /// Opens a URL in the user's browser (`webbrowser::open`); a seam so a
    /// test opens none.
    pub browser: Browser,
    /// The update check: whether it runs, which version is running, and how
    /// the latest release is fetched.
    pub updates: Updates,
    /// Loads the session's config again, with the launch's overrides: the
    /// folder is trusted, so its project config applies
    /// (`load_config_or_exit_with_fallback_cwd`).
    pub reload: Reload,
    /// The session's config as it stands: `config`, replaced by a reload.
    pub current: crate::launch::ConfigSlot,
    /// Restarts the embedded server with a reloaded config whose provider
    /// changed (lib.rs: `config.model_provider_id != previous_provider`).
    pub restart: tokio::sync::mpsc::UnboundedSender<Restart>,
    /// The usage page's open session (S20).
    pub analytics: crate::analytics::Store,
    /// The feature settings the command line asks of the shared server (`daemon::requested_features`);
    /// empty for a launch that does not use it (the compatibility check has nothing to ask).
    pub daemon_features: std::collections::BTreeMap<String, bool>,
    /// The failed compatibility check the recovery menu answers.
    pub daemon: DaemonSlot,
}

/// A restart of the embedded server with a config. `done` answers once the
/// new server is the one the app talks to (the gateway's sign-in reads its
/// account next); a reload that only changed the provider does not wait.
pub struct Restart {
    pub config: Arc<Config>,
    /// The request that scheduled this restart reports failure through its own continuation.
    pub done: Option<tokio::sync::oneshot::Sender<Result<(), String>>>,
    /// The server the app moves to instead of staying on the one it has (the recovery menu's
    /// answers).
    pub switch: Option<Switch>,
}

/// Where a `Restart` moves the app (`daemon_recovery.rs`).
pub enum Switch {
    /// Run without the shared server this time: the embedded one, with `warning` as a startup
    /// warning.
    Embedded { warning: String },
    /// The managed daemon restarts with `features`, then the app connects to it again.
    Daemon {
        features: std::collections::BTreeMap<String, bool>,
    },
}

/// Loads the config as the launch did; the text of the error when it fails.
pub type Reload = Arc<
    dyn Fn() -> std::pin::Pin<Box<dyn Future<Output = Result<Config, String>> + Send>>
        + Send
        + Sync,
>;

/// `updates.rs`: the update check of a launch.
#[derive(Clone)]
pub struct Updates {
    /// `updates.rs` is compiled out of debug builds: no offer, no refresh.
    pub enabled: bool,
    /// `CODEX_CLI_VERSION`: a source build (`0.0.0`) is never offered one.
    pub version: String,
    /// What updates this install (`get_update_action`): none for a build the
    /// installer does not know.
    pub action: Option<UpdateAction>,
    /// The latest release for the install method; a seam so a test fetches
    /// nothing over the network.
    pub fetch: Fetch,
}

impl Updates {
    /// The running build's check.
    pub fn system(config: &Config) -> Self {
        let factory = config.http_client_factory();
        Updates {
            enabled: !cfg!(debug_assertions),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            action: update_action(InstallContext::current()),
            fetch: Arc::new(move |action| {
                let factory = factory.clone();
                Box::pin(async move {
                    fetch_latest(factory, action)
                        .await
                        .map_err(|error| error.to_string())
                })
            }),
        }
    }
}

/// Fetches the latest released version (`check_for_update`): the text of the
/// error when it fails.
pub type Fetch = Arc<
    dyn Fn(
            Option<UpdateAction>,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;

/// The system's browser (`webbrowser::open`, as the oracle opens it).
pub fn system_browser() -> Browser {
    Arc::new(webbrowser::open)
}

/// The server's place (`AppServerTarget` without Cloud, which has no
/// startup side).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Embedded,
    LocalDaemon,
    Remote,
}

impl Target {
    /// `uses_remote_workspace`: the login's browser callback is not here.
    fn remote_workspace(self) -> bool {
        self == Target::Remote
    }

    /// `should_show_provider_setup_wizard`'s `supported_target`.
    fn supports(self, feature: Feature) -> bool {
        match self {
            Target::Embedded => true,
            Target::LocalDaemon => feature == Feature::BedrockSetupWizard,
            Target::Remote => false,
        }
    }
}

/// Runs one ask; the event that answers it, if any.
pub async fn run<F, Fut>(startup: Startup, ask: Ask, ids: Ids, send: F) -> Option<AppEv>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let ev = match ask {
        Ask::Facts { request } => match facts(&startup, &ids, &send).await {
            Ok(facts) => Ev::Facts { request, facts },
            Err(message) => return Some(AppEv::StartFailed(message)),
        },
        Ask::WriteTrust { request, target } => {
            let (key, context) = if startup.connected {
                (
                    target,
                    "config/batchWrite failed while persisting remote project trust",
                )
            } else {
                (
                    project_trust_key(Path::new(&target)),
                    "config/batchWrite failed in TUI",
                )
            };
            match send(wire::trust_write(&key, ids.next())).await {
                Ok(_) => Ev::TrustSaved { request },
                Err(error) => Ev::TrustFailed {
                    request,
                    message: format!("{context}: {}", error.message),
                },
            }
        }
        Ask::CheckConsent {
            request,
            target,
            cancel,
        } => match consent(&startup, target, cancel, &ids, &send).await {
            Ok(ConsentCheck {
                ask,
                checked: Some(checked),
            }) => Ev::ConsentAt {
                request,
                ask,
                checked,
            },
            Ok(ConsentCheck { ask, checked: None }) => Ev::Consent { request, ask },
            Err(message) => Ev::ConsentFailed { request, message },
        },
        Ask::DismissUpdate { version } => {
            // The oracle only logs a failed dismissal.
            let _ = dismiss_version(&version_file(&startup.config), &version);
            return None;
        }
        Ask::RunUpdate { request } => {
            // Stored here, before the answer: the page leaves and the app
            // ends only on it, and `main` reads the slot after that.
            let Ok(mut slot) = startup.update.lock() else {
                return Some(AppEv::StartFailed(
                    "the update could not be queued".to_owned(),
                ));
            };
            *slot = startup.updates.action;
            drop(slot);
            Ev::UpdateQueued { request }
        }
        Ask::Login { request, login } => {
            let result = send(wire::login_start(&login, ids.next())).await;
            let outcome = wire::login_outcome(&login, result);
            Ev::Auth(AuthResult { request, outcome })
        }
        Ask::RefreshUpdateCache => {
            // In the background: this ask runs beside the page, and the
            // launch has read the cache as it was.
            let file = version_file(&startup.config);
            if let Err(error) =
                refresh_version_cache(&file, startup.updates.action, &startup.updates.fetch).await
            {
                tracing::error!("Failed to update version: {error}");
            }
            return None;
        }
        Ask::ReloadConfig => {
            // The remote workspace's config is not this machine's.
            if startup.target.remote_workspace() {
                return None;
            }
            // Held while the config loads: the launch's prompts, asked
            // after, read the new one.
            let mut slot = startup.current.write().await;
            return Some(match (startup.reload)().await {
                Ok(config) => {
                    let previous = slot.model_provider_id.clone();
                    let config = Arc::new(config);
                    *slot = config.clone();
                    drop(slot);
                    if config.model_provider_id != previous && startup.target == Target::Embedded {
                        let _ = startup.restart.send(Restart {
                            config: config.clone(),
                            done: None,
                            switch: None,
                        });
                    }
                    let status = crate::facts::status(&config).await;
                    AppEv::Screen(ScreenEv::Chat(ChatEv::Status(Box::new(status))))
                }
                // `load_config_or_exit`.
                Err(error) => AppEv::StartFailed(format!("Error loading configuration: {error}")),
            });
        }
        Ask::OpenBrowser { url } => {
            // `show_chatgpt_login_in_browser`: only for a local workspace; a
            // failure is logged, not answered.
            if !startup.target.remote_workspace()
                && let Err(error) = open_in_browser(startup.browser.clone(), url).await
            {
                tracing::warn!("{error}");
            }
            return None;
        }
        Ask::Launch { request } => {
            let config = startup.current.read().await.clone();
            let cwd = startup
                .remote_cwd
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| config.cwd.display().to_string());
            // The oracle checks the shared server before anything is drawn; here the recovery
            // menu is the first prompt after the steps (the page is up already).
            let daemon = daemon_prompt(&startup, &cwd, &ids, &send).await;
            let mut prompts =
                crate::launch::prompts(&config, startup.target, cwd, false, &ids, &send).await;
            prompts.splice(0..0, daemon);
            Ev::Launched { request, prompts }
        }
        Ask::SaveResumeCwd { request, mode } => {
            let config = startup.current.read().await.clone();
            let saved = crate::launch::save_resume_cwd(&config, mode).await;
            return Some(AppEv::Screen(ScreenEv::Startup(
                codex_tui_core::screens::startup::Ev::CwdSaved {
                    request,
                    failed: !saved,
                },
            )));
        }
        Ask::TrustHooks { request, updates } => {
            let failure = crate::launch::trust_hooks(&updates, &ids, &send).await;
            return Some(AppEv::Screen(ScreenEv::Startup(
                codex_tui_core::screens::startup::Ev::HooksTrusted { request, failure },
            )));
        }
        Ask::SaveMigration {
            from,
            to,
            accepted,
            effort,
        } => {
            let config = startup.current.read().await.clone();
            crate::launch::save_migration(&config, &from, &to, accepted, effort).await;
            return None;
        }
        Ask::DaemonChoice { choice } => return daemon_choice(&startup, choice).await,
        // The provider panel is answered before this worker exists: the launch task takes it.
        Ask::OssChoice { .. } => return None,
        Ask::AcknowledgeGovCloud => {
            let config = startup.current.read().await.clone();
            crate::providers::acknowledge_gov_cloud(&config).await;
            return None;
        }
        Ask::SelectGateway {
            request,
            provider_id,
        } => {
            // `lib.rs` after `selected_gateway`: the config is read again
            // and must select that gateway, then the embedded server starts
            // with it.
            let mut slot = startup.current.write().await;
            let selected = match (startup.reload)().await {
                Ok(config) => config,
                Err(error) => {
                    return Some(AppEv::StartFailed(format!(
                        "Error loading configuration: {error}"
                    )));
                }
            };
            if selected.model_provider_id != provider_id {
                return Some(AppEv::StartFailed(format!(
                    "The current configuration selects {} instead of {provider_id}.",
                    selected.model_provider_id
                )));
            }
            let config = Arc::new(selected);
            *slot = config.clone();
            drop(slot);
            let (done, started) = tokio::sync::oneshot::channel();
            let restart = Restart {
                config,
                done: Some(done),
                switch: None,
            };
            startup.restart.send(restart).ok()?;
            if let Err(message) = started.await.ok()? {
                return Some(AppEv::StartFailed(message));
            }
            Ev::GatewayStarted { request }
        }
        Ask::GatewayLogins { request, .. } => {
            // `authenticate_gateway`: `get_login_status` on the new server,
            // and whether the provider has a gateway OAuth.
            let config = startup.current.read().await.clone();
            let account = match send(wire::account_read(ids.next())).await {
                Ok(account) => account,
                Err(error) => {
                    return Some(AppEv::StartFailed(format!("account/read failed: {error}")));
                }
            };
            let primary = match wire::sign_in_due(account) {
                Ok(due) => due,
                Err(error) => {
                    let (RpcError::Remote(text) | RpcError::Malformed(text)) = &error else {
                        return Some(AppEv::StartFailed("account/read failed".to_owned()));
                    };
                    return Some(AppEv::StartFailed(format!("account/read failed: {text}")));
                }
            };
            let auth = config.auth_config();
            Ev::GatewayLogins {
                request,
                logins: Logins {
                    primary,
                    oauth: config.model_provider.gateway_oauth.is_some(),
                    offer: Offer {
                        chatgpt: auth.is_login_method_allowed(ForcedLoginMethod::Chatgpt),
                        api_key: auth.is_login_method_allowed(ForcedLoginMethod::Api),
                        providers: Providers::default(),
                        env_api_key: read_openai_api_key_from_env(),
                    },
                },
            }
        }
        Ask::GatewayOAuth {
            request,
            provider_id,
        } => {
            let result = crate::gateway::authenticate(
                &provider_id,
                &ids,
                &send,
                crate::gateway::prefer_device_login(),
            )
            .await;
            Ev::GatewayOAuth { request, result }
        }
        Ask::GatewayDevice { request, thread } => {
            // `lib.rs`: `!uses_remote_workspace` gates the whole sign-in.
            if startup.target.remote_workspace() {
                return Some(AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayDevice {
                    request,
                    code: None,
                })));
            }
            let provider = device_provider(
                explicit_model_provider(
                    &*startup.current.read().await,
                    startup.oss_provider.as_deref(),
                ),
                thread,
            );
            match crate::gateway::start_device(
                provider.as_deref(),
                &ids,
                &send,
                crate::gateway::prefer_device_login(),
            )
            .await
            {
                Ok(code) => Ev::GatewayDevice { request, code },
                // `run_ratatui_app` returns the error.
                Err(message) => return Some(AppEv::StartFailed(message)),
            }
        }
        Ask::FailLaunch { message } => {
            // `run_gateway_device_login` returns the sign-in's error.
            return Some(AppEv::StartFailed(message));
        }
        Ask::CancelGatewayOAuth => {
            // `authenticate_gateway`: the oracle ignores the answer.
            let _ = send(wire::gateway_oauth_cancel(ids.next())).await;
            return None;
        }
        Ask::CancelLogin { login_id } => {
            // `cancel_login_attempt`: the oracle ignores the answer.
            let _ = send(wire::login_cancel(&login_id, ids.next())).await;
            return None;
        }
    };
    Some(AppEv::Screen(ScreenEv::Onboarding(ev)))
}

/// The recovery menu of a launch whose shared server lacks the settings the command line
/// asked for (`daemon_recovery.rs::check`), kept in the slot for its answer.
async fn daemon_prompt<F, Fut>(startup: &Startup, cwd: &str, ids: &Ids, send: &F) -> Option<Prompt>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    if startup.target != Target::LocalDaemon {
        return None;
    }
    let mismatch = crate::daemon::compatibility(startup.daemon_features.clone(), cwd, ids, send)
        .await
        .err()?;
    let prompt = mismatch.prompt(crate::daemon::managed(&mismatch).await);
    if let Ok(mut slot) = startup.daemon.lock() {
        *slot = Some(mismatch);
    }
    Some(Prompt::Daemon(prompt))
}

/// The answer of the recovery menu (`daemon_recovery.rs::check`): Cancel ends the launch with
/// the compatibility error; Run without moves the app to the embedded server with the warning;
/// Restart restarts the managed daemon and checks once more, never in a loop.
async fn daemon_choice(startup: &Startup, choice: DaemonChoice) -> Option<AppEv> {
    let mismatch = startup.daemon.lock().ok()?.take()?;
    let config = startup.current.read().await.clone();
    let (switch, check) = match choice {
        DaemonChoice::Cancel => return Some(AppEv::StartFailed(mismatch.error_text())),
        DaemonChoice::RunWithout => (
            Switch::Embedded {
                warning: mismatch.fallback_warning(),
            },
            false,
        ),
        DaemonChoice::Restart => {
            let Some(features) = mismatch.restart_features.clone() else {
                return Some(AppEv::StartFailed(mismatch.error_text()));
            };
            (Switch::Daemon { features }, true)
        }
    };
    let (done, moved) = tokio::sync::oneshot::channel();
    let restart = Restart {
        config: config.clone(),
        done: Some(done),
        switch: Some(switch),
    };
    startup.restart.send(restart).ok()?;
    if let Err(message) = moved.await.ok()? {
        return Some(AppEv::StartFailed(message));
    }
    if !check {
        return None;
    }
    // One check after the restart; a daemon that still differs ends the launch.
    let recheck = crate::daemon::recheck(
        startup.daemon_features.clone(),
        &config.cwd.display().to_string(),
    )
    .await;
    recheck
        .err()
        .map(|mismatch| AppEv::StartFailed(mismatch.error_text()))
}

/// The launch facts; the error text of one that could not be read.
async fn facts<F, Fut>(startup: &Startup, ids: &Ids, send: &F) -> Result<Facts, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let config = &startup.config;
    // `updates.rs` is compiled out of debug builds: no offer there.
    let updates = &startup.updates;
    let checks = updates.enabled
        && config.check_for_update_on_startup
        && !wire::is_source_build_version(&updates.version);
    // `skip_update_prompt`: a launch with a prompt offers none; the refresh below still runs.
    let update = if checks && !startup.launch_prompt {
        let action = updates.action;
        let cache = read_version_cache(&version_file(config));
        wire::update_offer(
            &updates.version,
            cache.as_ref(),
            action.map(UpdateAction::command_str),
        )
    } else {
        None
    };
    // `get_login_status`: a failed read ends the launch.
    let account = send(wire::account_read(ids.next()))
        .await
        .map_err(|error| format!("account/read failed: {error}"))?;
    let sign_in = wire::sign_in_offer(account, login_policy(startup)).map_err(|error| {
        let (RpcError::Remote(text) | RpcError::Malformed(text)) = &error else {
            return "account/read failed".to_owned();
        };
        format!("account/read failed: {text}")
    })?;
    // `(!uses_remote_workspace || remote_cwd_override.is_some())`.
    let asks_trust =
        startup.check_trust && (!startup.target.remote_workspace() || startup.remote_cwd.is_some());
    let trust = if asks_trust {
        trust(startup, ids, send).await?
    } else {
        None
    };
    Ok(Facts {
        update,
        welcome: Some(Welcome {
            animations: config.animations && config.tui_effects.welcome,
        }),
        sign_in,
        trust,
        // `get_upgrade_version`: a missing or stale cache is refreshed in the
        // background, so the launch is not blocked by a network call: this
        // run reads the previous value, the next one shows the offer.
        refresh_update: checks && version_cache_needs_refresh(&version_file(config)),
        // `resolve_cwd_for_resume_or_fork_selection`: what decides the
        // directory a named session opens in.
        resume: codex_tui_core::screens::startup::cwd::Setting {
            current: config.cwd.display().to_string(),
            configured: config.tui_resume_cwd.map(|mode| match mode {
                codex_config::types::ResumeCwdMode::Current => {
                    codex_tui_core::screens::startup::cwd::ResumeMode::Current
                }
                codex_config::types::ResumeCwdMode::Session => {
                    codex_tui_core::screens::startup::cwd::ResumeMode::Session
                }
            }),
            cwd_override: startup.cwd_override,
            remote: startup.target.remote_workspace(),
        },
    })
}

/// `updates.rs::check_for_update`: fetches the latest version and writes the
/// cache, keeping a dismissed version.
async fn refresh_version_cache(
    file: &Path,
    action: Option<UpdateAction>,
    fetch: &Fetch,
) -> Result<(), String> {
    // `check_for_update`: a daemon update has no release to look for.
    if matches!(action, Some(UpdateAction::Daemon(_))) {
        return Ok(());
    }
    let latest = fetch(action).await?;
    store_latest_version(file, &latest).map_err(|error| error.to_string())
}

// The homebrew cask is the latest for a brew install: it can lag behind the
// release.
const HOMEBREW_CASK_API_URL: &str = "https://formulae.brew.sh/api/cask/codex.json";
const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/openai/codex/releases/latest";
const NPM_PACKAGE_URL: &str = "https://registry.npmjs.org/@openai%2fcodex";

/// `updates.rs::check_for_update`'s network side: the latest version for the
/// install method.
async fn fetch_latest(
    factory: HttpClientFactory,
    action: Option<UpdateAction>,
) -> anyhow::Result<String> {
    let pool =
        RouteAwareClientPool::with_chatgpt_cloudflare_cookies(factory, ClientRouteClass::Other)
            .with_legacy_custom_ca_fallback();
    let get = |url: &'static str| pool.get(url).headers(default_headers()).send();
    let release = || async {
        let release = get(LATEST_RELEASE_URL)
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let tag = release["tag_name"].as_str().unwrap_or_default();
        latest_from_release_tag(tag).map_err(anyhow::Error::msg)
    };
    Ok(match action {
        Some(UpdateAction::BrewUpgrade) => {
            let cask = get(HOMEBREW_CASK_API_URL)
                .await?
                .error_for_status()?
                .json::<serde_json::Value>()
                .await?;
            cask["version"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("the cask has no version"))?
        }
        Some(
            UpdateAction::NpmGlobalLatest
            | UpdateAction::BunGlobalLatest
            | UpdateAction::VitePlusGlobalLatest
            | UpdateAction::PnpmGlobalLatest,
        ) => {
            let latest = release().await?;
            let package = get(NPM_PACKAGE_URL)
                .await?
                .error_for_status()?
                .json::<serde_json::Value>()
                .await?;
            ensure_npm_version_ready(&package, &latest).map_err(anyhow::Error::msg)?;
            latest
        }
        Some(UpdateAction::Daemon(_)) => anyhow::bail!("a daemon update has no release check"),
        Some(UpdateAction::StandaloneUnix | UpdateAction::StandaloneWindows) | None => {
            release().await?
        }
    })
}

/// The S03 provider wizards built: one not built yet is not offered, a
/// sign-in choice must not lead to a missing screen.
fn wizard_built(feature: Feature) -> bool {
    matches!(
        feature,
        Feature::BedrockSetupWizard | Feature::GcpSetupWizard | Feature::GatewaySetupWizard
    )
}

/// What `should_show_provider_setup_wizard` reads of the configuration: the wizard's
/// feature is on, a gateway wizard has no required provider, and the provider is OpenAI's.
pub(crate) fn wizard_configured(config: &Config, feature: Feature) -> bool {
    config.features.enabled(feature)
        && (feature != Feature::GatewaySetupWizard
            || config
                .config_layer_stack
                .required_model_provider()
                .is_none())
        && config.model_provider_id == "openai"
}

/// The login methods allowed and the provider wizards the configuration
/// and the server offer (`OnboardingScreenArgs` in `run_ratatui_app`,
/// `should_show_provider_setup_wizard`).
fn login_policy(startup: &Startup) -> wire::LoginPolicy {
    let config = &startup.config;
    let auth = config.auth_config();
    let wizard = |feature| {
        wizard_built(feature)
            && startup.target.supports(feature)
            && wizard_configured(config, feature)
    };
    wire::LoginPolicy {
        chatgpt: auth.is_login_method_allowed(ForcedLoginMethod::Chatgpt),
        api_key: auth.is_login_method_allowed(ForcedLoginMethod::Api),
        wizards: Providers {
            bedrock: wizard(Feature::BedrockSetupWizard),
            gcp: wizard(Feature::GcpSetupWizard),
            gateway: wizard(Feature::GatewaySetupWizard) && !provider_flag(startup),
        },
        env_api_key: read_openai_api_key_from_env(),
    }
}

/// The provider the device-code sign-in is for (`device_login::start_if_needed`):
/// the launch's own, overwritten by the resumed thread's when the launch chose
/// none or the server still holds the thread in memory (it is reattached with
/// its current provider; a provider override can take effect only when the
/// server loads the thread from disk).
fn device_provider(selected: Option<String>, thread: Option<Resumed>) -> Option<String> {
    match thread {
        Some(thread) if selected.is_none() || thread.loaded => Some(thread.provider),
        _ => selected,
    }
}

/// `explicit_model_provider`: the provider the launch chose. `--oss` is the
/// override, and a provider the managed requirements pin beats it; otherwise
/// the invocation (a session flag) or a named profile, none when the
/// config's own default stands.
fn explicit_model_provider(config: &Config, launch: Option<&str>) -> Option<String> {
    launch
        .map(|provider| {
            config
                .config_layer_stack
                .required_model_provider()
                .unwrap_or(provider)
                .to_owned()
        })
        .or_else(|| explicit_provider(config))
}

/// `explicit_provider`: the provider a session flag or a named profile chose.
fn explicit_provider(config: &Config) -> Option<String> {
    config
        .config_layer_stack
        .layers_high_to_low()
        .into_iter()
        .find(|layer| layer.config.get("model_provider").is_some())
        .filter(|layer| {
            matches!(
                layer.name,
                ConfigLayerSource::SessionFlags
                    | ConfigLayerSource::User {
                        profile: Some(_),
                        ..
                    }
            )
        })
        .map(|_| config.model_provider_id.clone())
}

/// `client_provider_override_for_new_chats` is some: the launch chose a
/// model provider (`-c model_provider=..`, a session flag) for an embedded
/// server. A remote one offers no gateway wizard at all.
fn provider_flag(startup: &Startup) -> bool {
    startup.target == Target::Embedded
        && startup
            .config
            .config_layer_stack
            .layers_high_to_low()
            .into_iter()
            .any(|layer| {
                layer.name == ConfigLayerSource::SessionFlags
                    && layer.config.get("model_provider").is_some()
            })
}

/// The folder trust to ask at launch (`check_directory_trust` with its
/// defaults: no resumed task, the agents overview as the cancel): the local
/// folder, or the remote workspace's `--cd` folder.
async fn trust<F, Fut>(startup: &Startup, ids: &Ids, send: &F) -> Result<Option<TrustAsk>, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let lookup = match startup
        .remote_cwd
        .as_deref()
        .filter(|_| startup.target.remote_workspace())
    {
        Some(cwd) => remote_lookup(cwd, ids, send).await?.1,
        None => local_lookup(&startup.config.cwd, ids, send).await?,
    };
    Ok(lookup.map(|lookup| TrustAsk {
        cwd: lookup.cwd,
        target: lookup.target,
        restricted: lookup.restricted,
        existing_task: false,
        cancel: TrustCancel::AgentsOverview,
        sandbox_hint: false,
        error: None,
        decision: None,
    }))
}

/// Resolve the destination and ask about an undecided or restricted local folder.
#[derive(Debug, Default)]
struct ConsentCheck {
    ask: Option<TrustAsk>,
    checked: Option<Checked>,
}

async fn consent<F, Fut>(
    startup: &Startup,
    target: ConsentTarget,
    cancel: TrustCancel,
    ids: &Ids,
    send: &F,
) -> Result<ConsentCheck, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let destination = matches!(target, ConsentTarget::BackendSwitch);
    let task = matches!(
        target,
        ConsentTarget::Task(_) | ConsentTarget::TaskRecheck { .. }
    );
    let existing_task = startup.target != Target::Embedded && task;
    let daemon_task = startup.target == Target::LocalDaemon && task;
    if matches!(target, ConsentTarget::TaskRecheck { .. }) && !daemon_task {
        return Err("The task's connection changed while checking folder trust".to_owned());
    }
    let (cwd, lookup) = if startup.target.remote_workspace() {
        let Some(cwd) = startup
            .remote_cwd
            .as_deref()
            .filter(|_| destination || cancel != TrustCancel::CurrentTask)
        else {
            return Ok(ConsentCheck::default());
        };
        remote_lookup(cwd, ids, send).await?
    } else {
        let current = startup.current.read().await.clone();
        let folder = match target {
            ConsentTarget::Folder(None) => startup.config.cwd.clone(),
            ConsentTarget::Folder(Some(cwd)) => AbsolutePathBuf::from_absolute_path(&cwd)
                .map_err(|_| format!("Cannot check folder trust: {cwd} is not an absolute path"))?,
            ConsentTarget::Task(thread) => task_folder(&thread, ids, send).await?,
            ConsentTarget::TaskRecheck { thread, checked } => {
                let folder = task_folder(&thread, ids, send).await?;
                if checked
                    .iter()
                    .any(|path| Path::new(path) == folder.as_path())
                {
                    return Ok(ConsentCheck {
                        ask: None,
                        checked: Some(Checked::ResumeIn(folder_text(folder.as_path())?)),
                    });
                }
                folder
            }
            ConsentTarget::BackendSwitch => current.cwd.clone(),
        };
        let lookup = local_lookup(&folder, ids, send).await?;
        if destination
            && !current.active_project.is_untrusted()
            && lookup.as_ref().is_some_and(|lookup| lookup.restricted)
        {
            return Err(
                "Folder trust changed. Reopen the destination with its restricted settings."
                    .to_owned(),
            );
        }
        (folder_text(folder.as_path())?, lookup)
    };
    let ask = lookup
        .filter(|lookup| destination || !lookup.restricted || cancel != TrustCancel::CurrentTask)
        .map(|lookup| TrustAsk {
            cwd: lookup.cwd,
            target: lookup.target,
            restricted: lookup.restricted,
            existing_task,
            cancel,
            ..TrustAsk::default()
        });
    let checked = if destination {
        Some(Checked::Destination(cwd))
    } else if daemon_task {
        Some(if ask.is_some() {
            Checked::Task(cwd)
        } else {
            Checked::ResumeIn(cwd)
        })
    } else {
        None
    };
    Ok(ConsentCheck { ask, checked })
}

async fn task_folder<F, Fut>(
    thread: &ThreadId,
    ids: &Ids,
    send: &F,
) -> Result<AbsolutePathBuf, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let ask = ThreadAsk::Read {
        thread: thread.to_string(),
    };
    let response = send(ask.wire(ids.next()))
        .await
        .map_err(|error| format!("thread/read failed: {error}"))?;
    match ask.decode(response).map_err(|error| match error {
        codex_tui_core::rpc::RpcError::Malformed(message) => message,
        _ => "thread/read: unexpected reply".to_owned(),
    })? {
        ThreadReply::Read { thread, .. } => Ok(thread.cwd),
        _ => Err("thread/read: unexpected reply".to_owned()),
    }
}

fn folder_text(cwd: &Path) -> Result<String, String> {
    cwd.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "Cannot check folder trust: directory path is not valid UTF-8".to_owned())
}

fn trust_read_error(error: ServerError) -> String {
    format!("config/read failed while checking remote project trust: {error}")
}

/// `read_remote_project_trust` for the local host.
async fn local_lookup<F, Fut>(
    folder: &AbsolutePathBuf,
    ids: &Ids,
    send: &F,
) -> Result<Option<wire::Lookup>, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let cwd = folder.as_path();
    let cwd_text = folder_text(cwd)?;
    let response = send(wire::trust_read(&cwd_text, ids.next()))
        .await
        .map_err(trust_read_error)?;
    let roots = local_roots(folder, &response).await?;
    Ok(wire::trust_lookup(
        &response,
        &cwd_text,
        &normalized_project_trust_keys(cwd),
        roots.as_ref(),
        cfg!(windows),
    ))
}

/// `read_remote_project_trust` for a remote host, and what
/// `check_directory_trust` makes of it: a saved untrusted folder is not asked
/// (the existing behaviour of remote connections).
async fn remote_lookup<F, Fut>(
    cwd: &Path,
    ids: &Ids,
    send: &F,
) -> Result<(String, Option<wire::Lookup>), String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let cwd_text = folder_text(cwd)?;
    let uri = match LegacyAppPathString::from_string(cwd_text.clone()).to_inferred_path_uri() {
        Some(uri) => uri,
        // A relative folder: the environment's cwd is its base.
        None => {
            let info = send(ClientRequest::EnvironmentInfo {
                request_id: ids.next(),
                params: EnvironmentInfoParams {
                    environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
                },
            })
            .await
            .map_err(|error| {
                format!("environment/info failed while resolving remote project trust: {error}")
            })?;
            let base = serde_json::from_value::<Option<PathUri>>(info["cwd"].clone())
                .ok()
                .flatten()
                .ok_or("remote app server did not provide a cwd")?;
            base.join(&cwd_text).map_err(|error| {
                format!("failed to resolve the remote project directory: {error}")
            })?
        }
    };
    let cwd = uri.inferred_native_path_string();
    let response = send(wire::trust_read(&cwd, ids.next()))
        .await
        .map_err(trust_read_error)?;
    let windows = uri.infer_path_convention() == Some(PathConvention::Windows);
    let key = if windows {
        cwd.to_ascii_lowercase()
    } else {
        cwd.clone()
    };
    let Some(lookup) = wire::trust_lookup(&response, &cwd, &[key], None, windows) else {
        return Ok((cwd, None));
    };
    let has_project_layer = response["layers"].as_array().is_some_and(|layers| {
        layers
            .iter()
            .any(|layer| layer["name"]["type"] == "project")
    });
    let inside_untrusted = !lookup.restricted
        && !has_project_layer
        && response["config"]["projects"]
            .as_object()
            .into_iter()
            .flatten()
            .any(|(path, project)| {
                project["trust_level"].as_str() == Some("untrusted")
                    && LegacyAppPathString::from_string(path.clone())
                        .to_inferred_path_uri()
                        .is_some_and(|project| uri.starts_with(&project))
            });
    if inside_untrusted {
        return Err("remote project directory is inside an explicitly untrusted project; pass the repository root explicitly with --cd".to_owned());
    }
    Ok((cwd, (!lookup.restricted).then_some(lookup)))
}

/// The project and repository roots of `cwd` on this machine: read only
/// where `config/read` found no project layer, as the oracle does, and a
/// read that fails is an error.
async fn local_roots(
    cwd: &AbsolutePathBuf,
    response: &serde_json::Value,
) -> Result<Option<wire::LocalRoots>, String> {
    let has_project_layer = response["layers"].as_array().is_some_and(|layers| {
        layers
            .iter()
            .any(|layer| layer["name"]["type"] == "project")
    });
    if has_project_layer {
        return Ok(None);
    }
    let markers = serde_json::from_value::<Option<Vec<String>>>(
        response["config"]["project_root_markers"].clone(),
    )
    .map_err(|error| error.to_string())?
    .unwrap_or_else(default_project_root_markers);
    let fs = LOCAL_FS.as_ref();
    let project_root = discover_project_root(fs, cwd, &markers)
        .await
        .map_err(|error| error.to_string())?;
    let git_root = resolve_root_git_project_for_trust(fs, cwd).await;
    let projectless = project_root.is_none()
        && discover_project_root(fs, cwd, &default_project_root_markers())
            .await
            .map_err(|error| error.to_string())?
            .is_none();
    Ok(Some(wire::LocalRoots {
        project_root: normalized_project_trust_keys(project_root.as_ref().unwrap_or(cwd).as_path()),
        git_root: git_root.map(|root| normalized_project_trust_keys(root.as_path())),
        projectless,
    }))
}

/// `updates_cache.rs::version_filepath`.
pub(crate) fn version_file(config: &Config) -> PathBuf {
    config.codex_home.join("version.json").into_path_buf()
}

/// `UpdateAction::from_install_context`.
fn update_action(context: &InstallContext) -> Option<UpdateAction> {
    Some(match &context.method {
        InstallMethod::Npm => UpdateAction::NpmGlobalLatest,
        InstallMethod::Bun => UpdateAction::BunGlobalLatest,
        InstallMethod::VitePlus => UpdateAction::VitePlusGlobalLatest,
        InstallMethod::Pnpm => UpdateAction::PnpmGlobalLatest,
        InstallMethod::Brew => UpdateAction::BrewUpgrade,
        InstallMethod::Standalone { platform, .. } => match platform {
            StandalonePlatform::Unix => UpdateAction::StandaloneUnix,
            StandalonePlatform::Windows => UpdateAction::StandaloneWindows,
        },
        InstallMethod::Other => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use codex_tui_rtwk::services::startup::write_version_cache;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    async fn startup_with(target: Target, browser: Browser) -> (Startup, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("a temp dir");
        let config = codex_core::config::ConfigBuilder::default()
            .codex_home(dir.path().to_path_buf())
            .fallback_cwd(Some(dir.path().to_path_buf()))
            .build()
            .await
            .expect("a config");
        let config = Arc::new(config);
        let startup = Startup {
            alternate_screen: true,
            // A copy: a test may still change `config` alone.
            current: Arc::new(tokio::sync::RwLock::new(Arc::new((*config).clone()))),
            restart: tokio::sync::mpsc::unbounded_channel().0,
            config,
            connected: false,
            check_trust: true,
            remote_cwd: None,
            cwd_override: false,
            oss_provider: None,
            launch_prompt: false,
            update: UpdateSlot::default(),
            reload: Arc::new(|| Box::pin(std::future::ready(Err("no reload".to_owned())))),
            updates: Updates {
                enabled: false,
                version: "1.0.0".to_owned(),
                action: None,
                fetch: Arc::new(|_| Box::pin(std::future::ready(Err("no network".to_owned())))),
            },
            target,
            browser,
            analytics: Default::default(),
            daemon_features: Default::default(),
            daemon: Default::default(),
        };
        (startup, dir)
    }

    async fn nothing(_: ClientRequest) -> Result<serde_json::Value, ServerError> {
        Ok(serde_json::Value::Null)
    }

    fn counting(calls: &Arc<AtomicUsize>, result: std::io::Result<()>) -> Browser {
        let calls = calls.clone();
        let result = std::io::Result::<()>::map_err(result, |e| e.kind());
        Arc::new(move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            result.map_err(std::io::Error::from)
        })
    }

    // oracle `show_chatgpt_login_in_browser`: the URL is opened for a local
    // workspace.
    #[tokio::test]
    async fn the_login_url_opens_through_the_seam_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (startup, _dir) = startup_with(Target::Embedded, counting(&calls, Ok(()))).await;
        let ev = run(
            startup,
            Ask::OpenBrowser {
                url: "https://auth.example".into(),
            },
            Ids::default(),
            nothing,
        )
        .await;
        assert!(ev.is_none(), "nothing answers an open");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    // The oracle only logs a failed open (`tracing::warn!`); the page stays
    // on the browser step, the host answers nothing and does not fail.
    #[tokio::test]
    async fn a_browser_that_fails_to_open_is_only_logged() {
        let calls = Arc::new(AtomicUsize::new(0));
        let failing = counting(&calls, Err(std::io::ErrorKind::NotFound.into()));
        let (startup, _dir) = startup_with(Target::LocalDaemon, failing).await;
        let ev = run(
            startup,
            Ask::OpenBrowser {
                url: "https://auth.example".into(),
            },
            Ids::default(),
            nothing,
        )
        .await;
        assert!(ev.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    // `uses_remote_workspace`: the login's callback is not on this machine.
    #[tokio::test]
    async fn a_remote_workspace_opens_no_browser() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (startup, _dir) = startup_with(Target::Remote, counting(&calls, Ok(()))).await;
        let ev = run(
            startup,
            Ask::OpenBrowser {
                url: "https://auth.example".into(),
            },
            Ids::default(),
            nothing,
        )
        .await;
        assert!(ev.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    // The open blocks (it may start a process and wait): on a one-thread
    // runtime another task must still run meanwhile.
    #[tokio::test]
    async fn opening_the_browser_does_not_block_the_runtime() {
        let ticked = Arc::new(AtomicBool::new(false));
        let seen = Arc::new(AtomicBool::new(false));
        let browser: Browser = {
            let (ticked, seen) = (ticked.clone(), seen.clone());
            Arc::new(move |_| {
                std::thread::sleep(Duration::from_millis(300));
                seen.store(ticked.load(Ordering::SeqCst), Ordering::SeqCst);
                Ok(())
            })
        };
        let (startup, _dir) = startup_with(Target::Embedded, browser).await;
        let flag = ticked.clone();
        let ticker = async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            flag.store(true, Ordering::SeqCst);
        };
        let open = run(
            startup,
            Ask::OpenBrowser {
                url: "https://auth.example".into(),
            },
            Ids::default(),
            nothing,
        );
        tokio::join!(open, ticker);
        assert!(
            seen.load(Ordering::SeqCst),
            "another task ran while the browser opened"
        );
    }

    fn mismatch() -> Mismatch {
        Mismatch {
            reason: "This session requires a to be enabled".to_owned(),
            restart_features: Some(std::collections::BTreeMap::from([("a".to_owned(), true)])),
        }
    }

    // `daemon_recovery.rs`: Cancel (and Ctrl-C) ends the launch with the compatibility error,
    // in the slot `main` reads.
    #[tokio::test]
    async fn cancelling_the_daemon_menu_ends_the_launch_with_the_compatibility_error() {
        let (startup, _dir) = startup_with(Target::LocalDaemon, system_browser()).await;
        *startup.daemon.lock().unwrap() = Some(mismatch());
        let ev = run(
            startup.clone(),
            Ask::DaemonChoice {
                choice: DaemonChoice::Cancel,
            },
            Ids::default(),
            nothing,
        )
        .await;
        let text = format!(
            "Cannot use the shared background server: This session requires a to be enabled.\n{}",
            crate::daemon::FAILURE_HINT
        );
        assert!(matches!(&ev, Some(AppEv::StartFailed(message)) if *message == text));
    }

    // Run without: the app moves to the embedded server with the warning, and the answer waits
    // for the move.
    #[tokio::test]
    async fn running_without_the_daemon_moves_the_app_to_the_embedded_server() {
        let (mut startup, _dir) = startup_with(Target::LocalDaemon, system_browser()).await;
        *startup.daemon.lock().unwrap() = Some(mismatch());
        let (restart, mut receiver) = restarting();
        startup.restart = restart;
        let (ev, seen) = tokio::join!(
            run(
                startup,
                Ask::DaemonChoice {
                    choice: DaemonChoice::RunWithout,
                },
                Ids::default(),
                nothing,
            ),
            async {
                let restart = receiver.recv().await.expect("a restart");
                let warning = match &restart.switch {
                    Some(Switch::Embedded { warning }) => warning.clone(),
                    _ => panic!("the embedded server"),
                };
                if let Some(done) = restart.done {
                    let _ = done.send(Ok(()));
                }
                warning
            }
        );
        assert!(ev.is_none());
        assert_eq!(
            seen,
            "Running without the shared background server: This session requires a to be enabled."
        );
    }

    // Restart asks the managed daemon to restart with the settings the menu showed; a server
    // that did not come up has ended the app (nothing to answer, no second check).
    #[tokio::test]
    async fn restarting_the_daemon_hands_the_settings_to_the_pump() {
        let (mut startup, _dir) = startup_with(Target::LocalDaemon, system_browser()).await;
        *startup.daemon.lock().unwrap() = Some(mismatch());
        let (restart, mut receiver) = restarting();
        startup.restart = restart;
        let (ev, features) = tokio::join!(
            run(
                startup,
                Ask::DaemonChoice {
                    choice: DaemonChoice::Restart,
                },
                Ids::default(),
                nothing,
            ),
            async {
                let restart = receiver.recv().await.expect("a restart");
                match restart.switch {
                    Some(Switch::Daemon { features }) => features,
                    _ => panic!("the daemon"),
                }
                // `done` drops here: the restart failed.
            }
        );
        assert!(ev.is_none());
        assert_eq!(
            features,
            std::collections::BTreeMap::from([("a".to_owned(), true)])
        );
    }

    // The menu opens first among the launch's prompts, only for a daemon whose settings differ;
    // a server that cannot answer is not restartable (no managed-daemon probe is made).
    #[tokio::test]
    async fn a_daemon_that_cannot_answer_opens_the_menu_without_a_restart() {
        let (mut startup, _dir) = startup_with(Target::LocalDaemon, system_browser()).await;
        startup.daemon_features = std::collections::BTreeMap::from([("a".to_owned(), true)]);
        let send = |request: ClientRequest| {
            std::future::ready(match request {
                ClientRequest::ConfigRead { .. } => Ok(serde_json::json!({"config": {}})),
                ClientRequest::ConfigRequirementsRead { .. } => {
                    Ok(serde_json::json!({"requirements": null}))
                }
                ClientRequest::ExperimentalFeatureList { .. } => Err(ServerError {
                    code: Some(-32601),
                    message: "method not found".into(),
                    data: None,
                }),
                other => panic!("unexpected request {other:?}"),
            })
        };
        let prompt = daemon_prompt(&startup, "/work", &Ids::default(), &send).await;
        let Some(Prompt::Daemon(prompt)) = prompt else {
            panic!("the menu opens: {prompt:?}");
        };
        assert_eq!(prompt.reason, "Experimental feature request failed");
        assert!(!prompt.can_restart());
        assert!(startup.daemon.lock().unwrap().is_some());
        // A launch that asked nothing of the server is not checked.
        startup.daemon_features.clear();
        assert!(
            daemon_prompt(&startup, "/work", &Ids::default(), &send)
                .await
                .is_none()
        );
    }

    // The S03 wizards are all built: the host offers each the configuration
    // and the server allow (`wizard_built` keeps a future one back).
    #[tokio::test]
    async fn the_built_provider_wizards_are_offered() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let config = Arc::get_mut(&mut startup.config).expect("one owner");
        for feature in [
            Feature::BedrockSetupWizard,
            Feature::GcpSetupWizard,
            Feature::GatewaySetupWizard,
        ] {
            config.features.enable(feature).expect("a feature");
        }
        assert_eq!(
            login_policy(&startup).wizards,
            Providers {
                bedrock: true,
                gcp: true,
                gateway: true,
            }
        );
    }

    /// The server answers `account/read` and `config/read` as given.
    fn serve(
        account: Result<serde_json::Value, ServerError>,
        config: Result<serde_json::Value, ServerError>,
    ) -> impl Fn(ClientRequest) -> std::future::Ready<Result<serde_json::Value, ServerError>> {
        move |request| {
            std::future::ready(match request {
                ClientRequest::GetAccount { .. } => account.clone(),
                ClientRequest::ConfigRead { .. } => config.clone(),
                other => panic!("unexpected request {other:?}"),
            })
        }
    }

    fn signed_out() -> serde_json::Value {
        serde_json::json!({"account": null, "requiresOpenaiAuth": false})
    }

    fn refused(message: &str) -> ServerError {
        ServerError {
            code: Some(-32603),
            data: None,
            message: message.to_owned(),
        }
    }

    async fn facts_ask(
        startup: &Startup,
        send: impl Fn(ClientRequest) -> std::future::Ready<Result<serde_json::Value, ServerError>>,
    ) -> Option<AppEv> {
        run(
            startup.clone(),
            Ask::Facts { request: 1 },
            Ids::default(),
            send,
        )
        .await
    }

    // oracle `get_login_status(..)?`: a failed `account/read` ends the launch
    // with `account/read failed: <error>`.
    #[tokio::test]
    async fn an_account_read_error_fails_the_launch() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let ev = facts_ask(
            &startup,
            serve(Err(refused("no account")), Ok(serde_json::json!({}))),
        )
        .await;
        let Some(AppEv::StartFailed(message)) = ev else {
            panic!("expected StartFailed, got {ev:?}");
        };
        assert_eq!(message, "account/read failed: no account");
    }

    // oracle `read_remote_project_trust`: `config/read failed while checking
    // remote project trust` ends the launch.
    #[tokio::test]
    async fn a_trust_read_error_fails_the_launch() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let ev = facts_ask(&startup, serve(Ok(signed_out()), Err(refused("no config")))).await;
        let Some(AppEv::StartFailed(message)) = ev else {
            panic!("expected StartFailed, got {ev:?}");
        };
        assert_eq!(
            message,
            "config/read failed while checking remote project trust: no config"
        );
    }

    #[tokio::test]
    async fn facts_that_read_fine_leave_the_launch_alone() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let config = serde_json::json!({"config": {}, "layers": []});
        let ev = facts_ask(&startup, serve(Ok(signed_out()), Ok(config))).await;
        assert!(matches!(
            ev,
            Some(AppEv::Screen(ScreenEv::Onboarding(Ev::Facts { .. })))
        ));
    }

    fn fetching(calls: &Arc<AtomicUsize>, latest: Result<&str, &str>) -> Fetch {
        let calls = calls.clone();
        let latest = latest.map(str::to_owned).map_err(str::to_owned);
        Arc::new(move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(std::future::ready(latest.clone()))
        })
    }

    /// The version cache a launch starts from.
    #[derive(Clone, Copy)]
    enum Cache {
        Missing,
        /// Checked in 2020: older than twenty hours.
        Old,
        /// Checked just now.
        Fresh,
    }

    /// The launch with the update check on: the version file and whether
    /// the facts ask for a refresh. Nothing is fetched by the facts.
    async fn launch_with(
        startup: &mut Startup,
        fetch: Fetch,
        cache: Cache,
        version: &str,
    ) -> (std::path::PathBuf, bool) {
        let (file, facts) = launch_facts(startup, fetch, cache, version).await;
        (file, facts.refresh_update)
    }

    /// The facts of a launch with the update check on, and the version file.
    async fn launch_facts(
        startup: &mut Startup,
        fetch: Fetch,
        cache: Cache,
        version: &str,
    ) -> (std::path::PathBuf, Facts) {
        startup.updates = Updates {
            enabled: true,
            version: version.to_owned(),
            action: startup.updates.action,
            fetch,
        };
        let file = version_file(&startup.config);
        match cache {
            Cache::Missing => {}
            Cache::Old => write_version_cache(
                &file,
                "2.0.0",
                Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            )
            .expect("the cache"),
            Cache::Fresh => store_latest_version(&file, "2.0.0").expect("the cache"),
        }
        let config = serde_json::json!({"config": {}, "layers": []});
        let ev = facts_ask(startup, serve(Ok(signed_out()), Ok(config))).await;
        let Some(AppEv::Screen(ScreenEv::Onboarding(Ev::Facts { facts, .. }))) = ev else {
            panic!("expected the facts, got {ev:?}");
        };
        (file, facts)
    }

    async fn refresh(startup: &Startup) -> Option<AppEv> {
        run(
            startup.clone(),
            Ask::RefreshUpdateCache,
            Ids::default(),
            nothing,
        )
        .await
    }

    // `get_upgrade_version`: no cache -> refreshed in the background; the
    // launch reads the cache as it was (none), the next run reads the new one.
    #[tokio::test]
    async fn a_missing_cache_is_refreshed_in_the_background() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch = fetching(&calls, Ok("9.9.9"));
        let (file, asked) = launch_with(&mut startup, fetch, Cache::Missing, "1.0.0").await;
        assert!(asked);
        assert_eq!(calls.load(Ordering::SeqCst), 0, "the facts fetch nothing");
        assert!(refresh(&startup).await.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            read_version_cache(&file).map(|cache| cache.latest),
            Some("9.9.9".to_owned())
        );
    }

    #[tokio::test]
    async fn a_cache_older_than_twenty_hours_is_refreshed() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch = fetching(&calls, Ok("2.1.0"));
        let (file, asked) = launch_with(&mut startup, fetch, Cache::Old, "1.0.0").await;
        assert!(asked);
        refresh(&startup).await;
        assert_eq!(
            read_version_cache(&file).map(|cache| cache.latest),
            Some("2.1.0".to_owned())
        );
    }

    #[tokio::test]
    async fn a_fresh_cache_is_not_refreshed() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch = fetching(&calls, Ok("2.1.0"));
        let (_, asked) = launch_with(&mut startup, fetch, Cache::Fresh, "1.0.0").await;
        assert!(!asked);
    }

    #[tokio::test]
    async fn a_failed_fetch_leaves_the_cache_as_it_was() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch = fetching(&calls, Err("offline"));
        let (file, _) = launch_with(&mut startup, fetch, Cache::Old, "1.0.0").await;
        assert!(refresh(&startup).await.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            read_version_cache(&file).map(|cache| cache.latest),
            Some("2.0.0".to_owned())
        );
    }

    #[tokio::test]
    async fn a_source_build_checks_nothing() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let fetch = fetching(&calls, Ok("2.1.0"));
        let (_, asked) = launch_with(&mut startup, fetch, Cache::Missing, "0.0.0").await;
        assert!(!asked);
    }

    // `update_action.rs::maps_install_context_to_update_action`.
    #[test]
    fn maps_install_context_to_update_action() {
        let release_dir =
            AbsolutePathBuf::from_absolute_path(std::env::temp_dir().join("native-release"))
                .expect("temp dir path should be absolute");
        let action = |method| {
            update_action(&InstallContext {
                method,
                package_layout: None,
            })
        };
        let standalone = |platform| InstallMethod::Standalone {
            platform,
            release_dir: release_dir.clone(),
            resources_dir: Some(release_dir.join("codex-resources")),
        };
        assert_eq!(action(InstallMethod::Other), None);
        assert_eq!(
            action(InstallMethod::Npm),
            Some(UpdateAction::NpmGlobalLatest)
        );
        assert_eq!(
            action(InstallMethod::Bun),
            Some(UpdateAction::BunGlobalLatest)
        );
        assert_eq!(
            action(InstallMethod::VitePlus),
            Some(UpdateAction::VitePlusGlobalLatest)
        );
        assert_eq!(
            action(InstallMethod::Pnpm),
            Some(UpdateAction::PnpmGlobalLatest)
        );
        assert_eq!(action(InstallMethod::Brew), Some(UpdateAction::BrewUpgrade));
        assert_eq!(
            action(standalone(StandalonePlatform::Unix)),
            Some(UpdateAction::StandaloneUnix)
        );
        assert_eq!(
            action(standalone(StandalonePlatform::Windows)),
            Some(UpdateAction::StandaloneWindows)
        );
    }

    // `skip_update_prompt` (lib.rs): a launch with a prompt offers no update, but
    // `get_upgrade_version` still runs, so a stale cache is refreshed.
    #[tokio::test]
    async fn a_launch_prompt_skips_the_offer_but_keeps_the_refresh() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        startup.updates.action = Some(UpdateAction::NpmGlobalLatest);
        let calls = Arc::new(AtomicUsize::new(0));
        let (_, facts) = launch_facts(
            &mut startup,
            fetching(&calls, Ok("2.1.0")),
            Cache::Fresh,
            "1.0.0",
        )
        .await;
        assert!(facts.update.is_some(), "no prompt: the offer is made");
        startup.launch_prompt = true;
        let (_, facts) = launch_facts(
            &mut startup,
            fetching(&calls, Ok("2.1.0")),
            Cache::Fresh,
            "1.0.0",
        )
        .await;
        assert!(facts.update.is_none(), "a prompt: no offer");
        assert!(!facts.refresh_update, "a fresh cache is not refreshed");
        let (_, facts) = launch_facts(
            &mut startup,
            fetching(&calls, Ok("2.1.0")),
            Cache::Old,
            "1.0.0",
        )
        .await;
        assert!(facts.update.is_none());
        assert!(facts.refresh_update, "the refresh still runs");
    }

    // R-F1: the update chosen is stored before the answer that lets the page
    // leave (and the app end) is sent.
    #[tokio::test]
    async fn the_update_is_stored_before_it_is_acknowledged() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        startup.updates.action = Some(UpdateAction::NpmGlobalLatest);
        let ev = run(
            startup.clone(),
            Ask::RunUpdate { request: 4 },
            Ids::default(),
            nothing,
        )
        .await;
        assert!(matches!(
            ev,
            Some(AppEv::Screen(ScreenEv::Onboarding(Ev::UpdateQueued {
                request: 4
            })))
        ));
        assert_eq!(
            *startup.update.lock().unwrap(),
            Some(UpdateAction::NpmGlobalLatest)
        );
    }

    /// The restarts the host asks of the server: the receiving end, which
    /// `restarted` answers as the pump does once the new handle is the app's.
    fn restarting() -> (
        tokio::sync::mpsc::UnboundedSender<Restart>,
        tokio::sync::mpsc::UnboundedReceiver<Restart>,
    ) {
        tokio::sync::mpsc::unbounded_channel()
    }

    /// The provider each restart was for, acknowledged in order; ends when
    /// the host has dropped its sender.
    async fn restarted(mut rx: tokio::sync::mpsc::UnboundedReceiver<Restart>) -> Vec<String> {
        let mut providers = Vec::new();
        while let Some(restart) = rx.recv().await {
            providers.push(restart.config.model_provider_id.clone());
            if let Some(done) = restart.done {
                let _ = done.send(Ok(()));
            }
        }
        providers
    }

    // `lib.rs` after `selected_gateway`: the config is read again, it must
    // select the gateway, the server restarts with it, and the page learns of
    // it only after the restart is acknowledged.
    #[tokio::test]
    async fn a_selected_gateway_restarts_the_server_and_is_answered_after_the_ack() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let config = (*startup.config).clone();
        let id = config.model_provider_id.clone();
        startup.reload = Arc::new(move || Box::pin(std::future::ready(Ok(config.clone()))));
        let (restart, receiver) = restarting();
        startup.restart = restart;
        let (ev, seen) = tokio::join!(
            run(
                startup,
                Ask::SelectGateway {
                    request: 4,
                    provider_id: id.clone(),
                },
                Ids::default(),
                nothing,
            ),
            restarted(receiver)
        );
        assert!(matches!(
            ev,
            Some(AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayStarted {
                request: 4
            })))
        ));
        assert_eq!(seen, [id]);
    }

    #[tokio::test]
    async fn a_failed_gateway_restart_answers_its_startup_ask() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let config = (*startup.config).clone();
        let provider_id = config.model_provider_id.clone();
        startup.reload = Arc::new(move || Box::pin(std::future::ready(Ok(config.clone()))));
        let (restart, mut receiver) = restarting();
        startup.restart = restart;
        let (event, ()) = tokio::join!(
            run(
                startup,
                Ask::SelectGateway {
                    request: 1,
                    provider_id
                },
                Ids::default(),
                nothing
            ),
            async {
                let restart = receiver.recv().await.expect("gateway restart");
                let done = restart.done.expect("its originating continuation");
                assert!(done.send(Err("could not install provider".into())).is_ok());
            }
        );
        assert!(
            matches!(event, Some(AppEv::StartFailed(message)) if message == "could not install provider")
        );
    }

    // The reloaded config must select that gateway: another provider ends
    // the start (the oracle's `bail!`), and no server is restarted.
    #[tokio::test]
    async fn a_reload_that_selects_another_provider_fails_the_start() {
        let (mut startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let config = (*startup.config).clone();
        startup.reload = Arc::new(move || Box::pin(std::future::ready(Ok(config.clone()))));
        let (restart, receiver) = restarting();
        startup.restart = restart;
        let (ev, seen) = tokio::join!(
            run(
                startup,
                Ask::SelectGateway {
                    request: 1,
                    provider_id: "office".into(),
                },
                Ids::default(),
                nothing,
            ),
            restarted(receiver)
        );
        let Some(AppEv::StartFailed(text)) = ev else {
            panic!("the start fails: {ev:?}");
        };
        assert!(text.ends_with("instead of office."), "{text}");
        assert!(seen.is_empty());
    }

    #[tokio::test]
    async fn a_reload_that_fails_ends_the_gateway_start_with_its_error() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let ev = run(
            startup,
            Ask::SelectGateway {
                request: 1,
                provider_id: "office".into(),
            },
            Ids::default(),
            nothing,
        )
        .await;
        assert!(
            matches!(ev, Some(AppEv::StartFailed(ref text)) if text == "Error loading configuration: no reload"),
            "{ev:?}"
        );
    }

    // `lib.rs`: a remote workspace shows no gateway sign-in, and asks the
    // server nothing; the page is told there is no code.
    #[tokio::test]
    async fn a_remote_workspace_has_no_gateway_device_code() {
        let (startup, _dir) = startup_with(Target::Remote, system_browser()).await;
        let ev = run(
            startup,
            Ask::GatewayDevice {
                request: 2,
                thread: None,
            },
            Ids::default(),
            |request: ClientRequest| async move { panic!("asked {request:?}") },
        )
        .await;
        assert!(matches!(
            ev,
            Some(AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayDevice {
                request: 2,
                code: None
            })))
        ));
    }

    // A server that has no sign-in to start answers with no code, the config's
    // own provider is read for (none named).
    #[tokio::test]
    async fn a_gateway_without_a_sign_in_answers_no_code() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let asked = Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = {
            let asked = asked.clone();
            move |request: ClientRequest| {
                let asked = asked.clone();
                async move {
                    if let ClientRequest::GatewayOAuthRead { params, .. } = &request {
                        asked.lock().unwrap().push(params.is_some());
                    }
                    Ok::<_, ServerError>(serde_json::json!({
                        "providerId": "openai",
                        "providerName": "OpenAI",
                        "required": false,
                        "supportsDeviceAuth": false,
                        "status": null,
                        "error": null,
                    }))
                }
            }
        };
        let ev = run(
            startup,
            Ask::GatewayDevice {
                request: 2,
                thread: None,
            },
            Ids::default(),
            record,
        )
        .await;
        assert!(matches!(
            ev,
            Some(AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayDevice {
                request: 2,
                code: None
            })))
        ));
        assert_eq!(*asked.lock().unwrap(), [false]);
    }

    // `run_ratatui_app` returns the error of the read: the launch ends with
    // the client's text, in the slot `main` reads.
    #[tokio::test]
    async fn a_failed_read_ends_the_launch_with_its_text() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let ev = run(
            startup.clone(),
            Ask::GatewayDevice {
                request: 2,
                thread: None,
            },
            Ids::default(),
            |_: ClientRequest| async {
                Err::<serde_json::Value, _>(ServerError {
                    code: Some(-32603),
                    data: None,
                    message: "account/gatewayOAuth/read failed: boom (code -32603)".into(),
                })
            },
        )
        .await;
        let text = "account/gatewayOAuth/read failed: boom (code -32603)";
        assert!(matches!(&ev, Some(AppEv::StartFailed(message)) if message == text));
    }

    // The sign-in that failed ends the launch with the server's message.
    #[tokio::test]
    async fn the_failed_sign_in_ends_the_launch_with_its_message() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let ev = run(
            startup.clone(),
            Ask::FailLaunch {
                message: "Denied".into(),
            },
            Ids::default(),
            nothing,
        )
        .await;
        assert!(matches!(&ev, Some(AppEv::StartFailed(message)) if message == "Denied"));
    }

    // Nothing names a provider on a plain config.
    #[tokio::test]
    async fn a_plain_config_names_no_provider() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        assert_eq!(explicit_provider(&startup.config), None);
    }

    // `--oss` is the launch's provider; with none chosen the flags and the
    // profile speak.
    #[tokio::test]
    async fn the_oss_choice_is_the_explicit_provider() {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        assert_eq!(
            explicit_model_provider(&startup.config, Some("ollama")).as_deref(),
            Some("ollama")
        );
        assert_eq!(explicit_model_provider(&startup.config, None), None);
    }

    // The managed requirements pin the provider: the `--oss` choice does not
    // beat it (`explicit_model_provider`: `required_model_provider`).
    #[tokio::test]
    async fn a_required_provider_beats_the_oss_choice() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let requirements = dir.path().join("requirements.toml");
        std::fs::File::create(&requirements)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, b"model_provider = \"ollama\"\n")
            })
            .expect("requirements");
        let config = codex_core::config::ConfigBuilder::default()
            .codex_home(dir.path().to_path_buf())
            .fallback_cwd(Some(dir.path().to_path_buf()))
            .loader_overrides(codex_config::LoaderOverrides {
                system_requirements_path: Some(requirements),
                ..codex_config::LoaderOverrides::without_managed_config_for_tests()
            })
            .build()
            .await
            .expect("a config");
        assert_eq!(
            explicit_model_provider(&config, Some("lmstudio")).as_deref(),
            Some("ollama")
        );
    }

    // The device sign-in is for the resumed thread's provider when the launch
    // chose none, or the server holds the thread loaded; otherwise the
    // launch's own choice stands.
    #[test]
    fn a_loaded_thread_overrides_the_selected_provider() {
        let thread = |loaded| {
            Some(Resumed {
                provider: "thread".to_owned(),
                loaded,
            })
        };
        let selected = || Some("chosen".to_owned());
        assert_eq!(
            device_provider(None, thread(false)).as_deref(),
            Some("thread")
        );
        assert_eq!(
            device_provider(selected(), thread(true)).as_deref(),
            Some("thread")
        );
        assert_eq!(
            device_provider(selected(), thread(false)).as_deref(),
            Some("chosen")
        );
        assert_eq!(device_provider(selected(), None).as_deref(), Some("chosen"));
        assert_eq!(device_provider(None, None), None);
    }

    // `start_gateway_oauth`'s answer reaches the page as its result, and the
    // cancel on exit asks the server and answers nothing.
    #[tokio::test]
    async fn the_gateway_login_is_answered_and_its_cancel_answers_nothing() {
        let asked = Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = {
            let asked = asked.clone();
            move |request: ClientRequest| {
                let asked = asked.clone();
                async move {
                    asked.lock().unwrap().push(request.method_name());
                    Ok::<_, ServerError>(serde_json::json!({
                        "providerId": "office",
                        "providerName": "Office",
                        "required": false,
                        "supportsDeviceAuth": false,
                        "status": null,
                        "error": null,
                    }))
                }
            }
        };
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let ev = run(
            startup.clone(),
            Ask::GatewayOAuth {
                request: 5,
                provider_id: "office".into(),
            },
            Ids::default(),
            record.clone(),
        )
        .await;
        assert!(matches!(
            ev,
            Some(AppEv::Screen(ScreenEv::Onboarding(Ev::GatewayOAuth {
                request: 5,
                result: Ok(())
            })))
        ));
        let ev = run(startup, Ask::CancelGatewayOAuth, Ids::default(), record).await;
        assert!(ev.is_none());
        assert_eq!(
            *asked.lock().unwrap(),
            ["account/gatewayOAuth/read", "account/gatewayOAuth/cancel"]
        );
    }

    /// What `config/read` answers for `cwd` with a project layer the folder's trust disables.
    fn undecided_read(cwd: &str, saved: Option<(&str, &str)>) -> serde_json::Value {
        let projects: serde_json::Map<String, serde_json::Value> = saved
            .into_iter()
            .map(|(path, level)| (path.to_owned(), serde_json::json!({"trust_level": level})))
            .collect();
        serde_json::json!({
            "config": {"projects": projects},
            "layers": [{
                "name": {"type": "project", "dotCodexFolder": format!("{cwd}/.codex")},
                "disabledReason": format!(
                    "config.toml is disabled, add {cwd} as a trusted project in config.toml"
                ),
            }],
        })
    }

    async fn consent_of(read: serde_json::Value, cwd: &Path) -> Option<AppEv> {
        let (startup, _dir) = startup_with(Target::Embedded, system_browser()).await;
        let send = move |_: ClientRequest| std::future::ready(Ok(read.clone()));
        run(
            startup,
            Ask::CheckConsent {
                request: 7,
                target: ConsentTarget::Folder(Some(cwd.to_str().expect("utf-8").to_owned())),
                cancel: TrustCancel::CurrentTask,
            },
            Ids::default(),
            send,
        )
        .await
    }

    fn saved_task_at(cwd: &str) -> serde_json::Value {
        serde_json::json!({"thread": {
            "id": "saved", "sessionId": "saved", "preview": "", "ephemeral": false,
            "modelProvider": "openai", "createdAt": 0, "updatedAt": 0,
            "status": {"type": "notLoaded"}, "cwd": cwd, "cliVersion": "0.0.0",
            "source": "cli", "turns": [], "canAcceptDirectInput": true
        }})
    }

    #[tokio::test]
    async fn consent_lookup_of_a_task_checks_its_working_directory() {
        let (startup, _dir) = startup_with(Target::LocalDaemon, system_browser()).await;
        let task_dir = tempfile::tempdir().expect("a task folder");
        let cwd = task_dir.path().to_str().expect("utf-8");
        let key = normalized_project_trust_keys(task_dir.path()).remove(0);
        let send = |request: ClientRequest| {
            let body = serde_json::to_value(request).expect("the request");
            let response = match body["method"].as_str() {
                Some("thread/read") => {
                    assert_eq!(body["params"]["threadId"], "saved");
                    assert!(!body["params"]["includeTurns"].as_bool().unwrap_or(false));
                    saved_task_at(cwd)
                }
                Some("config/read") => {
                    assert_eq!(body["params"]["cwd"], cwd);
                    undecided_read(cwd, Some((&key, "untrusted")))
                }
                other => panic!("unexpected request {other:?}"),
            };
            std::future::ready(Ok(response))
        };

        let check = consent(
            &startup,
            ConsentTarget::Task(ThreadId::new("saved")),
            TrustCancel::AgentsOverview,
            &Ids::default(),
            &send,
        )
        .await
        .expect("the check");

        assert_eq!(check.checked, Some(Checked::Task(cwd.into())));
        let ask = check.ask.expect("a restricted task still needs consent");
        assert_eq!(ask.cwd, cwd);
        assert_eq!(ask.cancel, TrustCancel::AgentsOverview);
        assert!(ask.restricted);
        assert!(ask.existing_task);
    }

    #[tokio::test]
    async fn consent_lookup_of_a_remote_task_without_an_explicit_folder_asks_nothing() {
        let (startup, _dir) = startup_with(Target::Remote, system_browser()).await;
        let check = consent(
            &startup,
            ConsentTarget::Task(ThreadId::new("remote")),
            TrustCancel::AgentsOverview,
            &Ids::default(),
            &|_| async { panic!("the remote task's folder is not looked up") },
        )
        .await
        .expect("the check");
        assert_eq!(check.ask, None);
        assert_eq!(check.checked, None);
    }

    #[tokio::test]
    async fn a_task_recheck_reads_the_same_task_and_checks_only_a_new_cwd() {
        let (startup, _dir) = startup_with(Target::LocalDaemon, system_browser()).await;
        let task_dir = tempfile::tempdir().expect("a task folder");
        let cwd = task_dir.path().to_str().expect("utf-8");
        for checked in [cwd, "/previous/workspace"] {
            let changed = checked != cwd;
            let reads = AtomicUsize::new(0);
            let send = |request: ClientRequest| {
                let body = serde_json::to_value(request).expect("the request");
                let response = match body["method"].as_str() {
                    Some("thread/read") => {
                        assert_eq!(body["params"]["threadId"], "saved");
                        saved_task_at(cwd)
                    }
                    Some("config/read") => {
                        reads.fetch_add(1, Ordering::SeqCst);
                        assert_eq!(body["params"]["cwd"], cwd);
                        undecided_read(cwd, None)
                    }
                    other => panic!("unexpected request {other:?}"),
                };
                std::future::ready(Ok(response))
            };
            let check = consent(
                &startup,
                ConsentTarget::TaskRecheck {
                    thread: ThreadId::new("saved"),
                    checked: vec![checked.into()],
                },
                TrustCancel::AgentsOverview,
                &Ids::default(),
                &send,
            )
            .await
            .expect("the check");
            assert_eq!(reads.load(Ordering::SeqCst), usize::from(changed));
            assert_eq!(
                check.ask.map(|ask| ask.cwd),
                changed.then(|| cwd.to_owned())
            );
            assert_eq!(
                check.checked,
                Some(if changed {
                    Checked::Task(cwd.into())
                } else {
                    Checked::ResumeIn(cwd.into())
                })
            );
        }
    }

    #[tokio::test]
    async fn switching_to_an_explicit_remote_folder_checks_that_destination() {
        let (mut startup, _dir) = startup_with(Target::Remote, system_browser()).await;
        let cwd = startup.config.cwd.to_string_lossy().into_owned();
        startup.remote_cwd = Some(PathBuf::from(&cwd));
        let check = consent(
            &startup,
            ConsentTarget::BackendSwitch,
            TrustCancel::CurrentTask,
            &Ids::default(),
            &|request| {
                let body = serde_json::to_value(request).expect("the request");
                assert_eq!(body["method"], "config/read");
                assert_eq!(body["params"]["cwd"], cwd);
                std::future::ready(Ok(undecided_read(&cwd, None)))
            },
        )
        .await
        .expect("the check");
        let ask = check.ask.expect("the explicit remote destination");
        assert_eq!(ask.cwd, cwd);
        assert_eq!(ask.cancel, TrustCancel::CurrentTask);
        assert_eq!(check.checked, Some(Checked::Destination(cwd)));
    }

    // `/cd` into a folder with no saved decision: the oracle's question, with the current task's
    // cancel choice.
    #[tokio::test]
    async fn consent_lookup_of_a_folder_with_no_decision_is_asked() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let cwd = dir.path().to_str().expect("utf-8").to_owned();
        let ev = consent_of(undecided_read(&cwd, None), dir.path()).await;
        let Some(AppEv::Screen(ScreenEv::Onboarding(Ev::Consent {
            request: 7,
            ask: Some(ask),
        }))) = ev
        else {
            panic!("a question: {ev:?}");
        };
        assert_eq!(
            (ask.cwd, ask.target, ask.restricted, ask.cancel),
            (cwd.clone(), cwd, false, TrustCancel::CurrentTask)
        );
    }

    // A folder saved as untrusted is not asked at `/cd` (the oracle gates on the project having
    // no trust level): the new thread opens with its restricted settings.
    #[tokio::test]
    async fn consent_lookup_of_a_saved_untrusted_folder_is_not_asked() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let cwd = dir.path().to_str().expect("utf-8").to_owned();
        let keys = normalized_project_trust_keys(dir.path());
        let key = keys.first().expect("a key").clone();
        let ev = consent_of(undecided_read(&cwd, Some((&key, "untrusted"))), dir.path()).await;
        assert!(
            matches!(
                ev,
                Some(AppEv::Screen(ScreenEv::Onboarding(Ev::Consent {
                    request: 7,
                    ask: None
                })))
            ),
            "{ev:?}"
        );
    }
}
