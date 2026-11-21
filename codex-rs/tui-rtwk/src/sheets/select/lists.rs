//! The named parameterisations as the generic list: the model picker and the
//! permissions menu are data in the model (`Menu`), their words are here
//! (ADR-010). A `Menu::List` is already the generic form.

use codex_tui_core::screens::select::list::{Item, List};
use codex_tui_core::screens::select::{Access, Menu};

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
