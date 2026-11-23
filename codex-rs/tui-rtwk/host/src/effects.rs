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

use codex_core::config::Config;
use codex_tui_core::app::AppEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::effects::Done;
use codex_tui_core::app::facts::Rendering;
use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::popovers;
use codex_tui_core::screens::select::quick_dials::{Dial, Origin, SLOTS, Slot};
use codex_tui_core::screens::select::review::Commit;
use codex_tui_rtwk::services::agents::workspace::Boxed;
use codex_tui_rtwk::services::clipboard::Clipboard;
use codex_tui_rtwk::services::clipboard::Machine;
use codex_tui_rtwk::services::clipboard::environment;
use codex_tui_rtwk::services::effects::Services;
use codex_tui_rtwk::services::file_search::InProcessSearch;
use codex_tui_rtwk::services::history::file::MessageHistory;
use codex_tui_rtwk::services::history::usage::MentionFile;
use codex_tui_rtwk::services::shell::WorkspaceExec;

use crate::startup::Browser;

/// Where the terminal's clipboard request goes: the owner of the terminal writes it.
type Terminal = Arc<dyn Fn(&str) + Send + Sync>;

/// The machine's services of one connection, and the way to the terminal.
pub(crate) struct Effects {
    services: Services,
    terminal: Terminal,
}

