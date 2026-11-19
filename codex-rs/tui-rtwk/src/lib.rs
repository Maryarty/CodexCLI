//! `codex-tui-rtwk`: the Codex terminal UI on rtwk, the UI crate. Its model is
//! `UiApp { core, ui }` (`app`): the core model of `codex-tui-core` (models,
//! `apply`, protocol mapping; no rtwk) plus UI-only state. The shell that runs
//! effects and owns the backend (`shell`), the widgets outside the screens
//! (`ui`), the screen directories (`screens`, view files only) and the
//! composition root (`run`) are here. Architecture:
//! `docs/rtwk-port/architecture.md`, "Layers".

pub mod app;
pub mod keymap;
pub mod l10n;
pub mod run;
pub mod screens;
pub mod shell;
pub mod theme;
pub mod ui;

/// The command line of `codex-rtwk`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cli {
    /// Open the debug menu instead of the chat.
    pub debug_screens: bool,
    /// `--locale`: the language of the app's text, over the environment's.
    pub locale: Option<rtwk::l10n::Locale>,
    /// `-m` / `--model`: the model for the new thread, over the config's.
    pub model: Option<String>,
    /// The `agents` subcommand: open the agent command center first.
    pub agents: bool,
    /// The `resume` subcommand: open the resume picker first.
    pub resume: bool,
    /// `-C` / `--cd`: the directory the agent works in (new tasks start there).
    pub cwd: Option<std::path::PathBuf>,
    /// `--no-alt-screen`: run inline, keeping the terminal's scrollback.
    pub no_alt_screen: bool,
    /// `--no-daemon`: the embedded app server, not the local daemon.
    pub no_daemon: bool,
    /// `--remote ADDR`: an explicit app server (`ws://host:port`,
    /// `wss://host:port`, `unix://[PATH]`); the host resolves it.
    pub remote: Option<String>,
    /// `--remote-auth-token-env VAR`: the variable holding its bearer token.
    pub remote_auth_token_env: Option<String>,
}

/// The refusal of the connection flags that needs no server to tell, as the
/// oracle words it (`startup_orchestration.rs`); the rest (`--remote` values,
/// the token variable) is the host's, with the oracle's words.
const NO_DAEMON_WITH_REMOTE: &str = "--no-daemon cannot be used with --remote.";

impl Cli {
    /// The text `--help`, `-h`, `help`, `--version` or `-V` asks for, if the
    /// command line asks for one: what the oracle's `codex` prints, for the
    /// top level and the `agents` subcommand. The first such word wins, as
    /// in clap; `--version` of a subcommand is no answer (the oracle's
    /// subcommands have none, and the parse says so).
    pub fn early_answer(args: impl IntoIterator<Item = String>) -> Option<&'static str> {
        use shell::help;
        let mut agents = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" => {
                    return Some(if agents {
                        help::AGENTS_LONG
                    } else {
                        help::TOP_LONG
                    });
                }
                "-h" => {
                    return Some(if agents {
                        help::AGENTS_SHORT
                    } else {
                        help::TOP_SHORT
                    });
                }
                "-V" | "--version" if !agents => return Some(help::VERSION),
                "help" if !agents => {
                    return match args.next().as_deref() {
                        None => Some(help::TOP_LONG),
                        Some("agents") => Some(help::AGENTS_LONG),
                        Some(_) => None,
                    };
                }
                "agents" => agents = true,
                // The value of a flag is not a word of the command line.
                "-m"
                | "--model"
                | "--locale"
                | "-C"
                | "--cd"
                | "--remote"
                | "--remote-auth-token-env" => {
                    args.next();
                }
                _ => {}
            }
        }
        None
    }

    pub fn parse(args: impl IntoIterator<Item = String>) -> anyhow::Result<Cli> {
        let mut cli = Cli::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--debug-screens" => cli.debug_screens = true,
                "agents" => cli.agents = true,
                "--no-alt-screen" => cli.no_alt_screen = true,
                "--no-daemon" => cli.no_daemon = true,
                "-C" | "--cd" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))?;
                    cli.cwd = Some(value.into());
                }
                "--remote" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--remote needs a value"))?;
                    cli.remote = Some(value);
                }
                "--remote-auth-token-env" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))?;
                    cli.remote_auth_token_env = Some(value);
                }
                "resume" => cli.resume = true,
                "-m" | "--model" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))?;
                    cli.model = Some(value);
                }
                "--locale" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--locale needs a value"))?;
                    cli.locale = Some(parse_locale(&value)?);
                }
                other => {
                    if let Some(value) = other.strip_prefix("--cd=") {
                        cli.cwd = Some(value.into());
                    } else if let Some(value) = other.strip_prefix("--remote=") {
                        cli.remote = Some(value.to_string());
                    } else if let Some(value) = other.strip_prefix("--remote-auth-token-env=") {
                        cli.remote_auth_token_env = Some(value.to_string());
                    } else if let Some(value) = other.strip_prefix("--locale=") {
                        cli.locale = Some(parse_locale(value)?);
                    } else if let Some(value) = other.strip_prefix("--model=") {
                        cli.model = Some(value.to_string());
                    } else {
                        anyhow::bail!("unknown argument: {other}");
                    }
                }
            }
        }
        if cli.remote.is_some() && cli.no_daemon {
            anyhow::bail!(NO_DAEMON_WITH_REMOTE);
        }
        Ok(cli)
    }

    /// The language to run in: `--locale` if given, else the first of the
    /// `requested` locales (the environment's) that has a table; English
    /// otherwise.
    pub fn lang(&self, requested: &[rtwk::l10n::Locale]) -> l10n::Lang {
        match self.locale {
            Some(locale) => l10n::Lang::negotiate(&[locale]),
            None => l10n::Lang::negotiate(requested),
        }
    }
}

fn parse_locale(value: &str) -> anyhow::Result<rtwk::l10n::Locale> {
    value
        .parse()
        .map_err(|error| anyhow::anyhow!("--locale {value}: {error}"))
}
