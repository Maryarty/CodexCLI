//! The items of the status-line and terminal-title setups (oracle `StatusLineItem` and
//! `TerminalTitleItem`: the names the config writes, in the oracle's order) and the dialogs
//! built over them from the config's facts. The words of an item (description, sample value)
//! are the view's, by id.

use std::collections::BTreeMap;

use super::catalog::{CatalogueItem, setup};
use super::{Setup, SetupKind};
use crate::screens::chat::{DEFAULT_STATUS_LINE_ITEMS, StatusFacts};

pub use crate::screens::chat::surface_items::{DEFAULT_TITLE, STATUS_LINE, TERMINAL_TITLE};
use crate::screens::chat::surface_items::{status_item, title_item};

fn catalogue(ids: &[&str]) -> Vec<CatalogueItem> {
    ids.iter()
        .map(|id| CatalogueItem {
            id: (*id).to_owned(),
            description: String::new(),
            placeholder: String::new(),
        })
        .collect()
}

/// `open_status_line_setup`: the configured items (the default list when the config has
/// none), the colour switch, and what the session shows now for each item.
pub fn status_line(facts: &StatusFacts) -> Setup {
    let selected: Vec<String> = match &facts.items {
        Some(items) => items
            .iter()
            .filter_map(|item| status_item(item).map(str::to_owned))
            .collect(),
        None => DEFAULT_STATUS_LINE_ITEMS
            .iter()
            .map(ToString::to_string)
            .collect(),
    };
    setup(
        SetupKind::StatusLine,
        Some(facts.colors),
        &selected,
        catalogue(&STATUS_LINE),
        values(facts, &STATUS_LINE),
    )
}

/// `open_terminal_title_setup`.
pub fn title(facts: &StatusFacts) -> Setup {
    let selected: Vec<String> = match &facts.terminal_title {
        Some(items) => items
            .iter()
            .filter_map(|item| title_item(item).map(str::to_owned))
            .collect(),
        None => DEFAULT_TITLE.iter().map(ToString::to_string).collect(),
    };
    setup(
        SetupKind::Title,
        None,
        &selected,
        catalogue(&TERMINAL_TITLE),
        values(facts, &TERMINAL_TITLE),
    )
}

fn values(facts: &StatusFacts, ids: &[&str]) -> BTreeMap<String, String> {
    ids.iter()
        .filter_map(|id| facts.value(id).map(|value| ((*id).to_owned(), value)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_name_listed_items() {
        for id in DEFAULT_STATUS_LINE_ITEMS {
            assert!(STATUS_LINE.contains(&id), "{id}");
        }
        for id in DEFAULT_TITLE {
            assert!(TERMINAL_TITLE.contains(&id), "{id}");
        }
    }

    #[test]
    fn a_setup_lists_the_configured_items_first_then_every_other_one() {
        let facts = StatusFacts {
            items: Some(vec!["git-branch".into(), "model-name".into()]),
            colors: true,
            ..StatusFacts::default()
        };
        let setup = status_line(&facts);
        let ids: Vec<&str> = setup.items.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(
            &ids[..4],
            [
                "use_theme_colors",
                "git-branch",
                "model",
                "model-with-reasoning"
            ]
        );
        assert_eq!(ids.len(), 1 + STATUS_LINE.len());
        assert!(setup.items[1].enabled && setup.items[2].enabled && !setup.items[3].enabled);
    }

    #[test]
    fn the_title_reads_the_old_names() {
        let facts = StatusFacts {
            terminal_title: Some(vec!["spinner".into(), "project".into()]),
            ..StatusFacts::default()
        };
        let setup = title(&facts);
        let on: Vec<&str> = setup
            .items
            .iter()
            .filter(|item| item.enabled)
            .map(|item| item.id.as_str())
            .collect();
        assert_eq!(on, ["activity", "project-name"]);
    }
}
