//! The `/feedback` flow as one sheet (ADR-013; oracle `bottom_pane/feedback_view.rs` and
//! `feedback_note_view.rs`): the category list, the consent list and the note, each the next
//! step of the same dialog. Which step it is, what was picked and the note typed so far are the
//! sheet's drawn state; only the submit crosses into the domain, as the verb `Intent::Submit`.
//! In Flutter it is one `showModalBottomSheet` whose content is swapped by a `PageView`.

use std::cell::Cell;
use std::rc::Rc;

use rtwk::editing::TextEditingController;
use rtwk::focus::FocusNode;
use rtwk::overlay::close_dialog;
use rtwk::provider::select;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::feedback::{Audience, Destination, Intent};
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::dialogs::Attachments;

use super::note::{self, NoteView};
use super::steps::{self, Step};
use crate::features::verb;
use crate::sheets::dialogs::strings;

/// The sheet: the step it opens at, a note it starts with (a debug entry's) and the thread's
/// last turn the report is tagged with.
#[derive(Clone, Debug, PartialEq)]
pub struct FeedbackSheet {
    pub start: Step,
    pub text: &'static str,
    pub turn_id: Option<String>,
}

pub struct SheetState {
    step: Step,
    text: TextEditingController,
    node: FocusNode,
    /// The first row of the disclosure shown, and what the last layout said of its room.
    scroll: i32,
    page: Rc<Cell<i32>>,
    most: Rc<Cell<i32>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Close,
    /// A category was picked (its row).
    Category(usize),
    /// The consent was answered.
    Consent(bool),
    /// The note changed: its rows may.
    Typed,
    /// Enter, or `Ctrl+S`: the report goes out.
    Submit(Destination),
    /// A page of the disclosure up or down.
    Scroll(i32),
}

impl StatefulWidget for FeedbackSheet {
    type State = SheetState;

    fn create_state(&self) -> SheetState {
        SheetState {
            step: self.start,
            text: TextEditingController::new(""),
            node: FocusNode::new(),
            scroll: 0,
            page: Rc::new(Cell::new(1)),
            most: Rc::new(Cell::new(0)),
        }
    }
}

impl_widget!(stateful eq FeedbackSheet);

impl SheetState {
    fn submit(&mut self, destination: Destination, w: &FeedbackSheet, cx: &mut Cx<'_, Self>) {
        let Step::Note {
            category,
            include_logs,
        } = self.step
        else {
            return;
        };
        let rollout = select::<ScreenStates, Option<String>, _>(cx, |states| {
            states.dialogs.attachments.rollout.clone()
        });
        verb(
            cx,
            Intent::Submit {
                category,
                note: self.text.value().text().to_string(),
                turn_id: w.turn_id.clone(),
                rollout,
                include_logs,
                destination,
            },
        );
        close_dialog::<_, ()>(cx, None);
    }
}

impl State for SheetState {
    type Widget = FeedbackSheet;
    type Msg = Msg;

    fn init_state(&mut self, w: &FeedbackSheet, _cx: &mut Cx<'_, Self>) {
        self.text.reset(w.text);
    }

    fn dispose(&mut self) {
        self.text.dispose();
    }

    fn update(&mut self, msg: Msg, w: &FeedbackSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Close => {
                close_dialog::<_, ()>(cx, None);
                Rebuild::No
            }
            Msg::Category(index) => {
                if let (Step::Category, Some(category)) = (self.step, steps::category(index)) {
                    self.step = Step::Consent(category);
                }
                Rebuild::Yes
            }
            Msg::Consent(include_logs) => {
                if let Step::Consent(category) = self.step {
                    self.step = Step::Note {
                        category,
                        include_logs,
                    };
                }
                Rebuild::Yes
            }
            Msg::Typed => Rebuild::Yes,
            Msg::Submit(destination) => {
                self.submit(destination, w, cx);
                Rebuild::No
            }
            Msg::Scroll(pages) => {
                self.scroll = (self.scroll + pages * self.page.get()).clamp(0, self.most.get());
                Rebuild::Yes
            }
        }
    }

    fn build(&self, _w: &FeedbackSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let (audience, can_save) = select::<FeatureStates, (Option<Audience>, bool), _>(cx, |f| {
            let state = &f.feedback;
            (
                state
                    .can_upload()
                    .then_some(state.environment.audience),
                state.can_save(),
            )
        });
        match self.step {
            Step::Note {
                category,
                include_logs: _,
            } => note::build(
                NoteView {
                    category,
                    audience,
                    can_save,
                    text: self.text.clone(),
                    node: self.node.clone(),
                    scroll: self.scroll,
                    page: self.page.clone(),
                    most: self.most.clone(),
                },
                cx,
            ),
            step => {
                let attachments = select::<ScreenStates, Attachments, _>(cx, |states| {
                    states.dialogs.attachments.clone()
                });
                steps::build(step, &attachments, strings::of(cx), cx).into()
            }
        }
    }
}
