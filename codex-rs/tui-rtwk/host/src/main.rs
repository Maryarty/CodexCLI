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
mod providers;
mod security_setup;
mod settings;
mod small;
mod startup;
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
use codex_tui_rtwk::app::run::run_on;
use codex_tui_rtwk::services::shell::Backend;

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

/// Runs the app against the in-process app-server until the user quits.
async fn run_main(cli: Cli, arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
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
        cwd: cli.cwd.clone(),
        config_overrides: cli.config_overrides.clone(),
        startup_warning,
        failed: failed_tx,
        agents: cli.agents,
        update: update.clone(),
        startup_failure: startup_failure.clone(),
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
        .with_lang(cli.lang(&rtwk::l10n::requested()));
    if let Some(start) = cli.session.clone() {
        app = app.with_start_session(start);
    }
    let result = run_on(app, !cli.no_alt_screen);
    // Quitting aborted the backend future (rtwk unmounts the app and aborts
    // its task), so nothing after its loop runs: the client came back
    // through `done_rx`, and the shutdown happens here.
    if let Ok(Ok(client)) = tokio::time::timeout(SHUTDOWN_WAIT, done_rx).await {
        let _ = client.shutdown().await;
    }
    result.map_err(|error| anyhow::anyhow!("the terminal app failed: {error}"))?;
    // The server did not start: the process ends with the error, as the
    // oracle's does (`Error: <message>`), after the terminal is restored.
    if let Ok(message) = failed_rx.try_recv() {
        anyhow::bail!(message);
    }
    // A launch fact that could not be read ends the process the same way.
    if let Some(message) = startup_failure.lock().ok().and_then(|mut slot| slot.take()) {
        anyhow::bail!(message);
    }
    let chosen = update.lock().ok().and_then(|mut slot| slot.take());
    if let Some(action) = chosen {
        update::run_update(action).map_err(anyhow::Error::msg)?;
    }
    Ok(())
}
