//! The host's side of the commands that are this machine's own: the effects of the chat (copy,
//! git diff, export, clipboard text, Desktop link), the composer's popovers (history file, file
//! search, mention usage, clipboard image) and the browser. In Flutter this is the repository
//! layer the app's `Command`s go through: the pump gives each `Command` to [`Effects::run`], runs
//! the future it returns and sends the app the event it answers.
//!
//! The services are the library's (`services::effects::Services`); this file builds them from the
//! loaded config, writes the terminal's clipboard request a copy asked for, and reads the `[tui]`
//! table into the settings the views read.

use std::path::PathBuf;
use std::sync::Arc;

use codex_app_server_client::AppServerRequestHandle;
use codex_config::types::CopyOnSelect;
use codex_core::config::Config;
use codex_terminal_detection::{TerminalInfo, TerminalName, terminal_info};
use codex_tui_core::app::AppEv;
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::effects::Chain;
use codex_tui_core::app::effects::ConfigEdit;
use codex_tui_core::app::effects::DaemonAsk;
use codex_tui_core::app::effects::Done;
use codex_tui_core::app::effects::EditorFailure;
use codex_tui_core::app::effects::HostDoors;
use codex_tui_core::app::effects::IdeFailure;
use codex_tui_core::app::effects::IdePrompt;
use codex_tui_core::app::effects::IdeReading;
use codex_tui_core::app::effects::run_ask;
use codex_tui_core::app::facts::Rendering;
use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::ids::{ConnEpoch, ThreadId};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::popovers;
use codex_tui_core::screens::select::quick_dials::{Dial, Origin, SLOTS, Slot};
use codex_tui_core::screens::select::review::Commit;
use codex_tui_rtwk::services::agents::workspace::Boxed;
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_tui_rtwk::services::clipboard::Clipboard;
use codex_tui_rtwk::services::clipboard::Machine;
use codex_tui_rtwk::services::clipboard::environment;
use codex_tui_rtwk::services::effects::Outcome;
use codex_tui_rtwk::services::effects::Services;
use codex_tui_rtwk::services::file_search::InProcessSearch;
use codex_tui_rtwk::services::history::file::MessageHistory;
use codex_tui_rtwk::services::history::usage::MentionFile;
use codex_tui_rtwk::services::shell::IngressTx;
use codex_tui_rtwk::services::shell::WorkspaceExec;

use crate::startup::{Browser, UpdateSlot};

mod pets;

/// Where the terminal's clipboard request goes: the owner of the terminal writes it.
type Terminal = Arc<dyn Fn(&str) + Send + Sync>;

/// The machine's services of one connection, and the way to the terminal.
pub(crate) struct Effects {
    services: Services,
    terminal: Terminal,
    /// The config the external editor's runner reads (the codex home, the sandbox policy).
    config: Option<Arc<Config>>,
    /// Where `/daemon`'s "Update and exit" leaves the update for `main` to run once the
    /// terminal is restored (the startup page's slot).
    update: UpdateSlot,
    audio: crate::voice::Audio,
}

/// Copy on release unless a direct terminal is known to forward its default copy shortcut
/// (oracle `local_settings.rs` `copy_on_select`): multiplexers, unknown terminals and Ghostty
/// without a recognized version default to copying.
fn copy_on_select(mode: CopyOnSelect, terminal: &TerminalInfo) -> bool {
    match mode {
        CopyOnSelect::Always => true,
        CopyOnSelect::Never => false,
        CopyOnSelect::Auto => {
            terminal.multiplexer.is_some()
                || match terminal.name {
                    // Since 1.2, both Ghostty's Cmd-C and Ctrl-Shift-C bindings are
                    // "performable": they forward the key if Ghostty has no selection.
                    TerminalName::Ghostty => !terminal
                        .version
                        .as_deref()
                        .and_then(version_at_least_1_2)
                        .unwrap_or(false),
                    // Kitty's Cmd-C uses copy_or_noop. Its Ctrl-Shift-C consumes the key.
                    TerminalName::Kitty => !cfg!(target_os = "macos"),
                    // Windows Terminal's Copy action forwards its key when unselected,
                    // including when the CLI runs in WSL.
                    TerminalName::WindowsTerminal => false,
                    // VS Code gates Copy on native selection. Only Windows' default (plain
                    // Ctrl-C) is also forwarded by the legacy xterm.js encoder.
                    TerminalName::VsCode => !cfg!(target_os = "windows"),
                    TerminalName::AppleTerminal
                    | TerminalName::Iterm2
                    | TerminalName::WarpTerminal
                    | TerminalName::WezTerm
                    | TerminalName::Rex
                    | TerminalName::Alacritty
                    | TerminalName::Konsole
                    | TerminalName::GnomeTerminal
                    | TerminalName::Vte
                    | TerminalName::Dumb
                    | TerminalName::Unknown => true,
                }
        }
    }
}

