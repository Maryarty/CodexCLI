//! `copy` (Ctrl+O; oracle `chatwidget/interaction.rs`: `copy_last_response_binding` →
//! `prepare_last_response_copy`): the last response goes to the clipboard as Markdown. What it
//! copies and what is said of it are the copy slice's; the key says its verb.

use rtwk::Cx;

use codex_tui_core::features::copy::Intent;
use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::GLOBAL_COPY;

use super::{KeyAction, Keyed};
use crate::features::verb_by;
use crate::pages::session::session_page::ChatState;

pub struct Copy;

impl KeyAction for Copy {
    const ID: ActionId = GLOBAL_COPY;

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        verb_by(cx, keyed.hs, Intent::Last);
    }
}
