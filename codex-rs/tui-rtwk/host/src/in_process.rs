//! The in-process app-server as a `Connector`. It lives in the host package
//! (`host/`, the `codex-rtwk` binary): the library links no server, so its
//! tests build and link fast. It is the shell's other half: one future the
//! shell runs, with no spawn and no lint allow of its own.

use std::sync::Arc;

use crate::target::Mode;
use crate::workspace::Local;
use anyhow::Context as _;
use codex_app_server_client::AppServerClient;
use codex_app_server_client::AppServerEvent;
use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_client::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY;
use codex_app_server_client::EmbeddedNetworkPolicy;
use codex_app_server_client::EnvironmentManager;
use codex_app_server_client::ExecServerRuntimeOptions;
use codex_app_server_client::InProcessAppServerClient;
use codex_app_server_client::InProcessClientStartArgs;
use codex_app_server_client::RemoteAppServerClient;
use codex_app_server_client::RemoteAppServerConnectArgs;
use codex_app_server_client::RemoteAppServerEndpoint;
use codex_app_server_client::TypedRequestError;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigWarningNotification;
use codex_arg0::Arg0DispatchPaths;
use codex_config::CloudConfigBundleLoader;
use codex_config::LoaderOverrides;
use codex_core::config::Config;
use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
use codex_feedback::CodexFeedback;
use codex_protocol::protocol::SessionSource;
use codex_tui_core::app::AppEv;
use codex_tui_core::app::BackendEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::effects::ConfigEdit;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::protocol;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::agents::Grouping as AgentsGrouping;
use codex_tui_core::screens::chat::Ev as ChatEv;
use codex_tui_core::screens::dialogs::Ev as DialogsEv;
use codex_tui_core::screens::dialogs::Warning;
use codex_tui_core::screens::onboarding::auth::provider::wire;
use codex_tui_core::screens::status::Ev as StatusEv;
use codex_tui_rtwk::services::agents::search::Latest;
use codex_tui_rtwk::services::agents::version;
use codex_tui_rtwk::services::agents::workspace::Boxed;
use codex_tui_rtwk::services::agents::workspace::Facts;
use codex_tui_rtwk::services::agents::workspace::Workspace;
use codex_tui_rtwk::services::agents::workspace::account;
use codex_tui_rtwk::services::agents::workspace::backend_ev;
use codex_tui_rtwk::services::agents::workspace::directories;
use codex_tui_rtwk::services::agents::workspace::save_grouping;
use codex_tui_rtwk::services::agents::workspace::{bind_worktree, create_worktree};
use codex_tui_rtwk::services::backend::reconnect;
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_tui_rtwk::services::backend::rpc::answer;
use codex_tui_rtwk::services::exec::AppServerExec;
use codex_tui_rtwk::services::shell::BackendTask;
use codex_tui_rtwk::services::shell::Connector;
use codex_tui_rtwk::services::shell::IngressTx;
use codex_tui_rtwk::services::shell::emit;
use codex_utils_absolute_path::AbsolutePathBuf;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

/// Hands its value to `main` when dropped, however the future that holds it
/// ends. Quitting unmounts the app, and rtwk aborts the backend task: its
/// future is dropped at an `.await`, so no code after the loop can run.
/// `main` therefore owns the shutdown: it takes the client back through
/// this guard and awaits `client.shutdown()` itself.
pub struct HandBack<T> {
    value: Option<T>,
    to: Option<oneshot::Sender<T>>,
}

impl<T> HandBack<T> {
    pub fn new(value: T, to: oneshot::Sender<T>) -> Self {
        Self {
            value: Some(value),
            to: Some(to),
        }
    }
}

impl<T> HandBack<T> {
    /// The value, until the guard is dropped.
    pub fn get(&mut self) -> Option<&mut T> {
        self.value.as_mut()
    }

    /// Puts `value` in place of the held one (a restored connection); the
    /// old one is dropped.
    pub fn set(&mut self, value: T) {
        self.value = Some(value);
    }

    /// Puts `value` in place of the held one and hands the old one back
    /// (a restart: the caller shuts it down).
    pub fn replace(&mut self, value: T) -> Option<T> {
        self.value.replace(value)
    }
}

impl<T> Drop for HandBack<T> {
    fn drop(&mut self) {
        if let (Some(value), Some(to)) = (self.value.take(), self.to.take()) {
            let _ = to.send(value);
        }
    }
}

