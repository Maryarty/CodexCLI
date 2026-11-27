//! The pager's close key returns the detailed transcript to compact mode.

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::PAGER_CLOSE_TRANSCRIPT;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;
use crate::pages::session::transcript_pane::{DetailAction, change_details};

pub struct CloseTranscript;

impl KeyAction for CloseTranscript {
    const ID: ActionId = PAGER_CLOSE_TRANSCRIPT;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        change_details(cx, keyed.ui, DetailAction::Close);
    }
}
