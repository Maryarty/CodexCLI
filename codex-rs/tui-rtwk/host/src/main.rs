mod in_process;

use anyhow::Context as _;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_core::config::ConfigBuilder;
use codex_tui_rtwk::Cli;
use codex_tui_rtwk::app::CodexApp;
use codex_tui_rtwk::shell::Backend;
use codex_tui_rtwk::ui::run::run;

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
        done: Some(done_tx),
    }));
    let app = CodexApp::new(backend).with_debug_screens(cli.debug_screens);
    let result = run(app);
    // The app is gone: its command channel closed, the client shuts down.
    let _ = done_rx.await;
    result.map_err(|error| anyhow::anyhow!("the terminal app failed: {error}"))
}
