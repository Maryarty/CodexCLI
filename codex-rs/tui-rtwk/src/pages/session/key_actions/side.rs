//! `toggle_side_conversation` (Ctrl+/): the side conversation. Without the kitty protocol Ctrl+/
//! is sent as the C0 byte 0x1f (rtwk reads Ctrl+_) and Ctrl+7 shares it (`keymap.rs:730-735`,
//! `2317`): the side conversation has those two as well (`fixed::SIDE_CONVERSATION_BYTES`).
//! The key shows the counterpart of the thread in use (a side conversation's parent, a parent's
//! side conversation) and does nothing when there is none (`toggle_side_conversation`); the return
//! from a side conversation with Ctrl+C or Ctrl+D (`maybe_return_from_side`) is a Need: it says
//! the feature's verb `Intent::Discard`, which no page may name (`features_are_named_only_through_the_list`).

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::features::FeatureStates;
use codex_tui_core::keymap::actions::GLOBAL_TOGGLE_SIDE_CONVERSATION;
use codex_tui_core::keymap::{ActionId, fixed};
use codex_tui_core::threads::ThreadModel;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct Side;

impl KeyAction for Side {
    const ID: ActionId = GLOBAL_TOGGLE_SIDE_CONVERSATION;
    const ALSO: &'static [ActionId] = &[fixed::SIDE_CONVERSATION_BYTES];

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        let thread = read::<ThreadModel, _>(cx).id().clone();
        let counterpart = read::<FeatureStates, _>(cx).side.counterpart(&thread).cloned();
        if let Some(target) = counterpart {
            cx.emit(&keyed.hs.open_thread, target);
        }
    }
}
