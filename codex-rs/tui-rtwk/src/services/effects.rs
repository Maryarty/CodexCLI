//! The runner of the app's asks that are this machine's own: the clipboard (write and read), the
//! git diff, the export file, the Desktop link, the composer's history file, mention usage and
//! file search, and the browser. In Flutter this is the repository layer behind the app's
//! `Command`s: the host gives each `Command` to [`Services::run`] and applies the answer.
//!
//! The seams are the services' own: `Routes` (clipboard), `HistoryFile`, `WorkspaceExec`,
//! `Browser`. The asks that are the app-server's (the title, the config file) are not here: the
//! host's `send` carries them.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::effects::CopyDestination;
use codex_tui_core::app::effects::CopyFormat;
use codex_tui_core::app::effects::CopyStatus;
use codex_tui_core::app::effects::Done;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::popovers;

use super::clipboard::Clipboard;
use super::clipboard::Destination;
use super::clipboard::Format;
use super::clipboard::Status;
use super::clipboard::environment;
use super::clipboard::read::TextReader;
use super::clipboard::read::paste_image;
use super::clipboard::render_markdown;
use super::file_search::InProcessSearch;
use super::history;
use super::history::HistoryFile;
use super::history::usage::MentionFile;
use super::open_url;
use super::shell::WorkspaceExec;
use super::shell::git_diff;
use super::startup::Browser;
use super::transcript_file;

/// How long the system clipboard's text may take to arrive.
const READ_WITHIN: Duration = Duration::from_secs(2);

/// What a command answered: the event for the app, and the OSC 52 text the owner of the terminal
/// is to write when a copy asked for the terminal's clipboard (rtwk has no `AppCx` clipboard
/// write: gap, see the pack).
#[derive(Debug)]
pub struct Ran {
    pub ev: AppEv,
    pub terminal: Option<String>,
}

impl Ran {
    fn of(ev: AppEv) -> Self {
        Self { ev, terminal: None }
    }
}

/// The machine's services, built once per app.
pub struct Services {
    clipboard: Clipboard,
    history: Arc<dyn HistoryFile>,
    mentions: MentionFile,
    browser: Browser,
    search: Option<InProcessSearch>,
}

impl Services {
    pub fn new(
        clipboard: Clipboard,
        history: Arc<dyn HistoryFile>,
        mentions: MentionFile,
        browser: Browser,
    ) -> Self {
        Self {
            clipboard,
            history,
            mentions,
            browser,
            search: None,
        }
    }

    /// File search in this process (its matches reach the app through the search's own sink);
    /// without it the host asks the app-server's `fuzzyFileSearch`.
    pub fn with_search(mut self, search: InProcessSearch) -> Self {
        self.search = Some(search);
        self
    }

    /// What the history file holds, as the composer asks for it when a thread opens (oracle
    /// `history_metadata`); the host sends it once the thread is there.
    pub async fn history_metadata(&self) -> AppEv {
        popover_ev(history::metadata(self.history.as_ref()).await)
    }

    /// The mention usage window, loaded at start.
    pub async fn mention_usage(&self) -> AppEv {
        popover_ev(self.mentions.load().await)
    }

    /// Runs `command` if this machine answers it; `None` for every other command (and for the
    /// asks that have no answer: an append, a file search, a recorded mention). `exec` runs the
    /// workspace's commands (the diff).
    pub async fn run(&self, command: Command, exec: &dyn WorkspaceExec) -> Option<Ran> {
        match command {
            Command::Effect { request, ask, .. } => self.effect(request, ask, exec).await,
            Command::Popovers { ask, thread, .. } => {
                let conversation = thread.as_ref().map_or("", |thread| thread.as_str());
                self.popovers(ask, conversation).await.map(Ran::of)
            }
            Command::OpenUrl { url, .. } => self.open_url(url).await.map(Ran::of),
            _ => None,
        }
    }

    /// One ask of `Intent::Effect`, answered under `request`; `None` for the asks of the
    /// app-server (the title, the config).
    pub async fn effect(&self, request: u64, ask: Ask, exec: &dyn WorkspaceExec) -> Option<Ran> {
        let (done, terminal) = self.done(ask, exec).await?;
        Some(Ran {
            ev: AppEv::Effect { request, done },
            terminal,
        })
    }

