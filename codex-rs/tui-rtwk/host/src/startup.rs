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
use codex_tui_core::protocol::onboarding as wire;
use codex_tui_core::rpc::RpcError;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::chat::Ev as ChatEv;
use codex_tui_core::screens::onboarding::auth::{AuthResult, Providers};
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts, TrustAsk, TrustCancel, Welcome};
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

/// What the startup asks need of the host.
#[derive(Clone)]
pub struct Startup {
    pub config: Arc<Config>,
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
    pub update: UpdateSlot,
    /// A launch fact that could not be read ends the launch with its error
    /// (`get_login_status(..)?`, `check_directory_trust(..)?`).
    pub failure: FailureSlot,
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
    pub restart: tokio::sync::mpsc::UnboundedSender<Arc<Config>>,
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
            Err(message) => return Some(fail_launch(&startup, message)),
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
        Ask::DismissUpdate { version } => {
            // The oracle only logs a failed dismissal.
            let _ = dismiss_version(&version_file(&startup.config), &version);
            return None;
        }
        Ask::RunUpdate { request } => {
            // Stored here, before the answer: the page leaves and the app
            // ends only on it, and `main` reads the slot after that.
            let Ok(mut slot) = startup.update.lock() else {
                return Some(fail_launch(
                    &startup,
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
                        let _ = startup.restart.send(config.clone());
                    }
                    let status = crate::facts::status(&config).await;
                    AppEv::Screen(ScreenEv::Chat(ChatEv::Status(Box::new(status))))
                }
                // `load_config_or_exit`.
                Err(error) => {
                    fail_launch(&startup, format!("Error loading configuration: {error}"))
                }
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
            let prompts = crate::launch::prompts(&config, startup.target, cwd, false, &ids, &send).await;
            Ev::Launched { request, prompts }
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
        Ask::AcknowledgeGovCloud => {
            let config = startup.current.read().await.clone();
            crate::providers::acknowledge_gov_cloud(&config).await;
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

/// The launch ends with `message`, as the oracle's does: the slot first, so
/// `main` finds it once the app has ended.
fn fail_launch(startup: &Startup, message: String) -> AppEv {
    if let Ok(mut slot) = startup.failure.lock() {
        *slot = Some(message.clone());
    }
    AppEv::StartFailed(message)
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
        && updates.version.trim() != SOURCE_BUILD;
    let update = if checks {
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
    })
}

/// `is_source_build_version`: no release is compared against a source build.
const SOURCE_BUILD: &str = "0.0.0";

/// `updates.rs::check_for_update`: fetches the latest version and writes the
/// cache, keeping a dismissed version.
async fn refresh_version_cache(
    file: &Path,
    action: Option<UpdateAction>,
    fetch: &Fetch,
) -> Result<(), String> {
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
        Feature::BedrockSetupWizard | Feature::GcpSetupWizard
    )
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
            && config.features.enabled(feature)
            && (feature != Feature::GatewaySetupWizard
                || config
                    .config_layer_stack
                    .required_model_provider()
                    .is_none())
            && config.model_provider_id == "openai"
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
        Some(cwd) => remote_lookup(cwd, ids, send).await?,
        None => local_lookup(&startup.config, ids, send).await?,
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

fn trust_read_error(error: ServerError) -> String {
    format!("config/read failed while checking remote project trust: {error}")
}

/// `read_remote_project_trust` for the local host.
async fn local_lookup<F, Fut>(
    config: &Config,
    ids: &Ids,
    send: &F,
) -> Result<Option<wire::Lookup>, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let cwd = config.cwd.as_path();
    let cwd_text = cwd
        .to_str()
        .ok_or("Cannot check folder trust: directory path is not valid UTF-8")?
        .to_owned();
    let response = send(wire::trust_read(&cwd_text, ids.next()))
        .await
        .map_err(trust_read_error)?;
    let roots = local_roots(&config.cwd, &response).await?;
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
) -> Result<Option<wire::Lookup>, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let cwd_text = cwd
        .to_str()
        .ok_or("Cannot check folder trust: directory path is not valid UTF-8")?
        .to_owned();
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
        return Ok(None);
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
    Ok((!lookup.restricted).then_some(lookup))
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
fn version_file(config: &Config) -> PathBuf {
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
            // A copy: a test may still change `config` alone.
            current: Arc::new(tokio::sync::RwLock::new(Arc::new((*config).clone()))),
            restart: tokio::sync::mpsc::unbounded_channel().0,
            config,
            connected: false,
            check_trust: true,
            remote_cwd: None,
            update: UpdateSlot::default(),
            failure: FailureSlot::default(),
            reload: Arc::new(|| Box::pin(std::future::ready(Err("no reload".to_owned())))),
            updates: Updates {
                enabled: false,
                version: "1.0.0".to_owned(),
                action: None,
                fetch: Arc::new(|_| Box::pin(std::future::ready(Err("no network".to_owned())))),
            },
            target,
            browser,
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

    // The S03 wizards (`providers` task) are not built yet: the host offers
    // none, whatever the configuration and the server allow, so no sign-in
    // choice leads to a screen that does not exist.
    #[tokio::test]
    async fn only_the_built_provider_wizards_are_offered() {
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
                gateway: false,
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
        assert_eq!(
            startup.failure.lock().unwrap().as_deref(),
            Some(message.as_str())
        );
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
        assert_eq!(
            startup.failure.lock().unwrap().as_deref(),
            Some(message.as_str())
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
        assert!(startup.failure.lock().unwrap().is_none());
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
        startup.updates = Updates {
            enabled: true,
            version: version.to_owned(),
            action: None,
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
        (file, facts.refresh_update)
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
}
