//! `find_transcript` (F3): Find opens on the transcript the reader sees (`begin_search`), whatever
//! mode the transcript is in; the footer's query is the field of the page's footer slot
//! (`transcript::find_footer::slot`).

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_FIND_TRANSCRIPT;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;
use crate::pages::session::transcript::{activity_focus, find};

pub struct Find;

impl KeyAction for Find {
    const ID: ActionId = GLOBAL_FIND_TRANSCRIPT;

    fn run(_: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        // a search lets go of the stop in focus (`begin_search`)
        let finder = find::finder(keyed.ui);
        activity_focus::of(&finder).leave();
        finder.begin(&keyed.ui.follow, &keyed.ui.selection);
    }
}