    /// The browser opens `url`; a failure is the app's to show.
    pub async fn open_url(&self, url: String) -> Option<AppEv> {
        open_url::open_url(Arc::clone(&self.browser), url)
            .await
            .err()
            .map(|failure| AppEv::ErrorLine {
                message: format!(
                    "Failed to open browser for {}: {}",
                    failure.url, failure.error
                ),
            })
    }

    async fn done(&self, ask: Ask, exec: &dyn WorkspaceExec) -> Option<(Done, Option<String>)> {
        Some(match ask {
            Ask::Copy {
                text,
                format,
                destination,
            } => {
                let format = format_of(&text, &format);
                let done = self
                    .clipboard
                    .copy(Arc::from(text), format, destination_of(destination))
                    .await;
                let status = done.result.map(status_of);
                return Some((Done::Copy(status), done.terminal));
            }
            Ask::GitDiff { cwd } => {
                let cwd = cwd.map_or_else(|| PathBuf::from("."), PathBuf::from);
                (Done::GitDiff(git_diff(exec, &cwd).await), None)
            }
            Ask::Export {
                path,
                cwd,
                markdown,
                ..
            } => {
                let written =
                    transcript_file::export(PathBuf::from(cwd), PathBuf::from(path), markdown)
                        .await
                        .map(|path| path.display().to_string());
                (Done::Export(written), None)
            }
            Ask::ClipboardText => {
                let text = TextReader::new(&environment())
                    .read_within(READ_WITHIN)
                    .await;
                (Done::ClipboardText(text), None)
            }
            Ask::OpenDesktop { thread } => {
                let opened = open_url::open_desktop_thread(thread.to_string()).await;
                (Done::Desktop(opened), None)
            }
            // A title needs an ephemeral thread and a structured turn on the app-server
            // (`generate_thread_title`); until that is built the answer is the oracle's for a
            // server-owned session: no suggestion, and the prompt keeps the name it has.
            Ask::SuggestTitle { .. } => (Done::Title(None), None),
            // The git lists of `/review` are the host's (it carries `codex-git-utils`).
            Ask::ConfigWrite { .. }
            | Ask::GitBranches { .. }
            | Ask::GitLog { .. }
            | Ask::Worktrees { .. }
            | Ask::WorktreeRemove { .. } => return None,
        })
    }

    /// One ask of the composer's popovers (`conversation` is the thread the entry belongs to).
    pub async fn popovers(&self, ask: popovers::Ask, conversation: &str) -> Option<AppEv> {
        match ask {
            popovers::Ask::Lookup(lookup) => {
                Some(popover_ev(history::lookup(&self.history, lookup).await))
            }
            popovers::Ask::Append(text) => {
                // A failed append is not shown (oracle: logged).
                let _ = history::append(self.history.as_ref(), conversation, &text).await;
                None
            }
            popovers::Ask::RecordMention(identity) => {
                let _ = self.mentions.record(identity).await;
                None
            }
            popovers::Ask::ClipboardImage => Some(match paste_image().await {
                Ok((path, _)) => popover_ev(popovers::Ev::ImagePasted(path)),
                // Oracle: `Failed to paste image: {error}` as an error line.
                Err(error) => AppEv::ErrorLine {
                    message: format!("Failed to paste image: {error}"),
                },
            }),
            popovers::Ask::FileSearch(ask) => {
                if let Some(search) = &self.search {
                    search.ask(ask);
                }
                None
            }
        }
    }
}

fn popover_ev(ev: popovers::Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Popovers(ev))
}

/// A Markdown copy is offered with its HTML too, rendered once here (oracle `clipboard_html`):
/// of the text itself, or of the semantic Markdown a literal selection came from.
fn format_of(text: &str, format: &CopyFormat) -> Format {
    match format {
        CopyFormat::PlainText => Format::Plain,
        CopyFormat::Markdown => Format::Html(Arc::from(render_markdown(text))),
        CopyFormat::MarkdownSelection(source) => Format::Html(Arc::from(render_markdown(source))),
    }
}

