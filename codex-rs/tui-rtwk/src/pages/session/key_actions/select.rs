//! `select_transcript` (`leader [` by default): select mode begins on the transcript the reader
//! sees (`begin_select_mode`): a caret on its last line that the keys of the mode move
//! (`transcript::select`); the caret is placed and shown by the transcript's pane.

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_SELECT_TRANSCRIPT;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;
use crate::pages::session::transcript::select as caret;
use crate::widgets::thread_ui::Input;

pub struct Select;

impl KeyAction for Select {
    const ID: ActionId = GLOBAL_SELECT_TRANSCRIPT;

    fn run(_: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        // Find's own keys hold while it is open
        if keyed.ui.find.active() {
            return;
        }
        caret::of(&keyed.ui.find).begin(&keyed.ui.follow);
        keyed.ui.modes.hold(Input::Select);
    }
}
