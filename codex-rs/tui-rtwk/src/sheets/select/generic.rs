//! A list of the sheet's own: a question a screen asks of the user that no domain menu stands
//! behind (the plan-implementation question, the denials of `/auto-review`, the debug entries'
//! fixtures). In Flutter it is `showModalBottomSheet<T>` over a `ListView`: the list is the
//! sheet's constructor argument, the cursor, the toggles and the order of the rows are its
//! `State`, and the answer is what the route is popped with. Nothing of it lives in the model
//! (ADR-013: the rows, the cursor and the checks are drawn state); only the answer reaches the
//! app, as the verb it stands for (`Purpose`).
//! Oracle: `bottom_pane/list_selection_view.rs`.

use std::rc::Rc;

use rtwk::focus::{Focus, FocusOnKeyEventCallback};
use rtwk::gestures::Listener;
use rtwk::layout::{Align, Alignment};
use rtwk::navigation::{Navigator, Route, RouteBehavior, RouteSettings};
use rtwk::overlay::{DialogOptions, DialogResult, close_dialog, show_dialog};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::read;
use rtwk::semantics::BlockSemantics;
use rtwk::{AnyWidget, BuildCx, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{AutoReview, Intent as AppIntent, SessionSlice};
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

const PLAN_ROUTE: &str = "/plan-implementation";

struct PlanRoute {
    list: Rc<List>,
    key: FocusOnKeyEventCallback,
}

impl RouteBehavior for PlanRoute {
    fn build(&self, _: &mut BuildCx<'_>) -> AnyWidget {
        let key = self.key.clone();
        let picker = Focus::new(ListSheet {
            list: self.list.clone(),
            purpose: Purpose::PlanImplementation,
        })
        .can_request_focus(false)
        .on_key_event(move |node, press| key(node, press));
        BlockSemantics::new(Align::new(
            Alignment::BOTTOM_CENTER,
            Listener::new()
                .behavior(HitTestBehavior::Opaque)
                .child(picker),
        ))
        .into()
    }

    fn opaque(&self) -> bool {
        false
    }
}

fn is_plan_route(route: &Route) -> bool {
    route.settings().name.as_deref() == Some(PLAN_ROUTE)
}

/// The answer pane owns keys and the space it occupies. The uncovered history takes mouse input.
pub(crate) fn plan_route_thread<S>(cx: &mut Cx<'_, S>) -> Option<ThreadId> {
    let page = Route::maybe_of(cx)?;
    if page.is_current() {
        return None;
    }
    let nav = page.navigator()?;
    nav.routes().last().and_then(plan_thread).cloned()
}

fn plan_thread(route: &Route) -> Option<&ThreadId> {
    if is_plan_route(route) {
        route.settings().arguments()
    } else {
        None
    }
}

fn close(cx: &mut Cx<'_, ListSheetState>) {
    if let Some(route) = Route::maybe_of(cx).filter(is_plan_route)
        && let Some(nav) = route.navigator()
    {
        nav.remove_route(&route);
    } else {
        close_dialog::<_, ()>(cx, None);
    }
}

/// The picker's list vocabulary as the sheet's working copy (the picker brick's own type, not
/// a model of a screen).
type Rows = List;

pub struct ListSheetState {
    /// The rows as the user left them: the toggles flipped, the order moved.
    list: Rows,
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
            if let Some(owner) = Route::maybe_of(cx).as_ref().and_then(plan_thread).cloned()
                && read::<SessionSlice, _>(cx).active.as_ref() != Some(&owner)
            {
                return;
            }
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
                close(cx);
            }
            Event::Secondary(id) => {
                if self.list.find(&id).is_some_and(|item| !item.is_disabled()) {
                    answer(cx, &w.purpose, &id);
                    close(cx);
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
                    close(cx);
                }
            }
            Event::Cancel => close(cx),
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

pub(crate) fn pick_plan<S>(
    cx: &mut Cx<'_, S>,
    thread: ThreadId,
    list: List,
    key: FocusOnKeyEventCallback,
) -> DialogResult<()> {
    Navigator::push(
        cx,
        Route::with_settings(
            PlanRoute {
                list: Rc::new(list),
                key,
            },
            RouteSettings::named(PLAN_ROUTE).with_arguments(thread),
        ),
    )
}
