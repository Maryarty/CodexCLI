//! File search for the `@` popup, as values: what the popup asks of a backend
//! ([`FileSearchAsk`]), what comes back ([`FileMatch`]) and the contract a
//! backend keeps ([`FileSearchBackend`]).
//!
//! The oracle runs `codex-file-search` in process (`file_search.rs`: a manager
//! the app owns, `StartFileSearch(query)` in, `FileSearchResult` out). The
//! default here is the app-server's session API (`fuzzyFileSearch/session*`),
//! which also works against a remote server; an in-process backend is the
//! same trait with another implementation (the open choice, pack Needs).

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::FuzzyFileSearchMatchType;
use codex_app_server_protocol::FuzzyFileSearchResult;
use codex_app_server_protocol::FuzzyFileSearchSessionStartParams;
use codex_app_server_protocol::FuzzyFileSearchSessionStopParams;
use codex_app_server_protocol::FuzzyFileSearchSessionUpdateParams;
use codex_app_server_protocol::FuzzyFileSearchSessionUpdatedNotification;
use codex_app_server_protocol::RequestId;

/// What kind of path a match is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchType {
    File,
    Directory,
}

/// One path the search found (`codex_file_search::FileMatch`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileMatch {
    /// Relative to the search root.
    pub path: String,
    pub match_type: MatchType,
    pub score: u32,
    /// The character positions of the path the query matched.
    pub indices: Option<Vec<u32>>,
}

/// The answer of the file search that came in last (the model keeps it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileMatches {
    pub query: String,
    pub matches: Vec<FileMatch>,
}

/// What the popup needs of the backend: a query to run (the empty one resets
/// the session so an identical query answers again), or an end to the
/// session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileSearchAsk {
    Query(String),
    Stop,
}

/// A file-search backend: the requests an ask costs, and the matches a
/// notification carries. The shell sends the requests; the matches go back
/// to the model as `popovers::Ev::FileMatches`.
pub trait FileSearchBackend {
    /// The requests `ask` needs, in order.
    fn requests(&mut self, ask: &FileSearchAsk, next_id: &mut dyn FnMut() -> RequestId) -> Vec<ClientRequest>;
}

/// The default backend: one app-server search session under `roots`.
#[derive(Clone, Debug)]
pub struct AppServerSearch {
    session_id: String,
    roots: Vec<String>,
    started: bool,
}

impl AppServerSearch {
    pub fn new(session_id: impl Into<String>, roots: Vec<String>) -> Self {
        Self {
            session_id: session_id.into(),
            roots,
            started: false,
        }
    }

    /// The matches of a `sessionUpdated` notification, when it is for this
    /// session: `(query, matches)`.
    pub fn matches_of(
        &self,
        notification: &FuzzyFileSearchSessionUpdatedNotification,
    ) -> Option<(String, Vec<FileMatch>)> {
        (notification.session_id == self.session_id).then(|| {
            (
                notification.query.clone(),
                notification.files.iter().map(file_match).collect(),
            )
        })
    }
}

impl FileSearchBackend for AppServerSearch {
    fn requests(
        &mut self,
        ask: &FileSearchAsk,
        next_id: &mut dyn FnMut() -> RequestId,
    ) -> Vec<ClientRequest> {
        match ask {
            FileSearchAsk::Query(query) => {
                let mut out = Vec::new();
                if !self.started {
                    self.started = true;
                    out.push(ClientRequest::FuzzyFileSearchSessionStart {
                        request_id: next_id(),
                        params: FuzzyFileSearchSessionStartParams {
                            session_id: self.session_id.clone(),
                            roots: self.roots.clone(),
                        },
                    });
                }
                out.push(ClientRequest::FuzzyFileSearchSessionUpdate {
                    request_id: next_id(),
                    params: FuzzyFileSearchSessionUpdateParams {
                        session_id: self.session_id.clone(),
                        query: query.clone(),
                    },
                });
                out
            }
            FileSearchAsk::Stop => {
                if !std::mem::take(&mut self.started) {
                    return Vec::new();
                }
                vec![ClientRequest::FuzzyFileSearchSessionStop {
                    request_id: next_id(),
                    params: FuzzyFileSearchSessionStopParams {
                        session_id: self.session_id.clone(),
                    },
                }]
            }
        }
    }
}

fn file_match(result: &FuzzyFileSearchResult) -> FileMatch {
    FileMatch {
        path: result.path.clone(),
        match_type: match result.match_type {
            FuzzyFileSearchMatchType::File => MatchType::File,
            FuzzyFileSearchMatchType::Directory => MatchType::Directory,
        },
        score: result.score,
        indices: result.indices.clone(),
    }
}

