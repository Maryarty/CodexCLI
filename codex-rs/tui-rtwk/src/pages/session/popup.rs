//! What the slash popup lists for the text before the caret: the commands
//! after a `/` at the start. The data and the filtering are core's
//! (`commands`); this turns rows into the suggestions the field shows. The
//! `$` and `@` popups are `popovers::mention`'s: they sit above the composer.

use rtwk::editing::Suggestion;

use codex_tui_core::commands::{CommandPopup, CommandPopupFlags, SlashCommandItem};
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::composer::{TriggerKind, trigger};

use super::strings::Strings;

/// The most rows a popup shows at once (`MAX_POPUP_ROWS`).
pub const ROWS: usize = codex_tui_core::commands::MAX_POPUP_ROWS;

/// The suggestions for `before`, the composer's text up to the caret.
pub fn suggestions(states: &ScreenStates, strings: &Strings, before: &str) -> Vec<Suggestion> {
    match trigger(before) {
        Some(trigger) if trigger.kind == TriggerKind::Slash => commands(states, strings, before),
        _ => Vec::new(),
    }
}

fn commands(states: &ScreenStates, strings: &Strings, before: &str) -> Vec<Suggestion> {
    let mut popup = CommandPopup::new(
        CommandPopupFlags::from(states.chat.flags),
        states.chat.service_tiers.clone(),
    );
    popup.on_composer_text_change(before.to_string());
    popup
        .rows()
        .into_iter()
        .filter(|row| !row.unavailable)
        .map(|row| {
            let name = format!("/{}", row.item.command());
            let description = row.description.unwrap_or_else(|| match &row.item {
                SlashCommandItem::Builtin(command) => (strings.description)(*command).to_string(),
                SlashCommandItem::ServiceTier(tier) => tier.description.clone(),
            });
            Suggestion::new(name.clone())
                .label(name)
                .detail(description)
        })
        .collect()
}
