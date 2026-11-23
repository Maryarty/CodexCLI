//! A list of the sheet's own: a question a screen asks of the user that no domain menu stands
//! behind (the plan-implementation question, the denials of `/auto-review`, the debug entries'
//! fixtures). In Flutter it is `showModalBottomSheet<T>` over a `ListView`: the list is the
//! sheet's constructor argument, the cursor, the toggles and the order of the rows are its
//! `State`, and the answer is what the route is popped with. Nothing of it lives in the model
//! (ADR-013: the rows, the cursor and the checks are drawn state); only the answer reaches the
//! app, as the verb it stands for (`Purpose`).
//! Oracle: `bottom_pane/list_selection_view.rs`.

use std::rc::Rc;

use rtwk::overlay::{DialogOptions, DialogResult, close_dialog, show_dialog};
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{AutoReview, Intent as AppIntent};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::chat::plan_mode::Answer;
use codex_tui_core::screens::select::Intent;

use crate::app::view::Hs;
use crate::widgets::picker::list::List;
use crate::widgets::picker::{Event, Picker};

/// What the answer of the list stands for.
#[derive(Clone, Debug, PartialEq)]
pub enum Purpose {
    /// "Implement this plan?": a row is an `Answer`.
    PlanImplementation,
    /// The denials of a thread: the row is one to approve for a retry.
    ApproveDenial(ThreadId),
    /// A debug fixture: the list is shown and nothing is asked of the app.
    Fixture,
}

/// The sheet over a list.
#[derive(Clone, Debug, PartialEq)]
pub struct ListSheet {
    pub list: Rc<List>,
    pub purpose: Purpose,
}

pub struct ListSheetState {
    /// The rows as the user left them: the toggles flipped, the order moved. Replaced when the
    /// opener gives the sheet another list.
    list: List,
}

impl StatefulWidget for ListSheet {
    type State = ListSheetState;

    fn create_state(&self) -> ListSheetState {
        ListSheetState {
            list: (*self.list).clone(),
        }
    }
}

impl_widget!(stateful eq ListSheet);

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

/// The verb an answer stands for, sent to the app.
fn answer(cx: &mut Cx<'_, ListSheetState>, purpose: &Purpose, id: &str) {
    let hs = read::<Hs, _>(cx);
    match purpose {
        Purpose::PlanImplementation => {
            if let Some(answer) = Answer::of_row(id) {
                cx.emit(&hs.screen, ScreenIntent::Select(Intent::PlanAnswer(answer)));
            }
        }
        Purpose::ApproveDenial(thread) => cx.emit(
            &hs.intent,
            AppIntent::AutoReview {
                thread: thread.clone(),
                what: AutoReview::Approve { id: id.to_owned() },
            },
        ),
        Purpose::Fixture => {}
    }
}

impl State for ListSheetState {
    type Widget = ListSheet;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &ListSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) => {
                let Some(item) = self.list.find(&id) else {
                    return Rebuild::No;
                };
                if item.is_disabled() {
                    return Rebuild::No;
                }
                if item.checkbox && item.toggle.is_some() {
                    self.list.flip(&id);
                    return Rebuild::Yes;
                }
                answer(cx, &w.purpose, &id);
                close_dialog::<_, ()>(cx, None);
            }
            Event::Secondary(id) => {
                if self.list.find(&id).is_some_and(|item| !item.is_disabled()) {
                    answer(cx, &w.purpose, &id);
                    close_dialog::<_, ()>(cx, None);
                }
            }
            Event::Toggle(id) => {
                self.list.flip(&id);
                return Rebuild::Yes;
            }
            Event::Reorder(id, down) => {
                self.list.reorder(&id, down);
                return Rebuild::Yes;
            }
            Event::Confirm => {
                if self.list.multi {
                    close_dialog::<_, ()>(cx, None);
                }
            }
            Event::Cancel => close_dialog::<_, ()>(cx, None),
            // Only the plugin browser asks for these (`Picker::extras`).
            Event::Tab(_) | Event::Hot(_) => return Rebuild::No,
        }
        Rebuild::No
    }

    fn build(&self, _: &ListSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        Picker::new(Rc::new(self.list.clone()), cx.map(Msg::Picked)).into()
    }
}

/// Shows `list` over the page that asks, as the bottom sheet the selection dialog is.
pub fn pick<S>(cx: &mut Cx<'_, S>, list: List, purpose: Purpose) -> DialogResult<()> {
    let list = Rc::new(list);
    show_dialog(
        cx,
        move |_| {
            ListSheet {
                list: list.clone(),
                purpose: purpose.clone(),
            }
            .into()
        },
        DialogOptions {
            use_root_navigator: false,
            ..crate::sheets::sheet_options()
        },
    )
}