/// `[tui]` as the views read it (oracle `local_settings.tui`).
pub(crate) fn tui_settings(config: &Config) -> TuiSettings {
    let rendering = config.tui_rendering;
    TuiSettings {
        collapse_turn_details: config.tui_collapse_turn_details,
        show_tooltips: config.show_tooltips,
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

/// What `/plugins` reads of the config: the features it depends on, the home directory the
/// personal marketplace is under, and the marketplaces the user configured (the ones a tab can
/// remove; the git ones it can upgrade).
pub(crate) fn plugin_config(config: &Config) -> codex_tui_core::screens::select::plugins::Config {
    use codex_features::Feature;
    use codex_tui_core::screens::select::plugins::{Config as Plugins, Configured};
    let stack = &config.config_layer_stack;
    let user = stack.effective_user_config();
    let git = |name: &str| {
        stack
            .get_active_user_layer()
            .and_then(|layer| layer.config.get("marketplaces"))
            .and_then(|marketplaces| marketplaces.get(name))
            .and_then(|marketplace| marketplace.get("source_type"))
            .and_then(|kind| kind.as_str())
            .is_some_and(|kind| kind == "git")
    };
    let configured = user
        .as_ref()
        .and_then(|user| user.get("marketplaces"))
        .and_then(|marketplaces| marketplaces.as_table())
        .map(|table| {
            table
                .keys()
                .map(|name| Configured {
                    git: git(name),
                    name: name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    Plugins {
        enabled: config.features.enabled(Feature::Plugins),
        remote: config.features.enabled(Feature::RemotePlugin),
        sharing: config.features.enabled(Feature::PluginSharing),
        home: std::env::var_os("HOME").map(|home| home.to_string_lossy().into_owned()),
        configured,
    }
}

impl Effects {
    /// The services of `config`: the system clipboard, the history file and mention usage of its
    /// Codex home, the system browser, and the in-process file search over its working root,
    /// whose matches go to `matches`.
    pub(crate) fn new(
        config: &Config,
        browser: Browser,
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
        }
    }

    /// With the terminal's writer of a test.
    #[cfg(test)]
    fn with(services: Services, terminal: Terminal) -> Self {
        Self { services, terminal }
    }

    /// The history file and the mention usage window, read when the first thread is there.
    pub(crate) async fn opened(&self) -> [AppEv; 2] {
        [
            self.services.history_metadata().await,
            self.services.mention_usage().await,
        ]
    }

    /// The work `command` asks of this machine, as the future that answers it; the command back
    /// when it is not this machine's (the pump hands it on). `exec` runs the workspace's
    /// commands (the diff).
    pub(crate) fn run(
        self: &Arc<Self>,
        command: &Command,
        exec: impl WorkspaceExec + 'static,
    ) -> Option<Boxed<Option<AppEv>>> {
        if let Command::Effect {
            request,
            ask: ask @ (Ask::GitBranches { .. } | Ask::GitLog { .. }),
            ..
        } = command
        {
            let (request, ask) = (*request, ask.clone());
            return Some(Box::pin(async move {
                Some(AppEv::Effect {
                    request,
                    done: git_list(ask).await?,
                })
            }));
        }
        if !matches!(
            command,
            Command::Effect { .. } | Command::Popovers { .. } | Command::OpenUrl { .. }
        ) {
            return None;
        }
        let this = Arc::clone(self);
        let command = command.clone();
        Some(Box::pin(async move {
            let ran = this.services.run(command, &exec).await?;
            if let Some(text) = ran.terminal {
                (this.terminal)(&text);
            }
            Some(ran.ev)
        }))
    }
}

/// The branches or the log of the `/review` pickers (oracle `local_git_branches`,
/// `current_branch_name`, `recent_commits`): an empty list where there is no repository.
async fn git_list(ask: Ask) -> Option<Done> {
    let at = |cwd: Option<String>| cwd.map_or_else(|| PathBuf::from("."), PathBuf::from);
    match ask {
        Ask::GitBranches { cwd } => {
            let cwd = at(cwd);
            Some(Done::GitBranches {
                branches: codex_git_utils::local_git_branches(&cwd).await,
                current: codex_git_utils::current_branch_name(&cwd).await,
            })
        }
        Ask::GitLog { cwd, limit } => Some(Done::GitLog(
            codex_git_utils::recent_commits(&at(cwd), limit)
                .await
                .into_iter()
                .map(|entry| Commit {
                    sha: entry.sha,
                    subject: entry.subject,
                })
                .collect(),
        )),
        _ => None,
    }
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
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::Instant;

    use codex_tui_core::app::AppFx;
    use codex_tui_core::app::CodexApp;
    use codex_tui_core::app::Intent;
    use codex_tui_core::app::apply_app;
    use codex_tui_core::app::effects::Ask;
    use codex_tui_core::app::effects::CopyDestination;
    use codex_tui_core::app::effects::CopyFormat;
    use codex_tui_core::app::effects::CopyStatus;
    use codex_tui_core::app::effects::Done;
    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::screens::composer::history::Lookup;
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

    async fn answer(rig: &Rig, command: Command) -> Option<AppEv> {
        rig.effects
            .run(&command, NoExec)
            .expect("this machine's command")
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
        let [metadata, usage] = rig.effects.opened().await;
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
                codex_tui_core::screens::composer::mentions::files::FileSearchAsk::Stop,
            )),
        )
        .await;
        assert!(ev.is_none());
    }

    // `Command::OpenUrl` is core's to make (a banner's link, `/app`); the arm hands the url to the
    // services, which the browser opens or reports.
    #[tokio::test]
    async fn open_url_says_nothing_when_it_opens_and_why_when_it_fails() {
        let rig = rig(false, false);
        assert!(
            rig.effects
                .services
                .open_url("https://x.test".into())
                .await
                .is_none()
        );
        assert_eq!(
            *rig.opened.lock().unwrap(),
            vec!["https://x.test".to_owned()]
        );
        let rig = self::rig(false, true);
        assert!(matches!(
            rig.effects.services.open_url("https://x.test".into()).await,
            Some(AppEv::ErrorLine { message })
                if message == "Failed to open browser for https://x.test: no display"
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

    #[test]
    fn the_defaults_of_the_config_are_the_apps() {
        let settings = TuiSettings::default();
        assert!(
            settings.show_tooltips && settings.rendering.tables && !settings.collapse_turn_details
        );
    }
}