/// What the in-process client is started with, for `config`.
async fn start_args(
    config: Arc<Config>,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<InProcessClientStartArgs> {
    let loader_overrides = LoaderOverrides::default();
    let embedded_network_policy = EmbeddedNetworkPolicy::load(&loader_overrides).await;
    let state_db = codex_core::init_state_db(&config).await;
    let runtime_paths = ExecServerRuntimeOptions::from_optional_paths(
        arg0_paths.codex_self_exe.clone(),
        arg0_paths.codex_linux_sandbox_exe.clone(),
    )
    .context("exec-server runtime paths")?;
    let environment_manager = EnvironmentManager::from_codex_home(
        config.codex_home.clone(),
        Some(runtime_paths),
        embedded_network_policy.bind(config.http_client_factory()),
    )
    .await
    .context("environment manager")?;
    let config_warnings = config
        .startup_warnings
        .iter()
        .map(|warning| ConfigWarningNotification {
            summary: warning.clone(),
            details: None,
            path: None,
            range: None,
        })
        .collect();
    Ok(InProcessClientStartArgs {
        required_human_approval: false,
        arg0_paths,
        config,
        cli_overrides: Vec::new(),
        loader_overrides,
        strict_config: false,
        cloud_config_bundle: CloudConfigBundleLoader::default(),
        embedded_network_policy,
        feedback: CodexFeedback::new(),
        log_db: None,
        state_db,
        environment_manager: Arc::new(environment_manager),
        config_warnings,
        session_source: SessionSource::Cli,
        enable_codex_api_key_env: true,
        client_name: "codex-rtwk".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        mcp_server_openai_form_elicitation: false,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
}

/// The in-process app-server, started when the app attaches it.
pub struct InProcess {
    pub arg0_paths: Arg0DispatchPaths,
    /// Which server: the embedded one, the local daemon or a remote one.
    pub mode: Mode,
    /// Receives the client when the backend future ends or is aborted, so
    /// that `main` shuts it down (see `HandBack`).
    pub done: oneshot::Sender<AppServerClient>,
    /// `--model`: the model of the thread, over the config's.
    pub model: Option<String>,
    /// `--cd`: the working root of the agent, over the process's directory.
    pub cwd: Option<std::path::PathBuf>,
    /// `-c key=value`, raw: applied over the config as the oracle does.
    pub config_overrides: Vec<String>,
    /// `target::startup_warning`: why the launch runs without the shared
    /// server, shown among the startup warnings.
    pub startup_warning: Option<String>,
    /// Receives why the server did not start: the app ends then, and `main`
    /// ends the process with this error (the oracle's `run_main_inner`
    /// returns it).
    pub failed: oneshot::Sender<String>,
    /// `codex agents`: the overview opens, no folder trust is asked.
    pub agents: bool,
    /// Where the startup page leaves the update to run after the app ends.
    pub update: crate::startup::UpdateSlot,
    /// Where the startup page leaves the error of a launch it failed.
    pub startup_failure: crate::startup::FailureSlot,
}

impl Connector for InProcess {
    fn run(
        self: Box<Self>,
        ingress: IngressTx,
        commands: UnboundedReceiver<Command>,
        epoch: ConnEpoch,
    ) -> BackendTask {
        Box::pin(run(*self, ingress, commands, epoch))
    }
}

/// The in-process server's side: the pump and the requests it makes, all in
/// this one future (nothing is spawned; the shell runs the future).
async fn run(
    spec: InProcess,
    ingress: IngressTx,
    mut commands: UnboundedReceiver<Command>,
    epoch: ConnEpoch,
) {
    let InProcess {
        arg0_paths,
        mode,
        done,
        model,
        cwd,
        config_overrides,
        startup_warning,
        failed,
        agents,
        update,
        startup_failure,
    } = spec;
    // A start that fails ends the app: `main` reports `error` once the
    // terminal is restored.
    let fail = |error: String| {
        let _ = failed.send(error.clone());
        emit(&ingress, epoch, AppEv::StartFailed(error));
    };
    let cli_overrides = match (codex_utils_cli::CliConfigOverrides {
        raw_overrides: config_overrides,
    })
    .parse_overrides()
    {
        Ok(overrides) => overrides,
        Err(error) => return fail(format!("Error parsing -c overrides: {error}")),
    };
    // Loaded here, after the first frame: config, state DB, environment manager.
    let startup_cwd = cwd.clone();
    let overrides = ConfigOverrides {
        cwd,
        ..Default::default()
    };
    // The launch's own way of loading the config, kept for the reload once
    // the folder is trusted.
    let reload: crate::startup::Reload = {
        let (cli_overrides, overrides) = (cli_overrides.clone(), overrides.clone());
        Arc::new(move || {
            let (cli_overrides, overrides) = (cli_overrides.clone(), overrides.clone());
            Box::pin(async move {
                ConfigBuilder::default()
                    .cli_overrides(cli_overrides)
                    .harness_overrides(overrides)
                    .build()
                    .await
                    .map_err(|error| error.to_string())
            })
        })
    };
    let config = match ConfigBuilder::default()
        .cli_overrides(cli_overrides)
        .harness_overrides(overrides)
        .build()
        .await
    {
        Ok(config) => Arc::new(config),
        Err(error) => return fail(format!("loading the config: {error}")),
    };
    // The server, and whether its workspace is this machine's (oracle:
    // `worktrees_enabled`: a local workspace, and no cloud list).
    let (client, local) = match connect_first(&mode, &config, arg0_paths.clone()).await {
        Ok(connected) => connected,
        Err(error) => return fail(error),
    };
    let facts = Facts {
        cloud: false,
        worktrees: local,
    };
    // `tui.status_line_use_colors`: whether the tasks get their own colour.
    let thread_colors = config.tui_status_line_use_colors;
    // `tui.agents_overview_grouping`: the grouping the center opens with.
    let grouping = config.tui_agents_overview_grouping;
    // What the status line and the footer show: read here, sent as events.
    let status = crate::facts::status(&config).await;
    let mut warnings = crate::facts::warnings(&config);
    warnings.extend(startup_warning.map(|details| Warning {
        source: "Startup".to_string(),
        details,
    }));
    // `tui.keymap.*`: a keymap the oracle refuses is a warning and the
    // defaults stay.
    let keymap = match crate::facts::keymap(&config) {
        Ok(keymap) => Some(keymap),
        Err(details) => {
            warnings.push(Warning {
                source: "Keymap".to_string(),
                details,
            });
            None
        }
    };
    let workspace: Arc<dyn Workspace> = Arc::new(Local::new(config.codex_home.to_path_buf()));
    // The startup page's asks: the trust is a local folder's question.
    let (restart, mut restarts) = tokio::sync::mpsc::unbounded_channel::<crate::startup::Restart>();
    let startup = crate::startup::Startup {
        current: Arc::new(tokio::sync::RwLock::new(config.clone())),
        restart,
        config: config.clone(),
        connected: !matches!(mode, Mode::Embedded),
        check_trust: !agents,
        remote_cwd: match mode {
            Mode::Remote { daemon: false, .. } => startup_cwd.clone(),
            _ => None,
        },
        cwd_override: startup_cwd.is_some(),
        update,
        failure: startup_failure,
        browser: crate::startup::system_browser(),
        analytics: Default::default(),
        updates: crate::startup::Updates::system(&config),
        reload,
        target: match mode {
            Mode::Embedded => crate::startup::Target::Embedded,
            Mode::Daemon | Mode::Remote { daemon: true, .. } => crate::startup::Target::LocalDaemon,
            Mode::Remote { daemon: false, .. } => crate::startup::Target::Remote,
        },
    };
    let show_version = config.tui_show_server_version_notice;
    // The machine's own work: clipboard, history file, file search, browser. The file
    // search answers through the ingress, as the server's events do.
    let effects = Arc::new(crate::effects::Effects::new(
        &config,
        crate::startup::system_browser(),
        {
            let ingress = ingress.clone();
            Arc::new(move |ev| emit(&ingress, epoch, crate::effects::popover_ev(ev)))
        },
    ));
    let mut held = HandBack::new(client, done);
    emit(&ingress, epoch, AppEv::Connected);
    emit(&ingress, epoch, backend_ev(facts));
    emit(
        &ingress,
        epoch,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::ThreadColors(thread_colors))),
    );
    if let Some(grouping) = AgentsGrouping::from_config(grouping.as_str()) {
        emit(
            &ingress,
            epoch,
            AppEv::Screen(ScreenEv::Agents(AgentsEv::Grouping(grouping))),
        );
    }
    emit(
        &ingress,
        epoch,
        AppEv::Screen(ScreenEv::Chat(ChatEv::Status(Box::new(status)))),
    );
    if let Some(keymap) = keymap {
        emit(&ingress, epoch, AppEv::Keymap(Box::new(keymap)));
    }
    emit(
        &ingress,
        epoch,
        AppEv::Tui(crate::effects::tui_settings(&config)),
    );
    emit(
        &ingress,
        epoch,
        AppEv::Presets(crate::effects::presets(&config)),
    );
    emit(
        &ingress,
        epoch,
        AppEv::Feature(plugins_fact(&config).into()),
    );
    emit(
        &ingress,
        epoch,
        AppEv::Feature(settings_fact(&config).into()),
    );
    emit(
        &ingress,
        epoch,
        AppEv::Feature(feedback_fact(&config, startup.target).into()),
    );
    if !warnings.is_empty() {
        emit(
            &ingress,
            epoch,
            AppEv::Screen(ScreenEv::Dialogs(DialogsEv::Retained(warnings))),
        );
    }
    // The first conversation opens with the update banner when the cache holds a newer release
    // (the startup page already refreshed a stale cache).
    if let Some(notice) =
        crate::update::update_available(&config, &crate::startup::Updates::system(&config))
    {
        emit(&ingress, epoch, AppEv::Announce(notice));
    }
    // The terminal's way of writing a link, the machine's clock and whether raw reasoning shows,
    // for the transcript's cells.
    for ev in crate::viewer::events(&config) {
        emit(&ingress, epoch, ev);
    }
    let daemon = matches!(mode, Mode::Daemon | Mode::Remote { daemon: true, .. });
    let mut version_shown = None;
    if show_version && let Some(client) = held.get() {
        emit_version(&ingress, epoch, client, daemon, &mut version_shown);
    }
    let Some(mut handle) = held.get().map(|client| client.request_handle()) else {
        return;
    };
    let ids = Ids::default();
    let mut inflight: FuturesUnordered<Boxed<Option<AppEv>>> = FuturesUnordered::new();
    // As the oracle's bootstrap: the account is read, so the center knows
    // the plan and whether usage can be fetched.
    inflight.push(Box::pin(account(ids.clone(), sender(handle.clone()))));
    // The status card's own: the account it shows, the catalog it names the model in, the server.
    status_feed(&mut inflight, &ids, &handle);
    if let Some(client) = held.get() {
        emit_connection(&ingress, epoch, client, reconnect_endpoint(&mode), daemon);
    }
    // And what words a refused cybersecurity request: the account's access and the catalog.
    let cyber = crate::facts::CyberPolicy::of(&config);
    inflight.push(Box::pin(crate::facts::daybreak(
        cyber,
        ids.clone(),
        sender(handle.clone()),
    )));
    // The history search's debounce: the oracle waits 180 ms and a newer
    // search drops the pending one.
    let mut latest = Latest::default();
    let endpoint = reconnect_endpoint(&mode);
    let mut request_epoch = epoch;
    // The history file and the mention usage are read once, when the first thread is asked.
    let mut thread_asked = false;
    while let Some(client) = held.get() {
        // The connection ended: restored or not, decided after the select,
        // when the client is no longer borrowed.
        let mut lost = false;
        // The provider changed with the reload: the server is started again
        // after the select, when the client is no longer borrowed.
        let mut restart: Option<crate::startup::Restart> = None;
        tokio::select! {
            event = client.next_event() => match event {
                None | Some(AppServerEvent::Disconnected { .. }) => lost = true,
                Some(AppServerEvent::Lagged { skipped }) => {
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Lagged(skipped)));
                }
                Some(AppServerEvent::ServerNotification(notification)) => {
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Notification(notification)));
                }
                Some(AppServerEvent::ServerRequest(request)) => {
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Request(request)));
                }
                // Cloud streams: this app has no cloud backend.
                Some(
                    AppServerEvent::ThreadStreamDisconnected(_)
                    | AppServerEvent::ThreadStreamReconnected(_),
                ) => {}
            },
            command = commands.recv() => match command {
                None => break,
                Some(Command::RejectRequest { key, .. }) => {
                    if key.epoch == request_epoch {
                        reject(client, &key).await;
                    }
                }
                Some(Command::ResolveRequest { key, result, .. }) => {
                    if key.epoch == request_epoch && let Some(ev) = resolve(client, key, result).await {
                        emit(&ingress, epoch, ev);
                    }
                }
                Some(command) => {
                    if !thread_asked && asks_thread(&command) {
                        thread_asked = true;
                        let (effects, ingress) = (effects.clone(), ingress.clone());
                        inflight.push(Box::pin(async move {
                            for ev in effects.opened().await {
                                emit(&ingress, epoch, ev);
                            }
                            None
                        }));
                    }
                    let exec = AppServerExec::new(sender(handle.clone()), ids.clone());
                    // A write of the config file is the app server's, and the facts the dialogs
                    // read follow it: it is not one of the machine's effects.
                    let job = if let Command::Effect {
                        request,
                        ask: Ask::ConfigWrite { edits },
                        ..
                    } = &command
                    {
                        config_write_job(
                            *request,
                            edits.clone(),
                            startup.clone(),
                            ids.clone(),
                            handle.clone(),
                            (ingress.clone(), epoch),
                        )
                    } else {
                        match effects.run(&command, exec) {
                            Some(job) => job,
                            None => work(
                                command.clone(),
                                handle.clone(),
                                ids.clone(),
                                facts,
                                workspace.clone(),
                                model.clone(),
                                Some(startup.clone()),
                            ),
                        }
                    };
                    inflight.push(latest.wrap(&command, job));
                }
            },
            // The reloaded config changed the provider: the embedded server
            // starts again with it (lib.rs:2079-2110).
            Some(next) = restarts.recv() => restart = Some(next),
            Some(done) = inflight.next(), if !inflight.is_empty() => {
                if let Some(ev) = done {
                    emit(&ingress, epoch, ev);
                }
            }
        }
        if let Some(crate::startup::Restart { config, done }) = restart.take() {
            match connect_first(&mode, &config, arg0_paths.clone()).await {
                Ok((restarted, _)) => {
                    handle = restarted.request_handle();
                    if let Some(old) = held.replace(restarted) {
                        let _ = old.shutdown().await;
                    }
                    // The commands after this one talk to the new server.
                    if let Some(done) = done {
                        let _ = done.send(());
                    }
                }
                Err(error) => emit(&ingress, epoch, AppEv::StartFailed(error)),
            }
            continue;
        }
        if !lost {
            continue;
        }
        let Some(endpoint) = endpoint.clone() else {
            // The embedded server has no connection to restore.
            emit(&ingress, epoch, AppEv::Disconnected);
            break;
        };
        // What the lost connection was asked will never answer.
        inflight = FuturesUnordered::new();
        let restored =
            reconnect::restore(|| connect_remote(endpoint.clone()), &ingress, epoch).await;
        let Some(restored) = restored else {
            // The notice stays; the app is closed with its commands.
            while commands.recv().await.is_some() {}
            break;
        };
        request_epoch = request_epoch.next();
        held.set(restored);
        let Some(client) = held.get() else {
            break;
        };
        handle = client.request_handle();
        if show_version {
            emit_version(&ingress, epoch, client, daemon, &mut version_shown);
        }
        inflight.push(Box::pin(account(ids.clone(), sender(handle.clone()))));
        status_feed(&mut inflight, &ids, &handle);
        emit_connection(&ingress, epoch, client, Some(endpoint.clone()), daemon);
        inflight.push(Box::pin(crate::facts::daybreak(
            cyber,
            ids.clone(),
            sender(handle.clone()),
        )));
    }
    // Leaving the loop drops `client`: the guard hands it to `main`.
}

