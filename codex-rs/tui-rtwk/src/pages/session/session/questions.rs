//! The questions the model asked while it works (`chatwidget/questions.rs`): the editor that takes
//! the composer's place and footer while it is open, and what the user decides in it. The editor is
//! `sheets::approvals::questions`; which questions wait is the feature's state
//! (`core::features::async_questions`); whether the editor is open and the drafts it keeps while
//! it is closed are the page's.
//!
//! An answer is a message like any other (`UserMessage` of the reply envelope): it goes out by
//! the page's own send (the thread decides: a turn, a steer, the queue) and the composer's draft
//! stays where it was, so the `Submitted` edge it raises is told not to clear it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Key, Rebuild};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::question_reply::Intent;
use codex_tui_core::screens::{ScreenThreadIntent, chat};
use codex_tui_core::threads::{Queued, ThreadModel, UserMessage};

use crate::app::view::Hs;
use crate::sheets::approvals::questions::{AQuestion, AsyncQuestions, Kept, Outcome, Seed};

use super::{ChatMsg, ChatState, send};

/// What the page keeps of the editor.
#[derive(Default)]
pub(super) struct Asking {
    /// The editor stands where the composer does (the oracle's `expanded`).
    open: Cell<bool>,
    /// An answer went out: the next `Submitted` edge is its, not the composer's.
    answered: Cell<bool>,
    /// The drafts and the question on show, kept while the editor is closed.
    saved: Rc<RefCell<Kept>>,
}

/// The questions of `thread` that wait.
fn waiting(cx: &mut Cx<'_, ChatState>, thread: &ThreadId) -> Rc<Vec<AQuestion>> {
    let thread = thread.clone();
    select::<FeatureStates, Rc<Vec<AQuestion>>, _>(cx, move |features| {
        Rc::new(
            features
                .async_questions
                .of(&thread)
                .iter()
                .map(|q| AQuestion {
                    id: q.id.clone(),
                    title: q.title.clone(),
                    options: q.options.clone(),
                })
                .collect(),
        )
    })
}

/// How many questions of `thread` wait (the status area and the summary follow it).
pub(super) fn count(cx: &mut Cx<'_, ChatState>, thread: &ThreadId) -> usize {
    let thread = thread.clone();
    select::<FeatureStates, usize, _>(cx, move |features| {
        features.async_questions.of(&thread).len()
    })
}

impl ChatState {
    /// Whether the editor shows: it is open and a question waits.
    pub(super) fn editing(&self, asked: usize) -> bool {
        if asked == 0 {
            self.asking.open.set(false);
        }
        self.asking.open.get()
    }

    /// The editor in the composer's place (none: the composer stands).
    pub(super) fn editor(
        &self,
        cx: &mut Cx<'_, ChatState>,
        thread: &ThreadModel,
    ) -> Option<AnyWidget> {
        let questions = waiting(cx, thread.id());
        if !self.editing(questions.len()) {
            return None;
        }
        let editor = AsyncQuestions {
            questions,
            seed: Seed::default(),
            queued: !thread.queue().is_empty() || !thread.rejected_steers().is_empty(),
            options_only: false,
            fit: true,
            saved: Some(self.asking.saved.clone()),
            on_outcome: Some(cx.map(ChatMsg::Question)),
        };
        Some(AnyWidget::from(editor).with_key(Key::value("questions")))
    }

    /// The edit-queued key: it opens the editor over the questions, and else is the key that
    /// brings the newest queued message back.
    pub(super) fn edit_key(&mut self, cx: &mut Cx<'_, ChatState>, hs: &Hs) -> Rebuild {
        let thread = read::<ThreadModel, _>(cx);
        let asked = count(cx, thread.id());
        if asked > 0 && !self.asking.open.get() {
            self.asking.open.set(true);
            return Rebuild::Yes;
        }
        if !self.asking.open.get() {
            send::edit_queued(cx, hs);
        }
        Rebuild::No
    }

    /// Esc closes the editor before anything else (`question_esc_back`).
    pub(super) fn close_editor(&mut self) -> bool {
        self.asking.open.replace(false)
    }

    /// The next `Submitted` edge is an answer's: the composer keeps its draft.
    pub(super) fn answer_sent(&self) -> bool {
        self.asking.answered.replace(false)
    }

    /// A send of the composer's own comes before the edge it raises.
    pub(super) fn composer_sends(&self) {
        self.asking.answered.set(false);
    }

    /// What the user decided in the editor.
    pub(super) fn outcome(
        &mut self,
        outcome: Outcome,
        cx: &mut Cx<'_, ChatState>,
        hs: &Hs,
    ) -> Rebuild {
        let thread = read::<ThreadModel, _>(cx);
        match outcome {
            Outcome::Submit { id, reply, queue } => {
                let owned = thread.screen_slots().chat.parent_owned;
                let message = UserMessage::text(reply);
                if queue && !owned {
                    let intent = chat::ThreadIntent::Queue(Queued::of(message, false));
                    cx.emit(
                        &hs.thread_screen,
                        (thread.id().clone(), ScreenThreadIntent::Chat(intent)),
                    );
                } else {
                    cx.emit(&hs.submit, message);
                }
                if !owned {
                    self.asking.answered.set(true);
                    self.dismiss(thread.id(), id, cx);
                }
            }
            Outcome::Skip { id } => self.dismiss(thread.id(), id, cx),
            Outcome::Edge { forward: false } => {
                self.asking.open.set(false);
            }
            // past the last question the newest queued message comes back to the composer
            Outcome::Edge { forward: true } => {
                if !thread.queue().is_empty() || !thread.rejected_steers().is_empty() {
                    self.asking.open.set(false);
                    send::edit_queued(cx, hs);
                }
            }
            Outcome::Interrupt => {
                if thread.can_interrupt() {
                    cx.emit(&hs.interrupt, thread.id().clone());
                }
            }
        }
        Rebuild::Yes
    }

    fn dismiss(&self, thread: &ThreadId, id: String, cx: &mut Cx<'_, ChatState>) {
        let thread = thread.clone();
        crate::features::verb(cx, Intent::Resolve { thread, id });
    }
}
