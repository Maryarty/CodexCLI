//! What the composer sends: Enter, Tab's queue, a command picked in a popup and the
//! edit-queued key. The line itself runs through `actions::run_line`.

use std::rc::Rc;

use rtwk::Cx;
use rtwk::provider::read;

use codex_tui_core::screens::{ScreenThreadIntent, chat};
use codex_tui_core::threads::ThreadModel;

use crate::app::view::Hs;
use crate::pages::session::actions::run_line;
use crate::pages::session::composer::queue::Delivery;
use crate::pages::session::composer::submission;

use super::{Chat, ChatState};

impl ChatState {
    /// Enter in the composer.
    pub(super) fn submit(
        &mut self,
        text: &str,
        w: &Chat,
        cx: &mut Cx<'_, ChatState>,
        hs: &Hs,
        primed: bool,
    ) {
        // The same Enter may have run it through a popup's pick. A
        // draft in shell mode has the `!` as its mode, not as text: a
        // bare `!` is the help line (`submit_shell_command`). Remote
        // images alone are a message of their own.
        self.composer_sends();
        // Enter on the startup draft confirms it: it is sent when the session is ready
        if self.confirm_startup(text, w, cx) {
            return;
        }
        if !w.ui.composer.text().is_empty() || w.ui.draft.shell() || w.ui.draft.remote_count() > 0 {
            run_line(text, w, cx, hs, Delivery::Submit);
            // a command that took the line cleared the composer
            self.sync_draft(w);
        } else if !primed {
            // an Enter on the empty composer is the reader's way
            // back (`empty_enter_returns_to_latest`)
            w.ui.follow.jump_to_latest();
        }
    }

    /// Tab or the queue key (`composer.queue`): queues behind a running turn and submits
    /// otherwise.
    pub(super) fn queue_draft(&mut self, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs) {
        // a compaction asked for holds the thread as a running turn does: a message waits then
        // (`is_user_turn_pending_or_running`); a server that still starts waits for nothing
        let thread = read::<ThreadModel, _>(cx);
        let running = thread.busy();
        // the queue key leaves a shell command alone while nothing runs
        // (`queue_keys` in `handle_key_event_without_popup`)
        if !thread.task_running() && w.ui.draft.shell() {
            return;
        }
        self.composer_sends();
        self.startup_confirm = None;
        let draft = submission(&w.ui);
        if !draft.is_empty() || w.ui.draft.remote_count() > 0 {
            let send = if running {
                Delivery::Queue
            } else {
                Delivery::Submit
            };
            run_line(&draft, w, cx, hs, send);
            self.sync_draft(w);
        }
    }

    /// `Autocomplete::on_selected`: a command picked in the popup
    /// runs as the pick names it (the command clears the composer
    /// itself); what `$` and `@` put in is text and stays.
    pub(super) fn picked(&mut self, text: &Rc<str>, w: &Chat, cx: &mut Cx<'_, ChatState>, hs: &Hs) {
        if text.starts_with('/') {
            // `/clear` picked in the popup is not kept for recall
            // (`stage_selected_slash_command_history`): it clears the line first, so the
            // command finds nothing to record
            if &**text == "/clear" {
                w.ui.composer.clear();
            }
            run_line(text, w, cx, hs, Delivery::Submit);
            self.sync_draft(w);
        }
    }
}

/// The edit-queued key: the newest queued message goes back to the composer
/// (`has_queued_follow_up_messages`: something waits behind the turn).
pub(super) fn edit_queued(cx: &mut Cx<'_, ChatState>, hs: &Hs) {
    let thread = read::<ThreadModel, _>(cx);
    if !thread.queue().is_empty() || !thread.rejected_steers().is_empty() {
        let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::EditQueued);
        cx.emit(&hs.thread_screen, (thread.id().clone(), intent));
    }
}
