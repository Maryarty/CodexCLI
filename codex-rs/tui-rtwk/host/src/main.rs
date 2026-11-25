// The in-process app-server client's start future nests the whole server's
// async call tree, so its layout needs more than the default query depth (the
// oracle's `codex-tui` and `codex-app-server-client` raise it to the same value).
#![recursion_limit = "256"]

mod analytics;
mod cloud;
mod daemon;
mod debug_config;
mod effects;
mod facts;
mod feeds;
mod gateway;
mod import;
mod in_process;
mod launch;
mod oss;
mod providers;
mod security_setup;
mod settings;
mod small;
mod startup;
mod state_db;
mod target;
mod update;
mod usage;
mod viewer;
mod windows_sandbox;
mod workspace;

use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_tui_rtwk::Cli;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::app::UnsentSlot;
use codex_tui_rtwk::app::run::run_on;
use codex_tui_rtwk::services::shell::Backend;
use codex_tui_rtwk::services::startup::UpdateAction;

/// How long `main` waits for the backend future to hand the client back.
const SHUTDOWN_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

fn main() -> anyhow::Result<()> {
    // The command line is parsed inside the closure: `arg0_dispatch_or_else`
    // first lets the helper re-runs of this binary (`--codex-run-as-fs-helper`,
    // the exec helper, apply_patch) take their arguments and exit.
    arg0_dispatch_or_else(|arg0_paths| async move {
        // `--help` and `--version` answer as the oracle's `codex` does, before
        // any server starts.
        if let Some(text) = Cli::early_answer(std::env::args().skip(1)) {
            print!("{text}");
            return Ok(());
        }
        let cli = Cli::parse(std::env::args().skip(1))?;
        run_main(cli, arg0_paths).await
    })
}

/// Runs the app against the in-process app-server until the user quits. A launch the local
/// database stopped is tried again, as the oracle CLI's `run_tui_with_recovery` does: a locked
/// database ends it with guidance, a damaged one is moved into a backup folder first, once per
/// database file.
async fn run_main(cli: Cli, arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    let mut attempted = std::collections::HashSet::new();
    loop {
        let startup_error = match launch(&cli, arg0_paths.clone()).await? {
            Launch::Ended => return Ok(()),
            Launch::StateDb(startup_error) => startup_error,
        };
        match state_db::recover(&startup_error, &mut attempted).await? {
            state_db::Recovery::Retry => {}
            state_db::Recovery::Fatal(message) => anyhow::bail!(message),
        }
    }
}

/// How one launch ended.
enum Launch {
    /// The user quit, or the process is to end with what the launch reported (an `Err`).
    Ended,
    /// The local database could not be opened; the terminal is restored.
    StateDb(state_db::StateDbStartupError),
}

