use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use codex_tui_core::data::history::Lookup;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::transcript::diff;
use pretty_assertions::assert_eq;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::Command;
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::effects::CopyDestination;
use codex_tui_core::app::effects::CopyFormat;
use codex_tui_core::app::effects::CopyStatus;
use codex_tui_core::app::effects::Done;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::popovers;
use codex_tui_rtwk::services::clipboard::Clipboard;
use codex_tui_rtwk::services::clipboard::Environment;
use codex_tui_rtwk::services::clipboard::Lease;
use codex_tui_rtwk::services::clipboard::Outcome;
use codex_tui_rtwk::services::clipboard::Routes;
use codex_tui_rtwk::services::effects::*;
use codex_tui_rtwk::services::history::Batch;
use codex_tui_rtwk::services::history::HistoryFile;
use codex_tui_rtwk::services::history::usage::MentionFile;
use codex_tui_rtwk::services::open_url;
use codex_tui_rtwk::services::shell::WorkspaceExec;
use codex_tui_rtwk::services::startup::Browser;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

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

    fn append<'a>(&'a self, conversation: &'a str, text: &'a str) -> Boxed<'a, Result<(), String>> {
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
    let selection = Arc::new(());
    let owner = codex_tui_core::app::effects::PrimaryOwner::new(&selection);
    let ask = |destination| Ask::Copy {
        text: "**hi**".into(),
        format: CopyFormat::PlainText,
        destination,
    };
    // The oracle answers a PRIMARY-only publication as a request, nothing confirms it.
    assert_eq!(
        done(&rig, ask(CopyDestination::Primary(owner.clone()))).await,
        Done::Copy(Ok(CopyStatus::Unconfirmed))
    );
    assert_eq!(*rig.primary.lock().unwrap(), vec!["**hi**".to_owned()]);
    assert!(rig.written.lock().unwrap().is_empty(), "PRIMARY only");
    assert_eq!(
        done(
            &rig,
            ask(CopyDestination::ClipboardAndPrimary("hi".into(), owner))
        )
        .await,
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
