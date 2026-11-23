//! What the popup shows of the file search (`mentions_v2/popup.rs` `FileSearch`).

use crate::data::files::FileMatch;

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
    use crate::data::files::MatchType;
    use pretty_assertions::assert_eq;

    fn found(path: &str) -> FileMatch {
        FileMatch {
            path: path.to_string(),
            match_type: MatchType::File,
            score: 1,
            indices: None,
        }
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
