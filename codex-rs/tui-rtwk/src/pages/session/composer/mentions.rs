//! The mention popups' keys and their effects on the composer. The options
//! and the highlight are the draft's `Mentions` (its controllers); this is
//! the `Shortcuts`/`Actions` half: which key moves the highlight, switches
//! the `@` popup's tab, closes it or picks a row, and what a pick asks of the
//! app (the file search, the usage file, the task list).
//! Oracle: `chat_composer.rs` `handle_key_event_with_skill_popup` and
//! `handle_key_event_with_mentions_v2_popup`.

use rtwk::Cx;
use rtwk::editing::TextEditingController;
use rtwk::focus::KeyPress;
use rtwk::prelude::Mods;
use rtwk::provider::read;

use codex_tui_core::kit::mentions::session::{Active, Pick, PickAction};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, popovers};

use super::editing::KeyData;
use super::vim;
use super::{Composer, ComposerMsg, ComposerState};
use crate::keymap::fixed::{Fixed, FixedKey};
use crate::app::view::Hs;

/// Whether the popup takes `key`: Up and Down (Ctrl+P, Ctrl+N) move the
/// highlight, Left and Right switch the `@` popup's tab while the caret is
/// in the token, Esc closes it for this token, Tab and Enter pick the
/// highlighted row. A popup with no row at all lets Enter go on (a submit)
/// when it is the `@` popup. Any other key edits, and the popup follows.
pub(super) fn take_key(data: &KeyData, c: &TextEditingController, key: &KeyPress) -> bool {
    let mentions = data.draft.mentions();
    if !mentions.is_open() {
        return false;
    }
    let plain = key.mods == Mods::NONE;
    let ctrl = key.mods == Mods::CTRL;
    let unified = mentions.read(|session| matches!(session.active(), Active::At(_)));
    match (key.fixed(), key.ch) {
        (Fixed::Up, _) => mentions.step(false),
        (Fixed::Char, Some('p')) if ctrl => mentions.step(false),
        (Fixed::Down, _) => mentions.step(true),
        (Fixed::Char, Some('n')) if ctrl => mentions.step(true),
        (Fixed::Left | Fixed::Right, _) if plain => {
            if !data.draft.can_switch_search_mode(c) {
                return false;
            }
            mentions.switch_search_mode(key.fixed() == Fixed::Right);
        }
        (Fixed::Escape, _) => data
            .draft
            .dismiss_mentions(c, &data.popovers.sources()),
        (Fixed::Tab, _) => {
            if let Some(at) = mentions.cursor() {
                (data.tell)(ComposerMsg::MentionPick(at));
            }
        }
        (Fixed::Enter, _) if plain => match mentions.cursor() {
            Some(at) => (data.tell)(ComposerMsg::MentionPick(at)),
            // the unified popup closes and the Enter is a submit
            None if unified => return false,
            None => {}
        },
        _ => return false,
    }
    mentions.notify();
    true
}

/// Brings the popups in line with the draft (`sync_popups`): none in history
/// search, the Vim `/` query, while the history is walked, or in a bare or
/// plain-text field. The file search the
/// popup asks for goes to the app; the popup is drawn again.
pub(super) fn sync(w: &Composer, states: &ScreenStates, cx: &mut Cx<'_, ComposerState>) {
    let (draft, c) = (&w.ui.draft, &w.ui.composer);
    let closes = w.bare
        || w.plain.is_some()
        || draft.searching()
        || vim::mode(draft).searching
        || draft.history_browsing(c, &states.popovers.history);
    let synced = if closes {
        draft.mentions().close()
    } else {
        draft.sync_mentions(c, &states.popovers.sources())
    };
    let asks = synced
        .asks
        .into_iter()
        .map(popovers::Intent::SearchFiles)
        .collect();
    emit(cx, asks);
    if synced.changed {
        draft.mentions().notify();
    }
}

/// The row `at` was picked (Enter, Tab or a click): what it stands for goes
/// in place of the token; the usage file keeps a skill's path and a task
/// spends the list the search found. Whether the draft changed.
pub(super) fn pick(
    w: &Composer,
    at: usize,
    image: impl Fn(usize) -> String,
    states: &ScreenStates,
    cx: &mut Cx<'_, ComposerState>,
) -> bool {
    let src = states.popovers.sources();
    let (draft, c) = (&w.ui.draft, &w.ui.composer);
    let Some(pick) = draft.mention_pick(c, at, &src) else {
        return false;
    };
    let skill = element_path(&pick).is_some_and(|path| src.is_skill(path));
    draft.apply_pick(c, &pick, image, skill);
    let mut intents = Vec::new();
    if let Some(path) = &pick.usage {
        intents.push(popovers::Intent::RecordMention(path.clone()));
    }
    if pick.is_task() {
        intents.push(popovers::Intent::SpendTasks);
    }
    emit(cx, intents);
    draft.mentions().notify();
    true
}

/// The path an element pick binds.
fn element_path(pick: &Pick) -> Option<&str> {
    match &pick.action {
        PickAction::Element {
            binding: Some(binding),
            ..
        } => Some(&binding.path),
        PickAction::Element { binding: None, .. }
        | PickAction::Path(_)
        | PickAction::Image { .. } => None,
    }
}

/// `intents` for the popovers' screen: the app runs what they ask.
fn emit(cx: &mut Cx<'_, ComposerState>, intents: Vec<popovers::Intent>) {
    if intents.is_empty() {
        return;
    }
    let hs = read::<Hs, _>(cx);
    for intent in intents {
        cx.emit(&hs.screen, ScreenIntent::from(intent));
    }
}
