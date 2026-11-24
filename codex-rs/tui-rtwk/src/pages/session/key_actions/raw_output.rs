//! `toggle_raw_output` (Alt+R, the same as `/raw`): the mode alone, where `/raw` says which one it
//! is now (`apply_raw_output_mode(.., notify: false)`).

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_TOGGLE_RAW_OUTPUT;
use codex_tui_core::screens::{ScreenIntent, ScreenStates, chat};

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct RawOutput;

impl KeyAction for RawOutput {
    const ID: ActionId = GLOBAL_TOGGLE_RAW_OUTPUT;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        let on = !read::<ScreenStates, _>(cx).chat.raw_output;
        cx.emit(
            &keyed.hs.screen,
            ScreenIntent::from(chat::Intent::RawOutput(on)),
        );
    }
}
