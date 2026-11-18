//! `status`: the slot of S19 (/status card), the `mock-dialogs` task's to fill. It holds
//! the empty form the skeleton registers: a screen widget, its opener, its
//! sub-enums of the root event and intent enums, its model part, and the
//! debug-menu entries. Nothing outside this directory (bar its test file)
//! needs an edit when it grows.

use rtwk::overlay::{Dialog, DialogOptions, DialogResult, show_dialog};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use crate::app::AppFx;
use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::Kind;

pub const NAME: &str = "status";
pub const ID: &str = "S19";
pub const KIND: Kind = Kind::Dialog;
pub const TITLE: &str = "/status card";

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug)]
pub enum Ev {}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug)]
pub enum Intent {}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = (model, fx);
    match ev {}
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {}
}

/// What the debug menu lists for this directory: nothing yet. A slice adds
/// one entry per state worth opening (a screen on made-up data).
pub fn debug_entries() -> Vec<DebugEntry> {
    Vec::new()
}

/// Opens the dialog from the widget that asks (`show_dialog`); the result
/// returns to the model through a handler, as an intent with its ticket.
pub fn open<S>(cx: &Cx<'_, S>) -> DialogResult<()> {
    show_dialog(
        cx,
        |_| Dialog::new(Screen).title(TITLE).into(),
        DialogOptions::default(),
    )
}

/// Opens the empty form of this slot (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|| Dialog::new(Screen).title(TITLE).into())
}

/// The empty screen widget of this slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: ids, drafts and controllers only (R14).
#[derive(Debug)]
pub struct ScreenState;

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn build(&self, _: &Screen, _cx: &mut Cx<'_, Self>) -> AnyWidget {
        Text::new(format!("{NAME} ({ID}): empty slot")).into()
    }
}