/// Whether `version` (semantic: `major.minor.patch`, a pre-release or build suffix allowed) is
/// 1.2.0 or later; none when it is not one.
fn version_at_least_1_2(version: &str) -> Option<bool> {
    let core = version.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    parts.next()?.parse::<u64>().ok()?;
    parts.next().is_none().then_some((major, minor) >= (1, 2))
}

/// `[tui]` as the views read it (oracle `local_settings.tui`).
pub(crate) fn tui_settings(config: &Config) -> TuiSettings {
    let rendering = config.tui_rendering;
    TuiSettings {
        copy_on_select: copy_on_select(config.tui_copy_on_select, &terminal_info()),
        primary_selection: environment().primary_available(
            |name| std::env::var_os(name).map(|value| value.to_string_lossy().into_owned()),
            cfg!(target_os = "linux"),
            std::env::var("TERM_PROGRAM")
                .is_ok_and(|program| program.eq_ignore_ascii_case("vscode")),
        ),
        deselect_after_copy: config.tui_copy_on_select_clear_selection,
        collapse_turn_details: config.tui_collapse_turn_details,
        wheel_rows_per_event: config.tui_mouse_scroll_speed,
        show_tooltips: config.show_tooltips,
        vim_mode_default: config.tui_vim_mode_default,
        hide_rate_limit_model_nudge: config.notices.hide_rate_limit_model_nudge.unwrap_or(false),
        transcript_owned: config.tui_fullscreen_transcript
            && config.tui_alternate_screen != codex_config::types::AltScreenMode::Never,
        rendering: Rendering {
            mermaid: rendering.mermaid,
            math: rendering.math,
            tables: rendering.tables,
            lists: rendering.lists,
        },
    }
}

/// `tui.presets` of the merged config, each with the layers that define it (`save_quick_dial`:
/// the user config's own file decides what the user may replace or clear).
pub(crate) fn presets(config: &Config) -> Vec<Slot> {
    use codex_config::ConfigLayerSource;
    let user_file = config
        .config_layer_stack
        .get_user_config_file()
        .cloned()
        .unwrap_or_else(|| config.codex_home.join("config.toml"));
    config
        .tui_quick_dials
        .iter()
        .filter_map(|(key, dial)| {
            let slot: u8 = key.parse().ok().filter(|slot| SLOTS.contains(slot))?;
            let layers: Vec<bool> = config
                .config_layer_stack
                .layers_high_to_low()
                .filter(|layer| {
                    layer
                        .config
                        .get("tui")
                        .and_then(|tui| tui.get("presets"))
                        .and_then(|dials| dials.get(key))
                        .is_some()
                })
                .map(|layer| {
                    matches!(&layer.name, ConfigLayerSource::User { file, .. } if *file == user_file)
                })
                .collect();
            let origin = match layers.first() {
                None => Origin::User,
                Some(true) if layers.iter().all(|user| *user) => Origin::User,
                Some(true) => Origin::Shadowing,
                Some(false) => Origin::Outside,
            };
            Some(Slot {
                slot,
                dial: Dial {
                    input: dial.input.clone(),
                    label: dial.label.clone(),
                },
                origin,
            })
        })
        .collect()
}

