//! Choose the startup viewport before the terminal's application modes are entered.

use std::io;
use std::path::Path;

use codex_config::types::AltScreenMode;
use codex_core::config::{
    Config, ConfigLoadOptions, find_codex_home, load_config_toml_with_layer_stack,
};
use codex_terminal_detection::{TerminalName, terminal_info};
use codex_tui_core::app::facts::TuiSettings;
use codex_tui_rtwk::Cli;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_cli::CliConfigOverrides;

pub(super) async fn alternate_screen(cli: &Cli) -> bool {
    if cli.no_alt_screen {
        return false;
    }
    // Read the same config layers as the backend. It still owns presentation of broken config.
    let configured = match find_codex_home() {
        Ok(home) => match load(home.as_path(), cli).await {
            Ok(mode) => mode,
            Err(error) => {
                tracing::debug!(%error, "could not read startup screen mode");
                AltScreenMode::Auto
            }
        },
        Err(_) => AltScreenMode::Auto,
    };
    match configured {
        AltScreenMode::Always => true,
        AltScreenMode::Never => false,
        AltScreenMode::Auto => !terminal_app_over_ssh(),
    }
}

async fn load(home: &Path, cli: &Cli) -> io::Result<AltScreenMode> {
    let cwd = match cli.cwd.as_deref() {
        Some(path) => AbsolutePathBuf::relative_to_current_dir(path)?,
        None => AbsolutePathBuf::current_dir()?,
    };
    let overrides = CliConfigOverrides {
        raw_overrides: cli.config_overrides.clone(),
    }
    .parse_overrides()
    .map_err(io::Error::other)?;
    let loaded = load_config_toml_with_layer_stack(
        home,
        Some(&cwd),
        overrides,
        ConfigLoadOptions::default(),
    )
    .await?;
    Ok(loaded
        .config_toml
        .tui
        .map_or(AltScreenMode::Auto, |tui| tui.alternate_screen))
}

fn terminal_app_over_ssh() -> bool {
    let ssh = std::env::var_os("SSH_TTY").is_some() || std::env::var_os("SSH_CONNECTION").is_some();
    if !ssh {
        return false;
    }
    let terminal = terminal_info();
    // SSH normally forwards TERM without TERM_PROGRAM. rtwk reads device identity during its
    // native probe, but the screen options must be fixed before open and its input queue is private.
    terminal.name == TerminalName::AppleTerminal && terminal.multiplexer.is_none()
}

/// A reloaded setting cannot switch the terminal's buffer. Copy mode only owns its transcript
/// when the launch actually drew on the alternate screen.
pub(super) fn tui_settings(config: &Config, alternate_screen: bool) -> TuiSettings {
    TuiSettings {
        transcript_owned: config.tui_fullscreen_transcript && alternate_screen,
        ..crate::effects::tui_settings(config)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    #[tokio::test]
    async fn terminal_mode_loads_tui_config_and_cli_override() -> anyhow::Result<()> {
        let home = tempfile::tempdir()?;
        let cwd = tempfile::tempdir()?;
        let mut cli = Cli {
            cwd: Some(cwd.path().to_owned()),
            ..Cli::default()
        };
        assert_eq!(load(home.path(), &cli).await?, AltScreenMode::Auto);
        std::fs::File::create(home.path().join("config.toml"))?
            .write_all(b"[tui]\nalternate_screen = 'never'\n")?;
        assert_eq!(load(home.path(), &cli).await?, AltScreenMode::Never);
        cli.config_overrides
            .push("tui.alternate_screen=always".into());
        assert_eq!(load(home.path(), &cli).await?, AltScreenMode::Always);
        cli.no_alt_screen = true;
        assert!(!alternate_screen(&cli).await);
        Ok(())
    }

    #[tokio::test]
    async fn terminal_mode_physical_buffer_controls_runtime_transcript_ownership()
    -> anyhow::Result<()> {
        let home = tempfile::tempdir()?;
        let config = codex_core::config::ConfigBuilder::default()
            .codex_home(home.path().to_owned())
            .fallback_cwd(Some(home.path().to_owned()))
            .cli_overrides(vec![
                ("tui.fullscreen_transcript".into(), true.into()),
                ("tui.alternate_screen".into(), "never".into()),
            ])
            .build()
            .await?;
        // Changing the configured mode during this launch cannot switch the physical buffer.
        assert!(tui_settings(&config, true).transcript_owned);
        assert!(!tui_settings(&config, false).transcript_owned);
        Ok(())
    }
}
