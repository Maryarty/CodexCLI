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
use codex_tui_core::app::effects::HostDoors;
use codex_tui_core::app::effects::run_ask;
use codex_tui_core::ids::ThreadId;
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
        let (done, terminal) = run_ask(
            &Doors {
                services: self,
                exec,
            },
            ask,
        )
        .await?;
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

/// The executors of the capabilities the services carry (the clipboard, the git diff, the export
/// file, the Desktop link and the browser): `Services` with the workspace executor the diff runs
/// through. The answer is the `Done` and the OSC 52 text a copy asked the terminal to write. The
/// host chains these with its own (`HostDoors`, `Chain`); every other capability is not here.
pub struct Doors<'a> {
    pub services: &'a Services,
    pub exec: &'a dyn WorkspaceExec,
}

impl HostDoors for Doors<'_> {
    type Out = (Done, Option<String>);

    async fn copy(
        &self,
        text: String,
        format: CopyFormat,
        destination: CopyDestination,
    ) -> Option<Self::Out> {
        let format = format_of(&text, &format);
        let done = self
            .services
            .clipboard
            .copy(Arc::from(text), format, destination_of(destination))
            .await;
        let status = done.result.map(status_of);
        Some((Done::Copy(status), done.terminal))
    }

    async fn git_diff(&self, cwd: Option<String>) -> Option<Self::Out> {
        let cwd = cwd.map_or_else(|| PathBuf::from("."), PathBuf::from);
        Some((Done::GitDiff(git_diff(self.exec, &cwd).await), None))
    }

    async fn export(
        &self,
        _thread: ThreadId,
        path: String,
        cwd: String,
        markdown: String,
    ) -> Option<Self::Out> {
        let written = transcript_file::export(PathBuf::from(cwd), PathBuf::from(path), markdown)
            .await
            .map(|path| path.display().to_string());
        Some((Done::Export(written), None))
    }

    async fn clipboard_text(&self) -> Option<Self::Out> {
        let text = TextReader::new(&environment())
            .read_within(READ_WITHIN)
            .await;
        Some((Done::ClipboardText(text), None))
    }

    async fn open_desktop(&self, thread: ThreadId) -> Option<Self::Out> {
        let opened = open_url::open_desktop_thread(thread.to_string()).await;
        Some((Done::Opened(opened), None))
    }

    async fn open_url(&self, url: String) -> Option<Self::Out> {
        let opened = open_url::open_url(Arc::clone(&self.services.browser), url)
            .await
            .map_err(|failure| failure.error);
        Some((Done::Opened(opened), None))
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
