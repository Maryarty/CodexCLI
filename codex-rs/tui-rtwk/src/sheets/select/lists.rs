//! The named parameterisations as the generic list: the model picker and the
//! permissions menu are data in the model (`Menu`), their words are here
//! (ADR-010). A `Menu::List` is already the generic form.

use codex_tui_core::screens::select::list::Columns;
use codex_tui_core::screens::select::list::{Item, List};
use codex_tui_core::screens::select::{Access, Confirm, Menu};

use super::strings::Strings;

/// The list `menu` is shown as.
pub fn of(menu: &Menu, s: &Strings) -> List {
    match menu {
        Menu::List(list) => (**list).clone(),
        Menu::Models(models) => List {
            title: Some(s.model_title.to_string()),
            items: models
                .iter()
                .map(|row| Item {
                    name: row.name.clone(),
                    description: Some(row.description.clone()),
                    current: row.current,
                    ..Item::default()
                })
                .collect(),
            ..List::default()
        },
        Menu::Confirm(confirm) => confirmation(*confirm, s),
        Menu::Permissions { current } => {
            let rows = [
                (Access::Ask, s.access_ask, s.access_ask_description),
                (Access::Full, s.access_full, s.access_full_description),
            ];
            List {
                title: Some(s.permissions_title.to_string()),
                footer_hint: Some(s.permissions_hint.to_string()),
                items: rows
                    .iter()
                    .map(|(access, name, description)| Item {
                        name: (*name).to_string(),
                        description: Some((*description).to_string()),
                        current: Some(access) == current.as_ref(),
                        ..Item::default()
                    })
                    .collect(),
                ..List::default()
            }
        }
    }
}

/// The two rows of a confirmation, No first (`SelectionViewParams::confirmation`:
/// the panel picker with the standard hint, its columns placed from all rows
/// and the description hidden when narrow).
fn confirmation(confirm: Confirm, s: &Strings) -> List {
    let (title, subtitle, no, yes) = match confirm {
        Confirm::Archive { running } => (
            s.archive_title,
            if running {
                s.archive_subtitle_running
            } else {
                s.archive_subtitle
            },
            (s.archive_no, s.archive_no_description),
            (s.archive_yes, s.archive_yes_description),
        ),
        Confirm::Delete => (
            s.delete_title,
            s.delete_subtitle,
            (s.delete_no, s.delete_no_description),
            (s.delete_yes, s.delete_yes_description),
        ),
    };
    let row = |(name, description): (&str, &str)| Item {
        name: name.to_string(),
        description: Some(description.to_string()),
        ..Item::default()
    };
    List {
        title: Some(title.to_string()),
        subtitle: Some(subtitle.to_string()),
        standard_hint: true,
        items: vec![row(no), row(yes)],
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}