/// What the settings dialogs read of the config and the disk: the theme in force, the custom
/// themes, the memory tool's switches (`settings::read`).
fn settings_fact(config: &Config) -> codex_tui_core::features::settings::Fact {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let read = crate::settings::read(config, home.as_deref());
    codex_tui_core::features::settings::Fact::facts(
        read.syntax_theme,
        read.custom_themes,
        read.themes_dir,
        read.memories,
    )
}

/// What `/feedback` reads of the config and the connection: whether uploads are on, whether the
/// workspace is remote (no ZIP can be saved here), the Codex home the ZIPs go under.
fn feedback_fact(
    config: &Config,
    target: crate::startup::Target,
) -> codex_tui_core::features::feedback::Fact {
    codex_tui_core::features::feedback::Fact::config(
        config.feedback_enabled,
        target == crate::startup::Target::Remote,
        Some(config.codex_home.to_string_lossy().into_owned()),
    )
}

/// `Ask::ConfigWrite` of a dialog (`/theme`, `/title`, `/statusline`, `/memories`): the edits go
/// through the app server, and once the config file holds them the status card, the terminal
/// settings and the dialogs' facts follow from the config as it is now, before the answer (the
/// dialog that reads it finds them).
fn config_write_job(
    request: u64,
    edits: Vec<ConfigEdit>,
    startup: crate::startup::Startup,
    ids: Ids,
    handle: AppServerRequestHandle,
    (ingress, epoch): (IngressTx, ConnEpoch),
) -> Boxed<Option<AppEv>> {
    Box::pin(async move {
        let written = crate::settings::write(edits, &startup, &ids, sender(handle)).await;
        if let Some(config) = &written.config {
            let status = crate::facts::status(config).await;
            emit(
                &ingress,
                epoch,
                AppEv::Screen(ScreenEv::Chat(ChatEv::Status(Box::new(status)))),
            );
            emit(
                &ingress,
                epoch,
                AppEv::Tui(crate::effects::tui_settings(config)),
            );
            emit(
                &ingress,
                epoch,
                AppEv::Feature(settings_fact(config).into()),
            );
            emit(
                &ingress,
                epoch,
                AppEv::Feature(feedback_fact(config, startup.target).into()),
            );
        }
        Some(AppEv::Effect {
            request,
            done: written.done,
        })
    })
}

