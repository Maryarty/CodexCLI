//! The sheet of a key the keymap cannot take (oracle `build_keymap_conflict_params`), and the
//! capture that "Pick another key" returns to. The feature asks for the first as a wish
//! (`Wish::KeymapConflict`) and the presenter shows it; the capture says its verb itself and
//! closes (there is no picker page below it to return to).

use std::rc::Rc;

use rtwk::overlay::{close_dialog, show_dialog};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::settings::Intent;
use codex_tui_core::features::settings::capture::Intent as EditIntent;
use codex_tui_core::features::settings::keymap;

use super::capture::{Capture, Done, Mode};
use super::{lists, strings};
use crate::features::verb;
use crate::keymap::keymap as live_keymap;
use crate::widgets::picker::{Event, Picker};

/// The conflict sheet: the action, the key that did not fit, the resolver's words.
#[derive(Clone, Debug, PartialEq)]
pub struct ConflictSheet {
    pub context: String,
    pub action: String,
    pub key: String,
    pub intent: EditIntent,
    pub error: String,
}

pub struct ConflictState;

#[derive(Clone, Debug)]
pub enum Msg {
    Picked(Event),
}

impl StatefulWidget for ConflictSheet {
    type State = ConflictState;

    fn create_state(&self) -> ConflictState {
        ConflictState
    }
}

impl_widget!(stateful eq ConflictSheet);

impl State for ConflictState {
    type Widget = ConflictSheet;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &ConflictSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Picked(event) = msg;
        match event {
            Event::Choose(id) if id == lists::AGAIN => {
                close_dialog::<_, ()>(cx, None);
                let sheet = CaptureSheet {
                    context: w.context.clone(),
                    action: w.action.clone(),
                    intent: w.intent.clone(),
                    mode: Mode::of_key(&w.key),
                };
                drop(show_dialog::<(), Self>(
                    cx,
                    move |_| sheet.clone().into(),
                    crate::sheets::sheet_options(),
                ));
            }
            Event::Choose(_) | Event::Cancel => close_dialog::<_, ()>(cx, None),
            _ => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &ConflictSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let list = lists::conflict(&w.context, &w.action, &w.key, &w.error, strings::of(cx));
        Picker::new(Rc::new(list), cx.map(Msg::Picked)).into()
    }
}

/// The capture of a key for an action, as a sheet of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptureSheet {
    pub context: String,
    pub action: String,
    pub intent: EditIntent,
    pub mode: Mode,
}

pub struct CaptureSheetState;

#[derive(Clone, Debug)]
pub enum CaptureMsg {
    Done(Done),
}

impl StatefulWidget for CaptureSheet {
    type State = CaptureSheetState;

    fn create_state(&self) -> CaptureSheetState {
        CaptureSheetState
    }
}

impl_widget!(stateful eq CaptureSheet);

impl State for CaptureSheetState {
    type Widget = CaptureSheet;
    type Msg = CaptureMsg;

    fn update(&mut self, msg: CaptureMsg, w: &CaptureSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let CaptureMsg::Done(done) = msg;
        if let Done::Key(key) = done {
            verb(
                cx,
                Intent::KeymapCaptured {
                    context: w.context.clone(),
                    action: w.action.clone(),
                    key,
                    intent: w.intent.clone(),
                },
            );
        }
        close_dialog::<_, ()>(cx, None);
        Rebuild::No
    }

    fn build(&self, w: &CaptureSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let live = live_keymap(cx);
        Capture {
            context: w.context.clone(),
            action: w.action.clone(),
            current: keymap::summary(&live, &w.context, &w.action),
            mode: w.mode,
            on_done: cx.map(CaptureMsg::Done),
        }
        .into()
    }
}
