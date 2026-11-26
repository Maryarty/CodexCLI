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
use codex_tui_core::screens::approvals::Prompt;
use codex_tui_core::screens::select::agent_picker::{Facts, PickerKey, Row, has_subagents};
use codex_tui_core::screens::{ScreenIntent, ScreenStates, agents};

use crate::app::view::Hs;
use crate::widgets::picker::{Event, Picker};

mod strings;

/// The picker over the agent threads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentPicker {
    source: ThreadId,
    root: ThreadId,
    key: PickerKey,
}

pub struct AgentPickerState {
    reported: bool,
    closed: bool,
    mounted: Option<StopReading>,
}

type StopReading = Box<dyn FnOnce(&mut Cx<'_, AgentPickerState>)>;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Picked(Event),
    Read,
    RootChanged,
}

impl StatefulWidget for AgentPicker {
    type State = AgentPickerState;

    fn create_state(&self) -> AgentPickerState {
        AgentPickerState {
            reported: false,
            closed: false,
            mounted: None,
        }
    }
}

impl_widget!(stateful eq AgentPicker);

/// The rows now: the agents screen's threads, the thread in use, the requests waiting.
fn current<S: 'static>(cx: &mut Cx<'_, S>) -> (Vec<Row>, Option<ThreadId>) {
    let facts = select::<ScreenStates, Facts, _>(cx, Facts::of);
    let active = select::<SessionSlice, Option<ThreadId>, _>(cx, |session| session.active.clone());
    let waiting = select::<RequestQueue, Vec<ThreadId>, _>(cx, waiting_of);
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
    let active = read::<SessionSlice, _>(cx).active.clone();
    let states = read::<ScreenStates, _>(cx);
    let root = active
        .as_ref()
        .and_then(|active| states.agents.descendants_root(active));
    let (Some(active), Some(root)) = (active.clone(), root) else {
        crate::features::report_no_subagents(cx, active);
        return;
    };
    let picker = AgentPicker {
        root,
        source: active,
        key: PickerKey::default(),
    };
    drop(show_dialog::<(), S>(
        cx,
        move |_| picker.clone().into(),
        crate::sheets::sheet_options(),
    ));
}

impl State for AgentPickerState {
    type Widget = AgentPicker;
    type Msg = Msg;

    fn did_change_dependencies(&mut self, w: &AgentPicker, cx: &mut Cx<'_, Self>) {
        if self.closed {
            return;
        }
        if self.mounted.is_none() {
            let hs = read::<Hs, _>(cx);
            let door = hs.screen.clone();
            let stop = ScreenIntent::Agents(agents::Intent::StopReadingDescendants(w.key.clone()));
            self.mounted = Some(Box::new(move |cx| cx.emit(&door, stop)));
            cx.emit(
                &hs.screen,
                ScreenIntent::Agents(agents::Intent::ReadDescendants {
                    active: w.source.clone(),
                    picker: w.key.clone(),
                }),
            );
            report(cx, w);
        }
    }

    fn dispose_with(&mut self, cx: &mut Cx<'_, Self>) {
        self.stop_reading(cx);
    }

    fn update(&mut self, msg: Msg, w: &AgentPicker, cx: &mut Cx<'_, Self>) -> Rebuild {
        if self.closed {
            return Rebuild::No;
        }
        if !matches!(msg, Msg::Picked(Event::Cancel) | Msg::RootChanged) {
            let active = read::<SessionSlice, _>(cx).active.clone();
            let states = read::<ScreenStates, _>(cx);
            if active
                .as_ref()
                .and_then(|active| states.agents.descendants_root(active))
                .as_ref()
                != Some(&w.root)
            {
                self.dismiss(cx);
                return Rebuild::No;
            }
        }
        match msg {
            Msg::Picked(Event::Choose(id)) => {
                let hs = read::<Hs, _>(cx);
                cx.emit(
                    &hs.screen,
                    ScreenIntent::Agents(agents::Intent::ChooseDescendant {
                        picker: w.key.clone(),
                        thread: ThreadId::new(id),
                    }),
                );
                self.dismiss(cx);
            }
            Msg::Picked(Event::Cancel) | Msg::RootChanged => self.dismiss(cx),
            Msg::Read if !self.reported => {
                let states = read::<ScreenStates, _>(cx);
                if !states.agents.descendants_read(&w.key) {
                    return Rebuild::No;
                }
                self.reported = true;
                report(cx, w);
                let active = read::<SessionSlice, _>(cx).active.clone();
                let has_children =
                    has_subagents(&Facts::of(&states).rows(active.as_ref(), &[], |_| false));
                if !states.chat.status.collab_enabled && !has_children {
                    let hs = read::<Hs, _>(cx);
                    self.dismiss(cx);
                    crate::features::offer_enable_subagents(cx, &hs);
                }
            }
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &AgentPicker, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let (list, active) = current(cx);
        let source = active.clone();
        let key = w.key.clone();
        let (root, finished) =
            select::<ScreenStates, (Option<ThreadId>, bool), _>(cx, move |states| {
                (
                    source
                        .as_ref()
                        .and_then(|thread| states.agents.descendants_root(thread)),
                    states.agents.descendants_read(&key),
                )
            });
        if !self.closed {
            if root.as_ref() != Some(&w.root) {
                cx.local_sender()(Msg::RootChanged);
            } else if finished && !self.reported {
                cx.local_sender()(Msg::Read);
            }
        }
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

fn report(cx: &mut Cx<'_, AgentPickerState>, picker: &AgentPicker) {
    crate::features::report_subagents(cx, picker.source.clone(), picker.key.clone());
}

impl AgentPickerState {
    fn dismiss(&mut self, cx: &mut Cx<'_, Self>) {
        self.closed = true;
        self.stop_reading(cx);
        close_dialog::<_, ()>(cx, None);
    }

    fn stop_reading(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(stop) = self.mounted.take() {
            stop(cx);
        }
    }
}
