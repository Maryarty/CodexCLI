//! The model's one-shots, told by the listener (`widgets/directives.rs`): each is carried out
//! here, once, whatever else the page is doing in the same flush.

use rtwk::provider::read;
use rtwk::{Cx, Rebuild};

use codex_tui_core::commands::Target;
use codex_tui_core::screens::{ScreenStates, ScreenThreadIntent, chat};
use codex_tui_core::threads::{Ran, ThreadModel};

use crate::app::view::Hs;
use crate::pages::session::actions::run_due;
use crate::pages::session::composer::clear_sent;
use crate::pages::session::live::{plan_mode, put_back, seed_history};
use crate::pages::session::open::open;
use crate::widgets::directives::Edge;

use super::{Chat, ChatState};

impl ChatState {
    pub(super) fn edge(&mut self, edge: Edge, w: &Chat, cx: &mut Cx<'_, ChatState>) -> Rebuild {
        match edge {
            // The composer clears when the model accepted (or queued) the message, not when it
            // was sent.
            Edge::Submitted => {
                clear_sent(&w.ui);
                self.sync_draft(w);
                Rebuild::Yes
            }
            // Messages the model gave back (an interrupt, a failed steer, the edit-queued key)
            // go into the composer.
            Edge::Restore(restore) => {
                let table = crate::pages::session::composer::strings::of(cx);
                let states = read::<ScreenStates, _>(cx);
                let draft = w.ui.draft.submission_with(
                    &w.ui.composer,
                    |n| table.image(n),
                    states.popovers.sources().mentions,
                );
                put_back(&w.ui, &restore.onto(draft), |n| table.image(n));
                self.sync_draft(w);
                Rebuild::Yes
            }
            // The user messages of a resumed thread's turns seed the composer's history
            // (`replay_backend_thread_turns`).
            Edge::Replayed => {
                let model = read::<ThreadModel, _>(cx);
                seed_history(&w.ui, model.replayed_submissions());
                Rebuild::No
            }
            // The prompt an edit of the backtrack put back (`restore_user_message_to_composer`).
            Edge::Draft(message) => {
                let table = crate::pages::session::composer::strings::of(cx);
                put_back(&w.ui, &message, |n| table.image(n));
                self.sync_draft(w);
                Rebuild::Yes
            }
            // A command the queue waited for: it runs through the dispatch a typed line does and
            // the thread hears what it opened (`submit_queued_slash_prompt`). A sheet it opened
            // keeps the queue until the page is in front again.
            Edge::Due(due) => {
                let hs = read::<Hs, _>(cx);
                let ran = run_due(&due, w, cx, &hs);
                self.resume |= matches!(&ran, Ran::Dispatched(d) if d.modal);
                let thread = read::<ThreadModel, _>(cx).id().clone();
                let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::QueueRan(ran));
                cx.emit(&hs.thread_screen, (thread, intent));
                Rebuild::Yes
            }
            Edge::Front => {
                if std::mem::take(&mut self.resume) {
                    let hs = read::<Hs, _>(cx);
                    let thread = read::<ThreadModel, _>(cx).id().clone();
                    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::QueueResume);
                    cx.emit(&hs.thread_screen, (thread, intent));
                }
                Rebuild::No
            }
            Edge::Plan => {
                plan_mode::ask(cx);
                Rebuild::No
            }
            // The thread was archived or deleted (`/archive`, `/delete`): the chat is empty again
            // and the command center opens over it (`return_to_agents_after_removal`).
            Edge::Removed => {
                open(cx, Target::Agents);
                Rebuild::No
            }
            // The fork ends with an error of the server (the copy opens in another thread, which
            // `did_update_widget` takes as its end too).
            Edge::ForkFailed => {
                if self.forking.take().is_some() {
                    return Rebuild::Yes;
                }
                Rebuild::No
            }
        }
    }
}