impl Effects {
    /// The services of `config`: the system clipboard, the history file and mention usage of its
    /// Codex home, the system browser, and the in-process file search over its working root,
    /// whose matches go to `matches`; `update` is where a chosen daemon update is kept.
    pub(crate) fn new(
        config: &Config,
        browser: Browser,
        update: UpdateSlot,
        matches: Arc<dyn Fn(popovers::Ev) + Send + Sync>,
    ) -> Self {
        let persist =
            config.history.persistence == codex_config::types::HistoryPersistence::SaveAll;
        let services = Services::new(
            Clipboard::new(Machine::default(), environment()),
            Arc::new(MessageHistory::new(
                config.codex_home.to_path_buf(),
                &config.history,
            )),
            MentionFile::new(config.codex_home.as_path(), persist),
            browser,
        )
        .with_search(InProcessSearch::new(
            PathBuf::from(config.cwd.as_path()),
            matches,
        ));
        Self {
            services,
            terminal: Arc::new(write_osc52),
            config: Some(Arc::new(config.clone())),
            update,
            audio: crate::voice::Audio::default(),
        }
    }

    /// With the terminal's writer of a test.
    #[cfg(test)]
    fn with(services: Services, terminal: Terminal) -> Self {
        Self {
            services,
            terminal,
            config: None,
            update: UpdateSlot::default(),
            audio: crate::voice::Audio::default(),
        }
    }

    /// The history file and the mention usage window, read when the first thread is there.
    pub(crate) async fn opened(&self) -> [AppEv; 3] {
        [
            self.services.history_metadata().await,
            self.services.mention_usage().await,
            AppEv::Screen(ScreenEv::Chat(
                codex_tui_core::screens::chat::Ev::VoiceAvailable(
                    codex_realtime_webrtc::RealtimeWebrtcSession::is_supported(),
                ),
            )),
        ]
    }

    /// The work `command` asks of this machine, as the future that answers it; the command back
    /// when it is not this machine's (the pump hands it on). `exec` runs the workspace's
    /// commands (the diff); `link` is the connection's door to its app server, for the
    /// capabilities that go through it (the config file, the worktree owners).
    ///
    /// An `Ask` is answered by the first executor of `HostDoors` that has its capability: the
    /// library's services, then this machine's own (`Own`).
    pub(crate) fn run(
        self: &Arc<Self>,
        request: u64,
        ask: Ask,
        exec: impl WorkspaceExec + 'static,
        link: Option<Link>,
        connection: impl HostDoors<Out = Outcome> + Send + 'static,
    ) -> Boxed<Option<AppEv>> {
        let this = Arc::clone(self);
        Box::pin(async move {
            let doors = Chain(
                codex_tui_rtwk::services::effects::Doors {
                    services: &this.services,
                    exec: &exec,
                },
                Chain(
                    Own {
                        effects: &this,
                        link: link.as_ref(),
                    },
                    connection,
                ),
            );
            let ran = run_ask(&doors, ask).await?.finish(request)?;
            if let Some(text) = ran.terminal {
                (this.terminal)(&text);
            }
            Some(ran.ev)
        })
    }
}

/// What a connection lends the executors that talk to its app server: the startup (the config in
/// force), the request ids and handle, and the way to tell the app (a config write's facts reach
/// the views before its answer does).
pub(crate) struct Link {
    pub startup: crate::startup::Startup,
    pub ids: Ids,
    pub handle: AppServerRequestHandle,
    pub ingress: IngressTx,
    pub epoch: ConnEpoch,
    pub codex_home: PathBuf,
    pub workspace_local: bool,
}

/// The executors that are this host's own: the git lists, the background server, the editor, the
/// config file and the worktrees. Five capabilities listed before they were built (suspend,
/// notifications, the screen mode, the terminal probe, the session lock) have no
/// executor yet: each is one method here, in the file of the package that builds it.
struct Own<'a> {
    effects: &'a Effects,
    link: Option<&'a Link>,
}

impl Own<'_> {
    async fn read_ide(&self, cwd: String) -> Result<Option<String>, IdeFailure> {
        match self.link {
            Some(link)
                if link.workspace_local
                    && link.startup.target != crate::startup::Target::Remote =>
            {
                crate::small::read_ide_context(PathBuf::from(cwd), link.codex_home.clone()).await
            }
            _ => Err(crate::small::ide_unavailable()),
        }
    }
}

