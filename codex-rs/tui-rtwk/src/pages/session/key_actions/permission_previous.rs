//! Move to the previous allowed permission mode of the current thread.

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::CHAT_PREVIOUS_PERMISSION_MODE;
use rtwk::Cx;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct PermissionPrevious;

impl KeyAction for PermissionPrevious {
    const ID: ActionId = CHAT_PREVIOUS_PERMISSION_MODE;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        super::permission_next::cycle(cx, keyed, false);
    }
}