/// What `/plugins` reads of the config: the features it depends on, the home directory the
/// personal marketplace is under, and the marketplaces the user configured (the ones a tab can
/// remove; the git ones it can upgrade).
fn plugins_fact(config: &Config) -> codex_tui_core::features::plugins::Fact {
    use codex_features::Feature;
    let stack = &config.config_layer_stack;
    let user = stack.effective_user_config();
    let git = |name: &str| {
        stack
            .get_active_user_layer()
            .and_then(|layer| layer.config.get("marketplaces"))
            .and_then(|marketplaces| marketplaces.get(name))
            .and_then(|marketplace| marketplace.get("source_type"))
            .and_then(|kind| kind.as_str())
            .is_some_and(|kind| kind == "git")
    };
    let configured: Vec<(String, bool)> = user
        .as_ref()
        .and_then(|user| user.get("marketplaces"))
        .and_then(|marketplaces| marketplaces.as_table())
        .map(|table| table.keys().map(|name| (name.clone(), git(name))).collect())
        .unwrap_or_default();
    codex_tui_core::features::plugins::Fact::config(
        config.features.enabled(Feature::Plugins),
        config.features.enabled(Feature::RemotePlugin),
        config.features.enabled(Feature::PluginSharing),
        std::env::var_os("HOME").map(|home| home.to_string_lossy().into_owned()),
        configured,
    )
}

