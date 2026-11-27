//! Move to the next allowed permission mode of the thread that received the key.

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::CHAT_NEXT_PERMISSION_MODE;
use codex_tui_core::screens::ScreenThreadIntent;
use codex_tui_core::threads::ThreadModel;
use rtwk::Cx;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct PermissionNext;

impl KeyAction for PermissionNext {
    const ID: ActionId = CHAT_NEXT_PERMISSION_MODE;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        cycle(cx, keyed, true);
    }
}

pub(super) fn cycle(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>, forward: bool) {
    let thread = rtwk::provider::read::<ThreadModel, _>(cx).id().clone();
    cx.emit(
        &keyed.hs.thread_screen,
        (thread, ScreenThreadIntent::CyclePermissions { forward }),
    );
}
