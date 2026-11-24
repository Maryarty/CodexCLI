//! Ctrl+C and Ctrl+D before the page's own meaning of them (interrupt, quit): a feature that owns
//! the thread shown takes them over an empty composer (the return from a side conversation,
//! `maybe_return_from_side`) and says its verb; the page names no feature.

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::app::Intent;
use codex_tui_core::threads::ThreadModel;

use crate::app::view::Hs;
use crate::features::{self, PageKey};
use crate::pages::session::composer::submission;

use super::{Chat, ChatState};

/// A feature said its verb for `key`: the page leaves the key to it.
pub(super) fn taken(key: PageKey, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs) -> bool {
    let thread = read::<ThreadModel, _>(cx).id().clone();
    match features::run_key(key, &thread, &submission(&w.ui), cx) {
        Some(intent) => {
            cx.emit(&hs.intent, Intent::Feature(intent));
            true
        }
        None => false,
    }
}

/// Ctrl+C: a feature that owns the thread takes it (the return from a side conversation), else it
/// interrupts the turn, else it quits (`on_ctrl_c`).
pub(super) fn ctrl_c(w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    if taken(PageKey::Interrupt, w, cx, hs) {
        return;
    }
    let thread = read::<ThreadModel, _>(cx);
    if thread.can_interrupt() {
        cx.emit(&hs.interrupt, thread.id().clone());
    } else {
        cx.emit(&hs.quit, ());
    }
}