/// The reads of the facts the status card shows (`/status`): the account and the model catalog.
fn status_feed(
    inflight: &mut FuturesUnordered<Boxed<Option<AppEv>>>,
    ids: &Ids,
    handle: &AppServerRequestHandle,
) {
    inflight.push(Box::pin(crate::facts::card_account(
        ids.clone(),
        sender(handle.clone()),
    )));
    // `/feedback`'s audience: an `@openai.com` account is an employee.
    inflight.push(Box::pin({
        let (ids, send) = (ids.clone(), sender(handle.clone()));
        async move {
            let email = crate::facts::account_email(ids, send).await;
            Some(AppEv::Feature(
                codex_tui_core::features::feedback::Fact::account(email.as_deref()).into(),
            ))
        }
    }));
    inflight.push(Box::pin(crate::facts::card_catalog(
        ids.clone(),
        sender(handle.clone()),
    )));
}

/// The server row of the status card: the address of a remote server and the version it
/// reports (none for the in-process one).
fn emit_connection(
    ingress: &IngressTx,
    epoch: ConnEpoch,
    client: &AppServerClient,
    endpoint: Option<RemoteAppServerEndpoint>,
    daemon: bool,
) {
    let AppServerClient::Remote(remote) = client else {
        return;
    };
    let connection = crate::facts::card_remote(endpoint.as_ref(), daemon, remote.server_version());
    emit(
        ingress,
        epoch,
        AppEv::Screen(ScreenEv::Status(StatusEv::Connection(connection))),
    );
}

