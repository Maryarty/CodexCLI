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

pub use crate::data::files::{FileMatch, FileMatches, FileSearchAsk, MatchType};

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
}
