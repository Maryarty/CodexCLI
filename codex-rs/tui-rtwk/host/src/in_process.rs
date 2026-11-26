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
use codex_tui_core::app::effects::ConfigEdit;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::ids::RequestKey;
use codex_tui_core::protocol;
use codex_tui_core::rpc::{Reply, ServerError};
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::agents::Grouping as AgentsGrouping;
use codex_tui_core::screens::chat::Ev as ChatEv;
use codex_tui_core::screens::dialogs::Ev as DialogsEv;
use codex_tui_core::screens::dialogs::Warning;
use codex_tui_core::screens::onboarding::auth::provider::wire;
use codex_tui_core::screens::status::Ev as StatusEv;
use codex_tui_core::screens::{ScreenCommand, ScreenEv, chat};
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
    // The embedded server fails the launch when the local database cannot be opened, with the
    // typed error `main` recovers from (`state_db::init`).
    let state_db = crate::state_db::init(&config).await?;
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
        state_db: Some(state_db),
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
    /// `--no-daemon`.
    pub no_daemon: bool,
    /// Receives the client when the backend future ends or is aborted, so
    /// that `main` shuts it down (see `HandBack`).
    pub done: oneshot::Sender<AppServerClient>,
    /// `--model`: the model of the thread, over the config's.
    pub model: Option<String>,
    /// `--oss`: the launch runs on a local provider.
    pub oss: bool,
    /// `--local-provider`: which one, over the config's and the probe's.
    pub oss_provider: Option<String>,
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
    /// The command line carries a prompt: no update is offered (`skip_update_prompt`).
    pub launch_prompt: bool,
    /// Where the startup page leaves the update to run after the app ends.
    pub update: crate::startup::UpdateSlot,
    /// Where the startup page leaves the error of a launch it failed.
    pub startup_failure: crate::startup::FailureSlot,
    /// Where the typed failure of the local database is left, next to the text `failed` gets.
    pub state_db_failure: crate::state_db::FailureSlot,
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
        mut mode,
        no_daemon,
        done,
        model,
        oss,
        oss_provider,
        cwd,
        config_overrides,
        startup_warning,
        failed,
        agents,
        launch_prompt,
        update,
        startup_failure,
        state_db_failure,
    } = spec;
    // A start that fails ends the app: `main` reports `error` once the
    // terminal is restored.
    let fail = |error: String| {
        let _ = failed.send(error.clone());
        emit(&ingress, epoch, AppEv::StartFailed(error));
    };
    let raw_overrides = config_overrides.clone();
    let mut cli_overrides = match (codex_utils_cli::CliConfigOverrides {
        raw_overrides: config_overrides,
    })
    .parse_overrides()
    {
        Ok(overrides) => overrides,
        Err(error) => return fail(format!("Error parsing -c overrides: {error}")),
    };
    // What the command line asks of the shared server, checked once it is connected
    // (kept only when the launch ends up on the daemon).
    let requested_features = crate::daemon::requested_features(&cli_overrides);
    // Loaded here, after the first frame: config, state DB, environment manager.
    let startup_cwd = cwd.clone();
    let mut overrides = ConfigOverrides {
        cwd,
        ..Default::default()
    };
    // `--oss`: the provider (flag, config, or the one running server), its
    // default model and raw reasoning, as session flags every reload keeps.
    let mut model = model;
    let mut launch_provider = None;
    // The provider the user picked in the panel: kept in the config once connected.
    let mut picked = None;
    if oss {
        let codex_home = match codex_core::config::find_codex_home() {
            Ok(home) => home,
            Err(error) => return fail(format!("loading the config: {error}")),
        };
        let config_cwd = startup_cwd
            .as_deref()
            .and_then(|cwd| AbsolutePathBuf::relative_to_current_dir(cwd).ok());
        match crate::oss::choose(
            oss_provider.as_deref(),
            &codex_home,
            config_cwd.as_ref(),
            cli_overrides.clone(),
        )
        .await
        {
            Ok(choice) => {
                let provider = match choice {
                    crate::oss::Choice::Provider(provider) => provider,
                    // Both or none running: the panel asks before the config exists.
                    crate::oss::Choice::Pick { lmstudio, ollama } => {
                        match crate::oss::pick(
                            lmstudio,
                            ollama,
                            |ev| emit(&ingress, epoch, ev),
                            &mut commands,
                        )
                        .await
                        {
                            Ok(provider) => {
                                picked = Some(provider.clone());
                                provider
                            }
                            Err(error) => return fail(error),
                        }
                    }
                };
                model = crate::oss::apply(&provider, model, &mut cli_overrides, &mut overrides);
                launch_provider = Some(provider);
            }
            Err(error) => return fail(error),
        }
    }
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
    // `ensure_oss_provider_ready`: the models are there, the service answers.
    if let Some(provider) = &launch_provider
        && let Err(error) = crate::oss::ensure_ready(provider.clone(), config.clone()).await
    {
        return fail(error);
    }
    // The shared server: the chat decides it here, once the config is read
    // (`startup_orchestration.rs`: `daemon_auto_start`, the exclusions, the notice).
    let mut startup_warning = startup_warning;
    let mut reuse = false;
    if matches!(mode, Mode::Embedded) && !agents {
        let launch = crate::target::Launch {
            no_daemon,
            agents,
            oss,
            remote: false,
            config_overrides: &raw_overrides,
            exec_server_url: std::env::var_os("CODEX_EXEC_SERVER_URL").is_some(),
        };
        let feature_on = config
            .features
            .enabled(codex_features::Feature::DaemonAutoStart);
        match crate::target::auto_start(&launch, feature_on) {
            crate::target::AutoStart::Daemon => mode = Mode::Daemon,
            crate::target::AutoStart::Embedded {
                exclusion,
                reusable,
            } => {
                startup_warning = exclusion.map(crate::target::exclusion_warning);
                // A daemon that already listens is attached (not started), and left
                // for the embedded server when it does not answer.
                if reusable
                    && crate::target::probe_daemon(crate::target::default_daemon_socket()).await
                {
                    mode = Mode::Daemon;
                    reuse = true;
                }
            }
        }
    }
    if matches!(mode, Mode::Daemon) && !reuse {
        // What the sign-in wizards and the home folder still exclude from it.
        let exclusion = match crate::daemon::wizard_exclusion(&config).await {
            Some(reason) => Some(reason),
            None => crate::daemon::uses_wsl_drvfs(config.codex_home.as_path())
                .then_some(crate::daemon::WSL_DRVFS_EXCLUSION),
        };
        if let Some(reason) = exclusion {
            mode = Mode::Embedded;
            startup_warning = Some(crate::target::exclusion_warning(reason));
        }
    }
    let daemon_features = if matches!(mode, Mode::Daemon) {
        requested_features
    } else {
        Default::default()
    };
    // The server, and whether its workspace is this machine's (oracle:
    // `worktrees_enabled`: a local workspace, and no cloud list).
    let (client, local) = match connect_first(&mut mode, reuse, &config, arg0_paths.clone()).await {
        Ok(connected) => connected,
        Err(error) => {
            // The typed failure first: `main` finds it once the app has ended.
            return match error.downcast::<crate::state_db::StateDbStartupError>() {
                Ok(typed) => {
                    let text = typed.to_string();
                    if let Ok(mut slot) = state_db_failure.lock() {
                        *slot = Some(typed);
                    }
                    fail(text)
                }
                Err(error) => fail(format!("{error:#}")),
            };
        }
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
    let mut startup = crate::startup::Startup {
        daemon_features,
        daemon: Default::default(),
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
        oss_provider: launch_provider,
        launch_prompt,
        update: update.clone(),
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
        update,
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
    let mut daemon = matches!(mode, Mode::Daemon | Mode::Remote { daemon: true, .. });
    let mut version_shown = None;
    if show_version && let Some(client) = held.get() {
        emit_version(&ingress, epoch, client, daemon, &mut version_shown);
    }
    let Some(mut handle) = held.get().map(|client| client.request_handle()) else {
        return;
    };
    let ids = Ids::default();
    let mut inflight: FuturesUnordered<Boxed<Option<AppEv>>> = FuturesUnordered::new();
    if let Some(provider) = picked {
        let send = sender(handle.clone());
        let ids = ids.clone();
        inflight.push(Box::pin(async move {
            crate::oss::persist(&provider, &ids, send).await;
            None
        }));
    }
    // As the oracle's bootstrap: the account is read, so the center knows
    // the plan and whether usage can be fetched.
    inflight.push(Box::pin(account(ids.clone(), sender(handle.clone()))));
    // The status card's own: the account it shows, the catalog it names the model in, the server.
    status_feed(&mut inflight, &ids, &handle);
    // What the slices read at start (their feeds): facts now, the server reads as jobs.
    let remote_workspace = matches!(mode, Mode::Remote { daemon: false, .. });
    deliver(
        crate::feeds::run(
            &crate::feeds::Event::Startup,
            &crate::feeds::FeedCx {
                config: &config,
                target: startup.target,
                remote_workspace,
                ids: &ids,
                handle: &handle,
            },
        ),
        &ingress,
        epoch,
        &mut inflight,
    );
    if let Some(client) = held.get() {
        emit_connection(&ingress, epoch, client, reconnect_endpoint(&mode), daemon);
        deliver(
            connection_feed(
                client,
                daemon,
                &crate::feeds::FeedCx {
                    config: &config,
                    target: startup.target,
                    remote_workspace,
                    ids: &ids,
                    handle: &handle,
                },
            ),
            &ingress,
            epoch,
            &mut inflight,
        );
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
    let mut endpoint = reconnect_endpoint(&mode);
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
                    // The slices that follow the stream tell their facts first.
                    deliver(
                        crate::feeds::run(
                            &crate::feeds::Event::Notification(&notification),
                            &crate::feeds::FeedCx {
                                config: &config,
                                target: startup.target,
                                remote_workspace,
                                ids: &ids,
                                handle: &handle,
                            },
                        ),
                        &ingress,
                        epoch,
                        &mut inflight,
                    );
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Notification(notification)));
                }
                Some(AppServerEvent::ServerRequest(request)) => {
                    emit(&ingress, epoch, AppEv::Backend(BackendEv::Request(request)));
                }
                // Only a Cloud client has streams per thread. This one is not: the oracle's handler
                // marks nothing and shows the error as the thread's line, and a stream that was
                // never marked has nothing to restore (`app_server_events.rs`).
                Some(AppServerEvent::ThreadStreamDisconnected(error)) => {
                    emit(
                        &ingress,
                        epoch,
                        AppEv::Backend(BackendEv::Notification(Box::new(
                            codex_app_server_protocol::ServerNotification::Error(*error),
                        ))),
                    );
                }
                Some(AppServerEvent::ThreadStreamReconnected(_)) => {}
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
                    let link = crate::effects::Link {
                        startup: startup.clone(),
                        ids: ids.clone(),
                        handle: handle.clone(),
                        ingress: ingress.clone(),
                        epoch,
                        codex_home: config.codex_home.to_path_buf(),
                    };
                    let job = match effects.run(&command, exec, Some(link)) {
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
        // The app moved to another server (the recovery menu's answer): what a reconnect tells
        // the app follows, the lost connection being the old one.
        let mut moved = false;
        if let Some(crate::startup::Restart {
            config,
            done,
            switch,
        }) = restart.take()
        {
            use crate::startup::Switch;
            // A launch that ends: the text is for `main`, once the terminal is restored.
            let end = |startup: &crate::startup::Startup, text: String| {
                if let Ok(mut slot) = startup.failure.lock() {
                    *slot = Some(text.clone());
                }
                emit(&ingress, epoch, AppEv::StartFailed(text));
            };
            if let Some(Switch::Daemon { features }) = &switch
                && let Err(text) = crate::daemon::restart_with(features).await
            {
                end(&startup, text);
                continue;
            }
            let embedded = matches!(switch, Some(Switch::Embedded { .. }));
            let next = if embedded {
                Mode::Embedded
            } else {
                mode.clone()
            };
            let mut next = next;
            match connect_first(&mut next, false, &config, arg0_paths.clone()).await {
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
                Err(error) => {
                    end(&startup, format!("{error:#}"));
                    continue;
                }
            }
            let Some(switch) = switch else {
                continue;
            };
            if let Switch::Embedded { warning } = switch {
                mode = next;
                endpoint = None;
                daemon = false;
                startup.target = crate::startup::Target::Embedded;
                startup.connected = false;
                // The card and the banner say the server is this process; the warning is the
                // launch's own (as `startup_warning`).
                emit(
                    &ingress,
                    epoch,
                    AppEv::Screen(ScreenEv::Status(StatusEv::Connection(None))),
                );
                emit(
                    &ingress,
                    epoch,
                    AppEv::Screen(ScreenEv::Agents(AgentsEv::ServerVersion(None))),
                );
                emit(
                    &ingress,
                    epoch,
                    AppEv::Screen(ScreenEv::Dialogs(DialogsEv::Retained(vec![Warning {
                        source: "Startup".to_string(),
                        details: warning,
                    }]))),
                );
            }
            // What the old connection was asked will never answer.
            inflight = FuturesUnordered::new();
            request_epoch = request_epoch.next();
            moved = true;
        }
        if !moved {
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
        }
        let Some(client) = held.get() else {
            break;
        };
        handle = client.request_handle();
        if show_version {
            emit_version(&ingress, epoch, client, daemon, &mut version_shown);
        }
        inflight.push(Box::pin(account(ids.clone(), sender(handle.clone()))));
        status_feed(&mut inflight, &ids, &handle);
        deliver(
            crate::feeds::run(
                &crate::feeds::Event::Reconnected,
                &crate::feeds::FeedCx {
                    config: &config,
                    target: startup.target,
                    remote_workspace,
                    ids: &ids,
                    handle: &handle,
                },
            ),
            &ingress,
            epoch,
            &mut inflight,
        );
        emit_connection(&ingress, epoch, client, endpoint.clone(), daemon);
        deliver(
            connection_feed(
                client,
                daemon,
                &crate::feeds::FeedCx {
                    config: &config,
                    target: startup.target,
                    remote_workspace,
                    ids: &ids,
                    handle: &handle,
                },
            ),
            &ingress,
            epoch,
            &mut inflight,
        );
        inflight.push(Box::pin(crate::facts::daybreak(
            cyber,
            ids.clone(),
            sender(handle.clone()),
        )));
    }
    // Leaving the loop drops `client`: the guard hands it to `main`.
}

