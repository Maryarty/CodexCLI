mod in_process;

use anyhow::Context as _;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_core::config::ConfigBuilder;
use codex_tui_rtwk::Cli;
use codex_tui_rtwk::app::CodexApp;
use codex_tui_rtwk::shell::Backend;
use codex_tui_rtwk::ui::run::run;

/// How long `main` waits for the backend future to hand the client back.
const SHUTDOWN_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse(std::env::args().skip(1))?;
    arg0_dispatch_or_else(move |arg0_paths| run_main(cli, arg0_paths))
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
    }));
    let app = CodexApp::new(backend)
        .with_debug_screens(cli.debug_screens)
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
