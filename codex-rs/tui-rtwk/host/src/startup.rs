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
use codex_config::ConfigLayerSource;
use codex_config::default_project_root_markers;
use codex_config::loader::discover_project_root;
use codex_config::loader::normalized_project_trust_keys;
use codex_config::loader::project_trust_key;
use codex_core::config::Config;
use codex_exec_server::LOCAL_FS;
use codex_features::Feature;
use codex_git_utils::resolve_root_git_project_for_trust;
use codex_install_context::InstallContext;
use codex_install_context::InstallMethod;
use codex_install_context::StandalonePlatform;
use codex_login::read_openai_api_key_from_env;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_tui_core::app::AppEv;
use codex_tui_core::protocol::onboarding as wire;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::auth::{AuthResult, Login, Providers};
use codex_tui_core::screens::onboarding::{Ask, Ev, Facts, TrustAsk, TrustCancel, Welcome};
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_tui_rtwk::services::startup::UpdateAction;
use codex_tui_rtwk::services::startup::dismiss_version;
use codex_tui_rtwk::services::startup::read_version_cache;
use codex_utils_absolute_path::AbsolutePathBuf;

/// The update the user chose to run, kept for `main` to run after the
/// terminal is restored.
pub type UpdateSlot = Arc<Mutex<Option<UpdateAction>>>;

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
    pub update: UpdateSlot,
    /// Which server the TUI talks to: what the provider wizards and the
    /// browser login are offered for (`AppServerTarget`).
    pub target: Target,
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
        Ask::Facts { request } => Ev::Facts {
            request,
            facts: facts(&startup, &ids, &send).await,
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
        Ask::RunUpdate => {
            if let Ok(mut slot) = startup.update.lock() {
                *slot = update_action(InstallContext::current());
            }
            return None;
        }
        Ask::Login { request, login } => {
            let result = send(wire::login_start(&login, ids.next()))
                .await
                .map_err(|error| error.message);
            let outcome = wire::login_outcome(&login, result);
            // `show_chatgpt_login_in_browser`: the browser opens where the
            // login's callback is; the oracle only logs a failure.
            if let (
                Login::ChatGpt,
                codex_tui_core::screens::onboarding::auth::Outcome::Browser { url, .. },
            ) = (&login, &outcome)
                && !startup.target.remote_workspace()
            {
                let _ = webbrowser::open(url);
            }
            Ev::Auth(AuthResult { request, outcome })
        }
        Ask::CancelLogin { login_id } => {
            // `cancel_login_attempt`: the oracle ignores the answer.
            let _ = send(wire::login_cancel(&login_id, ids.next())).await;
            return None;
        }
    };
    Some(AppEv::Screen(ScreenEv::Onboarding(ev)))
}

/// The launch facts.
async fn facts<F, Fut>(startup: &Startup, ids: &Ids, send: &F) -> Facts
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let config = &startup.config;
    // `updates.rs` is compiled out of debug builds: no offer there.
    let update = if cfg!(debug_assertions) || !config.check_for_update_on_startup {
        None
    } else {
        let cache = read_version_cache(&version_file(config));
        let command = update_action(InstallContext::current()).map(UpdateAction::command_str);
        wire::update_offer(env!("CARGO_PKG_VERSION"), cache.as_ref(), command)
    };
    let sign_in = match send(wire::account_read(ids.next())).await {
        Ok(result) => wire::sign_in_offer(result, login_policy(startup)).unwrap_or(None),
        Err(_) => None,
    };
    let trust = if startup.check_trust {
        trust(config, ids, send).await
    } else {
        None
    };
    Facts {
        update,
        welcome: Some(Welcome {
            animations: config.animations && config.tui_effects.welcome,
        }),
        sign_in,
        trust,
    }
}

/// The login methods allowed and the provider wizards the configuration
/// and the server offer (`OnboardingScreenArgs` in `run_ratatui_app`,
/// `should_show_provider_setup_wizard`).
fn login_policy(startup: &Startup) -> wire::LoginPolicy {
    let config = &startup.config;
    let auth = config.auth_config();
    let wizard = |feature| {
        startup.target.supports(feature)
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
/// defaults: no resumed task, the agents overview as the cancel).
async fn trust<F, Fut>(config: &Config, ids: &Ids, send: &F) -> Option<TrustAsk>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let cwd = config.cwd.as_path();
    let cwd_text = cwd.to_str()?.to_owned();
    let response = send(wire::trust_read(&cwd_text, ids.next())).await.ok()?;
    let roots = local_roots(&config.cwd, &response).await;
    let lookup = wire::trust_lookup(
        &response,
        &cwd_text,
        &normalized_project_trust_keys(cwd),
        roots.as_ref(),
        cfg!(windows),
    )?;
    Some(TrustAsk {
        cwd: lookup.cwd,
        target: lookup.target,
        restricted: lookup.restricted,
        existing_task: false,
        cancel: TrustCancel::AgentsOverview,
        sandbox_hint: false,
        error: None,
        decision: None,
    })
}

/// The project and repository roots of `cwd` on this machine.
async fn local_roots(
    cwd: &AbsolutePathBuf,
    response: &serde_json::Value,
) -> Option<wire::LocalRoots> {
    let markers = serde_json::from_value::<Option<Vec<String>>>(
        response["config"]["project_root_markers"].clone(),
    )
    .ok()
    .flatten()
    .unwrap_or_else(default_project_root_markers);
    let fs = LOCAL_FS.as_ref();
    let project_root = discover_project_root(fs, cwd, &markers).await.ok()?;
    let git_root = resolve_root_git_project_for_trust(fs, cwd).await;
    let projectless = project_root.is_none()
        && discover_project_root(fs, cwd, &default_project_root_markers())
            .await
            .ok()?
            .is_none();
    Some(wire::LocalRoots {
        project_root: normalized_project_trust_keys(project_root.as_ref().unwrap_or(cwd).as_path()),
        git_root: git_root.map(|root| normalized_project_trust_keys(root.as_path())),
        projectless,
    })
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
