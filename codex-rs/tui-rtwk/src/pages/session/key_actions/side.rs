//! `toggle_side_conversation` (Ctrl+/): the side conversation. Without the kitty protocol Ctrl+/
//! is sent as the C0 byte 0x1f (rtwk reads Ctrl+_) and Ctrl+7 shares it (`keymap.rs:730-735`,
//! `2317`): the side conversation has those two as well (`fixed::SIDE_CONVERSATION_BYTES`).
//! The conversation itself is not built in the chat yet: the key says so.

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::app::SessionSlice;
use codex_tui_core::commands::{Notice, SlashCommand};
use codex_tui_core::keymap::actions::GLOBAL_TOGGLE_SIDE_CONVERSATION;
use codex_tui_core::keymap::{ActionId, fixed};
use codex_tui_core::screens::{ScreenIntent, chat};
use codex_tui_core::threads::Note;

use super::{KeyAction, Keyed};
use crate::pages::session::session_page::ChatState;

pub struct Side;

impl KeyAction for Side {
    const ID: ActionId = GLOBAL_TOGGLE_SIDE_CONVERSATION;
    const ALSO: &'static [ActionId] = &[fixed::SIDE_CONVERSATION_BYTES];

    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>) {
        let note = Note::from(Notice::NotAvailable(SlashCommand::Side));
        let thread = read::<SessionSlice, _>(cx).active.clone();
        cx.emit(
            &keyed.hs.screen,
            ScreenIntent::from(chat::Intent::Note { thread, note }),
        );
    }
}