/// `Ask::ConfigWrite` of a dialog (`/theme`, `/title`, `/statusline`, `/memories`): the edits go
/// through the app server, and once the config file holds them the status card, the terminal
/// settings and the dialogs' facts follow from the config as it is now, before the answer (the
/// dialog that reads it finds them).
pub(crate) async fn config_write(
    edits: Vec<ConfigEdit>,
    crate::effects::Link {
        startup,
        ids,
        handle,
        ingress,
        epoch,
        ..
    }: &crate::effects::Link,
) -> codex_tui_core::app::effects::Done {
    let (ingress, epoch) = (ingress.clone(), *epoch);
    let written = crate::settings::write(edits, startup, ids, sender(handle.clone())).await;
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
        let mut jobs = FuturesUnordered::new();
        deliver(
            crate::feeds::run(
                &crate::feeds::Event::ConfigWritten,
                &crate::feeds::FeedCx {
                    config,
                    target: startup.target,
                    remote_workspace: false,
                    ids,
                    handle,
                },
            ),
            &ingress,
            epoch,
            &mut jobs,
        );
        while let Some(done) = jobs.next().await {
            if let Some(ev) = done {
                emit(&ingress, epoch, ev);
            }
        }
    }
    written.done
}

/// What the slices are told of the connection that is up (`Event::Connection`): nothing for the
/// in-process server.
fn connection_feed(
    client: &AppServerClient,
    local_daemon: bool,
    cx: &crate::feeds::FeedCx<'_>,
) -> Vec<crate::feeds::Feed> {
    let AppServerClient::Remote(remote) = client else {
        return Vec::new();
    };
    crate::feeds::run(
        &crate::feeds::Event::Connection {
            server_version: remote.server_version(),
            local_daemon,
        },
        cx,
    )
}

