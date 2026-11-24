//! `toggle_voice` (F8): the voice strip.

use rtwk::Cx;

use codex_tui_core::commands::Target;
use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::CHAT_TOGGLE_VOICE;

use super::{KeyAction, Keyed};
use crate::pages::session::open::open;
use crate::pages::session::session_page::ChatState;

pub struct Voice;

impl KeyAction for Voice {
    const ID: ActionId = CHAT_TOGGLE_VOICE;

    fn run(cx: &mut Cx<'_, ChatState>, _keyed: &Keyed<'_>) {
        open(cx, Target::Voice);
    }
}
