//! The `$` popup (`bottom_pane/skill_popup.rs`, carried; the oracle's
//! "legacy" popup, the one `$` always uses): skills and apps with a category
//! tag, ordered by how exact the match is, how often the user picked the
//! mention lately, then the fuzzy score.

use std::cmp::Reverse;
use std::collections::HashMap;

use crate::kit::fuzzy::fuzzy_match;
use crate::data::mention::uri;
use crate::data::mention::{Mention, MentionKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MentionItem {
    pub display_name: String,
    pub description: Option<String>,
    pub insert_text: String,
    pub search_terms: Vec<String>,
    pub path: Option<String>,
    /// What the item is (`[Skill]`, `[App]`: the view's words).
    pub kind: MentionKind,
    pub sort_rank: u8,
}

/// A row the query left: the item, the highlighted positions of its display
/// name and the score.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub index: usize,
    pub indices: Option<Vec<usize>>,
    pub score: i32,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum MatchTier {
    Exact,
    Prefix,
    Fuzzy,
}

/// The most characters of a name the `$` popup shows (`MENTION_NAME_TRUNCATE_LEN`).
pub const NAME_LIMIT: usize = 28;

impl MentionItem {
    /// The name as the popup lists it: at most [`NAME_LIMIT`] characters, the
    /// last three `...` when cut (`truncate_text`; scalar values, not
    /// grapheme clusters: the core crate carries no segmentation table).
    pub fn shown_name(&self) -> String {
        if self.display_name.chars().count() <= NAME_LIMIT {
            return self.display_name.clone();
        }
        let kept: String = self.display_name.chars().take(NAME_LIMIT - 3).collect();
        format!("{kept}...")
    }

    /// Matches the display name first, then the best distinct search term.
    pub fn fuzzy_match_query(&self, query: &str) -> Option<(Option<Vec<usize>>, i32)> {
        if let Some((indices, score)) = fuzzy_match(&self.display_name, query) {
            Some((Some(indices), score))
        } else {
            self.search_terms
                .iter()
                .filter(|term| *term != &self.display_name)
                .filter_map(|term| fuzzy_match(term, query).map(|(_indices, score)| score))
                .min()
                .map(|score| (None, score))
        }
    }

    fn match_tier(&self, lowercase_query: &str) -> MatchTier {
        let display_name = self.display_name.to_lowercase();
        let insert_name = self
            .insert_text
            .strip_prefix('$')
            .unwrap_or(&self.insert_text)
            .to_lowercase();
        if display_name == lowercase_query || insert_name == lowercase_query {
            MatchTier::Exact
        } else if display_name.starts_with(lowercase_query)
            || insert_name.starts_with(lowercase_query)
        {
            MatchTier::Prefix
        } else {
            MatchTier::Fuzzy
        }
    }
}

/// The items of the `$` popup (`mention_items`): skills and apps; plugins
/// only list under `@` (the unified popup is the default).
pub fn mention_items(mentions: &[Mention]) -> Vec<MentionItem> {
    mentions
        .iter()
        .filter_map(|mention| match mention.kind {
            MentionKind::Skill => {
                let description = mention.description.trim();
                let search_terms = if mention.name == mention.insert {
                    vec![mention.insert.clone()]
                } else {
                    vec![mention.insert.clone(), mention.name.clone()]
                };
                Some(MentionItem {
                    display_name: mention.name.clone(),
                    description: (!description.is_empty()).then(|| description.to_string()),
                    insert_text: format!("${}", mention.insert),
                    search_terms,
                    path: mention.path.clone(),
                    kind: MentionKind::Skill,
                    sort_rank: 1,
                })
            }
            MentionKind::App => {
                let description = mention.description.trim();
                let id = mention
                    .path
                    .as_deref()
                    .and_then(|path| path.strip_prefix("app://"))
                    .unwrap_or_default();
                Some(MentionItem {
                    display_name: mention.name.clone(),
                    description: (!description.is_empty()).then(|| description.to_string()),
                    insert_text: format!("${}", mention.insert),
                    search_terms: vec![
                        mention.name.clone(),
                        id.to_string(),
                        mention.insert.clone(),
                    ],
                    path: mention.path.clone().or_else(|| Some(uri("app", id))),
                    kind: MentionKind::App,
                    sort_rank: 1,
                })
            }
            MentionKind::Plugin => None,
        })
        .collect()
}

/// The rows `query` leaves of `items`, best first (`SkillPopup::filtered`).
pub fn filtered(
    items: &[MentionItem],
    frequencies: &HashMap<String, usize>,
    query: &str,
) -> Vec<Match> {
    let filter = query.trim();
    let lowercase_filter = filter.to_lowercase();
    let mut out: Vec<Match> = Vec::new();

    for (index, item) in items.iter().enumerate() {
        if filter.is_empty() {
            out.push(Match {
                index,
                indices: None,
                score: 0,
            });
        } else if let Some((indices, score)) = item.fuzzy_match_query(filter) {
            out.push(Match {
                index,
                indices,
                score,
            });
        }
    }

    out.sort_by_cached_key(|m| {
        let item = &items[m.index];
        let tier = if filter.is_empty() {
            MatchTier::Fuzzy
        } else {
            item.match_tier(&lowercase_filter)
        };
        let frequency = item
            .path
            .as_ref()
            .and_then(|path| frequencies.get(path))
            .copied()
            .unwrap_or_default();
        (
            tier,
            Reverse(frequency),
            m.indices.is_none(),
            m.score,
            item.sort_rank,
            item.display_name.clone(),
        )
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_long_name_is_cut_to_the_limit_with_three_dots() {
        let long = "x".repeat(40);
        let mut it = item(&long, "x", "/p", 0);
        assert_eq!(it.shown_name(), format!("{}...", "x".repeat(25)));
        it.display_name = "y".repeat(NAME_LIMIT);
        assert_eq!(it.shown_name(), "y".repeat(NAME_LIMIT));
    }

    fn item(name: &str, insert: &str, path: &str, rank: u8) -> MentionItem {
        MentionItem {
            display_name: name.to_string(),
            description: None,
            insert_text: format!("${insert}"),
            search_terms: vec![insert.to_string(), name.to_string()],
            path: Some(path.to_string()),
            kind: MentionKind::Skill,
            sort_rank: rank,
        }
    }

    fn order(
        items: &[MentionItem],
        frequencies: &HashMap<String, usize>,
        query: &str,
    ) -> Vec<String> {
        filtered(items, frequencies, query)
            .into_iter()
            .map(|m| items[m.index].display_name.clone())
            .collect()
    }

    #[test]
    fn match_tier_precedes_preference_and_plugin_rank() {
        let items = vec![
            item("Fuzzy Cal", "fc", "p/fc", 0),
            item("cal", "cal", "p/cal", 1),
            item("Calendar", "calendar", "p/calendar", 1),
        ];
        let frequencies = HashMap::from([("p/calendar".to_string(), 9)]);
        assert_eq!(
            order(&items, &frequencies, "cal"),
            ["cal", "Calendar", "Fuzzy Cal"]
        );
    }

    #[test]
    fn the_empty_query_lists_everything_in_its_order_by_use() {
        let items = vec![item("b", "b", "p/b", 1), item("a", "a", "p/a", 1)];
        let frequencies = HashMap::from([("p/b".to_string(), 2)]);
        assert_eq!(order(&items, &frequencies, ""), ["b", "a"]);
        assert_eq!(order(&items, &HashMap::new(), ""), ["a", "b"]);
    }

    #[test]
    fn a_name_match_beats_a_worse_search_term_match() {
        let mut by_term = item("Zed", "zed", "p/zed", 1);
        by_term.search_terms.push("alpha".to_string());
        let items = vec![by_term, item("alpha", "alpha", "p/alpha", 1)];
        assert_eq!(order(&items, &HashMap::new(), "alpha"), ["alpha", "Zed"]);
    }

    #[test]
    fn skills_and_apps_list_under_dollar_and_plugins_do_not() {
        let mut app = Mention::new(MentionKind::App, "Google Calendar", "", "google_calendar");
        app.path = Some("app://cal".to_string());
        let items = mention_items(&[
            Mention::new(
                MentionKind::Skill,
                "Google Calendar",
                "plan",
                "google-calendar-skill",
            ),
            app,
            Mention::new(MentionKind::Plugin, "p", "", "p"),
        ]);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].insert_text, "$google-calendar-skill");
        assert_eq!(items[0].kind, MentionKind::Skill);
        assert_eq!(items[1].path.as_deref(), Some("app://cal"));
        assert_eq!(
            items[1].search_terms,
            ["Google Calendar", "cal", "google_calendar"]
        );
    }
}
