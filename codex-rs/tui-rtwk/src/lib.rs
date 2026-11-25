//! `codex-tui-rtwk`: the Codex terminal UI on rtwk, the UI crate. Its model is
//! `UiApp { core, ui }` (`app`): the core model of `codex-tui-core` (models,
//! `apply`, protocol mapping; no rtwk) plus UI-only state. The shell that runs
//! effects and owns the backend (`shell`), the widgets outside the screens
//! (`ui`), the screen directories (`screens`, view files only) and the
//! composition root (`run`) are here. Architecture:
//! `docs/rtwk-port/architecture.md`, "Layers".

pub mod app;
pub mod features;
pub mod keymap;
pub mod l10n;
pub mod oracle_strings;
pub mod pages;
pub mod services;
pub mod sheets;
pub mod widgets;

/// The command line of `codex-rtwk`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cli {
    /// Open the debug menu instead of the chat.
    pub debug_screens: bool,
    /// `--locale`: the language of the app's text, over the environment's.
    pub locale: Option<rtwk::l10n::Locale>,
    /// `-m` / `--model`: the model for the new thread, over the config's.
    pub model: Option<String>,
    /// `--oss`: the open-source (local) provider, over the config's.
    pub oss: bool,
    /// `--local-provider ID`: which local provider `--oss` uses (`lmstudio`
    /// or `ollama`); without it the config's `oss_provider`, then the one
    /// server that is running.
    pub oss_provider: Option<String>,
    /// The `agents` subcommand: open the agent command center first.
    pub agents: bool,
    /// The `resume` subcommand: open the resume picker first.
    pub resume: bool,
    /// `resume <ID>` / `fork <ID>`: the session the first thread opens (the
    /// directory prompt comes first when it differs from this one).
    pub session: Option<codex_tui_core::screens::onboarding::session::Start>,
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
    /// The positional prompt of `codex "prompt"`: it is sent when the first thread starts.
    pub prompt: Option<String>,
    /// `-i` / `--image FILE[,FILE...]`: images that go with the prompt.
    pub images: Vec<std::path::PathBuf>,
    /// `-c key=value` / `--config key=value`, raw, in order (the host parses
    /// them as the oracle does and applies them over the config).
    pub config_overrides: Vec<String>,
}

/// The refusal of the connection flags that needs no server to tell, as the
/// oracle words it (`startup_orchestration.rs`); the rest (`--remote` values,
/// the token variable) is the host's, with the oracle's words.
const NO_DAEMON_WITH_REMOTE: &str = "--no-daemon cannot be used with --remote.";
/// The session picker of `codex fork` is not carried over: it names the
/// session.
const FORK_NEEDS_ID: &str = "`codex fork` needs a session ID";
const AGENTS_NO_PROMPT: &str = "`codex agents` does not accept an initial prompt or images";
const AGENTS_CONFLICTING_REMOTE: &str =
    "`codex agents` received conflicting remote server endpoints";