/// Whether `command` is the first of a thread: one that starts, drives or reads one.
fn asks_thread(command: &Command) -> bool {
    matches!(
        command,
        Command::StartThread { .. }
            | Command::StartTurn { .. }
            | Command::Screen { .. }
            | Command::Feature { .. }
    )
}

/// The endpoint a lost connection is restored through; `None` for the
/// embedded server.
fn reconnect_endpoint(mode: &Mode) -> Option<RemoteAppServerEndpoint> {
    match mode {
        Mode::Embedded => None,
        Mode::Remote { endpoint, .. } => Some(endpoint.clone()),
        // The daemon's socket is where it was started (the same one the
        // oracle's `LocalDaemon` target reconnects to).
        Mode::Daemon => codex_core::config::find_codex_home()
            .ok()
            .and_then(|home| codex_app_server_client::app_server_control_socket_path(&home).ok())
            .map(|socket_path| RemoteAppServerEndpoint::UnixSocket { socket_path }),
    }
}

/// The server's first connection, and whether its workspace is this
/// machine's. A failure is the text the app shows (`StartFailed`).
async fn connect_first(
    mode: &Mode,
    config: &Arc<Config>,
    arg0_paths: Arg0DispatchPaths,
) -> Result<(AppServerClient, bool), String> {
    match mode {
        Mode::Embedded => {
            let args = start_args(config.clone(), arg0_paths)
                .await
                .map_err(|error| format!("{error:#}"))?;
            let local = !args
                .environment_manager
                .default_environment()
                .is_some_and(|environment| environment.is_remote());
            // Boxed: the start future is large, and the pump should not carry it inline.
            let client = Box::pin(InProcessAppServerClient::start(args))
                .await
                .map_err(|error| error.to_string())?;
            Ok((AppServerClient::InProcess(client), local))
        }
        Mode::Daemon => {
            // As the oracle's startup: the daemon is started when it is not
            // running (`startup_orchestration.rs`), and a failure says how to
            // work without it.
            let started =
                codex_app_server_daemon::start_with_features_classified(&Default::default())
                    .await
                    .map_err(|failure| format!("{:#}\n{DAEMON_FAILURE_HINT}", failure.error))?;
            let socket_path = AbsolutePathBuf::from_absolute_path_checked(started.socket_path)
                .map_err(|error| error.to_string())?;
            let client = connect_remote(RemoteAppServerEndpoint::UnixSocket { socket_path })
                .await
                .ok_or_else(|| {
                    format!("failed to connect to the local daemon\n{DAEMON_FAILURE_HINT}")
                })?;
            Ok((client, true))
        }
        Mode::Remote { endpoint, daemon } => {
            let client = connect_remote(endpoint.clone())
                .await
                .ok_or_else(|| "failed to connect to remote app server".to_string())?;
            Ok((client, *daemon))
        }
    }
}

