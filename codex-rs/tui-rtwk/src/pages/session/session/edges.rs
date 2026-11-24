//! The model's one-shots, told by the listener (`widgets/directives.rs`): each is carried out
//! here, once, whatever else the page is doing in the same flush.

use rtwk::provider::read;
use rtwk::{Cx, Rebuild};

use codex_tui_core::commands::Target;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::threads::ThreadModel;

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