/// What the feeds told: the facts go to the app now, the jobs join the pump's work.
fn deliver(
    feeds: Vec<crate::feeds::Feed>,
    ingress: &IngressTx,
    epoch: ConnEpoch,
    inflight: &mut FuturesUnordered<Boxed<Option<AppEv>>>,
) {
    for feed in feeds {
        match feed {
            crate::feeds::Feed::Fact(fact) => emit(ingress, epoch, AppEv::Feature(fact)),
            crate::feeds::Feed::Job(job) => inflight.push(job),
        }
    }
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
pub(crate) fn reconnect_endpoint(mode: &Mode) -> Option<RemoteAppServerEndpoint> {
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
///
/// `fallback`: the daemon was found running, not started for this launch; one that does not
/// answer leaves the launch on the embedded server (`mode` says which it is).
async fn connect_first(
    mode: &mut Mode,
    fallback: bool,
    config: &Arc<Config>,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<(AppServerClient, bool)> {
    if fallback && matches!(mode, Mode::Daemon) {
        if let Some(endpoint) = reconnect_endpoint(mode)
            && let Some(client) = connect_remote(endpoint).await
        {
            return Ok((client, true));
        }
        *mode = Mode::Embedded;
    }
    match mode {
        Mode::Embedded => {
            let args = start_args(config.clone(), arg0_paths).await?;
            let local = !args
                .environment_manager
                .default_environment()
                .is_some_and(|environment| environment.is_remote());
            // Boxed: the start future is large, and the pump should not carry it inline.
            let client = Box::pin(InProcessAppServerClient::start(args))
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            Ok((AppServerClient::InProcess(client), local))
        }
        Mode::Daemon => {
            // As the oracle's startup: the daemon is started when it is not
            // running (`startup_orchestration.rs`), and a failure says how to
            // work without it.
            let started =
                codex_app_server_daemon::start_with_features_classified(&Default::default())
                    .await
                    .map_err(|failure| daemon_failure(&format!("{:#}", failure.error)))?;
            let socket_path = AbsolutePathBuf::from_absolute_path_checked(started.socket_path)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let client = connect_remote(RemoteAppServerEndpoint::UnixSocket { socket_path })
                .await
                .ok_or_else(|| daemon_failure("failed to connect to the local daemon"))?;
            Ok((client, true))
        }
        Mode::Remote { endpoint, daemon } => {
            let client = connect_remote(endpoint.clone())
                .await
                .ok_or_else(|| anyhow::anyhow!("failed to connect to remote app server"))?;
            Ok((client, *daemon))
        }
    }
}

/// A daemon that cannot be started or reached ends the launch with `detail` and the way to
/// work without it (`daemon_startup.rs::FAILURE_HINT`).
fn daemon_failure(detail: &str) -> anyhow::Error {
    anyhow::anyhow!("{detail}\n{}", crate::daemon::FAILURE_HINT)
}

/// One connection to a server at `endpoint` (oracle:
/// `connect_remote_app_server`); a failure is `None`, whose text the oracle
/// never shows.
pub(crate) async fn connect_remote(endpoint: RemoteAppServerEndpoint) -> Option<AppServerClient> {
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
    let send = sender(handle.clone());
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
        command @ (Command::StartTurn { ticket, .. }
        | Command::Screen {
            ticket,
            command: ScreenCommand::Chat(chat::Command::Steer { .. }),
            ..
        }) => Box::pin(async move {
            let request = protocol::client_request(&command, ids.next(), model.as_deref())?;
            if let AppServerRequestHandle::Cloud(cloud) = &handle {
                return crate::cloud::submit(ticket, request, |params| {
                    cloud.submit_user_message(params)
                })
                .await;
            }
            let own_message = wire::reads_message_only(&request);
            let response = handle.request_typed::<serde_json::Value>(request).await;
            let (ticket, reply) = match response {
                Err(error @ TypedRequestError::Transport { .. }) => {
                    (ticket, Reply::submission_unconfirmed(error.to_string()))
                }
                result => protocol::reply_from_server(
                    &command,
                    result.map_err(|error| server_error(own_message, &error)),
                )?,
            };
            Some(AppEv::Reply(ticket, reply))
        }),
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
                .map_err(|error| server_error(own_message, &error))
        })
    }
}

fn server_error(own_message: bool, error: &TypedRequestError) -> ServerError {
    let (code, data) = match error {
        TypedRequestError::Server { source, .. } => (Some(source.code), source.data.clone()),
        _ => (None, None),
    };
    ServerError {
        code,
        data,
        message: error_text(own_message, error),
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

    // `daemon_startup_falls_back_only_for_implicit_endpoints`, the port's half: the daemon is a
    // required attachment, so a failed start or connection says how to work without it.
    #[test]
    fn a_daemon_that_cannot_be_reached_names_the_way_to_work_without_it() {
        let message = daemon_failure("failed to connect to the local daemon").to_string();
        assert!(message.starts_with("failed to connect to the local daemon\n"));
        assert!(
            message.contains("rerun the same command with --no-daemon"),
            "{message}"
        );
    }

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