fn destination_of(destination: CopyDestination) -> Destination {
    match destination {
        CopyDestination::Clipboard => Destination::Clipboard,
        CopyDestination::Primary => Destination::Primary,
        CopyDestination::ClipboardAndPrimary(plain) => Destination::ClipboardAndPrimary(plain),
    }
}

fn status_of(status: Status) -> CopyStatus {
    match status {
        Status::Confirmed => CopyStatus::Confirmed,
        Status::Unconfirmed => CopyStatus::Unconfirmed,
        Status::Busy => CopyStatus::Busy,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::Instant;

    use codex_tui_core::ids::ThreadId;
    use codex_tui_core::data::history::Lookup;
    use codex_tui_core::screens::transcript::diff;
    use pretty_assertions::assert_eq;

    use super::super::clipboard::Environment;
    use super::super::clipboard::Lease;
    use super::super::clipboard::Outcome;
    use super::super::clipboard::Routes;
    use super::super::history::Batch;
    use super::*;

    type Boxed<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

    /// A clipboard whose native route writes into `written` (with the HTML flavour it was
    /// offered in `html`), whose PRIMARY route writes into `primary`, or which refuses.
    struct Native {
        written: Arc<Mutex<Vec<String>>>,
        html: Arc<Mutex<Vec<Option<String>>>>,
        primary: Arc<Mutex<Vec<String>>>,
        refuse: bool,
    }

    impl Routes for Native {
        fn native(&mut self, text: &str, html: Option<&str>) -> Result<Option<Lease>, String> {
            if self.refuse {
                return Err("clipboard unavailable".to_owned());
            }
            self.written.lock().unwrap().push(text.to_owned());
            self.html.lock().unwrap().push(html.map(str::to_owned));
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
            text: &str,
            begin_delivery: &dyn Fn() -> Result<(), String>,
        ) -> Result<Outcome, String> {
            begin_delivery()?;
            self.primary.lock().unwrap().push(text.to_owned());
            Ok(Outcome::Copied(Some(Lease::new(()))))
        }
    }

    /// A history file in memory: 3 entries under log 7, only row 0 readable.
    #[derive(Default)]
    struct File {
        appended: Mutex<Vec<(String, String)>>,
    }

    impl HistoryFile for File {
        fn metadata(&self) -> Boxed<'_, (u64, usize)> {
            Box::pin(async { (7, 3) })
        }

        fn append<'a>(
            &'a self,
            conversation: &'a str,
            text: &'a str,
        ) -> Boxed<'a, Result<(), String>> {
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

    /// A workspace in which no command runs.
    struct NoExec;

    impl WorkspaceExec for NoExec {
        fn run(
            &self,
            _call: diff::Call,
            _cwd: &Path,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<diff::Output, String>> + Send + '_>,
        > {
            Box::pin(async { Err("git is not here".to_owned()) })
        }
    }

    struct Rig {
        services: Services,
        written: Arc<Mutex<Vec<String>>>,
        html: Arc<Mutex<Vec<Option<String>>>>,
        primary: Arc<Mutex<Vec<String>>>,
        file: Arc<File>,
        opened: Arc<Mutex<Vec<String>>>,
        _home: tempfile::TempDir,
    }

    fn rig(refuse_native: bool, browser_fails: bool) -> Rig {
        let written = Arc::new(Mutex::new(Vec::new()));
        let html = Arc::new(Mutex::new(Vec::new()));
        let primary = Arc::new(Mutex::new(Vec::new()));
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
        let clipboard = Clipboard::new(
            Native {
                written: Arc::clone(&written),
                html: Arc::clone(&html),
                primary: Arc::clone(&primary),
                refuse: refuse_native,
            },
            Environment::default(),
        );
        let services = Services::new(
            clipboard,
            file.clone(),
            MentionFile::new(home.path(), true),
            browser,
        );
        Rig {
            services,
            written,
            html,
            primary,
            file,
            opened,
            _home: home,
        }
    }

    async fn done(rig: &Rig, ask: Ask) -> Done {
        match rig.services.effect(1, ask, &NoExec).await.map(|ran| ran.ev) {
            Some(AppEv::Effect { request: 1, done }) => done,
            other => panic!("not an effect answer: {other:?}"),
        }
    }

    fn popover(ev: Option<AppEv>) -> popovers::Ev {
        match ev {
            Some(AppEv::Screen(ScreenEv::Popovers(ev))) => ev,
            other => panic!("not a popovers answer: {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_copy_reaches_the_native_clipboard_and_answers_its_status() {
        let rig = rig(false, false);
        let ask = Ask::Copy {
            text: "hello".into(),
            format: CopyFormat::Markdown,
            destination: CopyDestination::Clipboard,
        };
        assert_eq!(done(&rig, ask).await, Done::Copy(Ok(CopyStatus::Confirmed)));
        assert_eq!(*rig.written.lock().unwrap(), vec!["hello".to_owned()]);
    }

    // Oracle `clipboard_copy.rs` `local_copy_offers_html_only_for_markdown`: the HTML flavour is
    // offered for a Markdown copy (of the text) and a Markdown selection (of its semantic source),
    // never for plain text.
    #[tokio::test]
    async fn html_is_offered_only_for_markdown() {
        let rig = rig(false, false);
        for (text, format) in [
            ("**hello**", CopyFormat::Markdown),
            ("**hello**", CopyFormat::PlainText),
            (
                "hello /repo/my_notes!.txt",
                CopyFormat::MarkdownSelection(r"**hello** /repo/my\_notes\!.txt".into()),
            ),
        ] {
            let ask = Ask::Copy {
                text: text.into(),
                format,
                destination: CopyDestination::Clipboard,
            };
            assert_eq!(done(&rig, ask).await, Done::Copy(Ok(CopyStatus::Confirmed)));
        }
        assert_eq!(
            *rig.html.lock().unwrap(),
            vec![
                Some("<p><strong>hello</strong></p>\n".to_owned()),
                None,
                Some("<p><strong>hello</strong> /repo/my_notes!.txt</p>\n".to_owned()),
            ]
        );
    }

    // Oracle `selection_clipboard.rs`: a selection publishes PRIMARY alone, or with the clipboard
    // (PRIMARY then holds the plain text the asker named).
    #[tokio::test]
    async fn a_selection_publishes_primary_alone_or_with_the_clipboard() {
        let rig = rig(false, false);
        let ask = |destination| Ask::Copy {
            text: "**hi**".into(),
            format: CopyFormat::PlainText,
            destination,
        };
        // The oracle answers a PRIMARY-only publication as a request, nothing confirms it.
        assert_eq!(
            done(&rig, ask(CopyDestination::Primary)).await,
            Done::Copy(Ok(CopyStatus::Unconfirmed))
        );
        assert_eq!(*rig.primary.lock().unwrap(), vec!["**hi**".to_owned()]);
        assert!(rig.written.lock().unwrap().is_empty(), "PRIMARY only");
        assert_eq!(
            done(&rig, ask(CopyDestination::ClipboardAndPrimary("hi".into()))).await,
            Done::Copy(Ok(CopyStatus::Confirmed))
        );
        assert_eq!(
            *rig.primary.lock().unwrap(),
            vec!["**hi**".to_owned(), "hi".to_owned()]
        );
        assert_eq!(*rig.written.lock().unwrap(), vec!["**hi**".to_owned()]);
    }

    // Oracle `clipboard_copy.rs`: a native clipboard that fails falls back to the terminal's
    // (OSC 52), which nothing confirms; the text goes to the owner of the terminal.
    #[tokio::test]
    async fn a_refused_native_copy_falls_back_to_the_terminal() {
        let rig = rig(true, false);
        let ask = Ask::Copy {
            text: "hello".into(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
        };
        let ran = rig
            .services
            .effect(1, ask, &NoExec)
            .await
            .expect("an answer");
        assert!(matches!(
            ran.ev,
            AppEv::Effect {
                done: Done::Copy(Ok(CopyStatus::Unconfirmed)),
                ..
            }
        ));
        assert!(ran.terminal.is_some_and(|osc| osc.contains("hello")));
    }

    #[tokio::test]
    async fn nothing_to_copy_is_a_failed_copy_with_the_hosts_text() {
        let rig = rig(false, false);
        let ask = Ask::Copy {
            text: String::new(),
            format: CopyFormat::PlainText,
            destination: CopyDestination::Clipboard,
        };
        let Done::Copy(Err(text)) = done(&rig, ask).await else {
            panic!("not a failed copy");
        };
        assert!(!text.is_empty());
    }

    #[tokio::test]
    async fn the_diff_runs_through_the_workspace_executor() {
        let rig = rig(false, false);
        let ask = Ask::GitDiff {
            cwd: Some("/work".into()),
        };
        let answer = done(&rig, ask).await;
        assert!(
            matches!(answer, Done::GitDiff(Err(_))),
            "a workspace whose git cannot run is a failed diff: {answer:?}"
        );
    }

    #[tokio::test]
    async fn an_export_is_written_once_and_never_over_a_file() {
        let rig = rig(false, false);
        let dir = tempfile::tempdir().unwrap();
        let ask = || Ask::Export {
            thread: ThreadId::new("thread-1"),
            path: "talk.md".into(),
            cwd: dir.path().display().to_string(),
            markdown: "# Codex conversation\nhi\n".into(),
        };
        let Done::Export(Ok(path)) = done(&rig, ask()).await else {
            panic!("not written");
        };
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# Codex conversation\nhi\n"
        );
        let again = done(&rig, ask()).await;
        assert!(matches!(again, Done::Export(Err(_))), "{again:?}");
    }

    #[tokio::test]
    async fn the_asks_of_the_app_server_are_not_answered_here() {
        let rig = rig(false, false);
        let ask = Ask::ConfigWrite { edits: Vec::new() };
        assert!(rig.services.effect(1, ask, &NoExec).await.is_none());
    }

    #[tokio::test]
    async fn a_title_is_not_generated_yet_and_the_answer_says_so() {
        let rig = rig(false, false);
        let ask = Ask::SuggestTitle {
            thread: ThreadId::new("thread-1"),
        };
        assert!(matches!(done(&rig, ask).await, Done::Title(None)));
    }

    #[tokio::test]
    async fn the_history_file_answers_lookups_and_takes_appends_of_the_conversation() {
        let rig = rig(false, false);
        assert!(matches!(
            popover(Some(rig.services.history_metadata().await)),
            popovers::Ev::HistoryMetadata {
                log_id: 7,
                entry_count: 3
            }
        ));
        let entry = rig
            .services
            .popovers(
                popovers::Ask::Lookup(Lookup::Entry {
                    log_id: 7,
                    offset: 0,
                }),
                "",
            )
            .await;
        assert!(matches!(
            popover(entry),
            popovers::Ev::HistoryEntry {
                offset: 0,
                text: Some(_),
                ..
            }
        ));
        let batch = rig
            .services
            .popovers(
                popovers::Ask::Lookup(Lookup::Batch {
                    log_id: 7,
                    end_offset: 3,
                }),
                "",
            )
            .await;
        assert!(matches!(
            popover(batch),
            popovers::Ev::HistoryBatchFailed { .. }
        ));
        let append = rig
            .services
            .popovers(popovers::Ask::Append("hello".into()), "thread-1")
            .await;
        assert!(append.is_none());
        assert_eq!(
            *rig.file.appended.lock().unwrap(),
            vec![("thread-1".to_owned(), "hello".to_owned())]
        );
    }

    #[tokio::test]
    async fn a_recorded_mention_is_in_the_window_the_next_load_reads() {
        let rig = rig(false, false);
        let recorded = rig
            .services
            .popovers(popovers::Ask::RecordMention("app://calendar".into()), "")
            .await;
        assert!(recorded.is_none());
        let popovers::Ev::MentionUsage(window) = popover(Some(rig.services.mention_usage().await))
        else {
            panic!("not the usage window");
        };
        assert_eq!(window, vec!["app://calendar".to_owned()]);
    }

    #[tokio::test]
    async fn a_browser_that_opens_says_nothing_and_one_that_fails_says_why() {
        let rig = rig(false, false);
        assert!(
            rig.services
                .open_url("https://x.test".into())
                .await
                .is_none()
        );
        assert_eq!(
            *rig.opened.lock().unwrap(),
            vec!["https://x.test".to_owned()]
        );
        let rig = self::rig(false, true);
        let failed = rig.services.open_url("https://x.test".into()).await;
        assert!(matches!(
            failed,
            Some(AppEv::ErrorLine { ref message })
                if message == "Failed to open browser for https://x.test: no display"
        ));
    }
}
