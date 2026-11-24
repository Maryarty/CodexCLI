//! `open_warnings` (F2): the warnings viewer over the chat.

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_OPEN_WARNINGS;

use super::{KeyAction, Keyed};
use crate::pages::session::open::open_warnings;
use crate::pages::session::session_page::ChatState;

pub struct Warnings;

impl KeyAction for Warnings {
    const ID: ActionId = GLOBAL_OPEN_WARNINGS;

    fn run(cx: &mut Cx<'_, ChatState>, _keyed: &Keyed<'_>) {
        open_warnings(cx);
    }
}
