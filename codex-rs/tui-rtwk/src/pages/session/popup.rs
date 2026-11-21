//! What the composer's popups list for the text before the caret: the
//! commands after a `/` at the start, the skills after `$`, the files and
//! plugins after `@`. The data and the filtering are core's (`commands`,
//! `popovers`); this turns rows into the suggestions the field shows and
//! says what picking one puts in.

use rtwk::editing::Suggestion;

use codex_tui_core::commands::{CommandPopup, CommandPopupFlags, SlashCommandItem};
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::composer::{TriggerKind, trigger};
use codex_tui_core::screens::popovers::{Candidate, CandidateKind, Sigil};

use super::strings::Strings;

/// The most rows a popup shows at once (`MAX_POPUP_ROWS`).
pub const ROWS: usize = codex_tui_core::commands::MAX_POPUP_ROWS;

/// The suggestions for `before`, the composer's text up to the caret.
pub fn suggestions(states: &ScreenStates, strings: &Strings, before: &str) -> Vec<Suggestion> {
    let Some(trigger) = trigger(before) else {
        return Vec::new();
    };
    let head = &before[..trigger.start];
    match trigger.kind {
        TriggerKind::Slash => commands(states, strings, before),
        TriggerKind::Skill => states
            .popovers
            .candidates(Sigil::Dollar, &trigger.query)
            .into_iter()
            .map(|c| mention(strings, head, '$', &c))
            .collect(),
        TriggerKind::Mention => states
            .popovers
            .candidates(Sigil::At, &trigger.query)
            .into_iter()
            .map(|c| mention(strings, head, '@', &c))
            .collect(),
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

fn mention(strings: &Strings, head: &str, sigil: char, candidate: &Candidate) -> Suggestion {
    let detail = match candidate.kind {
        CandidateKind::File => String::new(),
        CandidateKind::Skill => format!("{} {}", strings.tag_skill, candidate.description),
        CandidateKind::App => format!("{} {}", strings.tag_app, candidate.description),
        CandidateKind::Plugin => format!("{} {}", candidate.description, strings.tag_plugin),
    };
    // a skill is a `$` mention wherever it was found (`@` lists skills too)
    let sigil = if candidate.kind == CandidateKind::Skill {
        '$'
    } else {
        sigil
    };
    let suggestion = Suggestion::new(format!("{head}{sigil}{} ", candidate.insert))
        .label(candidate.label.clone());
    if detail.is_empty() {
        suggestion
    } else {
        suggestion.detail(detail)
    }
}
