//! The words of the status-line and terminal-title setups' items (the tables are in
//! `strings.rs`): what each row says it is and what its preview shows while the session has no
//! value for it (oracle `status_line_setup.rs` `StatusLineItem::description` / `preview_item`,
//! `title_setup.rs` `TerminalTitleItem::description` / `preview_item`). The app builds those
//! setups from the chat's status facts with the ids only (`core::features::settings::items`); a
//! setup that carries its own words (a fixture's) keeps them.

use codex_tui_core::features::settings::{Setup, SetupKind};

use super::strings::{STATUS_LINE_ITEMS, TERMINAL_TITLE_ITEMS};

/// `setup` with the words of the items that came without: the description and the preview text
/// by item id.
pub fn filled(mut setup: Setup) -> Setup {
    let table = match setup.kind {
        SetupKind::StatusLine => STATUS_LINE_ITEMS,
        SetupKind::Title => TERMINAL_TITLE_ITEMS,
        SetupKind::Skills => return setup,
    };
    for item in &mut setup.items {
        let Some((_, description, placeholder)) = table.iter().find(|(id, ..)| *id == item.id)
        else {
            continue;
        };
        if item.description.is_empty() {
            item.description = (*description).to_owned();
        }
        if item.placeholder.is_empty() {
            item.placeholder = (*placeholder).to_owned();
        }
    }
    setup
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::features::settings::items;

    #[test]
    fn the_tables_name_every_item_the_app_offers() {
        let status: Vec<&str> = STATUS_LINE_ITEMS.iter().map(|row| row.0).collect();
        let title: Vec<&str> = TERMINAL_TITLE_ITEMS.iter().map(|row| row.0).collect();
        assert_eq!(status, items::STATUS_LINE);
        assert_eq!(title, items::TERMINAL_TITLE);
    }
}