impl HostDoors for Own<'_> {
    type Out = Outcome;

    async fn audio(
        &self,
        command: codex_tui_core::screens::voice::AudioCommand,
    ) -> Option<Self::Out> {
        let config = self.effects.config.as_deref();
        Some(Outcome::Effect(
            Done::Audio(self.effects.audio.run(command, config).await),
            None,
        ))
    }

    async fn pet(&self, ask: codex_tui_core::screens::pets::AssetAsk) -> Option<Self::Out> {
        Some(Outcome::Effect(
            Done::Pet(pets::run(ask, self.effects.config.clone()).await),
            None,
        ))
    }

    async fn ide_context(&self, _thread: ThreadId, cwd: String) -> Option<Self::Out> {
        let read = self.read_ide(cwd).await.map(|context| IdeReading {
            has_context: context.is_some(),
        });
        Some(Outcome::Effect(Done::IdeContext(read), None))
    }

    async fn ide_prompt(&self, _thread: ThreadId, cwd: String) -> Option<Self::Out> {
        let read = self
            .read_ide(cwd)
            .await
            .map(|context| context.map(IdePrompt::new));
        Some(Outcome::Effect(Done::IdePrompt(read), None))
    }

    async fn git_branches(&self, cwd: Option<String>) -> Option<Self::Out> {
        let cwd = at(cwd);
        Some(Outcome::Effect(
            Done::GitBranches {
                branches: codex_git_utils::local_git_branches(&cwd).await,
                current: codex_git_utils::current_branch_name(&cwd).await,
            },
            None,
        ))
    }

    async fn git_log(&self, cwd: Option<String>, limit: usize) -> Option<Self::Out> {
        let commits = codex_git_utils::recent_commits(&at(cwd), limit)
            .await
            .into_iter()
            .map(|entry| Commit {
                sha: entry.sha,
                subject: entry.subject,
            })
            .collect();
        Some(Outcome::Effect(Done::GitLog(commits), None))
    }

    async fn background_server(&self, ask: DaemonAsk) -> Option<Self::Out> {
        Some(Outcome::Effect(
            crate::daemon::run(ask, &self.effects.update).await,
            None,
        ))
    }

    // The external editor is the host's own: it reads the config and runs a program.
    async fn editor(&self, draft: String, cwd: String) -> Option<Self::Out> {
        let done = match &self.effects.config {
            Some(config) => {
                let policy = config.permissions.file_system_sandbox_policy();
                codex_tui_rtwk::services::editor::ask(
                    config.codex_home.as_path(),
                    &policy,
                    config.cwd.as_path(),
                    draft,
                    cwd,
                )
                .await
            }
            None => Err(EditorFailure::Failed(
                "no config to read the sandbox policy from".to_owned(),
            )),
        };
        Some(Outcome::Effect(Done::Editor(done), None))
    }

    // A write of the config file is the app server's, and the facts the dialogs read follow it.
    async fn config_write(&self, edits: Vec<ConfigEdit>) -> Option<Self::Out> {
        let link = self.link?;
        Some(Outcome::Effect(
            crate::in_process::config_write(edits, link).await,
            None,
        ))
    }

    async fn worktrees(&self, cwd: String) -> Option<Self::Out> {
        let link = self.link?;
        let done =
            crate::workspace::list(link.codex_home.clone(), cwd, &link.ids, &link.handle).await;
        Some(Outcome::Effect(done, None))
    }

    async fn worktree_remove(&self, cwd: String, root: String) -> Option<Self::Out> {
        let link = self.link?;
        let removed = crate::workspace::remove_worktree(link.codex_home.clone(), cwd, root).await;
        Some(Outcome::Effect(Done::WorktreeRemoved(removed), None))
    }
}

/// The directory an ask names (`None`: the process's own).
fn at(cwd: Option<String>) -> PathBuf {
    cwd.map_or_else(|| PathBuf::from("."), PathBuf::from)
}

