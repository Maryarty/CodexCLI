//! `find_transcript` (F3): opens the transcript sheet the find prompt searches (the prompt is the
//! sheet's).

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_FIND_TRANSCRIPT;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct Find;

impl KeyAction for Find {
    const ID: ActionId = GLOBAL_FIND_TRANSCRIPT;

    fn run(_cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        keyed.ui.modes.open();
    }
}
