//! The `/subagents` picker (oracle `app/session_lifecycle.rs` `agent_picker_selection_view_params`,
//! `app/agent_navigation.rs`): one row per thread of the conversation's agent tree, the thread in
//! use marked. In Flutter it is a `showModalBottomSheet` over a list whose rows are read from the
//! app (the agents screen's threads, the requests waiting, the thread in use); a pick is the
//! app's one door to switch a thread (`Hs::open_thread`) and closes the sheet.

use std::rc::Rc;

use rtwk::overlay::{close_dialog, show_dialog};
use rtwk::provider::{read, select};
use rtwk::{AnyWidget, Cx, Handler, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::app::{RequestQueue, SessionSlice};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::approvals::Prompt;
use codex_tui_core::screens::select::agent_picker::{Facts, Row};

use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};

mod strings;

/// The picker over the agent threads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentPicker;

pub struct AgentPickerState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for AgentPicker {
    type State = AgentPickerState;

    fn create_state(&self) -> AgentPickerState {
        AgentPickerState
    }
}

impl_widget!(stateful eq AgentPicker);

/// The rows now: the agents screen's threads, the thread in use, the requests waiting.
fn current<S: 'static>(cx: &mut Cx<'_, S>, watching: bool) -> (Vec<Row>, Option<ThreadId>) {
    let (facts, active, waiting) = if watching {
        (
            select::<ScreenStates, Facts, _>(cx, Facts::of),
            select::<SessionSlice, Option<ThreadId>, _>(cx, |session| session.active.clone()),
            select::<RequestQueue, Vec<ThreadId>, _>(cx, waiting_of),
        )
    } else {
        (
            Facts::of(&read::<ScreenStates, _>(cx)),
            read::<SessionSlice, _>(cx).active.clone(),
            waiting_of(&read::<RequestQueue, _>(cx)),
        )
    };
    // A thread the server has not loaded and nothing in the app runs is closed: the app's own
    // threads are not read here, the server's status stands for them.
    let list = facts.rows(active.as_ref(), &waiting, |_| false);
    (list, active)
}

/// The threads with an approval waiting (a question for the user is not one).
fn waiting_of(queue: &RequestQueue) -> Vec<ThreadId> {
    queue
        .entries
        .iter()
        .filter(|entry| !matches!(entry.prompt.as_deref(), Some(Prompt::UserInput(_))))
        .map(|entry| entry.key.thread.clone())
        .collect()
}

/// Opens the picker over the chat of `cx`.
pub fn open<S: 'static>(cx: &mut Cx<'_, S>) {
    if current(cx, false).0.is_empty() {
        return;
    }
    drop(show_dialog::<(), S>(
        cx,
        |_| AgentPicker.into(),
        crate::sheets::sheet_options(),
    ));
}

impl State for AgentPickerState {
    type Widget = AgentPicker;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, _w: &AgentPicker, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.open_thread, ThreadId::new(id));
                close_dialog::<_, ()>(cx, None);
            }
            Event::Cancel => close_dialog::<_, ()>(cx, None),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, _w: &AgentPicker, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let (list, active) = current(cx, true);
        let on: Handler<Event> = cx.map(Msg::Picked);
        let initial = list
            .iter()
            .position(|entry| entry.needs_approval)
            .or_else(|| {
                list.iter()
                    .position(|entry| Some(&entry.thread) == active.as_ref())
            });
        Picker::new(Rc::new(strings::list(&list, initial)), on).into()
    }
}
