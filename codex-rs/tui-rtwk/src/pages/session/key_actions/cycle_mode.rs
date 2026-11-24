//! `cycle_collaboration_mode` (Shift+Tab, hardwired): Plan and Default in turn. The thread's own
//! switch; the model refuses it while a turn runs.

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::keymap::{ActionId, fixed};
use codex_tui_core::screens::{ScreenThreadIntent, chat};
use codex_tui_core::threads::ThreadModel;

use super::{KeyAction, Keyed};
use crate::pages::session::actions::refuses_parent_owned;
use crate::pages::session::session_page::ChatState;

pub struct CycleMode;

impl KeyAction for CycleMode {
    const ID: ActionId = fixed::CYCLE_COLLABORATION_MODE;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        let thread = read::<ThreadModel, _>(cx);
        if !thread.read_only() && !refuses_parent_owned(cx, keyed.hs) {
            let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::CycleMode);
            cx.emit(&keyed.hs.thread_screen, (thread.id().clone(), intent));
        }
    }
}
