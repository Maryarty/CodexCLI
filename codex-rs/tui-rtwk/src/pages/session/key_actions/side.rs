//! `toggle_side_conversation` (Ctrl+/): the side conversation. Without the kitty protocol Ctrl+/
//! is sent as the C0 byte 0x1f (rtwk reads Ctrl+_) and Ctrl+7 shares it (`keymap.rs:730-735`,
//! `2317`): the side conversation has those two as well (`fixed::SIDE_CONVERSATION_BYTES`).
//! The key shows the counterpart of the thread in use (a side conversation's parent, a parent's
//! side conversation) and does nothing when there is none (`toggle_side_conversation`); Ctrl+C and
//! Ctrl+D over an empty composer return from a side conversation and drop it
//! (`maybe_return_from_side`, the verb `Intent::Discard`, which the feature carries out once the
//! parent is the thread shown).

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::side::Intent;
use codex_tui_core::keymap::actions::GLOBAL_TOGGLE_SIDE_CONVERSATION;
use codex_tui_core::keymap::{ActionId, fixed};
use codex_tui_core::threads::ThreadModel;

use super::{KeyAction, Keyed};
use crate::app::view::Hs;
use crate::features::verb_by;
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

/// Ctrl+C and Ctrl+D with nothing typed, over a side conversation: the parent is shown and the
/// side conversation is dropped. Whether the key was taken.
pub fn return_from_side(cx: &mut Cx<'_, ChatState>, hs: &Hs, composer_empty: bool) -> bool {
    if !composer_empty {
        return false;
    }
    let child = read::<ThreadModel, _>(cx).id().clone();
    let parent = read::<FeatureStates, _>(cx)
        .side
        .link(&child)
        .map(|link| link.parent.clone());
    let Some(parent) = parent else {
        return false;
    };
    cx.emit(&hs.open_thread, parent);
    verb_by(cx, hs, Intent::Discard { child });
    true
}
