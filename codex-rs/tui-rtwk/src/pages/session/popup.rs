//! What the slash popup lists for the text before the caret: the commands
//! after a `/` at the start. The data and the filtering are core's
//! (`commands`); this turns rows into the suggestions the field shows. The
//! `$` and `@` popups are `popovers::mention`'s: they sit above the composer.

use rtwk::editing::Suggestion;

use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::commands::{
    CommandFlags, CommandPopup, CommandPopupFlags, ServiceTierCommand, SlashCommandItem,
};
use codex_tui_core::features::DaybreakSetting;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::composer::{TriggerKind, trigger};
use codex_tui_core::threads::ThreadModel;
use codex_tui_core::threads::daybreak::Row;

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
    /// What the `/daybreak` entry offers; none lists no entry.
    daybreak: Option<Row>,
}

/// What the thread says of Daybreak: the choice it holds and the provider it runs on
/// (`DaybreakSetting`), with the `[tui]` settings that gate the choice. The entry is derived at
/// the draw, not copied into the command flags, so it follows the thread's provider.
#[derive(Clone, Debug)]
pub struct Thread {
    saved: Option<bool>,
    provider: Option<String>,
    tui: TuiSettings,
}

impl Thread {
    pub fn of(thread: &ThreadModel, tui: &TuiSettings) -> Thread {
        Thread {
            saved: codex_tui_core::features::daybreak_of(thread.view()),
            provider: thread.screen_slots().chat.provider.clone(),
            tui: *tui,
        }
    }
}

/// The slice of `states` the popup draws for a thread whose model is `model`.
pub fn slash(states: &ScreenStates, model: Option<&str>, thread: &Thread) -> Slash {
    let tiers = codex_tui_core::screens::chat::facts::tier_commands(
        &states.chat.status,
        model,
        &states.select.catalog,
        &states.chat.service_tiers,
    );
    let mut flags = states.chat.flags;
    flags.service_tier_commands_enabled = !tiers.is_empty();
    let daybreak = DaybreakSetting::new(thread.saved, thread.provider.clone(), states, &thread.tui)
        .row(&states.chat.access);
    Slash {
        tiers,
        flags,
        daybreak,
    }
}

/// The suggestions for `before`, the composer's text up to the caret.
pub fn suggestions(slash: &Slash, strings: &Strings, before: &str) -> Vec<Suggestion> {
    match trigger(before) {
        Some(trigger) if trigger.kind == TriggerKind::Slash => commands(slash, strings, before),
        _ => Vec::new(),
    }
}

fn commands(slash: &Slash, strings: &Strings, before: &str) -> Vec<Suggestion> {
    let mut flags = slash.flags;
    flags.daybreak_command_description = slash.daybreak.map(crate::features::daybreak_row);
    let mut popup = CommandPopup::new(CommandPopupFlags::from(flags), slash.tiers.clone());
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
