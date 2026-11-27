//! `open_transcript` (Ctrl+T): the transcript shows its details, or goes back to the plain one.

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_OPEN_TRANSCRIPT;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;
use crate::pages::session::transcript_pane::toggle_details;

pub struct OpenTranscript;

impl KeyAction for OpenTranscript {
    const ID: ActionId = GLOBAL_OPEN_TRANSCRIPT;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        toggle_details(cx, keyed.ui);
    }
}
