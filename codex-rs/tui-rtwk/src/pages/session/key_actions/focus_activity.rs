//! `focus_activity` (F4): the keys of a list walk the folds of the turn details
//! (`transcript::activity_focus`); the key again lets go. A selection ends where the focus begins
//! (`handle_disclosure_key`: `end_selection`); the fold in focus is drawn and scrolled to by the
//! transcript's pane.

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_FOCUS_ACTIVITY;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;
use crate::pages::session::transcript::activity_focus;

pub struct FocusActivity;

impl KeyAction for FocusActivity {
    const ID: ActionId = GLOBAL_FOCUS_ACTIVITY;

    fn run(_: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        // the details of the Ctrl+T transcript are all open: nothing to fold there
        if keyed.ui.modes.detailed() {
            return;
        }
        let activity = activity_focus::of(&keyed.ui.find);
        if !activity.focused() {
            keyed.ui.selection.controller().clear();
        }
        activity.begin();
    }
}