/// What an empty list of the popup says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Empty {
    /// The search is still running.
    Loading,
    /// It answered and found nothing.
    NoMatches,
}

/// What the popup shows of the search (`mentions_v2/popup.rs` `FileSearch`):
/// the query it waits for and the matches that answered it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileSearch {
    pending_query: String,
    display_query: String,
    waiting: bool,
    matches: Vec<FileMatch>,
}

/// How many matches the popup keeps (`MAX_POPUP_ROWS`).
pub const MAX_MATCHES: usize = 8;

impl FileSearch {
    pub fn set_query(&mut self, query: &str) {
        if query.is_empty() {
            self.pending_query.clear();
            self.display_query.clear();
            self.waiting = false;
            self.matches.clear();
        } else if query != self.pending_query {
            self.pending_query = query.to_string();
            self.waiting = true;
        }
    }

    /// The answer to `query`; one for another query than the pending one is
    /// stale and ignored.
    pub fn set_matches(&mut self, query: &str, matches: Vec<FileMatch>) {
        if query != self.pending_query {
            return;
        }
        self.display_query = query.to_string();
        self.matches = matches.into_iter().take(MAX_MATCHES).collect();
        self.waiting = false;
    }

    pub fn matches(&self) -> &[FileMatch] {
        &self.matches
    }

    pub fn should_show_matches(&self) -> bool {
        !self.matches.is_empty()
    }

    /// What an empty list says: the search is still running, or found nothing.
    pub fn empty(&self) -> Empty {
        if self.waiting { Empty::Loading } else { Empty::NoMatches }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn ids() -> impl FnMut() -> RequestId {
        let mut n = 0;
        move || {
            n += 1;
            RequestId::Integer(n)
        }
    }

    fn found(path: &str) -> FileMatch {
        FileMatch {
            path: path.to_string(),
            match_type: MatchType::File,
            score: 1,
            indices: None,
        }
    }

    #[test]
    fn app_server_starts_its_session_once_then_updates() {
        let mut backend = AppServerSearch::new("s1", vec!["/repo".to_string()]);
        let mut next = ids();
        let first = backend.requests(&FileSearchAsk::Query("fo".into()), &mut next);
        assert!(matches!(
            first.as_slice(),
            [
                ClientRequest::FuzzyFileSearchSessionStart { .. },
                ClientRequest::FuzzyFileSearchSessionUpdate { .. }
            ]
        ));
        let second = backend.requests(&FileSearchAsk::Query("foo".into()), &mut next);
        assert!(matches!(
            second.as_slice(),
            [ClientRequest::FuzzyFileSearchSessionUpdate { .. }]
        ));
        let stop = backend.requests(&FileSearchAsk::Stop, &mut next);
        assert!(matches!(
            stop.as_slice(),
            [ClientRequest::FuzzyFileSearchSessionStop { .. }]
        ));
        assert!(backend.requests(&FileSearchAsk::Stop, &mut next).is_empty());
    }

    #[test]
    fn app_server_notifications_of_another_session_are_ignored() {
        let backend = AppServerSearch::new("s1", Vec::new());
        let note = |session: &str| FuzzyFileSearchSessionUpdatedNotification {
            session_id: session.to_string(),
            query: "foo".to_string(),
            files: vec![FuzzyFileSearchResult {
                root: "/repo".to_string(),
                path: "src/foo.rs".to_string(),
                match_type: FuzzyFileSearchMatchType::File,
                file_name: "foo.rs".to_string(),
                score: 9,
                indices: Some(vec![4, 5]),
            }],
        };
        assert_eq!(backend.matches_of(&note("other")), None);
        let (query, matches) = backend.matches_of(&note("s1")).unwrap();
        assert_eq!(query, "foo");
        assert_eq!(matches[0].path, "src/foo.rs");
        assert_eq!(matches[0].indices, Some(vec![4, 5]));
    }

    #[test]
    fn a_stale_answer_is_dropped_and_the_pending_query_shows_loading() {
        let mut search = FileSearch::default();
        search.set_query("foo");
        assert_eq!(search.empty(), Empty::Loading);
        search.set_matches("fo", vec![found("a")]);
        assert!(!search.should_show_matches());
        search.set_matches("foo", vec![found("a")]);
        assert!(search.should_show_matches());
        assert_eq!(search.empty(), Empty::NoMatches);
        search.set_query("");
        assert!(!search.should_show_matches());
    }
}
