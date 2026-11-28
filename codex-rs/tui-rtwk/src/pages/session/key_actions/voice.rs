//! Asks the app to control its live voice call, using this thread for a new call.

use rtwk::Cx;

use codex_tui_core::app::Intent;
use codex_tui_core::commands::VoiceControl;
use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::CHAT_TOGGLE_VOICE;
use codex_tui_core::threads::ThreadModel;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct Voice;

impl KeyAction for Voice {
    const ID: ActionId = CHAT_TOGGLE_VOICE;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        control(cx, keyed, VoiceControl::Toggle);
    }
}

pub(super) fn control(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>, action: VoiceControl) {
    let thread = rtwk::provider::read::<ThreadModel, _>(cx);
    cx.emit(
        &keyed.hs.intent,
        Intent::ControlVoice {
            thread: thread.id().clone(),
            control: action,
        },
    );
}
