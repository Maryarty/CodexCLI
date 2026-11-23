//! `/review` in the selection sheet (oracle `chatwidget/review_popups.rs`): the presets, the base
//! branches and the commits are lists of the shared picker (the rows are core's
//! `review::Menu::rows`, the words are `review/strings.rs`), the custom instructions are a text
//! prompt of their own (`CustomPromptView` on the shared-picker layout). In Flutter the lists are
//! the `showModalBottomSheet` builders over a `ListView`, the prompt a `TextField` over a
//! `TextEditingController` in the same sheet.

use rtwk::Handler;

use crate::widgets::picker::list::{Columns, Item, List};
use codex_tui_core::screens::select::review::{self, Menu};

pub mod strings;

use self::strings::Strings;
use super::prompt::{Answer, Prompt};

/// The list a review menu is shown as; none for the instructions (a prompt, `instructions`).
/// `cursor` is the row of the menu that opened the one above it, shown again by Esc.
pub fn list(menu: &Menu, cursor: Option<&str>, s: &Strings) -> Option<List> {
    let rows = menu.rows();
    let initial = cursor.and_then(|id| rows.iter().position(|row| row.id == id));
    Some(match menu {
        Menu::Presets => List {
            title: Some(s.presets_title.to_string()),
            standard_hint: true,
            items: rows.iter().map(|row| preset(&row.id, s)).collect(),
            initial,
            panel: true,
            columns: Columns::AutoAllRows,
            hide_description_below: Some(24),
            ..List::default()
        },
        Menu::Branches { current, .. } => List {
            title: Some(s.branches_title.to_string()),
            subtitle: Some(s.current_branch(current.as_deref())),
            standard_hint: true,
            items: rows.iter().map(named).collect(),
            initial,
            searchable: true,
            search_placeholder: Some(s.branches_search.to_string()),
            panel: true,
            ..List::default()
        },
        Menu::Commits(_) => List {
            title: Some(s.commits_title.to_string()),
            standard_hint: true,
            items: rows.iter().map(named).collect(),
            initial,
            searchable: true,
            search_placeholder: Some(s.commits_search.to_string()),
            panel: true,
            ..List::default()
        },
        Menu::Instructions => return None,
    })
}

/// A row of the presets: its name by id, `(PR Style)` after the first.
fn preset(id: &str, s: &Strings) -> Item {
    let (name, description) = match id {
        review::BASE => (s.base_name, Some(s.base_description.to_string())),
        review::UNCOMMITTED => (s.uncommitted_name, None),
        review::COMMIT => (s.commit_name, None),
        _ => (s.custom_name, None),
    };
    Item {
        id: id.to_string(),
        name: name.to_string(),
        description,
        ..Item::default()
    }
}

/// A row of the branches or of the commits: the model's own words, searched by its value.
fn named(row: &review::Row) -> Item {
    Item {
        id: row.id.clone(),
        name: row.name.clone().unwrap_or_default(),
        search_value: row.search.clone(),
        ..Item::default()
    }
}

/// The text prompt of `Custom review instructions` (the shared prompt, words of the review table).
pub fn instructions(on_answer: Handler<Answer>, s: &'static Strings) -> Prompt {
    Prompt {
        title: s.instructions_title,
        context: None,
        placeholder: s.instructions_placeholder,
        hint: s.instructions_hint,
        mark: s.prompt_mark,
        on_answer,
    }
}
