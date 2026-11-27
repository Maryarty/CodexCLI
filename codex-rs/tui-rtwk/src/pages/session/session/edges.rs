//! The model's one-shots, told by the listener (`widgets/directives.rs`): each is carried out
//! here, once, whatever else the page is doing in the same flush.

use rtwk::provider::read;
use rtwk::{Cx, Rebuild};

use codex_tui_core::commands::Target;
use codex_tui_core::question_reply::Intent as QuestionIntent;
use codex_tui_core::screens::{ScreenStates, ScreenThreadIntent, chat};
use codex_tui_core::threads::{Ran, Restore, ThreadModel};

use crate::app::view::Hs;
use crate::pages::session::composer::queue::run_due;
use crate::pages::session::composer::{burst, clear_sent};
use crate::pages::session::live::{plan_mode, put_back, recalled_entry, seed_history};
use crate::pages::session::open::open;
use crate::widgets::directives::Edge;

use super::{Chat, ChatState};

impl ChatState {
    /// What the page keeps of the draft (the footer and the Tab binding read it): empty (shell
    /// mode's `!` is content), in shell mode, in history search.
    pub(super) fn sync_draft(&mut self, w: &Chat) {
        self.search_rev = w.ui.draft.search_rev();
        self.draft_shell = w.ui.draft.shell();
        self.draft_vim = w.ui.draft.vim_label();
        self.draft_empty = w.ui.composer.text().is_empty() && !self.draft_shell;
    }

    fn restore_draft(
        restore: &Restore,
        rejected_start: bool,
        w: &Chat,
        cx: &mut Cx<'_, ChatState>,
    ) {
        let table = crate::pages::session::composer::strings::of(cx);
        let states = read::<ScreenStates, _>(cx);
        // A reverse search is showing a preview. Edit the stored draft that Esc brings back.
        w.ui.draft.edit_stored(&w.ui.composer, || {
            burst::flush_pending(
                &w.ui.draft,
                &w.ui.composer,
                |chars| table.pasted(chars),
                |n| table.image(n),
            );
            if rejected_start {
                w.ui.draft.prepend_rejected_message(
                    &w.ui.composer,
                    recalled_entry(&restore.merged()),
                    |n| table.image(n),
                );
                return;
            }
            let draft = w.ui.draft.submission_with(
                &w.ui.composer,
                |n| table.image(n),
                states.popovers.sources().mentions,
            );
            put_back(&w.ui, &restore.onto(draft), restore.replaces(), |n| {
                table.image(n)
            });
        });
    }

    pub(super) fn edge(&mut self, edge: Edge, w: &Chat, cx: &mut Cx<'_, ChatState>) -> Rebuild {
        match edge {
            // The composer clears when the model accepted (or queued) the message, not when it
            // was sent.
            Edge::Submitted => {
                // an answer to a question is no send of the composer: its draft stays
                if !self.answer_sent() {
                    clear_sent(&w.ui);
                    self.sync_draft(w);
                    if w.ui.follow.away() || w.ui.follow.beginning() {
                        w.ui.follow.jump_to_latest();
                    }
                }
                Rebuild::Yes
            }
            // Messages the model gave back (an interrupt, a failed steer, the edit-queued key)
            // go into the composer.
            Edge::Restore {
                thread,
                restore,
                rejected_start,
            } => {
                if read::<ThreadModel, _>(cx).id() != &thread
                    || rejected_start.is_some_and(|seq| !w.ui.draft.accept_rejected_restore(seq))
                {
                    return Rebuild::No;
                }
                Self::restore_draft(&restore, rejected_start.is_some(), w, cx);
                self.sync_draft(w);
                Rebuild::Yes
            }
            Edge::TurnEnded(thread, end) => {
                let model = read::<ThreadModel, _>(cx);
                if model.id() != &thread || !self.asking.ended.take(&model, &end.turn) {
                    return Rebuild::No;
                }
                if let Some(restore) = &end.restore {
                    Self::restore_draft(restore, false, w, cx);
                }
                let restored = end.restore.is_some();
                let recovered = self.recover_questions(&thread, &end, &w.ui, cx);
                crate::features::verb(
                    cx,
                    QuestionIntent::TurnEnded {
                        thread,
                        turn: end.turn,
                    },
                );
                if restored || recovered {
                    self.sync_draft(w);
                    Rebuild::Yes
                } else {
                    Rebuild::No
                }
            }
            // The user messages of a resumed thread's turns seed the composer's history
            // (`replay_backend_thread_turns`).
            Edge::Replayed => {
                let model = read::<ThreadModel, _>(cx);
                seed_history(&w.ui, model.replayed_submissions());
                Rebuild::No
            }
            // The prompt of the command line went out: Up recalls it (`record_local_submission`).
            Edge::Recorded(message) => {
                w.ui.draft.replayed(recalled_entry(&message));
                Rebuild::No
            }
            // The prompt an edit of the backtrack put back (`restore_user_message_to_composer`).
            Edge::Draft(message) => {
                let table = crate::pages::session::composer::strings::of(cx);
                w.ui.draft.edit_stored(&w.ui.composer, || {
                    put_back(&w.ui, &message, true, |n| table.image(n))
                });
                self.sync_draft(w);
                Rebuild::Yes
            }
            // A command the queue waited for: it runs through the dispatch a typed line does and
            // the thread receives the report for that operation (`submit_queued_slash_prompt`).
            Edge::Due(due) => {
                if read::<ThreadModel, _>(cx).due_command() != Some(&due) {
                    return Rebuild::No;
                }
                let hs = read::<Hs, _>(cx);
                if let Some(ran) = run_due(&due, w, cx, &hs) {
                    self.resume |= matches!(&ran, Ran::Dispatched(d) if d.modal);
                    let thread = due.operation().thread().clone();
                    let intent =
                        ScreenThreadIntent::Chat(chat::ThreadIntent::QueueEvent(due.report(ran)));
                    cx.emit(&hs.thread_screen, (thread, intent));
                }
                Rebuild::Yes
            }
            Edge::Front => {
                if std::mem::take(&mut self.resume) {
                    let hs = read::<Hs, _>(cx);
                    let thread = read::<ThreadModel, _>(cx).id().clone();
                    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::QueueResume);
                    cx.emit(&hs.thread_screen, (thread, intent));
                }
                // a confirmed startup draft that waited for the page to be in front
                self.startup_ready(w, cx)
            }
            // The session is configured and its thread takes input: the draft Enter confirmed
            // before goes out, once.
            Edge::Ready => self.startup_ready(w, cx),
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
