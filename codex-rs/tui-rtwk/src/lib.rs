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
}

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
                "-m" | "--model" | "--locale" => {
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
                    if let Some(value) = other.strip_prefix("--locale=") {
                        cli.locale = Some(parse_locale(value)?);
                    } else if let Some(value) = other.strip_prefix("--model=") {
                        cli.model = Some(value.to_string());
                    } else {
                        anyhow::bail!("unknown argument: {other}");
                    }
                }
            }
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
