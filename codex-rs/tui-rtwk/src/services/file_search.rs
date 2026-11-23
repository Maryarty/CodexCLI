//! File search for the `@` popup, in process (oracle `file_search.rs`): one
//! `codex-file-search` session over the search directory, re-queried on every
//! change of the `@token` and dropped when the query becomes empty.
//!
//! This is the second implementation of the popup's file search. The first is
//! the app-server's session API (core's `AppServerSearch`), which also works
//! against a remote server; this one reads the local disk, so it is for a
//! backend that shares the machine with the UI. Both answer the same pair:
//! `popovers::Ask::FileSearch(FileSearchAsk)` in, `popovers::Ev::FileMatches`
//! out, so the choice between them (the person's open decision) is the one
//! place that routes the ask.
//!
//! In Flutter this is a `Stream<List<FileMatch>>` fed by a search isolate;
//! here the session's own threads call the reporter, which hands every
//! snapshot to the sink (the app's ingress) as an event.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;

use codex_file_search as search;
use codex_tui_core::data::files::FileMatch;
use codex_tui_core::data::files::FileSearchAsk;
use codex_tui_core::data::files::MatchType;
use codex_tui_core::screens::popovers::Ev;

/// Where the answers go: the app's ingress, one event per snapshot.
pub type Sink = Arc<dyn Fn(Ev) + Send + Sync>;

struct State {
    /// The query the popup showed last; the session answers for it only.
    latest_query: String,
    session: Option<search::FileSearchSession>,
    /// Which session may still answer: a dropped session's late snapshot
    /// must not reach the popup.
    session_token: usize,
}

/// The manager the app owns: one search directory, one session at a time.
pub struct InProcessSearch {
    state: Arc<Mutex<State>>,
    search_dir: PathBuf,
    sink: Sink,
}

impl InProcessSearch {
    pub fn new(search_dir: PathBuf, sink: Sink) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                latest_query: String::new(),
                session: None,
                session_token: 0,
            })),
            search_dir,
            sink,
        }
    }

    /// The directory searched changed (a resumed thread's cwd): the session
    /// is dropped and the next query starts one over the new directory.
    pub fn set_search_dir(&mut self, search_dir: PathBuf) {
        self.search_dir = search_dir;
        let mut state = lock(&self.state);
        state.session.take();
        state.latest_query.clear();
    }

    /// What the popup asks (`Ask::FileSearch`): a query, or the end of the
    /// session.
    pub fn ask(&self, ask: FileSearchAsk) {
        match ask {
            FileSearchAsk::Query(query) => self.query(query),
            FileSearchAsk::Stop => self.stop(),
        }
    }

    /// Call whenever the `@token` changes. The empty query drops the session,
    /// so an identical query answers again after it.
    pub fn query(&self, query: String) {
        let mut state = lock(&self.state);
        if query == state.latest_query {
            return;
        }
        state.latest_query.clear();
        state.latest_query.push_str(&query);

        if query.is_empty() {
            state.session.take();
            return;
        }
        if state.session.is_none() {
            self.start_session(&mut state);
        }
        if let Some(session) = state.session.as_ref() {
            session.update_query(&query);
        }
    }

    /// The popup closed: drop the session.
    pub fn stop(&self) {
        let mut state = lock(&self.state);
        state.session.take();
        state.latest_query.clear();
    }

    fn start_session(&self, state: &mut State) {
        state.session_token = state.session_token.wrapping_add(1);
        let reporter = Arc::new(Reporter {
            state: Arc::clone(&self.state),
            sink: Arc::clone(&self.sink),
            session_token: state.session_token,
        });
        let session = search::create_session(
            vec![self.search_dir.clone()],
            search::FileSearchOptions {
                compute_indices: true,
                ..Default::default()
            },
            reporter,
            /*cancel_flag*/ None,
        );
        match session {
            Ok(session) => state.session = Some(session),
            Err(error) => {
                tracing::warn!("file search session failed to start: {error}");
                state.session = None;
            }
        }
    }
}

fn lock(state: &Mutex<State>) -> std::sync::MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

struct Reporter {
    state: Arc<Mutex<State>>,
    sink: Sink,
    session_token: usize,
}

