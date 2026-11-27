//! Starts or stops the voice call belonging to the thread that handled the key.

use rtwk::Cx;

use codex_tui_core::commands::VoiceControl;
use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::CHAT_TOGGLE_VOICE;
use codex_tui_core::screens::ScreenThreadIntent;
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
    if let Some(intent) = thread.screen_slots().voice.control(action) {
        cx.emit(
            &keyed.hs.thread_screen,
            (thread.id().clone(), ScreenThreadIntent::Voice(intent)),
        );
    }
}
