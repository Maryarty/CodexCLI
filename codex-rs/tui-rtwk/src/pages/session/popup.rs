//! What the slash popup lists for the text before the caret: the commands
//! after a `/` at the start. The data and the filtering are core's
//! (`commands`); this turns rows into the suggestions the field shows. The
//! `$` and `@` popups are `popovers::mention`'s: they sit above the composer.

use rtwk::editing::Suggestion;

use codex_tui_core::commands::{
    CommandFlags, CommandPopup, CommandPopupFlags, ServiceTierCommand, SlashCommandItem,
};
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::composer::{TriggerKind, trigger};

use super::strings::Strings;

/// The most rows a popup shows at once (`MAX_POPUP_ROWS`).
pub const ROWS: usize = codex_tui_core::commands::MAX_POPUP_ROWS;

/// What the slash popup is made of, the slice of the app's states it draws: the service-tier
/// commands of the thread's model and the gates of the commands. A composer `select`s it, so it
/// wakes when these change and not on every change of any screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Slash {
    tiers: Vec<ServiceTierCommand>,
    flags: CommandFlags,
}

/// The slice of `states` the popup draws for a thread whose model is `model`.
pub fn slash(states: &ScreenStates, model: Option<&str>) -> Slash {
    let tiers = codex_tui_core::screens::chat::facts::tier_commands(
        &states.chat.status,
        model,
        &states.select.catalog,
        &states.chat.service_tiers,
    );
    let mut flags = states.chat.flags;
    flags.service_tier_commands_enabled = !tiers.is_empty();
    Slash { tiers, flags }
}

/// The suggestions for `before`, the composer's text up to the caret.
pub fn suggestions(slash: &Slash, strings: &Strings, before: &str) -> Vec<Suggestion> {
    match trigger(before) {
        Some(trigger) if trigger.kind == TriggerKind::Slash => commands(slash, strings, before),
        _ => Vec::new(),
    }
}

fn commands(slash: &Slash, strings: &Strings, before: &str) -> Vec<Suggestion> {
    let mut popup = CommandPopup::new(CommandPopupFlags::from(slash.flags), slash.tiers.clone());
    popup.on_composer_text_change(before.to_string());
    popup
        .rows()
        .into_iter()
        .map(|row| {
            let name = format!("/{}", row.item.command());
            let description = row.description.unwrap_or_else(|| match &row.item {
                SlashCommandItem::Builtin(command) => (strings.description)(*command).to_string(),
                SlashCommandItem::ServiceTier(tier) => tier.description.clone(),
            });
            // a command a side conversation cannot run is listed after a search, muted and no
            // pick (`rows_from_matches`: `is_disabled`)
            if row.unavailable {
                let [before, after] = strings.command_reason;
                return Suggestion::new(name.clone())
                    .label([name.as_str(), strings.command_disabled].concat())
                    .detail(
                        [
                            description.as_str(),
                            before,
                            strings.command_unavailable,
                            after,
                        ]
                        .concat(),
                    )
                    .enabled(false);
            }
            Suggestion::new(name.clone())
                .label(name)
                .detail(description)
        })
        .collect()
}
