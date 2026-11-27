//! Esc, the page's ladder of what it ends first: the shortcut overlay, a provider lock, the
//! read-only notice, the detailed transcript's backtrack, the reader's way back, a running
//! turn, a page over another, the backtrack's priming.

use rtwk::Cx;
use rtwk::Rebuild;
use rtwk::navigation::Navigator;
use rtwk::provider::read;

use codex_tui_core::commands::Target;
use codex_tui_core::keymap::{
    actions::{CHAT_INTERRUPT_TURN, PAGER_CLOSE_TRANSCRIPT},
    fixed,
};
use codex_tui_core::screens::backtrack::{EscStep, Targets};
use codex_tui_core::screens::{ScreenThreadIntent, chat};
use codex_tui_core::threads::ThreadModel;

use crate::app::view::Hs;
use crate::pages::session::open::open;
use crate::pages::session::transcript_pane::{
    DetailAction, cancel_browsing, change_details, remember_browsing,
};

use super::{Chat, ChatState};

impl ChatState {
    /// `Some` is the update's answer; `None` goes on to the comparison of what was shown.
    pub(super) fn escape(
        &mut self,
        w: &Chat,
        cx: &mut Cx<'_, ChatState>,
        hs: &Hs,
    ) -> Option<Rebuild> {
        let modes = &w.ui.modes;
        if self.shortcuts {
            self.shortcuts = false;
            return Some(Rebuild::Yes);
        }
        // Esc cancels the confirmed startup draft: the text stays (`cancel_startup_submission`)
        if self.startup_confirm.take().is_some() {
            return Some(Rebuild::Yes);
        }
        // the editor of the model's questions goes back to the composer
        if self.close_editor() {
            return Some(Rebuild::Yes);
        }
        let thread = read::<ThreadModel, _>(cx);
        // the chat is locked for its model provider: Esc opens the chats
        // to resume, whatever the reader shows (`OpenResumePicker`)
        if thread.provider_error().is_some() {
            modes.close();
            open(cx, Target::Resume);
            return Some(Rebuild::Yes);
        }
        // another process writes the thread: Esc leaves for the
        // command center, whatever the reader shows
        // (`ExternalWriter` arm of `handle_tui_event`)
        if thread.read_only() {
            modes.close();
            open(cx, Target::Agents);
            return Some(Rebuild::Yes);
        }
        if modes.browsing() {
            cancel_browsing(&w.ui);
            return Some(Rebuild::Yes);
        }
        // the detailed transcript with an empty composer: Esc begins
        // browsing the prompts, and with none to browse the transcript
        // closes and the chat says so (`begin_overlay_backtrack_preview`)
        if modes.detailed() && w.ui.composer.text().is_empty() {
            let targets = Targets::of(thread.transcript(), thread.can_load_history());
            remember_browsing(cx, &w.ui, false);
            if modes.esc(Some(thread.id()), targets) == EscStep::NoPrevious {
                modes.close();
                let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::NoPreviousMessage);
                cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
            }
            return Some(Rebuild::Yes);
        }
        if super::input::may_close_transcript(&w.ui)
            && crate::keymap::press_for(fixed::ESCAPE).is_some_and(|esc| {
                let keymap = crate::keymap::keymap(cx);
                crate::keymap::pressed(&keymap, PAGER_CLOSE_TRANSCRIPT, &esc)
            })
        {
            change_details(cx, &w.ui, DetailAction::Close);
            return Some(Rebuild::No);
        }
        // Esc is the reader's way back before it is the chat's
        // (`handle_scroll_key`); on an empty composer it also primes
        // the backtrack, below.
        let empty = w.ui.composer.text().is_empty();
        let editing = thread.screen_slots().backtrack.editing();
        let away = w.ui.follow.away();
        if !thread.read_only() && away && (!empty || thread.can_interrupt() || editing) {
            w.ui.follow.jump_to_latest();
            if !empty || thread.can_interrupt() {
                return Some(Rebuild::No);
            }
        }
        if thread.can_interrupt() {
            let keymap = crate::keymap::keymap(cx);
            if crate::keymap::press_for(fixed::ESCAPE)
                .is_some_and(|esc| crate::keymap::pressed(&keymap, CHAT_INTERRUPT_TURN, &esc))
            {
                return Some(interrupt(&thread, cx, hs));
            }
            return Some(Rebuild::No);
        }
        // The chat as a page over another (a debug entry) goes back.
        if Navigator::maybe_pop(cx) {
            return Some(Rebuild::No);
        }
        // the first Esc primes, the second opens the detailed transcript
        // with the newest prompt marked (`handle_backtrack_esc_key`); while
        // an edit is under way no other starts
        if !editing {
            let targets = Targets::of(thread.transcript(), thread.can_load_history());
            match modes.chat_esc(Some(thread.id()), empty, targets) {
                EscStep::Primed { .. } => remember_browsing(cx, &w.ui, away),
                EscStep::NoPrevious => {
                    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::NoPreviousMessage);
                    cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
                }
                _ => {}
            }
        }
        None
    }
}

/// Interrupting with steers waiting sends them together. The core also handles a waiting
/// rejected steer in a review turn.
pub(super) fn interrupt(thread: &ThreadModel, cx: &mut Cx<'_, ChatState>, hs: &Hs) -> Rebuild {
    if !thread.can_interrupt() {
        return Rebuild::No;
    }
    let waiting = !thread.pending_steers().is_empty()
        || (thread.screen_slots().chat.review_mode() && !thread.rejected_steers().is_empty());
    if waiting {
        let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::InterruptForSteers);
        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
    } else {
        cx.emit(&hs.interrupt, thread.id().clone());
    }
    Rebuild::No
}
