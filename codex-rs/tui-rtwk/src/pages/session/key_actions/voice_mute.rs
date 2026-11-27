//! The live call's independently configurable microphone mute shortcut.

use rtwk::Cx;

use codex_tui_core::commands::VoiceControl;
use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::VOICE_TOGGLE_VOICE_MUTE;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct VoiceMute;

impl KeyAction for VoiceMute {
    const ID: ActionId = VOICE_TOGGLE_VOICE_MUTE;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        super::voice::control(cx, keyed, VoiceControl::Mute);
    }
}