impl search::SessionReporter for Reporter {
    fn on_update(&self, snapshot: &search::FileSearchSnapshot) {
        {
            let state = lock(&self.state);
            if state.session_token != self.session_token
                || state.latest_query.is_empty()
                || snapshot.query.is_empty()
            {
                return;
            }
        }
        (self.sink)(Ev::FileMatches {
            query: snapshot.query.clone(),
            matches: snapshot.matches.iter().map(file_match).collect(),
        });
    }

    fn on_complete(&self) {}
}

fn file_match(found: &search::FileMatch) -> FileMatch {
    FileMatch {
        path: found.path.to_string_lossy().into_owned(),
        match_type: match found.match_type {
            search::MatchType::File => MatchType::File,
            search::MatchType::Directory => MatchType::Directory,
        },
        score: found.score,
        indices: found.indices.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use pretty_assertions::assert_eq;

    use super::*;

    const WAIT: Duration = Duration::from_secs(30);

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("README.md"), "docs").unwrap();
        dir
    }

    fn search_of(dir: &tempfile::TempDir) -> (InProcessSearch, mpsc::Receiver<Ev>) {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |ev| {
            let _ = tx.lock().unwrap().send(ev);
        });
        (InProcessSearch::new(dir.path().to_owned(), sink), rx)
    }

    /// The next answer for `query` that has matches.
    fn answer(rx: &mpsc::Receiver<Ev>, query: &str) -> Vec<FileMatch> {
        loop {
            match rx.recv_timeout(WAIT).expect("an answer in time") {
                Ev::FileMatches {
                    query: asked,
                    matches,
                } if asked == query && !matches.is_empty() => return matches,
                _ => {}
            }
        }
    }

    #[test]
    fn a_query_answers_the_matching_paths_with_their_indices() {
        let dir = project();
        let (search, rx) = search_of(&dir);

        search.query("main".to_owned());

        let matches = answer(&rx, "main");
        let main = matches
            .iter()
            .find(|found| found.path == "src/main.rs")
            .expect("src/main.rs matches");
        assert_eq!(main.match_type, MatchType::File);
        assert_eq!(
            main.indices.as_deref().map(<[u32]>::len),
            Some(4),
            "the four characters of `main` are marked"
        );
    }

    #[test]
    fn the_empty_query_resets_the_session_so_an_identical_query_answers_again() {
        let dir = project();
        let (search, rx) = search_of(&dir);
        search.query("readme".to_owned());
        answer(&rx, "readme");

        search.ask(FileSearchAsk::Query(String::new()));
        search.ask(FileSearchAsk::Query("readme".to_owned()));

        let matches = answer(&rx, "readme");
        assert_eq!(matches[0].path, "README.md");
    }

    #[test]
    fn a_stopped_session_answers_nothing_more() {
        let dir = project();
        let (search, rx) = search_of(&dir);
        search.query("main".to_owned());
        answer(&rx, "main");

        search.ask(FileSearchAsk::Stop);
        // Nothing the dropped session still had in flight reaches the sink.
        while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
        assert_eq!(lock(&search.state).latest_query, "");
        assert!(lock(&search.state).session.is_none());
    }

    #[test]
    fn a_new_search_directory_starts_a_new_session_over_it() {
        let first = project();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(second.path().join("other.txt"), "x").unwrap();
        let (mut search, rx) = search_of(&first);
        search.query("main".to_owned());
        answer(&rx, "main");

        search.set_search_dir(second.path().to_owned());
        search.query("other".to_owned());

        let matches = answer(&rx, "other");
        assert_eq!(matches[0].path, "other.txt");
    }

    #[test]
    fn a_found_path_keeps_its_kind_score_and_indices() {
        let found = search::FileMatch {
            score: 42,
            path: PathBuf::from("src"),
            match_type: search::MatchType::Directory,
            root: PathBuf::from("/work"),
            indices: Some(vec![0, 1]),
        };

        assert_eq!(
            file_match(&found),
            FileMatch {
                path: "src".to_owned(),
                match_type: MatchType::Directory,
                score: 42,
                indices: Some(vec![0, 1]),
            }
        );
    }
}
