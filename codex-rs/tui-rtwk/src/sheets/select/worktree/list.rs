//! The pages of the worktree sheet as the shared picker's lists (oracle
//! `chatwidget/worktree_picker.rs`, every page of it `SelectionViewParams::picker()` but the
//! loading row and the confirmation): the rows are core's `Menu::rows` (their ids, names, what a
//! search matches, what is disabled), the words are `strings.rs`. In Flutter these are the
//! `ListView` bodies of the `MaterialPageRoute`s of a bottom sheet's `Navigator`.

use codex_tui_core::screens::select::worktree::{self, Conversation, Menu};

use super::strings::Strings;
use crate::widgets::picker::list::{Columns, Item, List};

/// The list a page is shown as; `now` is the clock the browser's ages are counted from.
pub fn of(menu: &Menu, now: i64, s: &Strings) -> List {
    let rows = menu.rows();
    let picker = |title: &str, items: Vec<Item>| List {
        title: Some(title.to_string()),
        standard_hint: true,
        items,
        panel: true,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        ..List::default()
    };
    let fixed = |id: &str, name: &str, description: Option<&str>| Item {
        id: id.to_string(),
        name: name.to_string(),
        description: description.map(str::to_string),
        ..Item::default()
    };
    match menu {
        Menu::Checkout { mode, .. } => {
            let title = match mode {
                Conversation::New => s.checkout_new_title,
                Conversation::Fork => s.checkout_fork_title,
            };
            picker(
                title,
                vec![
                    fixed(
                        worktree::CURRENT,
                        s.current_name,
                        Some(s.current_description),
                    ),
                    fixed(worktree::CREATE, s.create_name, Some(s.create_description)),
                ],
            )
        }
        Menu::Worktrees => picker(
            s.worktrees_title,
            vec![
                fixed(
                    worktree::CONTINUE,
                    s.continue_name,
                    Some(s.continue_description),
                ),
                fixed(worktree::START, s.start_name, Some(s.start_description)),
                fixed(worktree::BROWSE, s.browse_name, Some(s.browse_description)),
            ],
        ),
        // The loading row: a panel, not the picker preset (columns from the rows in view).
        Menu::Loading => List {
            title: Some(s.browser_title.to_string()),
            standard_hint: true,
            items: vec![Item {
                disabled: true,
                ..fixed("loading", s.loading, None)
            }],
            panel: true,
            ..List::default()
        },
        Menu::Browser(entries) => List {
            subtitle: Some(
                if entries.is_empty() {
                    s.browser_empty
                } else {
                    s.browser_subtitle
                }
                .to_string(),
            ),
            searchable: true,
            ..picker(
                s.browser_title,
                entries
                    .iter()
                    .zip(&rows)
                    .map(|(entry, row)| Item {
                        id: row.id.clone(),
                        name: entry.name().to_string(),
                        description: Some(s.describe(entry, now)),
                        search_value: row.search.clone(),
                        ..Item::default()
                    })
                    .collect(),
            )
        },
        Menu::Actions { entry, .. } => {
            let items = rows
                .iter()
                .map(|row| {
                    let (name, reason) = match row.id.as_str() {
                        worktree::RESUME => (s.resume, None),
                        worktree::COPY => (s.copy, None),
                        _ => (s.delete, row.disabled.then_some(s.delete_blocked)),
                    };
                    Item {
                        id: row.id.clone(),
                        name: name.to_string(),
                        disabled_reason: reason.map(str::to_string),
                        ..Item::default()
                    }
                })
                .collect();
            List {
                subtitle: Some(entry.cwd.clone()),
                ..picker(&s.actions_heading(entry), items)
            }
        }
        Menu::Confirm { root } => List {
            subtitle: Some(root.clone()),
            ..picker(
                s.confirm_title,
                vec![
                    fixed(worktree::CANCEL, s.confirm_cancel, None),
                    fixed(
                        worktree::DELETE,
                        s.confirm_delete,
                        Some(s.confirm_delete_description),
                    ),
                ],
            )
        },
    }
}
