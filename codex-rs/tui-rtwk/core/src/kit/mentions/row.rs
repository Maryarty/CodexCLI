//! What a row of the `@` popup shows of itself (`mentions_v2/candidate.rs`, carried): the row a
//! candidate becomes for a query, where a file row's name starts in its path, and the name the
//! first column shows. The values are `core::data::mention`.

use crate::data::mention::{Candidate, Row, Selection};

/// The row a query leaves of a candidate: the positions it matched and its score.
pub fn candidate_row(candidate: &Candidate, match_indices: Option<Vec<usize>>, score: i32) -> Row {
    Row {
        display_name: candidate.display_name.clone(),
        description: candidate.description.clone(),
        capabilities: candidate.capabilities.clone(),
        mention_type: candidate.mention_type,
        selection: candidate.selection.clone(),
        match_indices,
        score,
    }
}

/// Where the file name starts in the display name, in characters (the path before it is what the
/// row's second column shows); `None` for a row that is no file or directory.
pub fn file_name_start(row: &Row) -> Option<usize> {
    match row.selection {
        Selection::File(_) if row.mention_type.is_filesystem() => Some(
            row.display_name
                .rfind(['/', '\\'])
                .map_or(0, |at| row.display_name[..=at].chars().count()),
        ),
        Selection::File(_) | Selection::Tool { .. } => None,
    }
}

/// The file name of a file row, the display name of any other.
pub fn primary(row: &Row) -> &str {
    match file_name_start(row) {
        Some(start) => {
            let at = row
                .display_name
                .char_indices()
                .nth(start)
                .map_or(row.display_name.len(), |(at, _)| at);
            &row.display_name[at..]
        }
        None => &row.display_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::mention::MentionType;

    fn file(name: &str, mention_type: MentionType) -> Row {
        Row {
            display_name: name.to_string(),
            description: None,
            capabilities: Vec::new(),
            mention_type,
            selection: Selection::File(name.to_string()),
            match_indices: None,
            score: 0,
        }
    }

    #[test]
    fn a_file_row_splits_its_path_from_its_name() {
        let row = file("src/ui/main.rs", MentionType::File);
        assert_eq!(file_name_start(&row), Some(7));
        assert_eq!(primary(&row), "main.rs");
        let row = file("README.md", MentionType::File);
        assert_eq!(file_name_start(&row), Some(0));
        assert_eq!(primary(&row), "README.md");
        let row = file("dir\\sub\\", MentionType::Directory);
        assert_eq!(primary(&row), "");
    }

    #[test]
    fn a_tool_row_is_its_display_name() {
        let mut row = file("Sample Plugin", MentionType::Plugin);
        row.selection = Selection::Tool {
            insert_text: "@sample".to_string(),
            path: None,
        };
        assert_eq!(file_name_start(&row), None);
        assert_eq!(primary(&row), "Sample Plugin");
    }
}