impl Cli {
    /// The text `--help`, `-h`, `help`, `--version` or `-V` asks for, if the
    /// command line asks for one: what the oracle's `codex` prints, for the
    /// top level and the `agents` subcommand. The first such word wins, as
    /// in clap; `--version` of a subcommand is no answer (the oracle's
    /// subcommands have none, and the parse says so).
    pub fn early_answer(args: impl IntoIterator<Item = String>) -> Option<&'static str> {
        use services::cli_help as help;
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
                | "--local-provider"
                | "-C"
                | "--cd"
                | "-c"
                | "--config"
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
        // `resume` / `fork` and the session id (a word of its own) they name.
        let mut verb: Option<codex_tui_core::screens::startup::Action> = None;
        let mut session_id: Option<String> = None;
        // What `codex agents` refuses: a prompt or images, and two endpoints.
        let mut prompt_or_images = false;
        let mut other_remote: Option<String> = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--debug-screens" => cli.debug_screens = true,
                "agents" => cli.agents = true,
                "--no-alt-screen" => cli.no_alt_screen = true,
                "--no-daemon" => cli.no_daemon = true,
                "--oss" => cli.oss = true,
                "--local-provider" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--local-provider needs a value"))?;
                    cli.oss_provider = Some(value);
                }
                "-c" | "--config" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))?;
                    cli.config_overrides.push(value);
                }
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
                    if cli.remote.as_ref().is_some_and(|first| *first != value) {
                        other_remote = Some(value.clone());
                    }
                    cli.remote = Some(value);
                }
                "-i" | "--image" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))?;
                    cli.images.extend(value.split(',').map(Into::into));
                    prompt_or_images = true;
                }
                "--remote-auth-token-env" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("{arg} needs a value"))?;
                    cli.remote_auth_token_env = Some(value);
                }
                "resume" => verb = Some(codex_tui_core::screens::startup::Action::Resume),
                "fork" => verb = Some(codex_tui_core::screens::startup::Action::Fork),
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
                    if let Some(value) = other.strip_prefix("--config=") {
                        cli.config_overrides.push(value.to_string());
                    } else if let Some(value) = other.strip_prefix("--cd=") {
                        cli.cwd = Some(value.into());
                    } else if let Some(value) = other.strip_prefix("--remote=") {
                        if cli.remote.as_deref().is_some_and(|first| first != value) {
                            other_remote = Some(value.to_string());
                        }
                        cli.remote = Some(value.to_string());
                    } else if let Some(value) = other.strip_prefix("--image=") {
                        cli.images.extend(value.split(',').map(Into::into));
                        prompt_or_images = true;
                    } else if verb.is_some() && session_id.is_none() && !other.starts_with('-') {
                        session_id = Some(other.to_owned());
                    } else if !other.starts_with('-') && cli.prompt.is_none() {
                        // The positional prompt of `codex`.
                        cli.prompt = Some(other.to_owned());
                        prompt_or_images = true;
                    } else if let Some(value) = other.strip_prefix("--remote-auth-token-env=") {
                        cli.remote_auth_token_env = Some(value.to_string());
                    } else if let Some(value) = other.strip_prefix("--locale=") {
                        cli.locale = Some(parse_locale(value)?);
                    } else if let Some(value) = other.strip_prefix("--model=") {
                        cli.model = Some(value.to_string());
                    } else if let Some(value) = other.strip_prefix("--local-provider=") {
                        cli.oss_provider = Some(value.to_string());
                    } else {
                        anyhow::bail!("unknown argument: {other}");
                    }
                }
            }
        }
        match (verb, session_id) {
            (Some(action), Some(id)) => {
                cli.session =
                    Some(codex_tui_core::screens::onboarding::session::Start { action, id });
            }
            (Some(codex_tui_core::screens::startup::Action::Resume), None) => cli.resume = true,
            (Some(codex_tui_core::screens::startup::Action::Fork), None) => {
                anyhow::bail!(FORK_NEEDS_ID);
            }
            (None, _) => {}
        }
        if cli.agents && other_remote.is_some() {
            anyhow::bail!(AGENTS_CONFLICTING_REMOTE);
        }
        if cli.agents && prompt_or_images {
            anyhow::bail!(AGENTS_NO_PROMPT);
        }
        if cli.remote.is_some() && cli.no_daemon {
            anyhow::bail!(NO_DAEMON_WITH_REMOTE);
        }
        Ok(cli)
    }

    /// The prompt of the command line as the first message: the text, then the placeholder of
    /// each image after it (`create_initial_user_message`).
    pub fn initial_message(
        &self,
        lang: l10n::Lang,
    ) -> Option<codex_tui_core::threads::UserMessage> {
        let mut text = self.prompt.clone().unwrap_or_default();
        if text.is_empty() && self.images.is_empty() {
            return None;
        }
        let mut message = codex_tui_core::threads::UserMessage::default();
        for (index, path) in self.images.iter().enumerate() {
            if !text.is_empty() {
                text.push(' ');
            }
            let start = text.len();
            text.push_str(&pages::session::composer::strings::image_label(
                lang,
                index + 1,
            ));
            message.images.push(codex_tui_core::threads::LocalImage {
                path: path.clone(),
                range: start..text.len(),
            });
        }
        message.text = text;
        Some(message)
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
