// The in-process app-server client's start future nests the whole server's
// async call tree, so its layout needs more than the default query depth (the
// oracle's `codex-tui` and `codex-app-server-client` raise it to the same value).
#![recursion_limit = "256"]

mod in_process;
mod workspace;

use anyhow::Context as _;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_core::config::ConfigBuilder;
use codex_tui_rtwk::Cli;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::run::run;
use codex_tui_rtwk::shell::Backend;

/// How long `main` waits for the backend future to hand the client back.
const SHUTDOWN_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

fn main() -> anyhow::Result<()> {
    // The command line is parsed inside the closure: `arg0_dispatch_or_else`
    // first lets the helper re-runs of this binary (`--codex-run-as-fs-helper`,
    // the exec helper, apply_patch) take their arguments and exit.
    arg0_dispatch_or_else(|arg0_paths| async move {
        let cli = Cli::parse(std::env::args().skip(1))?;
        run_main(cli, arg0_paths).await
    })
}

/// Runs the app against the in-process app-server until the user quits.
async fn run_main(cli: Cli, arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    let config = ConfigBuilder::default()
        .build()
        .await
        .context("loading the config")?;
    let args = in_process::start_args(config, arg0_paths).await?;
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let backend = Backend::Connector(Box::new(in_process::InProcess {
        args,
        done: done_tx,
        model: cli.model.clone(),
    }));
    let app = UiApp::new(backend)
        .with_debug_screens(cli.debug_screens)
        .with_start_agents(cli.agents)
        .with_lang(cli.lang(&rtwk::l10n::requested()));
    let result = run(app);
    // Quitting aborted the backend future (rtwk unmounts the app and aborts
    // its task), so nothing after its loop runs: the client came back
    // through `done_rx`, and the shutdown happens here.
    if let Ok(Ok(client)) = tokio::time::timeout(SHUTDOWN_WAIT, done_rx).await {
        let _ = client.shutdown().await;
    }
    result.map_err(|error| anyhow::anyhow!("the terminal app failed: {error}"))
}
