//! The ranking of the mention popups' filter for a query whose letters come from a lowercase
//! expansion (oracle `skill_popup.rs` `lowercase_expansion_preserves_match_ranking_and_highlighting`:
//! the screen cannot hold that query, so the order is pinned here).

use codex_tui_core::data::mention::{Candidate, MentionType, Selection};
use codex_tui_core::kit::mentions::filter::filtered_candidates;
use codex_tui_core::kit::mentions::search_mode::SearchMode;

fn skill(name: &str) -> Candidate {
    Candidate {
        display_name: name.to_string(),
        description: None,
        capabilities: Vec::new(),
        search_terms: Vec::new(),
        mention_type: MentionType::Skill,
        selection: Selection::Tool {
            insert_text: format!("${name}"),
            path: None,
        },
    }
}

#[test]
fn lowercase_expansion_keeps_the_oracles_order_and_marks() {
    let candidates = [
        skill("İx"),
        skill("i\u{0307}x"),
        skill("aİx"),
        skill("ai\u{0307}x"),
    ];
    let rows = filtered_candidates(&candidates, &[], "\u{0307}x", SearchMode::Results, false);
    // each pair lowercases alike: the dot starts a contiguous match, not a prefix, even when it
    // comes from the expansion of `İ`; the oracle lists the candidates 3, 2, 1, 0 with these marks
    let got: Vec<(&str, Option<Vec<usize>>, i32)> = rows
        .iter()
        .map(|row| {
            (
                row.display_name.as_str(),
                row.match_indices.clone(),
                row.score,
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("ai\u{0307}x", Some(vec![2, 3]), 0),
            ("aİx", Some(vec![1, 2]), 0),
            ("i\u{0307}x", Some(vec![1, 2]), 0),
            ("İx", Some(vec![0, 1]), 0),
        ]
    );
}
