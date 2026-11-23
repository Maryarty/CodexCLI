//! The unified `@` popup as a value (`mentions_v2/popup.rs`, carried): the
//! live catalog, the query, the search tab and the file search, and the rows
//! they leave. The highlight and the scroll window are the view's controllers
//! (`SelectionController`, `ListController`); [`Popup::position_of`] lets
//! the view keep a highlight on the same pick when rows change.

use super::files::{Empty, FileSearch};
use super::filter::filtered_candidates;
use super::search_mode::SearchMode;
use crate::data::files::FileMatch;
use crate::data::mention::{Candidate, Row, Selection};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Popup {
    query: String,
    file_search: FileSearch,
    candidates: Vec<Candidate>,
    rows: Vec<Row>,
    search_mode: SearchMode,
}

impl Popup {
    pub fn new(candidates: Vec<Candidate>, query: &str) -> Self {
        let mut file_search = FileSearch::default();
        file_search.set_query(query);
        let mut popup = Self {
            query: query.to_string(),
            file_search,
            candidates,
            rows: Vec::new(),
            search_mode: SearchMode::Results,
        };
        popup.refresh_rows();
        popup
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn search_mode(&self) -> SearchMode {
        self.search_mode
    }

    /// What an empty list says: waiting for the search or no match.
    pub fn empty(&self) -> Empty {
        self.file_search.empty()
    }

    pub fn set_candidates(&mut self, candidates: Vec<Candidate>) {
        self.candidates = candidates;
        self.refresh_rows();
    }

    pub fn set_query(&mut self, query: &str) {
        if self.query == query {
            return;
        }
        self.query = query.to_string();
        self.file_search.set_query(query);
        self.refresh_rows();
    }

    pub fn set_file_matches(&mut self, query: &str, matches: Vec<FileMatch>) {
        self.file_search.set_matches(query, matches);
        self.refresh_rows();
    }

    pub fn set_search_mode(&mut self, mode: SearchMode) {
        if self.search_mode != mode {
            self.search_mode = mode;
            self.refresh_rows();
        }
    }

    pub fn previous_search_mode(&mut self) {
        self.set_search_mode(self.search_mode.previous());
    }

    pub fn next_search_mode(&mut self) {
        self.set_search_mode(self.search_mode.next());
    }

    pub fn position_of(&self, selection: &Selection) -> Option<usize> {
        self.rows.iter().position(|row| row.selection == *selection)
    }

    fn refresh_rows(&mut self) {
        self.rows = filtered_candidates(
            &self.candidates,
            self.file_search.matches(),
            &self.query,
            self.search_mode,
            self.file_search.should_show_matches(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::files::MatchType;
    use crate::data::mention::MentionType;

    fn skill(name: &str) -> Candidate {
        Candidate {
            display_name: name.to_string(),
            description: None,
            capabilities: Vec::new(),
            search_terms: vec![name.to_string()],
            mention_type: MentionType::Skill,
            selection: Selection::Tool {
                insert_text: format!("${name}"),
                path: None,
            },
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
    fn file_matches_join_the_rows_once_the_search_answered() {
        let mut popup = Popup::new(vec![skill("fig")], "fi");
        assert_eq!(popup.rows().len(), 1);
        popup.set_file_matches("fi", vec![found("a/fig.rs")]);
        assert_eq!(popup.rows().len(), 2);
        // another query: the old matches stay while the new search runs, the
        // answer replaces them, an empty query drops them
        popup.set_query("fo");
        assert_eq!(popup.rows().len(), 1);
        assert_eq!(popup.empty(), Empty::Loading);
        popup.set_file_matches("fo", Vec::new());
        assert!(
            popup
                .rows()
                .iter()
                .all(|row| !row.mention_type.is_filesystem())
        );
        popup.set_file_matches("fo", vec![found("b/fo.rs")]);
        popup.set_query("");
        assert!(
            popup
                .rows()
                .iter()
                .all(|row| !row.mention_type.is_filesystem())
        );
    }

    #[test]
    fn the_tab_narrows_the_rows_and_the_pick_keeps_its_position() {
        let mut popup = Popup::new(vec![skill("fig")], "f");
        popup.set_file_matches("f", vec![found("f.rs")]);
        let pick = Selection::File("f.rs".to_string());
        assert_eq!(popup.position_of(&pick), Some(1));
        popup.next_search_mode();
        assert_eq!(popup.search_mode(), SearchMode::FilesystemOnly);
        assert_eq!(popup.position_of(&pick), Some(0));
    }
}
