//! Which rows a query leaves of the catalog and the file matches, in what
//! order (`mentions_v2/filter.rs`, carried).

use crate::screens::composer::mentions::candidate::{Candidate, MentionType, Row, Selection};
use crate::screens::composer::mentions::files::{FileMatch, MatchType};
use super::search_mode::SearchMode;
use crate::commands::fuzzy_match;

pub fn filtered_candidates(
    candidates: &[Candidate],
    file_matches: &[FileMatch],
    query: &str,
    search_mode: SearchMode,
    show_file_matches: bool,
) -> Vec<Row> {
    let filter = query.trim();
    let mut out = Vec::new();

    for candidate in candidates {
        if !search_mode.accepts(candidate.mention_type) {
            continue;
        }
        if filter.is_empty() {
            out.push(candidate.to_row(None, 0));
            continue;
        }
        if let Some((indices, score)) = best_tool_match(candidate, filter) {
            out.push(candidate.to_row(indices, score));
        }
    }

    if show_file_matches {
        out.extend(
            file_matches
                .iter()
                .map(file_match_to_row)
                .filter(|row| search_mode.accepts(row.mention_type)),
        );
    }

    sort_rows(&mut out, filter);
    out
}

fn best_tool_match(candidate: &Candidate, filter: &str) -> Option<(Option<Vec<usize>>, i32)> {
    if let Some((indices, score)) = fuzzy_match(&candidate.display_name, filter) {
        return Some((Some(indices), score));
    }
    candidate
        .search_terms
        .iter()
        .filter(|term| *term != &candidate.display_name)
        .filter_map(|term| fuzzy_match(term, filter).map(|(_indices, score)| score))
        .min()
        .map(|score| (None, score))
}

fn sort_rows(rows: &mut [Row], filter: &str) {
    let type_order = |mention_type: MentionType| match mention_type {
        MentionType::Plugin => 0,
        MentionType::Skill => 1,
        MentionType::Task => 2,
        MentionType::File | MentionType::Directory => 3,
    };
    rows.sort_by(|a, b| {
        if a.mention_type == MentionType::Task && b.mention_type == MentionType::Task {
            return std::cmp::Ordering::Equal;
        }
        type_order(a.mention_type)
            .cmp(&type_order(b.mention_type))
            .then_with(|| compare_within_rank(a, b, filter))
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
}

fn compare_within_rank(a: &Row, b: &Row, filter: &str) -> std::cmp::Ordering {
    if a.mention_type.is_filesystem() && b.mention_type.is_filesystem() {
        return b.score.cmp(&a.score);
    }
    if filter.is_empty() {
        return a.display_name.cmp(&b.display_name);
    }
    a.match_indices
        .is_none()
        .cmp(&b.match_indices.is_none())
        .then_with(|| a.score.cmp(&b.score))
}

fn file_match_to_row(file_match: &FileMatch) -> Row {
    Row {
        display_name: file_match.path.clone(),
        description: None,
        capabilities: Vec::new(),
        mention_type: match file_match.match_type {
            MatchType::File => MentionType::File,
            MatchType::Directory => MentionType::Directory,
        },
        selection: Selection::File(file_match.path.clone()),
        match_indices: file_match
            .indices
            .as_ref()
            .map(|indices| indices.iter().map(|idx| *idx as usize).collect()),
        score: file_match.score as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn tool(name: &str, ty: MentionType) -> Candidate {
        Candidate {
            display_name: name.to_string(),
            description: None,
            capabilities: Vec::new(),
            search_terms: vec![name.to_string()],
            mention_type: ty,
            selection: Selection::Tool {
                insert_text: format!("${name}"),
                path: None,
            },
        }
    }

    fn file(path: &str, score: u32) -> FileMatch {
        FileMatch {
            path: path.to_string(),
            match_type: MatchType::File,
            score,
            indices: None,
        }
    }

    fn names(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.display_name.as_str()).collect()
    }

    #[test]
    fn sections_run_plugins_skills_tasks_files() {
        let candidates = vec![
            tool("zeta", MentionType::Task),
            tool("beta", MentionType::Skill),
            tool("alpha", MentionType::Plugin),
        ];
        let rows = filtered_candidates(
            &candidates,
            &[file("a.rs", 3)],
            "",
            SearchMode::Results,
            true,
        );
        assert_eq!(names(&rows), ["alpha", "beta", "zeta", "a.rs"]);
    }

    #[test]
    fn files_rank_by_score_high_first_and_tabs_filter() {
        let candidates = vec![tool("alpha", MentionType::Plugin)];
        let files = [file("low.rs", 1), file("high.rs", 9)];
        let rows = filtered_candidates(&candidates, &files, "", SearchMode::FilesystemOnly, true);
        assert_eq!(names(&rows), ["high.rs", "low.rs"]);
        let rows = filtered_candidates(&candidates, &files, "", SearchMode::Tools, true);
        assert_eq!(names(&rows), ["alpha"]);
    }

    #[test]
    fn a_query_keeps_matches_and_a_name_match_beats_a_term_match() {
        let mut by_term = tool("Zed", MentionType::Skill);
        by_term.search_terms.push("alpha-tool".to_string());
        let candidates = vec![by_term, tool("alpha", MentionType::Skill), tool("beta", MentionType::Skill)];
        let rows = filtered_candidates(&candidates, &[], "alpha", SearchMode::Results, false);
        assert_eq!(names(&rows), ["alpha", "Zed"]);
        assert!(rows[0].match_indices.is_some() && rows[1].match_indices.is_none());
    }

    #[test]
    fn file_matches_are_hidden_until_the_search_answered() {
        let rows = filtered_candidates(&[], &[file("a.rs", 1)], "a", SearchMode::Results, false);
        assert!(rows.is_empty());
    }
}