/// The popovers' event of the file search, as the app takes it.
pub(crate) fn popover_ev(ev: popovers::Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Popovers(ev))
}

/// Puts `text` on the terminal's clipboard (OSC 52, oracle `osc52_sequence`): one write of the
/// whole sequence, behind tmux's passthrough inside tmux. rtwk has no clipboard write for an app
/// task (gap in the pack), so the request goes to the terminal as it is.
fn write_osc52(text: &str) {
    use std::io::Write as _;
    let osc = format!("\x1b]52;c;{}\x07", base64(text.as_bytes()));
    let sequence = if std::env::var_os("TMUX").is_some() {
        format!("\x1bPtmux;\x1b{osc}\x1b\\")
    } else {
        osc
    };
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(sequence.as_bytes());
    let _ = out.flush();
}

/// Standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, byte)| n | u32::from(*byte) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(TABLE[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn copy_on_select_follows_the_terminal() {
        use codex_terminal_detection::Multiplexer;
        let info = |name, version: Option<&str>, multiplexer| TerminalInfo {
            name,
            term_program: None,
            version: version.map(str::to_owned),
            term: None,
            multiplexer,
            backend: None,
        };
        let auto = |terminal: &TerminalInfo| super::copy_on_select(CopyOnSelect::Auto, terminal);
        // the settings decide outright
        assert!(super::copy_on_select(
            CopyOnSelect::Always,
            &info(TerminalName::WindowsTerminal, None, None)
        ));
        assert!(!super::copy_on_select(
            CopyOnSelect::Never,
            &info(TerminalName::Unknown, None, None)
        ));
        // a direct terminal that forwards its own copy shortcut leaves the copy to it
        assert!(!auto(&info(TerminalName::WindowsTerminal, None, None)));
        assert!(!auto(&info(TerminalName::Ghostty, Some("1.2.0"), None)));
        assert!(!auto(&info(
            TerminalName::Ghostty,
            Some("1.10.3-dev+abc"),
            None
        )));
        assert!(auto(&info(TerminalName::Ghostty, Some("1.1.9"), None)));
        assert!(auto(&info(TerminalName::Ghostty, None, None)));
        assert!(auto(&info(TerminalName::Ghostty, Some("tip"), None)));
        // a multiplexer in front of it, or a terminal unknown, copies
        assert!(auto(&info(
            TerminalName::WindowsTerminal,
            None,
            Some(Multiplexer::Tmux { version: None })
        )));
        assert!(auto(&info(TerminalName::Unknown, None, None)));
        assert!(auto(&info(TerminalName::Iterm2, None, None)));
    }

    use std::path::Path;
    use std::sync::Mutex;
    use std::time::Instant;

    use codex_tui_core::app::AppFx;
    use codex_tui_core::app::CodexApp;
    use codex_tui_core::app::Command;
    use codex_tui_core::app::Intent;
    use codex_tui_core::app::apply_app;
    use codex_tui_core::app::effects::Ask;
    use codex_tui_core::app::effects::CopyDestination;
    use codex_tui_core::app::effects::CopyFormat;
    use codex_tui_core::app::effects::CopyStatus;
    use codex_tui_core::app::effects::Done;
    use codex_tui_core::data::history::Lookup;
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::screens::transcript::diff;
    use codex_tui_rtwk::services::clipboard::Environment;
    use codex_tui_rtwk::services::clipboard::Lease;
    use codex_tui_rtwk::services::clipboard::Outcome;
    use codex_tui_rtwk::services::clipboard::Routes;
    use codex_tui_rtwk::services::history::Batch;
    use codex_tui_rtwk::services::history::HistoryFile;

    use super::*;

    type Fut<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

    /// A clipboard whose native route writes into `written`, or refuses.
    struct Native {
        written: Arc<Mutex<Vec<String>>>,
        refuse: bool,
    }

    impl Routes for Native {
        fn native(&mut self, text: &str, _html: Option<&str>) -> Result<Option<Lease>, String> {
            if self.refuse {
                return Err("clipboard unavailable".to_owned());
            }
            self.written.lock().unwrap().push(text.to_owned());
            Ok(Some(Lease::new(())))
        }
        fn windows(&mut self, _text: &str, _deadline: Instant) -> Result<(), String> {
            Err("not WSL".to_owned())
        }
        fn tmux(&mut self, _text: &str) -> Result<(), String> {
            Err("not tmux".to_owned())
        }
        fn primary(
            &mut self,
            _text: &str,
            _begin_delivery: &dyn Fn() -> Result<(), String>,
        ) -> Result<Outcome, String> {
            Err("no PRIMARY".to_owned())
        }
    }

    /// A history file in memory: 3 entries under log 7.
    #[derive(Default)]
    struct File {
        appended: Mutex<Vec<(String, String)>>,
    }

    impl HistoryFile for File {
        fn metadata(&self) -> Fut<'_, (u64, usize)> {
            Box::pin(async { (7, 3) })
        }
        fn append<'a>(
            &'a self,
            conversation: &'a str,
            text: &'a str,
        ) -> Fut<'a, Result<(), String>> {
            Box::pin(async move {
                self.appended
                    .lock()
                    .unwrap()
                    .push((conversation.to_owned(), text.to_owned()));
                Ok(())
            })
        }
        fn lookup(&self, _log_id: u64, offset: usize) -> Option<String> {
            (offset == 0).then(|| "first".to_owned())
        }
        fn lookup_batch(&self, _log_id: u64, _end_offset: usize) -> Result<Batch, String> {
            Err("unreadable".to_owned())
        }
    }

    struct NoExec;

    impl WorkspaceExec for NoExec {
        fn run(&self, _call: diff::Call, _cwd: &Path) -> Fut<'_, Result<diff::Output, String>> {
            Box::pin(async { Err("git is not here".to_owned()) })
        }
    }

    struct Rig {
        effects: Arc<Effects>,
        written: Arc<Mutex<Vec<String>>>,
        terminal: Arc<Mutex<Vec<String>>>,
        file: Arc<File>,
        opened: Arc<Mutex<Vec<String>>>,
        _home: tempfile::TempDir,
    }

    fn rig(refuse_native: bool, browser_fails: bool) -> Rig {
        let written = Arc::new(Mutex::new(Vec::new()));
        let terminal = Arc::new(Mutex::new(Vec::new()));
        let opened = Arc::new(Mutex::new(Vec::new()));
        let file = Arc::new(File::default());
        let home = tempfile::tempdir().unwrap();
        let log = Arc::clone(&opened);
        let browser: Browser = Arc::new(move |url: &str| {
            log.lock().unwrap().push(url.to_owned());
            if browser_fails {
                Err(std::io::Error::other("no display"))
            } else {
                Ok(())
            }
        });
        let services = Services::new(
            Clipboard::new(
                Native {
                    written: Arc::clone(&written),
                    refuse: refuse_native,
                },
                Environment::default(),
            ),
            file.clone(),
            MentionFile::new(home.path(), true),
            browser,
        );
        let sink = Arc::clone(&terminal);
        let effects = Arc::new(Effects::with(
            services,
            Arc::new(move |text| sink.lock().unwrap().push(text.to_owned())),
        ));
        Rig {
            effects,
            written,
            terminal,
            file,
            opened,
            _home: home,
        }
    }

    struct NoConnection;

    impl HostDoors for NoConnection {
        type Out = codex_tui_rtwk::services::effects::Outcome;
    }

    async fn answer(rig: &Rig, command: Command) -> Option<AppEv> {
        let Command::Effect { request, ask, .. } = command else {
            panic!("a host effect");
        };
        rig.effects
            .run(request, ask, NoExec, None, NoConnection)
            .await
    }

    /// The one command the app makes of `intent` (commands are core's to make: no host code
    /// builds one).
    fn command_of(intent: Intent) -> Command {
        let mut app = CodexApp::new();
        let mut fx = AppFx::default();
        apply_app(&mut app, AppEv::Intent(intent), &mut fx);
        match fx.commands() {
            [command] => command.clone(),
            other => panic!("not one command: {other:?}"),
        }
    }

    fn effect(ask: Ask) -> Command {
        command_of(Intent::Effect { request: 1, ask })
    }

    fn popover(intent: popovers::Intent) -> Command {
        command_of(Intent::Screen(ScreenIntent::Popovers(intent)))
    }

    fn done(ev: Option<AppEv>) -> Done {
        match ev {
            Some(AppEv::Effect { request: 1, done }) => done,
            other => panic!("not an effect answer: {other:?}"),
        }
    }

    #[tokio::test]
    async fn ide_context_receipt_fails_when_no_local_workspace_is_lent() {
        let rig = rig(false, false);
        let ask = Ask::IdeContext {
            thread: ThreadId::new("thread-1"),
            cwd: "/work".to_owned(),
        };
        let reply = done(answer(&rig, effect(ask)).await);
        assert_eq!(
            reply,
            Done::IdeContext(Err(crate::small::ide_unavailable()))
        );
        let ask = Ask::IdePrompt {
            thread: ThreadId::new("thread-1"),
            cwd: "/work".to_owned(),
        };
        let reply = done(answer(&rig, effect(ask)).await);
        assert_eq!(reply, Done::IdePrompt(Err(crate::small::ide_unavailable())));
        assert!(rig.written.lock().unwrap().is_empty());
        assert!(rig.terminal.lock().unwrap().is_empty());
        assert!(rig.opened.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn copy_reaches_the_native_clipboard_and_answers_its_status() {
        let rig = rig(false, false);
        let ev = answer(
            &rig,
            effect(Ask::Copy {
                text: "hello".into(),
                format: CopyFormat::Markdown,
                destination: CopyDestination::Clipboard,
            }),
        )
        .await;
        assert_eq!(done(ev), Done::Copy(Ok(CopyStatus::Confirmed)));
        assert_eq!(*rig.written.lock().unwrap(), vec!["hello".to_owned()]);
        assert!(rig.terminal.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_refused_native_copy_is_handed_to_the_terminal() {
        let rig = rig(true, false);
        let ev = answer(
            &rig,
            effect(Ask::Copy {
                text: "hello".into(),
                format: CopyFormat::PlainText,
                destination: CopyDestination::Clipboard,
            }),
        )
        .await;
        assert_eq!(done(ev), Done::Copy(Ok(CopyStatus::Unconfirmed)));
        assert_eq!(*rig.terminal.lock().unwrap(), vec!["hello".to_owned()]);
    }

    #[tokio::test]
    async fn git_diff_runs_through_the_workspace_executor() {
        let rig = rig(false, false);
        let ev = answer(
            &rig,
            effect(Ask::GitDiff {
                cwd: Some("/work".into()),
            }),
        )
        .await;
        assert!(matches!(done(ev), Done::GitDiff(Err(_))));
    }

    #[tokio::test]
    async fn export_writes_the_file_once() {
        let rig = rig(false, false);
        let dir = tempfile::tempdir().unwrap();
        let ask = || {
            effect(Ask::Export {
                thread: ThreadId::new("thread-1"),
                path: "talk.md".into(),
                cwd: dir.path().display().to_string(),
                markdown: "# Codex conversation\nhi\n".into(),
            })
        };
        let Done::Export(Ok(path)) = done(answer(&rig, ask()).await) else {
            panic!("not written");
        };
        // The file is the export's markdown, whole (the disk is read through its size here:
        // file reads are the services' alone, R12).
        let written = std::fs::metadata(path).unwrap().len();
        assert_eq!(written, "# Codex conversation\nhi\n".len() as u64);
        assert!(matches!(
            done(answer(&rig, ask()).await),
            Done::Export(Err(_))
        ));
    }

    #[tokio::test]
    async fn the_asks_of_the_app_server_answer_nothing_here() {
        let rig = rig(false, false);
        let ask = Ask::ConfigWrite { edits: Vec::new() };
        assert!(answer(&rig, effect(ask)).await.is_none());
    }

    #[tokio::test]
    async fn popovers_read_and_append_the_history_file_and_record_mentions() {
        let rig = rig(false, false);
        let entry = answer(
            &rig,
            popover(popovers::Intent::Lookup(Lookup::Entry {
                log_id: 7,
                offset: 0,
            })),
        )
        .await;
        assert!(matches!(
            entry,
            Some(AppEv::Screen(ScreenEv::Popovers(
                popovers::Ev::HistoryEntry {
                    offset: 0,
                    text: Some(_),
                    ..
                }
            )))
        ));
        let appended = answer(&rig, popover(popovers::Intent::Record("hello".into()))).await;
        assert!(appended.is_none());
        assert_eq!(
            rig.file
                .appended
                .lock()
                .unwrap()
                .iter()
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>(),
            vec!["hello"]
        );
        let recorded = answer(
            &rig,
            popover(popovers::Intent::RecordMention("app://calendar".into())),
        )
        .await;
        assert!(recorded.is_none());
        let [metadata, usage, voice] = rig.effects.opened().await;
        assert!(matches!(
            voice,
            AppEv::Screen(ScreenEv::Chat(codex_tui_core::screens::chat::Ev::VoiceAvailable(available)))
                if available == codex_realtime_webrtc::RealtimeWebrtcSession::is_supported()
        ));
        assert!(matches!(
            metadata,
            AppEv::Screen(ScreenEv::Popovers(popovers::Ev::HistoryMetadata {
                log_id: 7,
                entry_count: 3
            }))
        ));
        assert!(matches!(
            usage,
            AppEv::Screen(ScreenEv::Popovers(popovers::Ev::MentionUsage(window)))
                if window == vec!["app://calendar".to_owned()]
        ));
    }

    // A file search that has nothing to search answers nothing now: its matches come through
    // the sink the host gave, not through the command.
    #[tokio::test]
    async fn a_file_search_answers_through_the_sink_not_the_command() {
        let rig = rig(false, false);
        let ev = answer(
            &rig,
            popover(popovers::Intent::SearchFiles(
                codex_tui_core::data::files::FileSearchAsk::Stop,
            )),
        )
        .await;
        assert!(ev.is_none());
    }

    #[tokio::test]
    async fn open_url_says_nothing_when_it_opens_and_why_when_it_fails() {
        use codex_tui_core::app::effects::BrowserReport;

        let url = "https://x.test";
        let ask = || {
            Command::host(Ask::OpenUrl {
                url: url.into(),
                report: BrowserReport::ErrorLine,
            })
        };
        let opened = rig(false, false);
        assert!(answer(&opened, ask()).await.is_none());
        assert_eq!(*opened.opened.lock().unwrap(), vec![url.to_owned()]);

        let failed = self::rig(false, true);
        assert!(matches!(
            answer(&failed, ask()).await,
            Some(AppEv::ErrorLine { message })
                if message == "Failed to open browser for https://x.test: no display"
        ));
        assert!(matches!(
            answer(&failed, command_of(Intent::Effect { request: 13, ask: Ask::open_url(url.into()) })).await,
            Some(AppEv::Effect { request: 13, done: Done::Opened(Err(error)) })
                if error == "no display"
        ));
    }

    #[test]
    fn base64_pads_like_the_standard_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64("héllo".as_bytes()), "aMOpbGxv");
    }

    #[tokio::test]
    async fn transcript_wheel_speed_preserves_the_optional_config_value() {
        let dir = tempfile::tempdir().expect("config directory");
        for speed in [None, Some(1.5)] {
            let overrides = speed
                .map(|speed| ("tui.mouse_scroll_speed".into(), speed.into()))
                .into_iter()
                .collect();
            let config = codex_core::config::ConfigBuilder::default()
                .codex_home(dir.path().to_owned())
                .fallback_cwd(Some(dir.path().to_owned()))
                .cli_overrides(overrides)
                .build()
                .await
                .expect("load wheel speed config");
            assert_eq!(tui_settings(&config).wheel_rows_per_event, speed);
        }
    }

    #[test]
    fn the_defaults_of_the_config_are_the_apps() {
        let settings = TuiSettings::default();
        assert!(
            settings.show_tooltips && settings.rendering.tables && !settings.collapse_turn_details
        );
    }
}
