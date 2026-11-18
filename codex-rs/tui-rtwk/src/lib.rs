//! `codex-tui-rtwk`: the Codex terminal UI on rtwk. The skeleton: the model
//! and its one writer (`app`, `threads`), the shell that owns the backend
//! (`shell`), the widgets outside the screens (`ui`) and the screen slots
//! (`screens`) the port's slices fill. Architecture: `docs/rtwk-port/architecture.md`.

pub mod app;
pub mod commands;
pub mod ids;
pub mod keymap;
pub mod screens;
pub mod shell;
pub mod theme;
pub mod threads;
pub mod ui;

/// The command line of `codex-rtwk`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cli {
    /// Open the debug menu instead of the chat.
    pub debug_screens: bool,
}

impl Cli {
    pub fn parse(args: impl IntoIterator<Item = String>) -> anyhow::Result<Cli> {
        let mut cli = Cli::default();
        for arg in args {
            match arg.as_str() {
                "--debug-screens" => cli.debug_screens = true,
                other => anyhow::bail!("unknown argument: {other}"),
            }
        }
        Ok(cli)
    }
}