/// One launch: the app on its backend, until it ends.
async fn launch(cli: &Cli, arg0_paths: Arg0DispatchPaths) -> anyhow::Result<Launch> {
    // First frame first: the config, the state DB and the environment manager
    // are loaded by the backend future, after the terminal has painted the
    // composer (the oracle's `StartupDraft`); a failure arrives as `StartFailed`.
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let (failed_tx, mut failed_rx) = tokio::sync::oneshot::channel::<String>();
    let flags = target::Flags {
        no_daemon: cli.no_daemon,
        agents: cli.agents,
        remote: cli.remote.as_deref(),
        remote_auth_token_env: cli.remote_auth_token_env.as_deref(),
        config_overrides: &cli.config_overrides,
    };
    let mode = target::Mode::resolve(&flags, |name| std::env::var(name))?;
    {
        use std::io::IsTerminal;
        if let Some(refusal) = target::terminal_refusal(
            &flags,
            &mode,
            std::io::stdin().is_terminal(),
            std::io::stdout().is_terminal(),
        ) {
            anyhow::bail!(refusal);
        }
    }
    let startup_warning = target::startup_warning(&flags);
    // The update the startup page's prompt chose: run once the terminal is
    // restored (the oracle's `UpdatePromptOutcome::RunUpdate`).
    let update = startup::UpdateSlot::default();
    let startup_failure = startup::FailureSlot::default();
    let state_db_failure = state_db::FailureSlot::default();
    // What the user wrote and the launch never took, printed when it fails.
    let unsent = UnsentSlot::default();
    let cloud = cloud::Cloud {
        id: codex_tui_rtwk::services::shell::CLOUD_BACKEND,
        mode: mode.clone(),
        model: cli.model.clone(),
        cwd: cli.cwd.clone(),
        config_overrides: cli.config_overrides.clone(),
    };
    let first = Backend::Connector(Box::new(in_process::InProcess {
        arg0_paths,
        mode,
        done: done_tx,
        model: cli.model.clone(),
        oss: cli.oss,
        oss_provider: cli.oss_provider.clone(),
        cwd: cli.cwd.clone(),
        config_overrides: cli.config_overrides.clone(),
        startup_warning,
        failed: failed_tx,
        agents: cli.agents,
        launch_prompt: cli.prompt.as_ref().is_some_and(|prompt| !prompt.is_empty()),
        update: update.clone(),
        startup_failure: startup_failure.clone(),
        state_db_failure: state_db_failure.clone(),
    }));
    // The Cloud connection next to the first, when the oracle's gate lets it
    // be tried (it decides inside, once the config is read).
    let backend = Backend::Several {
        first: Box::new(first),
        others: vec![(
            codex_tui_rtwk::services::shell::CLOUD_BACKEND,
            Backend::Connector(Box::new(cloud)),
        )],
    };
    let mut app = UiApp::new(backend)
        .with_debug_screens(cli.debug_screens)
        .with_start_agents(cli.agents)
        .with_start_resume(cli.resume)
        .with_awaited_facts(true)
        .with_folder_consent(true)
        .with_unsent_slot(unsent.clone())
        .with_lang(cli.lang(&rtwk::l10n::requested()));
    if let Some(start) = cli.session.clone() {
        app = app.with_start_session(start);
    }
    if let Some(message) = cli.initial_message(cli.lang(&rtwk::l10n::requested())) {
        app = app.with_initial_prompt(message);
    }
    let result = run_on(app, !cli.no_alt_screen);
    // Quitting aborted the backend future (rtwk unmounts the app and aborts
    // its task), so nothing after its loop runs: the client came back
    // through `done_rx`, and the shutdown happens here.
    if let Ok(Ok(client)) = tokio::time::timeout(SHUTDOWN_WAIT, done_rx).await {
        let _ = client.shutdown().await;
    }
    result.map_err(|error| anyhow::anyhow!("the terminal app failed: {error}"))?;
    let failed = failed_rx.try_recv().ok();
    // A launch fact that could not be read ends the process the same way.
    let startup_failed = startup_failure.lock().ok().and_then(|mut slot| slot.take());
    // The terminal is restored: what the user wrote is printed before the error, as the
    // oracle's `restore_terminal_before_fatal_exit` does.
    if failed.is_some() || startup_failed.is_some() {
        print_unsent_draft(&unsent);
    }
    if let Some(typed) = state_db_failure
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
    {
        return Ok(Launch::StateDb(typed));
    }
    // The server did not start: the process ends with the error, as the
    // oracle's does (`Error: <message>`), after the terminal is restored.
    if let Some(message) = failed {
        anyhow::bail!(message);
    }
    if let Some(message) = startup_failed {
        anyhow::bail!(message);
    }
    let chosen = update.lock().ok().and_then(|mut slot| slot.take());
    match chosen {
        Some(UpdateAction::Daemon(source)) => {
            update::run_daemon_update(daemon::update_output(source, cli.config_overrides.clone()))
                .await
                .map_err(anyhow::Error::msg)?;
        }
        Some(action) => update::run_update(action).map_err(anyhow::Error::msg)?,
        None => {}
    }
    Ok(Launch::Ended)
}

/// The unsent draft on stderr, once (`startup_recovery::print_unsent_draft`).
fn print_unsent_draft(unsent: &UnsentSlot) {
    use std::io::Write as _;
    if let Some(text) = unsent.lock().ok().and_then(|mut slot| slot.take()) {
        let _ = writeln!(std::io::stderr().lock(), "\nUnsent draft:\n{text}\n");
    }
}