/// `daemon_startup.rs::FAILURE_HINT`.
const DAEMON_FAILURE_HINT: &str = "To work without the background server, rerun the same command with --no-daemon (including resume or fork and its arguments).";

/// One connection to a server at `endpoint` (oracle:
/// `connect_remote_app_server`); a failure is `None`, whose text the oracle
/// never shows.
async fn connect_remote(endpoint: RemoteAppServerEndpoint) -> Option<AppServerClient> {
    RemoteAppServerClient::connect(RemoteAppServerConnectArgs {
        required_human_approval: true,
        endpoint,
        client_name: "codex-rtwk".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        mcp_server_openai_form_elicitation: false,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await
    .ok()
    .map(AppServerClient::Remote)
}

/// The older-server words of the connected server, or their absence: the command center's
/// banner, and, once per distinct notice, a notice in the open conversation (oracle
/// `pending_server_version_notice`: the message is the key, a reconnect to the same server says
/// nothing again).
fn emit_version(
    ingress: &IngressTx,
    epoch: ConnEpoch,
    client: &AppServerClient,
    daemon: bool,
    shown: &mut Option<String>,
) {
    let AppServerClient::Remote(remote) = client else {
        return;
    };
    let server = remote.server_version();
    let text = server.and_then(|server| version::notice(env!("CARGO_PKG_VERSION"), server, daemon));
    emit(
        ingress,
        epoch,
        AppEv::Screen(ScreenEv::Agents(AgentsEv::ServerVersion(text))),
    );
    if let Some(message) = server
        .and_then(|server| crate::update::server_version_message(env!("CARGO_PKG_VERSION"), server))
        && shown.as_ref() != Some(&message)
    {
        *shown = Some(message.clone());
        emit(
            ingress,
            epoch,
            AppEv::Announce(codex_tui_core::threads::Notice::ServerVersion {
                message,
                offer_update: daemon,
            }),
        );
    }
}

/// The work a command of ours asks, as one boxed future (the pump holds many
/// of them; boxing keeps its type, and its stack, small) and the event the
/// app is waiting for, if any. Core's `wire` makes the requests and reads the
/// results; the shell's `answer`, `directories` and the worktree effects run them; this
/// only sends.
pub(crate) fn work(
    command: Command,
    handle: AppServerRequestHandle,
    ids: Ids,
    facts: Facts,
    workspace: Arc<dyn Workspace>,
    model: Option<String>,
    startup: Option<crate::startup::Startup>,
) -> Boxed<Option<AppEv>> {
    let send = sender(handle);
    match command {
        // The startup page's asks; a backend with no startup side (Cloud)
        // answers none.
        Command::Startup { ask, .. } => {
            Box::pin(async move { crate::startup::run(startup?, ask, ids, send).await })
        }
        // The import flow's asks; a backend with no startup side (Cloud)
        // answers none.
        Command::Import { ask, .. } => {
            Box::pin(async move { crate::import::run(startup?, ask, ids, send).await })
        }
        // The usage page's loads; a backend with no startup side (Cloud)
        // answers none.
        Command::Usage { ask, .. } => {
            Box::pin(async move { crate::analytics::run(startup?, ask, ids, send).await })
        }
        Command::Directories { cwds, .. } => {
            Box::pin(async move { Some(directories(workspace, cwds).await) })
        }
        Command::SaveGrouping { grouping, .. } => {
            Box::pin(async move { save_grouping(workspace, grouping).await })
        }
        Command::CreateWorktree { ticket, cwd, .. } => {
            Box::pin(async move { Some(create_worktree(workspace, ticket, cwd).await) })
        }
        Command::BindWorktree {
            ticket,
            checkout,
            thread,
            cwd,
            ..
        } => {
            Box::pin(
                async move { Some(bind_worktree(workspace, ticket, checkout, thread, cwd).await) },
            )
        }
        // `RejectRequest` is handled by the pump, which owns the client.
        command => Box::pin(async move {
            answer(command, facts, ids, model, send)
                .await
                .map(|(ticket, reply)| AppEv::Reply(ticket, reply))
        }),
    }
}

/// The client of the shell's effects: one request over the handle. A
/// server's refusal keeps its JSON-RPC code (the pin read tells "no pins" by
/// it); the message is the client's own text, but for the requests whose
/// refusal the oracle shows as the server's message ([`error_text`]).
pub(crate) fn sender(
    handle: AppServerRequestHandle,
) -> impl Fn(ClientRequest) -> Boxed<Result<serde_json::Value, ServerError>> {
    move |request: ClientRequest| {
        let handle = handle.clone();
        let own_message = wire::reads_message_only(&request);
        Box::pin(async move {
            handle
                .request_typed::<serde_json::Value>(request)
                .await
                .map_err(|error| ServerError {
                    code: match &error {
                        TypedRequestError::Server { source, .. } => Some(source.code),
                        _ => None,
                    },
                    message: error_text(own_message, &error),
                    data: match &error {
                        TypedRequestError::Server { source, .. } => source.data.clone(),
                        _ => None,
                    },
                })
        })
    }
}

/// The text of a failed request. The oracle's GCP setup and gateway wizard
/// (`own_message`) read `source.message` of a server's refusal, so the
/// message is the server's own, without `<method> failed: ... (code n)`;
/// everything else (and every other kind of failure) is the client's text
/// (`error.to_string()`), which core's callers wrap as the oracle does.
fn error_text(own_message: bool, error: &TypedRequestError) -> String {
    match error {
        TypedRequestError::Server { source, .. } if own_message => source.message.clone(),
        _ => error.to_string(),
    }
}

pub(crate) async fn reject(client: &AppServerClient, key: &RequestKey) {
    let Some(request_id) = protocol::reject_target(key) else {
        return;
    };
    let _ = client
        .reject_server_request(request_id, protocol::dismissal())
        .await;
}

pub(crate) async fn resolve(
    client: &AppServerClient,
    key: RequestKey,
    result: serde_json::Value,
) -> Option<AppEv> {
    let request_id = protocol::reject_target(&key)?;
    client
        .resolve_server_request(request_id, result)
        .await
        .err()
        .map(|error| AppEv::ServerAnswerFailed {
            key,
            message: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_guard_hands_its_value_back() {
        let (tx, mut rx) = oneshot::channel();
        drop(HandBack::new(7_u8, tx));
        assert_eq!(rx.try_recv().ok(), Some(7));
    }

    /// The case that matters: the future is dropped at an `.await`, as
    /// rtwk's abort does on quit.
    #[test]
    fn a_future_dropped_at_an_await_hands_its_value_back() {
        use futures::FutureExt as _;
        let (tx, mut rx) = oneshot::channel();
        let mut future = Box::pin(async move {
            let _held = HandBack::new(7_u8, tx);
            std::future::pending::<()>().await;
        });
        assert!(
            (&mut future).now_or_never().is_none(),
            "parked at the await"
        );
        assert!(rx.try_recv().is_err(), "still held");
        drop(future);
        assert_eq!(rx.try_recv().ok(), Some(7));
    }

    fn refusal(method: &str, message: &str) -> TypedRequestError {
        TypedRequestError::Server {
            method: method.to_owned(),
            source: codex_app_server_protocol::JSONRPCErrorError {
                code: -32600,
                message: message.to_owned(),
                data: None,
            },
        }
    }

    // The gateway wizard's error text is the oracle's `source.message`, so a
    // refusal reads as the server wrote it; every other request keeps the
    // client's `<method> failed: ... (code n)`.
    #[test]
    fn a_wizards_refusal_is_the_servers_message_and_the_rest_keep_the_client_text() {
        let error = refusal(
            "modelProvider/list",
            "Enter an HTTP(S) base URL without credentials, a query, or a fragment.",
        );
        assert_eq!(
            error_text(true, &error),
            "Enter an HTTP(S) base URL without credentials, a query, or a fragment."
        );
        assert_eq!(
            error_text(false, &error),
            "modelProvider/list failed: Enter an HTTP(S) base URL without credentials, a query, or a fragment. (code -32600)"
        );
    }

    // Only a server's refusal has a message of its own to read.
    #[test]
    fn a_failure_that_is_not_a_refusal_keeps_the_client_text() {
        let error = TypedRequestError::Transport {
            method: "modelProvider/list".to_owned(),
            source: std::io::Error::other("closed"),
        };
        assert_eq!(error_text(true, &error), error.to_string());
    }
}
